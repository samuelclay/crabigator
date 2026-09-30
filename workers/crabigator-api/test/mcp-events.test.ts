import { env, SELF } from 'cloudflare:test';
import { afterEach, describe, expect, it } from 'vitest';
import { attachDesktopToAccount, getAccount, mintViewerToken, upsertAccountFromProfile } from '../src/auth/accounts';
import {
    commitEventData,
    eventDefinitions,
    promptEventData,
    recapEventData,
    subscriptionId,
} from '../src/mcp/events';
import { deliverMcpEvent, type McpEventOccurrence } from '../src/mcp/subscriptions';
import {
    canonicalCallbackUrl,
    isPublicAddress,
    postWebhook,
    setWebhookPoster,
    signWebhook,
    type WebhookRequest,
} from '../src/mcp/webhook';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

const testEnv = env as unknown as Env;
const ORIGIN = 'https://self-host.example';

afterEach(() => {
    setWebhookPoster(null);
});

function webhookSecret(byte: number, length = 24): string {
    const bytes = new Uint8Array(length).fill(byte);
    let binary = '';
    bytes.forEach((value) => { binary += String.fromCharCode(value); });
    return `whsec_${btoa(binary)}`;
}

async function linkedAccount(suffix: string): Promise<{ token: string; accountId: string; groupId: string }> {
    await ensureAccountSchema(testEnv);
    const deviceId = `44444444-4444-4444-8444-44444444${suffix}`;
    await testEnv.DB.prepare(`
        INSERT OR IGNORE INTO devices (id, secret_hash, name, created_at, last_seen_at)
        VALUES (?, ?, ?, unixepoch(), unixepoch())
    `).bind(deviceId, 'b'.repeat(64), `MCP Mac ${suffix}`).run();
    const code = `MCP-EVT-${suffix}`;
    await testEnv.TOKENS.put(`pairing_code:${code}`, `mcp-evt-${suffix}`);
    await testEnv.TOKENS.put(`pairing:mcp-evt-${suffix}`, JSON.stringify({
        device_id: deviceId,
        code,
        expires_at: Math.floor(Date.now() / 1000) + 3600,
        claimed: false,
    }));
    const account = await upsertAccountFromProfile(testEnv, {
        provider: 'github',
        provider_user_id: `mcp-evt-${suffix}`,
        email: `mcp-evt-${suffix}@example.com`,
        name: 'MCP',
        username: `mcp-evt-${suffix}`,
    });
    await attachDesktopToAccount(testEnv, account.id, code);
    const linked = await getAccount(testEnv, account.id);
    if (!linked?.group_id) throw new Error('account missing');
    const minted = await mintViewerToken(testEnv, linked, 'MCP events test');
    return { token: minted.token, accountId: linked.id, groupId: linked.group_id };
}

function rpc(token: string, method: string, params?: Record<string, unknown>) {
    return SELF.fetch(`${ORIGIN}/mcp`, {
        method: 'POST',
        headers: {
            'Content-Type': 'application/json',
            Authorization: `Bearer ${token}`,
            'MCP-Protocol-Version': '2026-07-28',
        },
        body: JSON.stringify({ jsonrpc: '2.0', id: 1, method, params }),
    });
}

function echoPoster() {
    const calls: WebhookRequest[] = [];
    setWebhookPoster(async (request) => {
        calls.push(request);
        const body = JSON.parse(request.body) as { type?: string; challenge?: string };
        if (body.type === 'verification') {
            return { status: 200, body: JSON.stringify({ challenge: body.challenge }) };
        }
        return { status: 200, body: '' };
    });
    return calls;
}

function subscribeBody(url: string, secret: string, args: Record<string, unknown> = {}, extra: Record<string, unknown> = {}) {
    return {
        name: 'session.state_changed',
        arguments: args,
        delivery: { mode: 'webhook', url, secret },
        cursor: null,
        ...extra,
    };
}

describe.sequential('MCP events', () => {
    it('signs a Standard Webhooks payload', async () => {
        const signature = await signWebhook(
            'msg_p5jXN8AQM9LWM0D4loKWxJek',
            1614265330,
            JSON.stringify({ test: 2432232314 }),
            ['whsec_MfKQ9r8GKYqrTwjUPD8ILPZIo2LaLaSw'],
        );
        expect(signature).toBe('v1,Vif40peJBP7Iyl0XGmu61n4MwdrcHov5CFREBpE0svs=');
    });

    it('rejects private and local callback addresses', () => {
        expect(isPublicAddress('8.8.8.8')).toBe(true);
        expect(isPublicAddress('1.1.1.1')).toBe(true);
        expect(isPublicAddress('172.32.0.1')).toBe(true);
        expect(isPublicAddress('100.128.0.1')).toBe(true);
        expect(isPublicAddress('2606:4700:4700::1111')).toBe(true);
        expect(isPublicAddress('::ffff:8.8.8.8')).toBe(true);
        for (const address of [
            '0.0.0.0', '10.1.2.3', '127.0.0.1', '169.254.1.1', '172.16.0.1', '172.31.255.1',
            '192.168.1.1', '100.64.0.1', '100.127.255.1', '192.0.2.1', '198.51.100.1',
            '203.0.113.1', '198.18.0.1', '224.0.0.1', '255.255.255.255',
            '::1', 'fe80::1', 'fc00::1', 'fd00::1', '2001:db8::1', '::ffff:127.0.0.1', '[::1]',
        ]) {
            expect(isPublicAddress(address), address).toBe(false);
        }
        expect(() => canonicalCallbackUrl('http://hooks.example/mcp')).toThrow(/https/);
        expect(() => canonicalCallbackUrl('https://user:pw@hooks.example/mcp')).toThrow(/https/);
        expect(() => canonicalCallbackUrl('https://10.1.2.3/hook')).toThrow(/not public/);
        expect(() => canonicalCallbackUrl('https://localhost/hook')).toThrow(/not public/);
        expect(() => canonicalCallbackUrl('https://metadata.google.internal/hook')).toThrow(/not public/);
    });

    it('posts the original URL after DNS says the host is public', async () => {
        const original = globalThis.fetch;
        const calls: string[] = [];
        const install = (answers: Record<string, string | null>) => {
            globalThis.fetch = (async (input: RequestInfo | URL, init?: RequestInit) => {
                const url = String(input);
                calls.push(url);
                if (url.startsWith('https://cloudflare-dns.com/dns-query')) {
                    const type = new URL(url).searchParams.get('type');
                    const data = answers[type || ''] ?? null;
                    return Response.json(data
                        ? { Status: 0, Answer: [{ type: type === 'AAAA' ? 28 : 1, data }] }
                        : { Status: 0, Answer: [] });
                }
                expect(init?.method).toBe('POST');
                expect(init?.redirect).toBe('error');
                expect(init?.body).toBe('{"ok":true}');
                expect(url.startsWith('https://hooks.example/') || url === 'https://8.8.8.8/hook').toBe(true);
                return new Response('{"challenge":"abc"}', { status: 200 });
            }) as typeof fetch;
        };
        try {
            calls.length = 0;
            install({ A: '8.8.8.8', AAAA: '2606:4700:4700::1111' });
            const response = await postWebhook({
                url: 'https://hooks.example/mcp',
                body: '{"ok":true}',
                headers: { 'Content-Type': 'application/json' },
            });
            expect(response).toEqual({ status: 200, body: '{"challenge":"abc"}' });
            expect(calls.filter((url) => url.startsWith('https://hooks.example/'))).toEqual([
                'https://hooks.example/mcp',
            ]);

            calls.length = 0;
            install({ A: '8.8.8.8', AAAA: 'fd00::1' });
            await expect(postWebhook({
                url: 'https://hooks.example/private-aaaa',
                body: '{"ok":true}',
                headers: {},
            })).rejects.toThrow(/not public/);
            expect(calls.some((url) => url.startsWith('https://hooks.example/'))).toBe(false);

            calls.length = 0;
            install({ A: '10.1.2.3', AAAA: null });
            await expect(postWebhook({
                url: 'https://hooks.example/private-a',
                body: '{"ok":true}',
                headers: {},
            })).rejects.toThrow(/not public/);
            expect(calls.some((url) => url.startsWith('https://hooks.example/'))).toBe(false);

            calls.length = 0;
            install({});
            const literal = await postWebhook({
                url: 'https://8.8.8.8/hook',
                body: '{"ok":true}',
                headers: {},
            });
            expect(literal.status).toBe(200);
            expect(calls.some((url) => url.includes('cloudflare-dns.com'))).toBe(false);
            expect(calls).toEqual(['https://8.8.8.8/hook']);

            calls.length = 0;
            await expect(postWebhook({
                url: 'https://127.0.0.1/hook',
                body: '{"ok":true}',
                headers: {},
            })).rejects.toThrow(/not public/);
            expect(calls).toEqual([]);
        } finally {
            globalThis.fetch = original;
        }
    });

    it('derives the same subscription id when argument key order changes', async () => {
        const left = await subscriptionId(
            'account:1',
            'https://hooks.example/mcp',
            'session.state_changed',
            { state: 'question', session_id: 'abc' },
        );
        const right = await subscriptionId(
            'account:1',
            'https://hooks.example/mcp',
            'session.state_changed',
            { session_id: 'abc', state: 'question' },
        );
        expect(left).toBe(right);
        expect(left.startsWith('sub_')).toBe(true);
    });

    it('keeps event payloads inside their declared fields', () => {
        const prompt = promptEventData({
            prompt_type: 'question',
            questions: [{
                question: 'Ship it?',
                header: 'Ship',
                options: [{ label: 'Yes', value: '1' }],
            }],
        });
        const recap = recapEventData({
            type: 'recap',
            status: 'ready',
            latest: {
                prompt_count: 1,
                generated_at: 1,
                variant: 'brief',
                headline: 'Added the hook',
                bullets: ['One'],
                next_prompt_notes: ['do this next'],
                artifacts: [],
                line_delta: { additions: 3, deletions: 1 },
            },
        });
        const commit = commitEventData({
            hash: 'abc',
            short_hash: 'abc',
            timestamp: 1_700_000_000,
            subject: 'Add events',
        });
        const samples: Record<string, Record<string, unknown>> = {
            'session.prompt': { session_id: 's', ...prompt },
            'session.recap': { session_id: 's', ...recap! },
            'session.commit': { session_id: 's', ...commit },
        };
        for (const [name, data] of Object.entries(samples)) {
            const definition = eventDefinitions.find((event) => event.name === name);
            const properties = definition?.payloadSchema.properties || {};
            for (const key of Object.keys(data)) expect(properties, name).toHaveProperty(key);
            for (const key of definition?.payloadSchema.required || []) {
                expect(data, `${name}.${key}`).toHaveProperty(key);
            }
        }
        expect(recap).not.toHaveProperty('next_prompt_notes');
    });

    it('advertises events on discover and lists them', async () => {
        const { token } = await linkedAccount('EV1');
        const discovered = await rpc(token, 'server/discover');
        expect(discovered.status).toBe(200);
        const discoverBody = await discovered.json() as {
            result?: {
                supportedVersions?: string[];
                capabilities?: { events?: { listChanged?: boolean } };
            };
        };
        expect(discoverBody.result?.supportedVersions).toContain('2026-07-28');
        expect(discoverBody.result?.capabilities?.events?.listChanged).toBe(false);

        const initialized = await rpc(token, 'initialize', { protocolVersion: '2026-07-28' });
        const initBody = await initialized.json() as {
            result?: { protocolVersion?: string; capabilities?: { events?: unknown } };
        };
        expect(initBody.result?.protocolVersion).toBe('2026-07-28');
        expect(initBody.result?.capabilities?.events).toBeTruthy();

        const listed = await rpc(token, 'events/list');
        const names = ((await listed.json()) as { result?: { events?: Array<{ name: string; delivery: string[] }> } })
            .result?.events || [];
        expect(names.map((event) => event.name)).toEqual(eventDefinitions.map((event) => event.name));
        expect(names.every((event) => event.delivery.includes('webhook'))).toBe(true);

        const page = await (await SELF.fetch(`${ORIGIN}/mcp-tools`)).text();
        expect(page).toContain('session.state_changed');
        expect(page).toContain('session.prompt');
    });

    it('verifies the callback, stores one subscription, and refreshes it', async () => {
        const { token } = await linkedAccount('EV2');
        const calls = echoPoster();
        const secret = webhookSecret(7);
        const url = 'https://hooks.example/state';
        const first = await rpc(token, 'events/subscribe', subscribeBody(
            url,
            secret,
            { state: 'question', session_id: 'sess' },
            { ttlMs: 3_600_000 },
        ));
        const firstBody = await first.json() as {
            result?: { id: string; refreshBefore: string; cursor: null; truncated: boolean };
            error?: { code: number; message: string };
        };
        expect(firstBody.error).toBeUndefined();
        expect(firstBody.result?.cursor).toBeNull();
        expect(firstBody.result?.truncated).toBe(false);
        const refreshAt = Date.parse(firstBody.result?.refreshBefore || '');
        expect(refreshAt).toBeGreaterThan(Date.now() + 50 * 60 * 1000);
        expect(refreshAt).toBeLessThan(Date.now() + 70 * 60 * 1000);
        expect(calls).toHaveLength(1);
        expect(calls[0].headers['webhook-id']).toMatch(/^msg_verification_/);
        expect(calls[0].headers['X-MCP-Subscription-Id']).toBe(firstBody.result?.id);
        expect(JSON.parse(calls[0].body).type).toBe('verification');

        const second = await rpc(token, 'events/subscribe', subscribeBody(url, secret, { session_id: 'sess', state: 'question' }));
        const secondBody = await second.json() as { result?: { id: string } };
        expect(secondBody.result?.id).toBe(firstBody.result?.id);
        expect(calls).toHaveLength(1);

        const rows = await testEnv.DB.prepare(
            'SELECT COUNT(*) AS n FROM mcp_event_subscriptions WHERE callback_url = ?',
        ).bind(url).first<{ n: number }>();
        expect(rows?.n).toBe(1);
    });

    it('rejects a bad secret, a private callback, and an unknown event before connecting', async () => {
        const { token } = await linkedAccount('EV3');
        let posts = 0;
        setWebhookPoster(async () => {
            posts += 1;
            return { status: 200, body: '' };
        });
        const badSecret = await rpc(token, 'events/subscribe', subscribeBody('https://hooks.example/bad-secret', 'whsec_aa'));
        expect((await badSecret.json() as { error?: { code: number } }).error?.code).toBe(-32602);

        const privateUrl = await rpc(token, 'events/subscribe', subscribeBody('https://127.0.0.1/hook', webhookSecret(3)));
        const privateBody = await privateUrl.json() as { error?: { code: number; data?: { reason?: string } } };
        expect(privateBody.error?.code).toBe(-32015);
        expect(privateBody.error?.data?.reason).toBe('connection_refused');

        const missing = await rpc(token, 'events/subscribe', {
            ...subscribeBody('https://hooks.example/missing', webhookSecret(3)),
            name: 'screen.updated',
        });
        expect((await missing.json() as { error?: { code: number; data?: { kind?: string } } }).error).toMatchObject({
            code: -32011,
            data: { kind: 'event' },
        });

        const polled = await rpc(token, 'events/subscribe', {
            ...subscribeBody('https://hooks.example/poll', webhookSecret(3)),
            delivery: { mode: 'poll', url: 'https://hooks.example/poll', secret: webhookSecret(3) },
        });
        expect((await polled.json() as { error?: { code: number } }).error?.code).toBe(-32014);
        expect(posts).toBe(0);
    });

    it('delivers a matching state change once, signed, and skips other filters', async () => {
        const { token, groupId } = await linkedAccount('EV4');
        const calls = echoPoster();
        const secret = webhookSecret(9);
        const subscribed = await rpc(token, 'events/subscribe', subscribeBody(
            'https://hooks.example/question',
            secret,
            { state: 'question' },
        ));
        const subscriptionIdValue = ((await subscribed.json()) as { result?: { id: string } }).result?.id;
        calls.length = 0;

        const thinking: McpEventOccurrence = {
            eventId: 'evt_thinking',
            name: 'session.state_changed',
            timestamp: '2026-09-29T12:00:00.000Z',
            data: {
                session_id: 'sess-1',
                previous_state: 'ready',
                state: 'thinking',
                platform: 'claude',
                cwd: '/tmp/demo',
                title: 'Demo',
            },
            cursor: null,
        };
        await deliverMcpEvent(testEnv, groupId, thinking);
        expect(calls).toHaveLength(0);

        const question: McpEventOccurrence = {
            ...thinking,
            eventId: 'evt_question',
            data: { ...thinking.data, previous_state: 'thinking', state: 'question' },
        };
        await deliverMcpEvent(testEnv, groupId, question);
        expect(calls).toHaveLength(1);
        const sent = calls[0];
        expect(sent.headers['webhook-id']).toBe('evt_question');
        expect(sent.headers['X-MCP-Subscription-Id']).toBe(subscriptionIdValue);
        expect(sent.body).toBe(JSON.stringify(question));
        const timestamp = Number(sent.headers['webhook-timestamp']);
        expect(sent.headers['webhook-signature']).toBe(
            await signWebhook('evt_question', timestamp, sent.body, [secret]),
        );
    });

    it('signs with the old and new secret during a refresh', async () => {
        const { token, groupId } = await linkedAccount('EV5');
        const calls = echoPoster();
        const first = webhookSecret(4);
        const second = webhookSecret(5);
        const url = 'https://hooks.example/rotate';
        await rpc(token, 'events/subscribe', subscribeBody(url, first));
        await rpc(token, 'events/subscribe', subscribeBody(url, second));
        calls.length = 0;

        const occurrence: McpEventOccurrence = {
            eventId: 'evt_rotate',
            name: 'session.state_changed',
            timestamp: '2026-09-29T12:00:00.000Z',
            data: { session_id: 'sess-r', previous_state: 'ready', state: 'complete' },
            cursor: null,
        };
        await deliverMcpEvent(testEnv, groupId, occurrence);
        expect(calls).toHaveLength(1);
        const timestamp = Number(calls[0].headers['webhook-timestamp']);
        expect(calls[0].headers['webhook-signature']).toBe(
            await signWebhook('evt_rotate', timestamp, calls[0].body, [second, first]),
        );
    });

    it('drops a subscription when the callback is gone and retries a transient failure', async () => {
        const { token, groupId } = await linkedAccount('EV6');
        const calls: WebhookRequest[] = [];
        let eventStatus = 500;
        setWebhookPoster(async (request) => {
            calls.push(request);
            const body = JSON.parse(request.body) as { type?: string; challenge?: string };
            if (body.type === 'verification') {
                return { status: 200, body: JSON.stringify({ challenge: body.challenge }) };
            }
            const status = eventStatus;
            if (status === 500) eventStatus = 200;
            return { status, body: '' };
        });
        const url = 'https://hooks.example/retry';
        const subscribed = await rpc(token, 'events/subscribe', subscribeBody(url, webhookSecret(6)));
        const id = ((await subscribed.json()) as { result?: { id: string } }).result?.id;
        calls.length = 0;
        const occurrence: McpEventOccurrence = {
            eventId: 'evt_retry',
            name: 'session.state_changed',
            timestamp: '2026-09-29T12:00:00.000Z',
            data: { session_id: 'sess-retry', previous_state: 'ready', state: 'thinking' },
            cursor: null,
        };
        await deliverMcpEvent(testEnv, groupId, occurrence);
        expect(calls.map((call) => JSON.parse(call.body).eventId)).toEqual(['evt_retry', 'evt_retry']);

        eventStatus = 410;
        calls.length = 0;
        await deliverMcpEvent(testEnv, groupId, { ...occurrence, eventId: 'evt_gone' });
        expect(calls).toHaveLength(1);
        const row = await testEnv.DB.prepare(
            'SELECT id FROM mcp_event_subscriptions WHERE id = ?',
        ).bind(id).first();
        expect(row).toBeNull();
    });

    it('stops delivery when the account no longer owns the group', async () => {
        const { token, accountId, groupId } = await linkedAccount('EV7');
        const calls = echoPoster();
        await rpc(token, 'events/subscribe', subscribeBody('https://hooks.example/revoked', webhookSecret(8)));
        calls.length = 0;
        await testEnv.DB.prepare(
            'INSERT OR IGNORE INTO device_groups (id) VALUES (?)',
        ).bind('revoked-group-ev7').run();
        await testEnv.DB.prepare(
            'UPDATE accounts SET group_id = ? WHERE id = ?',
        ).bind('revoked-group-ev7', accountId).run();
        await deliverMcpEvent(testEnv, groupId, {
            eventId: 'evt_revoked',
            name: 'session.state_changed',
            timestamp: '2026-09-29T12:00:00.000Z',
            data: { session_id: 'sess-revoked', previous_state: 'ready', state: 'question' },
            cursor: null,
        });
        expect(calls).toHaveLength(0);
    });

    it('unsubscribes only the caller and ignores a second request', async () => {
        const alice = await linkedAccount('EV8');
        const bob = await linkedAccount('EV9');
        echoPoster();
        const url = 'https://hooks.example/shared';
        const secret = webhookSecret(2);
        const args = { state: 'permission' };
        await rpc(alice.token, 'events/subscribe', subscribeBody(url, secret, args));
        await rpc(bob.token, 'events/subscribe', subscribeBody(url, secret, args));

        const removed = await rpc(alice.token, 'events/unsubscribe', {
            name: 'session.state_changed',
            arguments: { state: 'permission' },
            delivery: { mode: 'webhook', url },
        });
        expect(await removed.json()).toMatchObject({ result: {} });
        const again = await rpc(alice.token, 'events/unsubscribe', {
            name: 'session.state_changed',
            arguments: { state: 'permission' },
            delivery: { mode: 'webhook', url },
        });
        expect((await again.json() as { result?: unknown }).result).toEqual({});

        const left = await testEnv.DB.prepare(
            'SELECT principal FROM mcp_event_subscriptions WHERE callback_url = ?',
        ).bind(url).all<{ principal: string }>();
        expect(left.results?.map((row) => row.principal)).toEqual([`account:${bob.accountId}`]);
    });
});
