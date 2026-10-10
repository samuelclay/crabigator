import type { Env } from './types/env';

interface ServiceAccount { project_id: string; client_email: string; private_key: string }
let cached: { email: string; token: string; expires: number } | undefined;
const encode = (bytes: Uint8Array) => btoa(String.fromCharCode(...bytes)).replace(/=/g, '').replace(/\+/g, '-').replace(/\//g, '_');
const json64 = (value: unknown) => encode(new TextEncoder().encode(JSON.stringify(value)));
// One shared deadline covers authentication and all deliveries inside waitUntil.
// A short timeout per attempt leaves room to recover from a dropped connection.
async function fetchWithRetry(url: string, init: RequestInit, deadline: number): Promise<Response> {
    for (let attempt = 0; ; attempt++) {
        const remaining = deadline - Date.now();
        if (remaining <= 0) throw new Error('FCM delivery timed out');
        let response: Response | undefined;
        let networkError: unknown;
        try {
            response = await fetch(url, { ...init, signal: AbortSignal.timeout(Math.min(5000, remaining)) });
            if (response.status !== 429 && response.status < 500) return response;
        } catch (error) {
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
    if (!env.FIREBASE_SERVICE_ACCOUNT) return;
    const deadline = Date.now() + 25000;
    const rows = await env.DB.prepare(`SELECT DISTINCT p.mobile_id, p.token, p.token_hash FROM mobile_push_tokens p
        JOIN linked_devices l ON l.mobile_id = p.mobile_id AND l.mobile_token_hash = p.token_hash
        JOIN devices d ON d.id = l.desktop_id
        WHERE d.group_id = ? AND l.revoked_at IS NULL`).bind(groupId).all<{ mobile_id: string; token: string; token_hash: string }>();
    if (!rows.results.length) return;
    const account = JSON.parse(env.FIREBASE_SERVICE_ACCOUNT) as ServiceAccount;
    const bearer = await accessToken(account, deadline);
    const deliveries = await Promise.allSettled(rows.results.map(async row => {
        // Revoked/expired credentials must stop background delivery as well as API access.
        if (!await env.TOKENS.get(`mobile:${row.token_hash}`)) return;
        const response = await fetchWithRetry(`https://fcm.googleapis.com/v1/projects/${account.project_id}/messages:send`, {
            method: 'POST', headers: { Authorization: `Bearer ${bearer}`, 'Content-Type': 'application/json' },
            body: JSON.stringify({ message: { token: row.token, data: { session_id: sessionId },
                android: { priority: 'high', ttl: '300s' } } }),
        }, deadline);
        if (!response.ok) {
            const result = await response.json() as { error?: { details?: { errorCode?: string }[] } };
            if (result.error?.details?.some(d => d.errorCode === 'UNREGISTERED')) {
                await env.DB.prepare('DELETE FROM mobile_push_tokens WHERE mobile_id = ? AND token = ?').bind(row.mobile_id, row.token).run();
            } else throw new Error(`FCM delivery failed (${response.status})`);
        }
    }));
    const failures = deliveries.filter((result): result is PromiseRejectedResult => result.status === 'rejected');
    if (failures.length) throw new AggregateError(failures.map(result => result.reason), 'Mobile notification delivery failed');
}
