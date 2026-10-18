import { env, evictDurableObject, runInDurableObject } from 'cloudflare:test';
import { beforeAll, expect, it } from 'vitest';
import { conductorRoute } from '../src/conductor';
import { hmacSign, sha256 } from '../src/auth/tokens';
import { ensureAccountSchema } from './schema';
import type { Env } from '../src/types/env';

const testEnv = env as unknown as Env;
beforeAll(() => ensureAccountSchema(testEnv));

async function account(group: string) {
    const device = `${group}-desktop`, token = `${group}-mobile`, secret = 'test-only-secret';
    const hash = await sha256(token);
    await env.DB.prepare('INSERT INTO devices (id, secret_hash, group_id) VALUES (?, ?, ?)').bind(device, secret, group).run();
    await env.DB.prepare('INSERT INTO linked_devices (id, desktop_id, mobile_id, mobile_token_hash) VALUES (?, ?, ?, ?)').bind(group, device, group, hash).run();
    await env.TOKENS.put(`mobile:${hash}`, JSON.stringify({ desktop_id: device, mobile_id: group, group_id: group }));
    async function call(action = '', body?: unknown, desktop = false) {
        const path = `/api/conductor${action ? '/' + action : ''}`, method = body === undefined ? 'GET' : 'POST';
        const timestamp = String(Date.now());
        const headers = desktop ? { 'X-Device-Id': device, 'X-Timestamp': timestamp, 'X-Signature': await hmacSign(secret, `${method}:${path}:${timestamp}`) } : { Authorization: `Bearer ${token}` };
        const response = await conductorRoute(new Request('https://example.com' + path, { method, headers, ...(body === undefined ? {} : { body: JSON.stringify(body) }) }), testEnv);
        // Release the DO response stream even when a test checks only its status.
        return new Response(await response.text(), { status: response.status, headers: response.headers });
    }
    return { device, call };
}

it('scopes conversations and host tools to the authenticated account and deduplicates messages', async () => {
    const a = await account('conductor-owner'), b = await account('conductor-stranger');
    expect((await conductorRoute(new Request('https://example.com/api/conductor'), testEnv)).status).toBe(401);
    expect((await a.call('host', { host_id: 'a', name: 'Computer' })).status).toBe(403);
    await a.call('message', { id: 'request', text: 'Check the app stores' });
    await a.call('message', { id: 'request', text: 'Check the app stores' });
    expect((await a.call('message', { id: 'request', text: 'Different text' })).status).toBe(409);
    expect((await (await a.call()).json<any>()).messages).toHaveLength(1);
    expect((await (await b.call()).json<any>()).messages).toEqual([]);
    const claim = await (await a.call('host', { host_id: 'a', name: 'Computer', claim: true }, true)).json<any>();
    expect(claim.turn.id).toBeTruthy();
    expect((await a.call('host', { host_id: 'b', name: 'Other computer' }, true)).status).toBe(409);
    expect((await b.call('tool', { host_id: 'a', turn_id: claim.turn.id, name: 'read_tracks' }, true)).status).toBe(409);
});

it('delivers once, keeps progress separate from old output, and accepts answers only from the assigned session', async () => {
    const a = await account('conductor-delivery');
    const id = 'conductor-target';
    await env.DB.prepare('INSERT INTO sessions (id, device_id, client_session_id, cwd, platform) VALUES (?, ?, ?, ?, ?)').bind(id, a.device, 'local-target', '/tmp/NewsBlur', 'codex').run();
    const stub = env.SESSION.get(env.SESSION.idFromName(id));
    const connection = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
    const socket = connection.webSocket!; socket.accept();
    const inputs: any[] = [];
    socket.addEventListener('message', e => { const m = JSON.parse(String(e.data)); if (m.type === 'conductor_input') inputs.push(m); });
    try {
        socket.send(JSON.stringify({ type: 'heartbeat', conductor_command: "'/tmp/crabigator'", timestamp: Date.now() }));
        socket.send(JSON.stringify({ type: 'scrollback_history', content: 'An old answer that must not complete this task.' }));
        await expect.poll(async () => (await (await stub.fetch('https://internal/snapshot')).json<any>()).conductor_command).toBe("'/tmp/crabigator'");
        await a.call('message', { id: 'request', text: 'Check versions' });
        const host = { host_id: 'host', name: 'Computer', claim: true };
        const { turn } = await (await a.call('host', host, true)).json<any>();
        const tool = (name: string, args: unknown) => a.call('tool', { host_id: host.host_id, turn_id: turn.id, name, arguments: args }, true);
        const args = { request_id: 'request', tasks: [{ session_id: id, title: 'NewsBlur stores', instruction: 'Read current versions.' }] };
        const dispatched = await (await tool('dispatch', args)).json<any>();
        const track = dispatched.tracks[0];
        expect(track.state).toBe('forwarded');
        await evictDurableObject(env.SESSION_LIST.get(env.SESSION_LIST.idFromName('conductor:conductor-delivery')));
        expect((await (await tool('dispatch', args)).json<any>()).tracks[0].id).toBe(track.id);
        await expect.poll(() => inputs.length).toBe(1);
        expect(inputs[0].text).toContain(`conductor report ${track.id}`);
        expect((await (await a.call()).json<any>()).tracks[0].result).toBeUndefined();
        const report = (state: string, text: string, session = 'local-target') => a.call('report', { task_id: track.id, session_id: session, state, text }, true);
        expect((await report('answered', 'Wrong session', 'different-session')).status).toBe(403);
        await report('working', 'Checking Apple');
        await report('delivered', 'Delayed desktop acknowledgment');
        expect((await (await a.call()).json<any>()).tracks[0].state).toBe('working');
        await report('answered', 'iOS 2.0 is in review. Android 2.1 is live.');
        await report('working', 'Late progress');
        const final = (await (await a.call()).json<any>()).tracks[0];
        expect(final.state).toBe('answered');
        expect(final.result).toContain('iOS 2.0');
        expect(final.events).toHaveLength(3);
        await env.DB.prepare('UPDATE sessions SET titles=?, session_mark=?, additions=?, deletions=? WHERE id=?').bind(JSON.stringify(['Old title', 'Latest app review']), JSON.stringify({glyph:'⌬⌬',fg:[100,200,150],bg:[20,30,40]}), 12, 3, id).run();
        expect((await tool('reply', { text: 'Both stores checked.', highlights: [{session_id: id, task_id: track.id, text: 'iOS **2.0** in review.'}] })).status).toBe(200);
        const reply = (await (await a.call()).json<any>()).messages.at(-1);
        const renamed = await (await a.call('archive', {query: 'Latest app review', status: 'answered'})).json<any>();
        expect(renamed.entries.map((e: any) => e.id)).toContain(track.id);
        expect(reply.highlights[0]).toMatchObject({ session_title: 'Latest app review', session_mark: { glyph: '⌬⌬', fg_hex: '#64c896' }, additions: 12, deletions: 3, state: 'answered' });
        await tool('reply', { text: 'Duplicate automatic final', automatic: true });
        expect((await (await a.call()).json<any>()).messages.at(-1).id).toBe(reply.id);
        expect(reply.highlights[0].blocks).toEqual([{type: 'paragraph', text: 'iOS **2.0** in review.'}]);
        expect((await tool('reply', { text: 'Invalid identity', highlights: [{session_id: 'another-account-session', text: 'Anything'}] })).status).toBe(404);
        expect((await tool('inspect_session', { session_id: 'outside-account' })).status).toBe(404);
        const other = await account('conductor-moved-target');
        await env.DB.prepare('UPDATE sessions SET device_id = ? WHERE id = ?').bind(other.device, id).run();
        const moved = (await (await a.call()).json<any>()).tracks[0];
        expect(moved.connected).toBe(false);
        expect(moved.activity).toBeUndefined();
        expect(moved.screen).toBeUndefined();
        expect((await tool('inspect_session', { session_id: id })).status).toBe(404);
        expect((await tool('finish', {})).status).toBe(200);
        expect((await tool('finish', {})).status).toBe(200); // Lost HTTP response may retry completion.
        const next = await (await a.call('host', host, true)).json<any>();
        expect(next.turn.id).not.toBe(turn.id); // Reports arriving during the first turn still wake it.

    } finally { socket.close(); }
});

it('rejects input while a prompt or unsent remote draft owns the terminal', async () => {
    const stub = env.SESSION.get(env.SESSION.idFromName('conductor-attention'));
    const response = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
    const desktop = response.webSocket!; desktop.accept();
    const send = () => stub.fetch('https://internal/conductor-input', { method: 'POST', body: JSON.stringify({ task_id: crypto.randomUUID(), text: 'Follow-up' }) });
    try {
        expect((await send()).status).toBe(409);
        desktop.send(JSON.stringify({ type: 'heartbeat', conductor_command: "'crabigator'", timestamp: Date.now() }));
        desktop.send(JSON.stringify({ type: 'state', state: 'question' }));
        await expect.poll(async () => (await (await stub.fetch('https://internal/snapshot')).json<any>()).attention_pending).toBe(true);
        expect((await send()).status).toBe(409);
        desktop.send(JSON.stringify({ type: 'state', state: 'thinking' }));
        await expect.poll(async () => (await (await stub.fetch('https://internal/snapshot')).json<any>()).attention_pending).toBe(false);
        await stub.fetch('https://internal/draft', { method: 'POST', body: JSON.stringify({ text: 'My unfinished message' }) });
        expect((await send()).status).toBe(409);
    } finally { desktop.close(); }
});


it('retires old completed requests without losing unfinished work or imposing a lifetime task limit', async () => {
    const a = await account('conductor-history');
    const id = 'conductor-history-target';
    await env.DB.prepare('INSERT INTO sessions (id, device_id, client_session_id, cwd, platform) VALUES (?, ?, ?, ?, ?)').bind(id, a.device, 'local-history', '/tmp/history', 'codex').run();
    const store = env.SESSION_LIST.get(env.SESSION_LIST.idFromName('conductor:conductor-history'));
    await runInDurableObject(store, async (_instance, state) => {
        const track = (id: string, request_id: string, status: string) => ({ id, request_id, session_id: 'old-target', title: 'Status', instruction: 'Read status', state: status, updated_at: Date.now(), events: [] });
        const data = {
            messages: [ { id: 'unfinished', role: 'user', text: 'Keep working', at: 0 }, { id: 'recent', role: 'user', text: 'Check status', at: 1 } ],
            tracks: [track('unfinished-track', 'unfinished', 'working'), ...Array.from({ length: 105 }, (_, i) => track(`done-${i}`, 'recent', 'answered'))],
            revision: 2, handled: 0,
        };
        await state.storage.put({ 'conductor:chunks': 1, 'conductor:0': JSON.stringify(data) });
    });
    const { turn } = await (await a.call('host', { host_id: 'host', name: 'Computer', claim: true }, true)).json<any>();
    const dispatch = (request_id: string) => a.call('tool', { host_id: 'host', turn_id: turn.id, name: 'dispatch', arguments: { request_id, tasks: [{ session_id: id, title: 'Current status', instruction: 'Read status' }] } }, true);
    expect((await dispatch('recent')).status).toBe(200); // 105 finished tasks do not block new work.
    expect((await (await a.call()).json<any>()).tracks.some((t: any) => t.id === 'unfinished-track')).toBe(true);
    for (let i = 0; i < 110; i++) expect((await a.call('message', { id: `request-${i}`, text: `Status ${i}` })).status).toBe(200);
    expect((await dispatch('recent')).status).toBe(400); // Forgotten requests cannot be dispatched again.
    const snapshot = await (await a.call()).json<any>();
    const saved = await (await a.call('archive', { query: 'Status', status: 'answered' })).json<any>();
    expect(saved.total).toBe(105);
    expect(saved.entries).toHaveLength(30);
    const nextPage = await (await a.call('archive', { query: 'Status', status: 'answered', offset: saved.next_offset })).json<any>();
    expect(nextPage.entries.some((e: any) => saved.entries.some((old: any) => old.id === e.id))).toBe(false);
    expect((await a.call('message', { id: 'recent', text: 'Check status' })).status).toBe(200);
    expect((await a.call('message', { id: 'recent', text: 'Overwrite archived question' })).status).toBe(409);
    expect((await (await a.call()).json<any>()).messages).toHaveLength(51);
    const other = await account('conductor-archive-stranger');
    expect((await (await other.call('archive', { query: 'Status' })).json<any>()).total).toBe(0);
    expect(snapshot.messages).toHaveLength(51); // Latest 50 plus the unfinished request.
    expect(snapshot.messages.some((m: any) => m.id === 'unfinished')).toBe(true);
    expect(snapshot.tracks.map((t: any) => t.id)).toEqual(['unfinished-track']);
    const stopped = await (await a.call('tool', { host_id: 'host', turn_id: turn.id, name: 'stop_tracking', arguments: { task_id: 'unfinished-track' } }, true)).json<any>();
    expect(stopped.track.state).toBe('cancelled');
    expect(stopped.track.events[0].text).toContain('continues running');
});
