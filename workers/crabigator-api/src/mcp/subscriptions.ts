import type { Env } from '../types/env';
import type { McpAuth } from './session';
import {
    CallbackEndpointError,
    McpEventError,
    eventByName,
    eventMatches,
    subscriptionId,
    validateArguments,
} from './events';
import {
    canonicalCallbackUrl,
    decodeWhsec,
    postWebhook,
    signedWebhookHeaders,
    statusReason,
    webhookBodyTooLarge,
} from './webhook';

const DEFAULT_TTL_MS = 24 * 60 * 60 * 1000;
const MIN_TTL_MS = 5 * 60 * 1000;
const MAX_TTL_MS = 7 * 24 * 60 * 60 * 1000;
const MAX_SUBSCRIPTIONS = 100;
const VERIFICATION_TTL_MS = 60 * 60 * 1000;
const SECRET_ROTATION_MS = 5 * 60 * 1000;
const RETRY_DELAYS_MS = [0, 250, 1000];

export interface McpEventOccurrence {
    eventId: string;
    name: string;
    timestamp: string;
    data: Record<string, unknown>;
    cursor: null;
}

interface SubscriptionRow {
    id: string;
    principal: string;
    account_id: string | null;
    group_id: string;
    event_name: string;
    arguments_json: string;
    callback_url: string;
    secret: string;
    previous_secret: string | null;
    previous_secret_until: number | null;
    expires_at: number;
}

interface SubscribeResult {
    id: string;
    refreshBefore: string;
    cursor: null;
    truncated: boolean;
}

export function principalOf(auth: McpAuth): string {
    return auth.account_id ? `account:${auth.account_id}` : `group:${auth.group_id}`;
}

export async function subscribeToEvent(
    auth: McpAuth,
    env: Env,
    params: Record<string, unknown>,
): Promise<SubscribeResult> {
    const name = typeof params.name === 'string' ? params.name : '';
    const event = eventByName(name);
    if (!event) {
        throw new McpEventError(-32011, `Unknown event: ${name || '(missing)'}`, { kind: 'event' });
    }
    const delivery = deliveryOf(params.delivery);
    if (delivery.mode !== 'webhook') {
        throw new McpEventError(-32014, 'Only webhook delivery is supported', {
            feature: 'deliveryMode',
            value: delivery.mode || '',
        });
    }
    const callbackUrl = canonicalCallbackUrl(delivery.url);
    if (!decodeWhsec(delivery.secret)) {
        throw new McpEventError(-32602, 'delivery.secret must be a whsec_ key of 24 to 64 bytes');
    }
    const args = validateArguments(event, params.arguments);
    const principal = principalOf(auth);
    const id = await subscriptionId(principal, callbackUrl, name, args);
    const now = Date.now();
    const { expiresAt, refreshBefore } = grantTtl(params.ttlMs, now);

    await env.DB.prepare(
        'DELETE FROM mcp_event_subscriptions WHERE principal = ? AND expires_at <= ?',
    ).bind(principal, now).run();

    const existing = await env.DB.prepare(
        `SELECT id, secret, previous_secret, previous_secret_until
         FROM mcp_event_subscriptions WHERE id = ? AND principal = ?`,
    ).bind(id, principal).first<{
        id: string;
        secret: string;
        previous_secret: string | null;
        previous_secret_until: number | null;
    }>();

    if (!existing) {
        const count = await env.DB.prepare(
            'SELECT COUNT(*) AS n FROM mcp_event_subscriptions WHERE principal = ? AND expires_at > ?',
        ).bind(principal, now).first<{ n: number }>();
        if ((count?.n || 0) >= MAX_SUBSCRIPTIONS) {
            throw new McpEventError(-32013, 'Too many event subscriptions', {
                limit: 'subscriptions',
                max: MAX_SUBSCRIPTIONS,
            });
        }
    }

    const secretChanged = Boolean(existing && existing.secret !== delivery.secret);
    await verifyCallback(env, principal, callbackUrl, delivery.secret, id, secretChanged);

    let previousSecret: string | null = null;
    let previousUntil: number | null = null;
    if (existing && existing.secret !== delivery.secret) {
        previousSecret = existing.secret;
        previousUntil = now + SECRET_ROTATION_MS;
    } else if (
        existing?.previous_secret
        && existing.previous_secret_until
        && existing.previous_secret_until > now
    ) {
        previousSecret = existing.previous_secret;
        previousUntil = existing.previous_secret_until;
    }

    await env.DB.prepare(
        `INSERT INTO mcp_event_subscriptions (
            id, principal, account_id, group_id, event_name, arguments_json,
            callback_url, secret, previous_secret, previous_secret_until,
            expires_at, created_at, updated_at
         ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(id) DO UPDATE SET
            secret = excluded.secret,
            previous_secret = excluded.previous_secret,
            previous_secret_until = excluded.previous_secret_until,
            arguments_json = excluded.arguments_json,
            callback_url = excluded.callback_url,
            expires_at = excluded.expires_at,
            updated_at = excluded.updated_at`,
    ).bind(
        id,
        principal,
        auth.account_id || null,
        auth.group_id,
        name,
        JSON.stringify(args),
        callbackUrl,
        delivery.secret,
        previousSecret,
        previousUntil,
        expiresAt,
        now,
        now,
    ).run();

    const askedForReplay = params.cursor != null && params.cursor !== '';
    return {
        id,
        refreshBefore,
        cursor: null,
        truncated: askedForReplay,
    };
}

export async function unsubscribeFromEvent(
    auth: McpAuth,
    env: Env,
    params: Record<string, unknown>,
): Promise<void> {
    const name = typeof params.name === 'string' ? params.name : '';
    const event = eventByName(name);
    if (!event) return;
    const delivery = deliveryOf(params.delivery);
    if (delivery.mode && delivery.mode !== 'webhook') {
        throw new McpEventError(-32014, 'Only webhook delivery is supported', {
            feature: 'deliveryMode',
            value: delivery.mode,
        });
    }
    const callbackUrl = canonicalCallbackUrl(delivery.url);
    const args = validateArguments(event, params.arguments);
    const id = await subscriptionId(principalOf(auth), callbackUrl, name, args);
    await env.DB.prepare(
        'DELETE FROM mcp_event_subscriptions WHERE id = ? AND principal = ?',
    ).bind(id, principalOf(auth)).run();
}

export async function deliverMcpEvent(
    env: Env,
    groupId: string,
    occurrence: McpEventOccurrence,
): Promise<void> {
    const now = Date.now();
    await env.DB.prepare(
        'DELETE FROM mcp_event_subscriptions WHERE group_id = ? AND expires_at <= ?',
    ).bind(groupId, now).run();

    const rows = await env.DB.prepare(
        `SELECT id, principal, account_id, group_id, event_name, arguments_json,
                callback_url, secret, previous_secret, previous_secret_until, expires_at
         FROM mcp_event_subscriptions
         WHERE group_id = ? AND event_name = ? AND expires_at > ?`,
    ).bind(groupId, occurrence.name, now).all<SubscriptionRow>();

    for (const row of rows.results || []) {
        try {
            await deliverOne(env, row, occurrence);
        } catch (error) {
            console.error('MCP event delivery', { subscription: row.id, error });
        }
    }
}

async function deliverOne(env: Env, row: SubscriptionRow, occurrence: McpEventOccurrence): Promise<void> {
    if (!(await accountStillInGroup(env, row.account_id, row.group_id))) {
        await env.DB.prepare('DELETE FROM mcp_event_subscriptions WHERE id = ?').bind(row.id).run();
        return;
    }
    let args: Record<string, unknown> = {};
    try {
        const parsed: unknown = JSON.parse(row.arguments_json);
        if (parsed && typeof parsed === 'object' && !Array.isArray(parsed)) {
            args = parsed as Record<string, unknown>;
        }
    } catch {
        args = {};
    }
    if (!eventMatches(args, occurrence.data)) return;

    const body = JSON.stringify(occurrence);
    if (webhookBodyTooLarge(body)) {
        console.error('MCP event payload exceeds 256 KiB', occurrence.eventId);
        return;
    }
    const secrets = [row.secret];
    if (row.previous_secret && row.previous_secret_until && row.previous_secret_until > Date.now()) {
        secrets.push(row.previous_secret);
    }

    for (let attempt = 0; attempt < RETRY_DELAYS_MS.length; attempt++) {
        if (RETRY_DELAYS_MS[attempt] > 0) {
            await sleep(RETRY_DELAYS_MS[attempt]);
        }
        // A fresh timestamp and signature on every attempt. The event id stays put.
        const headers = await signedWebhookHeaders(occurrence.eventId, body, secrets, row.id);
        let response: { status: number; body: string };
        try {
            response = await postWebhook({ url: row.callback_url, body, headers });
        } catch (error) {
            if (attempt === RETRY_DELAYS_MS.length - 1) throw error;
            continue;
        }
        if (response.status >= 200 && response.status < 300) return;
        if (response.status === 410) {
            await env.DB.prepare('DELETE FROM mcp_event_subscriptions WHERE id = ?').bind(row.id).run();
            return;
        }
        if (response.status === 413 || (response.status >= 400 && response.status < 500 && response.status !== 408 && response.status !== 429)) {
            return;
        }
    }
}

async function accountStillInGroup(
    env: Env,
    accountId: string | null,
    groupId: string,
): Promise<boolean> {
    if (!accountId) return true;
    const row = await env.DB.prepare(
        'SELECT group_id FROM accounts WHERE id = ?',
    ).bind(accountId).first<{ group_id: string | null }>();
    return row?.group_id === groupId;
}

async function verifyCallback(
    env: Env,
    principal: string,
    callbackUrl: string,
    secret: string,
    subscription: string,
    secretChanged: boolean,
): Promise<void> {
    const now = Date.now();
    await env.DB.prepare(
        'DELETE FROM mcp_callback_verifications WHERE principal = ? AND verified_until <= ?',
    ).bind(principal, now).run();
    if (!secretChanged) {
        const cached = await env.DB.prepare(
            `SELECT verified_until FROM mcp_callback_verifications
             WHERE principal = ? AND callback_url = ?`,
        ).bind(principal, callbackUrl).first<{ verified_until: number }>();
        if (cached && cached.verified_until > now) return;
    }

    const challenge = crypto.randomUUID();
    const webhookId = `msg_verification_${crypto.randomUUID().replace(/-/g, '')}`;
    const body = JSON.stringify({ type: 'verification', challenge });
    const headers = await signedWebhookHeaders(webhookId, body, [secret], subscription);
    let response: { status: number; body: string };
    try {
        response = await postWebhook({ url: callbackUrl, body, headers });
    } catch (error) {
        if (error instanceof CallbackEndpointError || error instanceof McpEventError) throw error;
        throw new CallbackEndpointError('connection_refused', 'Callback address is not reachable');
    }
    const reason = statusReason(response.status);
    if (reason) {
        throw new CallbackEndpointError(
            reason,
            reason === 'http_5xx'
                ? 'Callback endpoint failed verification'
                : reason === 'http_4xx'
                    ? 'Callback endpoint rejected verification'
                    : 'Callback address is not reachable',
        );
    }
    let echoed = '';
    try {
        const parsed = JSON.parse(response.body) as { challenge?: unknown };
        if (typeof parsed.challenge === 'string') echoed = parsed.challenge;
    } catch {
        echoed = '';
    }
    if (!timingSafeEqual(echoed, challenge)) {
        throw new CallbackEndpointError('challenge_failed', 'Callback verification did not echo the challenge');
    }
    await env.DB.prepare(
        `INSERT INTO mcp_callback_verifications (principal, callback_url, verified_until)
         VALUES (?, ?, ?)
         ON CONFLICT(principal, callback_url) DO UPDATE SET verified_until = excluded.verified_until`,
    ).bind(principal, callbackUrl, now + VERIFICATION_TTL_MS).run();
}

function deliveryOf(raw: unknown): { mode: string; url: string; secret: string } {
    if (!raw || typeof raw !== 'object' || Array.isArray(raw)) {
        throw new McpEventError(-32602, 'delivery must be an object');
    }
    const delivery = raw as Record<string, unknown>;
    return {
        mode: typeof delivery.mode === 'string' ? delivery.mode : '',
        url: typeof delivery.url === 'string' ? delivery.url : '',
        secret: typeof delivery.secret === 'string' ? delivery.secret : '',
    };
}

function grantTtl(requested: unknown, now: number): { expiresAt: number; refreshBefore: string } {
    let ms = DEFAULT_TTL_MS;
    if (requested === null) {
        ms = MAX_TTL_MS;
    } else if (typeof requested === 'number' && Number.isFinite(requested)) {
        ms = Math.min(Math.max(Math.floor(requested), MIN_TTL_MS), MAX_TTL_MS);
    } else if (requested !== undefined) {
        throw new McpEventError(-32602, 'ttlMs must be a number of milliseconds or null');
    }
    const expiresAt = now + ms;
    return { expiresAt, refreshBefore: new Date(expiresAt).toISOString() };
}

function timingSafeEqual(left: string, right: string): boolean {
    const encoder = new TextEncoder();
    const a = encoder.encode(left);
    const b = encoder.encode(right);
    const length = Math.max(a.length, b.length);
    let diff = a.length ^ b.length;
    for (let index = 0; index < length; index++) {
        diff |= (a[index] || 0) ^ (b[index] || 0);
    }
    return diff === 0;
}

function sleep(ms: number): Promise<void> {
    return new Promise((resolve) => setTimeout(resolve, ms));
}
