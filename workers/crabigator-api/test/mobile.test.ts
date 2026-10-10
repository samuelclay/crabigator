import { env, evictDurableObject, runInDurableObject } from 'cloudflare:test';
import { beforeAll, expect, it, vi } from 'vitest';
import { notifyMobilePrompt } from '../src/mobile-push';
import { sha256 } from '../src/auth/tokens';
import { mobileSnapshot, registerPush } from '../src/handlers/mobile';
import type { Env } from '../src/types/env';
import type { CloudToDesktopMessage } from '../src/types/session';
import { ensureAccountSchema } from './schema';
import migration from '../migrations/0022_mobile_push.sql?raw';

const testEnv = env as unknown as Env;

beforeAll(async () => {
    await ensureAccountSchema(testEnv);
    await testEnv.DB.prepare(migration).run();
});

async function pairedMobile(name: string, group = 'mobile-group') {
    const token = `${name}-credential`;
    const hash = await sha256(token);
    await testEnv.DB.prepare('INSERT INTO devices (id, secret_hash, group_id) VALUES (?, ?, ?)')
        .bind(`${name}-desktop`, 'test-secret', group).run();
    await testEnv.DB.prepare(`INSERT INTO linked_devices (id, desktop_id, mobile_id, mobile_token_hash)
        VALUES (?, ?, ?, ?)`).bind(name, `${name}-desktop`, name, hash).run();
    await testEnv.TOKENS.put(`mobile:${hash}`, JSON.stringify({ desktop_id: `${name}-desktop`, mobile_id: name, group_id: group }));
    return { token, hash, request: (path: string, body?: unknown) => new Request(`https://example.com${path}`, {
        headers: { Authorization: `Bearer ${token}`, 'Content-Type': 'application/json' },
        ...(body === undefined ? {} : { method: 'POST', body: JSON.stringify(body) }),
    }) };
}

it('authenticates push registration, rotates tokens, and transfers a reinstalled token', async () => {
    expect((await registerPush(new Request('https://example.com/api/mobile/push', { method: 'POST' }), testEnv)).status).toBe(401);
    const first = await pairedMobile('push-first');
    const token = 'firebase-token-for-the-first-install';
    const register = (mobile: typeof first, token: string) => registerPush(mobile.request('/api/mobile/push', { platform: 'android', token }), testEnv);
    expect((await register(first, token)).status).toBe(200);
    const rotated = token + '-rotated';
    expect((await register(first, rotated)).status).toBe(200);
    expect(await testEnv.DB.prepare('SELECT token, token_hash FROM mobile_push_tokens WHERE mobile_id = ?')
        .bind('push-first').first()).toEqual({ token: rotated, token_hash: first.hash });
    const reinstalled = await pairedMobile('push-reinstalled');
    expect((await register(reinstalled, rotated)).status).toBe(200);
    expect(await testEnv.DB.prepare('SELECT mobile_id FROM mobile_push_tokens WHERE token = ?').bind(rotated).first())
        .toEqual({ mobile_id: 'push-reinstalled' });
    await testEnv.DB.prepare('UPDATE linked_devices SET revoked_at = 1 WHERE id = ?').bind('push-reinstalled').run();
    expect((await register(reinstalled, rotated)).status).toBe(401);
});

it('keeps mobile snapshots within the paired device group', async () => {
    const owner = await pairedMobile('snapshot-owner', 'snapshot-group');
    const stranger = await pairedMobile('snapshot-stranger', 'another-group');
    await testEnv.DB.prepare(`INSERT INTO sessions (id, device_id, client_session_id, cwd, platform)
        VALUES ('mobile-snapshot', 'snapshot-owner-desktop', 'local', '/tmp/project', 'claude')`).run();
    const params = { id: 'mobile-snapshot' };
    expect((await mobileSnapshot(stranger.request('/api/mobile/sessions/mobile-snapshot'), testEnv, params)).status).toBe(403);
    const response = await mobileSnapshot(owner.request('/api/mobile/sessions/mobile-snapshot'), testEnv, params);
    expect(response.status).toBe(200);
    expect(await response.json()).toMatchObject({ prompt: null, prompt_revision: 0, desktop_connected: false });
});

it('forwards only one concurrent reply per prompt and rejects old revisions after hibernation', async () => {
    const stub = testEnv.SESSION.get(testEnv.SESSION.idFromName('mobile-revisions'));
    const response = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
    const desktop = response.webSocket!;
    desktop.accept();
    const forwarded: CloudToDesktopMessage[] = [];
    desktop.addEventListener('message', event => { forwarded.push(JSON.parse(String(event.data))); });
    const snapshot = async () => (await stub.fetch('https://internal/snapshot')).json<{ prompt: unknown; prompt_revision: number; attention_pending: boolean; state: string; desktop_connected: boolean }>();
    const send = (path: string, body: unknown) => stub.fetch(`https://internal/${path}`, { method: 'POST', body: JSON.stringify(body) });
    const prompt = { prompt_type: 'question', questions: [{ question: 'Where?', options: [{ label: 'Here', value: '1' }], multi_select: true }], cursor_row: 1 };
    try {
        desktop.send(JSON.stringify({ type: 'state', state: 'question' }));
        await expect.poll(async () => (await snapshot()).attention_pending).toBe(true);
        const unstructuredRevision = (await snapshot()).prompt_revision;
        expect((await snapshot()).prompt).toBeNull();
        const genericReply = await send('answer', { text: 'A typed answer', expected_prompt_revision: unstructuredRevision });
        expect(genericReply.status).toBe(200);
        await genericReply.text();
        expect((await snapshot()).attention_pending).toBe(false);
        await evictDurableObject(stub);
        expect((await snapshot()).attention_pending).toBe(false);
        await expect.poll(() => forwarded.filter(m => m.type === 'answer').length).toBe(1);
        forwarded.length = 0;
        desktop.send(JSON.stringify({ type: 'prompt', prompt }));
        await expect.poll(async () => (await snapshot()).prompt).toEqual(prompt);
        const revision = (await snapshot()).prompt_revision;
        const replies = await Promise.all([
            send('answer', { text: 'Here', expected_prompt_revision: revision }),
            send('key-sequence', { steps: [{ type: 'key', key: 'enter' }], expected_prompt_revision: revision }),
        ]);
        expect(replies.map(r => r.status).sort()).toEqual([200, 409]);
        await Promise.all(replies.map(reply => reply.text()));
        await expect.poll(() => forwarded.filter(m => m.type === 'answer' || m.type === 'key_sequence').length).toBe(1);
        expect((await snapshot()).prompt).toBeNull();
        await evictDurableObject(stub);
        expect((await snapshot()).prompt).toBeNull();
        expect((await send('key', { key: 'enter', expected_prompt_revision: revision })).status).toBe(409);

        // A checkbox/page change reopens interaction using the new screen revision.
        const changed = { ...prompt, checked: [1] };
        desktop.send(JSON.stringify({ type: 'prompt', prompt: changed }));
        await expect.poll(async () => (await snapshot()).prompt).toEqual(changed);
        const next = (await snapshot()).prompt_revision;
        expect(next).toBeGreaterThan(revision);
        expect((await send('answer', { text: 'stale', expected_prompt_revision: revision })).status).toBe(409);
        expect((await send('key', { key: 'enter', expected_prompt_revision: next })).status).toBe(200);

        // A desktop answer clears the prompt and every delayed phone reply.
        desktop.send(JSON.stringify({ type: 'state', state: 'thinking' }));
        await expect.poll(async () => (await snapshot()).state).toBe('thinking');
        expect((await snapshot()).prompt).toBeNull();
        expect((await send('answer', { text: 'late', expected_prompt_revision: next })).status).toBe(409);
    } finally {
        desktop.close();
    }
    await expect.poll(async () => (await snapshot()).desktop_connected).toBe(false);
});


it('sends only refresh hints to current paired devices and removes invalid FCM tokens', async () => {
    const group = 'push-delivery-group';
    const valid = await pairedMobile('delivery-valid', group);
    const expired = await pairedMobile('delivery-expired', group);
    const revoked = await pairedMobile('delivery-revoked', group);
    const unregistered = await pairedMobile('delivery-unregistered', group);
    for (const [name, mobile] of [['valid', valid], ['expired', expired], ['revoked', revoked], ['unregistered', unregistered]] as const) {
        expect((await registerPush(mobile.request('/api/mobile/push', {
            platform: 'android', token: `firebase-delivery-token-${name}`,
        }), testEnv)).status).toBe(200);
    }
    await testEnv.TOKENS.delete(`mobile:${expired.hash}`);
    await testEnv.DB.prepare("UPDATE linked_devices SET revoked_at = 1 WHERE id = 'delivery-revoked'").run();
    const keys = await crypto.subtle.generateKey({ name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048,
        publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' }, true, ['sign', 'verify']) as CryptoKeyPair;
    const der = new Uint8Array(await crypto.subtle.exportKey('pkcs8', keys.privateKey));
    const account = { project_id: 'test-project', client_email: 'test@firebase.example',
        private_key: `-----BEGIN PRIVATE KEY-----\n${btoa(String.fromCharCode(...der))}\n-----END PRIVATE KEY-----` };
    const messages: Record<string, unknown>[] = [];
    const fetch = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
        if (String(input) === 'https://oauth2.googleapis.com/token') {
            return Response.json({ access_token: 'test-access', expires_in: 3600 });
        }
        expect(String(input)).toBe('https://fcm.googleapis.com/v1/projects/test-project/messages:send');
        const { message } = JSON.parse(String(init?.body));
        messages.push(message);
        return message.token.endsWith('-unregistered')
            ? Response.json({ error: { details: [{ errorCode: 'UNREGISTERED' }] } }, { status: 404 })
            : Response.json({ name: 'message-id' });
    });
    try {
        await notifyMobilePrompt({ ...testEnv, FIREBASE_SERVICE_ACCOUNT: JSON.stringify(account) }, group, 'session-needing-attention');
        expect(messages).toHaveLength(2);
        for (const message of messages) {
            expect(message).toEqual({ token: expect.any(String), data: { session_id: 'session-needing-attention' },
                android: { priority: 'high', ttl: '300s' } });
        }
        expect(messages.map(m => m.token).sort()).toEqual(['firebase-delivery-token-unregistered', 'firebase-delivery-token-valid']);
        expect(await testEnv.DB.prepare("SELECT mobile_id FROM mobile_push_tokens WHERE mobile_id = 'delivery-unregistered'").first()).toBeNull();
    } finally {
        fetch.mockRestore();
    }
});


it('pushes only transitions that create, update, or clear pending attention', async () => {
    const stub = testEnv.SESSION.get(testEnv.SESSION.idFromName('mobile-attention-transitions'));
    await runInDurableObject(stub, async instance => {
        const session = instance as unknown as { handleEvent(event: unknown): Promise<void>; notifyMobileAttention(): void };
        const notify = vi.spyOn(session, 'notifyMobileAttention').mockImplementation(() => {});
        try {
            for (const state of ['thinking', 'complete', 'ready', 'thinking']) {
                await session.handleEvent({ type: 'state', state });
            }
            expect(notify).not.toHaveBeenCalled();
            await session.handleEvent({ type: 'state', state: 'permission' });
            expect(notify).toHaveBeenCalledTimes(1);
            await session.handleEvent({ type: 'prompt', prompt: { prompt_type: 'permission', tool_name: 'Bash', options: [] } });
            expect(notify).toHaveBeenCalledTimes(2);
            await session.handleEvent({ type: 'state', state: 'thinking' });
            expect(notify).toHaveBeenCalledTimes(3);
            await session.handleEvent({ type: 'state', state: 'complete' });
            expect(notify).toHaveBeenCalledTimes(3);
        } finally {
            notify.mockRestore();
        }
    });
});

it('retries transient delivery failures with a limit and leaves permanent errors alone', async () => {
    const group = 'retry-group';
    for (const name of ['recover', 'persistent', 'permanent', 'quota', 'retry-after']) {
        const mobile = await pairedMobile(`retry-${name}`, group);
        await registerPush(mobile.request('/api/mobile/push', { platform: 'android', token: `firebase-retry-token-${name}` }), testEnv);
    }
    const keys = await crypto.subtle.generateKey({ name: 'RSASSA-PKCS1-v1_5', modulusLength: 2048,
        publicExponent: new Uint8Array([1, 0, 1]), hash: 'SHA-256' }, true, ['sign', 'verify']) as CryptoKeyPair;
    const der = new Uint8Array(await crypto.subtle.exportKey('pkcs8', keys.privateKey));
    const account = { project_id: 'test-project', client_email: 'retry@firebase.example',
        private_key: `-----BEGIN PRIVATE KEY-----\n${btoa(String.fromCharCode(...der))}\n-----END PRIVATE KEY-----` };
    const calls = new Map<string, number>();
    const fetch = vi.spyOn(globalThis, 'fetch').mockImplementation(async (input, init) => {
        if (String(input) === 'https://oauth2.googleapis.com/token') return Response.json({ access_token: 'retry-access', expires_in: 3600 });
        const { message } = JSON.parse(String(init?.body));
        const token = message.token as string;
        const attempt = (calls.get(token) || 0) + 1;
        calls.set(token, attempt);
        expect(init?.signal).toBeInstanceOf(AbortSignal);
        if (token.endsWith('-recover')) {
            if (attempt === 1) throw new TypeError('Connection closed');
            return Response.json({}, { status: attempt === 2 ? 503 : 200 });
        }
        if (token.endsWith('-retry-after')) return Response.json({}, { status: 503, headers: { 'Retry-After': '60' } });
        const status = token.endsWith('-persistent') ? 503 : token.endsWith('-quota') ? 429 : 400;
        return Response.json({ error: {} }, { status });
    });
    try {
        await expect(notifyMobilePrompt({ ...testEnv, FIREBASE_SERVICE_ACCOUNT: JSON.stringify(account) }, group, 'retry-session')).rejects.toBeInstanceOf(AggregateError);
        expect(Object.fromEntries(calls)).toEqual({
            'firebase-retry-token-recover': 3, 'firebase-retry-token-persistent': 3,
            'firebase-retry-token-permanent': 1, 'firebase-retry-token-quota': 1, 'firebase-retry-token-retry-after': 1,
        });
    } finally {
        fetch.mockRestore();
    }
}, 10000);


it.each([false, true])('does not recreate a cleared structured question after answering (thinking transition: %s)', async thinking => {
    const stub = testEnv.SESSION.get(testEnv.SESSION.idFromName(`mobile-cleared-prompt-${thinking}`));
    const response = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
    const desktop = response.webSocket!;
    desktop.accept();
    const snapshot = async () => (await stub.fetch('https://internal/snapshot')).json<{ prompt: unknown; attention_pending: boolean; prompt_revision: number; state: string }>();
    const prompt = { prompt_type: 'question', questions: [{ question: 'Choose?', options: [{ label: 'First', value: '1' }] }] };
    const event = (value: unknown) => desktop.send(JSON.stringify(value));
    try {
        event({ type: 'stats', prompts: 1 });
        event({ type: 'state', state: 'question' });
        event({ type: 'prompt', prompt });
        await expect.poll(async () => (await snapshot()).prompt).toEqual(prompt);
        const revision = (await snapshot()).prompt_revision;
        const answered = await stub.fetch('https://internal/answer', { method: 'POST',
            body: JSON.stringify({ text: 'First', expected_prompt_revision: revision }) });
        expect(answered.status).toBe(200);
        await answered.text();
        if (thinking) event({ type: 'state', state: 'thinking' });
        event({ type: 'prompt', prompt: null });
        event({ type: 'state', state: 'question' });
        await expect.poll(async () => (await snapshot()).prompt_revision).toBeGreaterThan(revision);
        await expect.poll(async () => (await snapshot()).state).toBe('question');
        expect(await snapshot()).toMatchObject({ prompt: null, attention_pending: false });
        await evictDurableObject(stub);
        expect(await snapshot()).toMatchObject({ prompt: null, attention_pending: false });
        const stale = await stub.fetch('https://internal/answer', { method: 'POST',
            body: JSON.stringify({ text: 'Ghost reply', expected_prompt_revision: (await snapshot()).prompt_revision }) });
        expect(stale.status).toBe(409);
        await stale.text();

        // A new structured page is still actionable in the same assistant turn.
        const nextPrompt = { ...prompt, current_question: 1 };
        event({ type: 'prompt', prompt: nextPrompt });
        await expect.poll(async () => (await snapshot()).prompt).toEqual(nextPrompt);
        expect((await snapshot()).attention_pending).toBe(true);
        event({ type: 'prompt', prompt: null });
        await expect.poll(async () => (await snapshot()).attention_pending).toBe(false);

        // A new user turn can genuinely ask an unstructured question.
        event({ type: 'state', state: 'thinking' });
        event({ type: 'stats', prompts: 2 });
        event({ type: 'state', state: 'question' });
        await expect.poll(async () => (await snapshot()).attention_pending).toBe(true);
        expect((await snapshot()).prompt).toBeNull();
    } finally {
        desktop.close();
    }
});

it('keeps pre-upgrade accepted questions resolved when their structured prompt is already gone', async () => {
    const stub = testEnv.SESSION.get(testEnv.SESSION.idFromName('mobile-legacy-answered'));
    await runInDurableObject(stub, async (_instance, state) => {
        await state.storage.put('persistentState', {
            sessionId: '', state: 'question', currentPrompt: null,
            promptRevision: 6, answeredRevision: 4,
        });
    });
    await evictDurableObject(stub);
    const snapshot = await (await stub.fetch('https://internal/snapshot')).json();
    expect(snapshot).toMatchObject({ prompt: null, prompt_revision: 6, attention_pending: false });
    const diagnostics = await (await stub.fetch('https://internal/state')).json();
    expect(diagnostics).toMatchObject({ unstructured_attention_blocked: true, answered_prompt_revision: 4, attention_prompt_count: null });
});
