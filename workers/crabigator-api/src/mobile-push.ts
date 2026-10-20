import { auditedPushFetch, PushOutcomeUnknownError, type PushFetch } from './notification-history';
import { sendApplePush } from './apple-push';
import type { Env } from './types/env';

interface ServiceAccount { project_id: string; client_email: string; private_key: string }
let cached: { email: string; token: string; expires: number } | undefined;
const encode = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes)).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
const json64 = (value: unknown) => encode(new TextEncoder().encode(JSON.stringify(value)));
// One shared deadline covers authentication and all deliveries inside waitUntil.
// A short timeout per attempt leaves room to recover from a dropped connection.
async function fetchWithRetry(url: string, init: RequestInit, deadline: number, send: PushFetch = fetch): Promise<Response> {
    for (let attempt = 0; ; attempt++) {
        const remaining = deadline - Date.now();
        if (remaining <= 0) throw new Error('FCM delivery timed out');
        let response: Response | undefined;
        let networkError: unknown;
        try {
            response = await send(url, { ...init, signal: AbortSignal.timeout(Math.min(5000, remaining)) });
            if (response.status !== 429 && response.status < 500) return response;
        } catch (error) {
            if (error instanceof PushOutcomeUnknownError) throw error;
            networkError = error;
        }
        const retryAfter = response?.headers.get('Retry-After');
        const retryAfterMs = retryAfter
            ? (/^\d+(\.\d+)?$/.test(retryAfter) ? Number(retryAfter) * 1000 : Date.parse(retryAfter) - Date.now())
            : 0;
        // FCM asks clients to back off at least a minute for quota errors.
        // Those need a later reconciliation, beyond this Worker's lifetime.
        const delay = Math.max(1000 * 2 ** attempt, response?.status === 429 ? 60000 : 0, retryAfterMs || 0);
        if (attempt >= 2 || Date.now() + delay >= deadline) {
            if (response) return response;
            throw networkError;
        }
        await response?.body?.cancel();
        await new Promise(resolve => setTimeout(resolve, delay));
    }
}

async function accessToken(account: ServiceAccount, deadline: number): Promise<string> {
    if (cached?.email === account.client_email && cached.expires > Date.now() + 60000) return cached.token;
    const now = Math.floor(Date.now() / 1000);
    const payload = `${json64({ alg: 'RS256', typ: 'JWT' })}.${json64({ iss: account.client_email,
        scope: 'https://www.googleapis.com/auth/firebase.messaging', aud: 'https://oauth2.googleapis.com/token', iat: now, exp: now + 3600 })}`;
    const der = Uint8Array.from(atob(account.private_key.replace(/-----[^-]+-----|\s/g, '')), c => c.charCodeAt(0));
    const key = await crypto.subtle.importKey('pkcs8', der, { name: 'RSASSA-PKCS1-v1_5', hash: 'SHA-256' }, false, ['sign']);
    const signature = new Uint8Array(await crypto.subtle.sign('RSASSA-PKCS1-v1_5', key, new TextEncoder().encode(payload)));
    const response = await fetchWithRetry('https://oauth2.googleapis.com/token', { method: 'POST',
        body: new URLSearchParams({ grant_type: 'urn:ietf:params:oauth:grant-type:jwt-bearer', assertion: `${payload}.${encode(signature)}` }) }, deadline);
    if (!response.ok) throw new Error(`FCM authentication failed (${response.status})`);
    const result = await response.json() as { access_token: string; expires_in: number };
    cached = { email: account.client_email, token: result.access_token, expires: Date.now() + result.expires_in * 1000 };
    return result.access_token;
}

/** Push carries only a session id. Devices authenticate and fetch the latest prompt,
 * so delayed/out-of-order pushes cannot revive answered questions or leak their text. */
export async function notifyMobilePrompt(env: Env, groupId: string, sessionId: string): Promise<void> {
    if (!env.FIREBASE_SERVICE_ACCOUNT && !env.APNS_CREDENTIALS) return;
    const deadline = Date.now() + 25000;
    const rows = await env.DB.prepare(`SELECT DISTINCT p.mobile_id, p.token, p.token_hash, p.platform, p.environment, p.sound FROM mobile_push_tokens p
        JOIN linked_devices l ON l.mobile_id = p.mobile_id AND l.mobile_token_hash = p.token_hash
        JOIN devices d ON d.id = l.desktop_id
        WHERE d.group_id = ? AND l.revoked_at IS NULL`).bind(groupId).all<{ mobile_id: string; token: string; token_hash: string; platform: string; environment: string; sound: number }>();
    if (!rows.results.length) return;
    // Cache each platform's preparation independently. A provider outage must not
    // prevent the other platform from receiving its notifications.
    let firebase: Promise<{ account: ServiceAccount; bearer: string }> | undefined;
    const prepareFirebase = () => firebase ??= (async () => {
        const account = JSON.parse(env.FIREBASE_SERVICE_ACCOUNT!) as ServiceAccount;
        return { account, bearer: await accessToken(account, deadline) };
    })();
    const stub = env.SESSION.get(env.SESSION.idFromName(sessionId));
    const readSnapshot = async () => {
        const response = await stub.fetch('https://internal/snapshot');
        if (!response.ok) throw new Error('Could not read notification state');
        return response.json<{ attention_pending: boolean; desktop_connected: boolean;
            notification_id: string; notification_prompt: string | null; prompt_revision: number; title: string | null }>();
    };
    const snapshot = await readSnapshot();
    const pending = Boolean(snapshot.attention_pending && snapshot.desktop_connected);
    const deliveries = await Promise.allSettled(rows.results.map(async row => {
        // Revoked/expired credentials must stop background delivery as well as API access.
        if (!await env.TOKENS.get(`mobile:${row.token_hash}`)) return;
        if (row.platform === 'ios' ? !env.APNS_CREDENTIALS : !env.FIREBASE_SERVICE_ACCOUNT) return;
        // Apple edits must not make another banner. Android data pushes refresh its
        // existing panel, so those follow the answer revision instead.
        const kind = row.platform === 'ios' && pending ? 'alert' : 'refresh';
        const key = kind === 'alert' ? snapshot.notification_id
            : `${snapshot.prompt_revision}:${pending}:${snapshot.desktop_connected}`;
        const claim = await env.DB.prepare(`INSERT INTO notification_deliveries
            (id, group_id, session_id, mobile_id, platform, environment, notification_key, kind,
             prompt_revision, session_title, prompt_snapshot, created_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            ON CONFLICT(group_id, session_id, mobile_id, notification_key, kind) DO UPDATE
            SET status = 'reserved', completed_at = NULL, prompt_revision = excluded.prompt_revision,
                prompt_snapshot = excluded.prompt_snapshot
            WHERE notification_deliveries.status IN ('superseded', 'failed')
              AND NOT EXISTS (SELECT 1 FROM notification_attempts WHERE delivery_id = notification_deliveries.id)
            RETURNING id`)
            .bind(crypto.randomUUID(), groupId, sessionId, row.mobile_id, row.platform, row.environment, key, kind,
                snapshot.prompt_revision, snapshot.title, snapshot.notification_prompt, Date.now()).first<{ id: string }>();
        if (!claim) return;
        const { id } = claim;
        const send = auditedPushFetch(env, id);
        let superseded = false;
        try {
            const firebaseAuth = row.platform === 'ios' ? undefined : await prepareFirebase();
            // Do not promote a queued resolution into a fresh alert or send a prompt
            // that was answered while credentials and the audit reservation loaded.
            const current = await readSnapshot();
            if (current.notification_id !== snapshot.notification_id
                || Boolean(current.attention_pending && current.desktop_connected) !== pending
                || (kind === 'refresh' && current.prompt_revision !== snapshot.prompt_revision)) {
                superseded = true;
                await env.DB.prepare("UPDATE notification_deliveries SET status = 'superseded', completed_at = ? WHERE id = ?")
                    .bind(Date.now(), id).run();
                return;
            }
            if (row.platform === 'ios') {
                await sendApplePush(env, row, sessionId, pending, send);
            } else {
                const { account, bearer } = firebaseAuth!;
                const response = await fetchWithRetry(`https://fcm.googleapis.com/v1/projects/${account.project_id}/messages:send`, {
                    method: 'POST', headers: { Authorization: `Bearer ${bearer}`, 'Content-Type': 'application/json' },
                    body: JSON.stringify({ message: { token: row.token, data: { session_id: sessionId },
                        android: { priority: 'high', ttl: '300s' } } }),
                }, deadline, send);
                if (!response.ok) {
                    const result = await response.json() as { error?: { details?: { errorCode?: string }[] } };
                    if (result.error?.details?.some(d => d.errorCode === 'UNREGISTERED')) {
                        await env.DB.prepare('DELETE FROM mobile_push_tokens WHERE mobile_id = ? AND token = ?').bind(row.mobile_id, row.token).run();
                    } else throw new Error(`FCM delivery failed (${response.status})`);
                }
            }
        } finally {
            // Provider acceptance is not device receipt. Interrupted sends remain
            // reserved/sending; never silently label them delivered or resend them.
            // A superseded reservation can already belong to a new caller.
            if (!superseded) {
                await env.DB.prepare(`UPDATE notification_deliveries SET status = COALESCE(
                    (SELECT status FROM notification_attempts WHERE delivery_id = ? ORDER BY started_at DESC, rowid DESC LIMIT 1),
                    'failed'), completed_at = ? WHERE id = ?`)
                    .bind(id, Date.now(), id).run();
            }
        }
    }));
    const failures = deliveries.filter((result): result is PromiseRejectedResult => result.status === 'rejected');
    if (failures.length) throw new AggregateError(failures.map(result => result.reason), 'Mobile notification delivery failed');
}
