import { env, evictDurableObject, runInDurableObject } from 'cloudflare:test';
import { expect, it } from 'vitest';
import { SessionDO } from '../src/durable-objects/SessionDO';
import type { CloudToDesktopMessage, SessionEvent } from '../src/types/session';

it('reconnects an older desktop that cannot refresh its missing screen', async () => {
    const stub = env.SESSION.get(env.SESSION.idFromName('legacy-screen-recovery'));
    const connect = async () => {
        const response = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
        const socket = response.webSocket!;
        socket.accept();
        return socket;
    };
    const desktop = await connect();
    const closed: number[] = [];
    desktop.addEventListener('close', event => { closed.push(event.code); });
    const response = await stub.fetch('https://internal/events', { headers: { Upgrade: 'websocket' } });
    const viewer = response.webSocket!;
    const events: SessionEvent[] = [];
    viewer.addEventListener('message', event => { events.push(JSON.parse(String(event.data))); });
    viewer.accept();
    let replacement: WebSocket | undefined;
    try {
        // Recovery must retain its deadline even if the object hibernates.
        await evictDurableObject(stub);
        await stub.fetch('https://internal/viewer-active', { method: 'POST' });
        expect(closed).toEqual([]);
        await new Promise(resolve => setTimeout(resolve, 10_100));
        await stub.fetch('https://internal/viewer-active', { method: 'POST' });
        await expect.poll(() => closed).toContain(1012);
        await expect.poll(() => events.some(event => event.type === 'desktop_status' && !event.connected)).toBe(true);
        events.length = 0;
        replacement = await connect();
        await expect.poll(() => events.some(event => event.type === 'desktop_status' && event.connected)).toBe(true);
        replacement.send(JSON.stringify({ type: 'screen', content: 'Recovered idle screen' }));
        await expect.poll(() => events).toContainEqual({ type: 'screen', content: 'Recovered idle screen' });
        // A valid empty response also completes recovery (capture may be disabled).
        replacement.send(JSON.stringify({ type: 'screen', content: '' }));
        await expect.poll(() => events).toContainEqual({ type: 'screen', content: '' });
        await stub.fetch('https://internal/viewer-active', { method: 'POST' });
        expect(replacement.readyState).toBe(WebSocket.OPEN);
    } finally { viewer.close(); replacement?.close(); desktop.close(); }
}, 20_000);

it('replays full transcripts to late viewers and replaces history on resync', async () => {
    const stub = env.SESSION.get(env.SESSION.idFromName('transcript-replay'));
    const response = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
    const desktop = response.webSocket!;
    desktop.accept();
    const snapshot = async () => (await stub.fetch('https://internal/snapshot')).json<{ scrollback: string }>();
    let viewer: WebSocket | undefined;
    try {
        desktop.send(JSON.stringify({ type: 'scrollback_history', content: 'First prompt\nFirst answer\n' }));
        await expect.poll(async () => (await snapshot()).scrollback).toBe('First prompt\nFirst answer\n');
        desktop.send(JSON.stringify({ type: 'scrollback', diff: 'Next prompt\n', total_lines: 3 }));
        await expect.poll(async () => (await snapshot()).scrollback).toContain('Next prompt');
        desktop.send(JSON.stringify({ type: 'scrollback_history', content: 'A complete replacement\n' }));
        desktop.send(JSON.stringify({ type: 'screen', content: 'Small screen first' }));
        await expect.poll(async () => (await snapshot()).scrollback).toBe('A complete replacement\n');
        const result = await stub.fetch('https://internal/events', { headers: { Upgrade: 'websocket' } });
        viewer = result.webSocket!;
        const events: SessionEvent[] = [];
        viewer.addEventListener('message', e => { events.push(JSON.parse(String(e.data))); });
        viewer.accept();
        await expect.poll(() => events).toContainEqual({ type: 'scrollback_history', content: 'A complete replacement\n' });
        expect(events.findIndex(event => event.type === 'screen')).toBeGreaterThanOrEqual(0);
        expect(events.findIndex(event => event.type === 'screen')).toBeLessThan(events.findIndex(event => event.type === 'scrollback_history'));
        desktop.send(JSON.stringify({ type: 'scrollback_history', content: 'old\n'.repeat(150000) + 'last line\n' }));
        await expect.poll(async () => (await snapshot()).scrollback.endsWith('last line\n')).toBe(true);
        await expect.poll(async () => (await snapshot()).scrollback.length).toBeLessThanOrEqual(500 * 1024);
    } finally { viewer?.close(); desktop.close(); }
});

it('recovers an idle screen after hibernation and retries until the desktop replies', async () => {
    const sessions = env.SESSION as DurableObjectNamespace;
    const stub = sessions.get(sessions.idFromName('screen-recovery'));
    const desktopResponse = await stub.fetch('https://internal/connect', {
        headers: { Upgrade: 'websocket' },
    });
    const desktop = desktopResponse.webSocket!;
    desktop.accept();
    const requests: CloudToDesktopMessage[] = [];
    desktop.addEventListener('message', (event) => {
        requests.push(JSON.parse(String(event.data)));
    });
    const viewers: WebSocket[] = [];
    const heartbeat = async () => (await stub.fetch('https://internal/viewer-active', { method: 'POST' })).json();
    const state = async () => (await stub.fetch('https://internal/state')).json<{ has_screen: boolean }>();
    const refreshRequests = () => requests.filter(
        (message) => message.type === 'viewer_status' && message.refresh_screen,
    );
    const connectViewer = async () => {
        const response = await stub.fetch('https://internal/events', {
            headers: { Upgrade: 'websocket' },
        });
        const viewer = response.webSocket!;
        const events: SessionEvent[] = [];
        viewer.addEventListener('message', (event) => {
            events.push(JSON.parse(String(event.data)));
        });
        viewer.accept();
        viewers.push(viewer);
        return events;
    };

    try {
        const screen = { type: 'screen', content: 'An idle terminal with no new output' };
        desktop.send(JSON.stringify(screen));
        await expect.poll(async () => (await state()).has_screen).toBe(true);
        const existingViewer = await connectViewer();
        await expect.poll(() => existingViewer).toContainEqual(screen);
        await heartbeat();
        await expect.poll(() => requests).toContainEqual({
            type: 'viewer_status', active: true, refresh_screen: false,
        });

        // The desktop still considers viewers active; hibernation preserves
        // both sockets but drops the cloud's in-memory screen.
        await evictDurableObject(stub);
        expect((await state()).has_screen).toBe(false);
        const newViewer = await connectViewer();
        await expect.poll(() => refreshRequests().length).toBe(1);

        // A missing response must not leave the pane waiting forever.
        await heartbeat();
        await expect.poll(() => refreshRequests().length).toBe(2);
        desktop.send(JSON.stringify(screen));
        await expect.poll(() => newViewer).toContainEqual(screen);
        await expect.poll(() => existingViewer.filter(event => event.type === 'screen').length).toBe(2);

        requests.length = 0;
        await heartbeat();
        await expect.poll(() => requests).toContainEqual({
            type: 'viewer_status', active: true, refresh_screen: false,
        });
        expect(refreshRequests()).toHaveLength(0);

        // An empty frame is still a response, including with capture disabled.
        const emptyScreen = { type: 'screen', content: '' };
        desktop.send(JSON.stringify(emptyScreen));
        await expect.poll(() => newViewer).toContainEqual(emptyScreen);
        const emptyViewer = await connectViewer();
        await expect.poll(() => emptyViewer).toContainEqual(emptyScreen);
        requests.length = 0;
        await heartbeat();
        await expect.poll(() => requests).toContainEqual({
            type: 'viewer_status', active: true, refresh_screen: false,
        });
        expect(refreshRequests()).toHaveLength(0);
    } finally {
        for (const viewer of viewers) viewer.close();
        desktop.close();
    }
});

it('sends input to the newest desktop socket after a wake, not one still closing', async () => {
    const stub = env.SESSION.get(env.SESSION.idFromName('stale-desktop-socket'));
    const connect = async () => (await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } })).webSocket!;
    // The first socket is never accepted, so it never answers the server's
    // close, like a desktop that slept or changed networks and reconnected.
    const stale = await connect();
    const live = await connect();
    live.accept();
    const received: CloudToDesktopMessage[] = [];
    live.addEventListener('message', event => { received.push(JSON.parse(String(event.data))); });
    try {
        // A wake builds a new instance over the same sockets and storage.
        const status = await runInDurableObject(stub, async (_instance, state) => {
            const woken = new SessionDO(state, env);
            await state.blockConcurrencyWhile(async () => {});
            const response = await woken.fetch(new Request('https://internal/answer', { method: 'POST', body: JSON.stringify({ text: 'hello' }) }));
            return response.status;
        });
        expect(status).toBe(200);
        await expect.poll(() => received).toContainEqual({ type: 'answer', text: 'hello' });
    } finally { live.close(); }
});


it('ends a disconnected desktop after the reconnect grace period, including across hibernation', async () => {
    const { ensureAccountSchema } = await import('./schema');
    await ensureAccountSchema(env);
    const id = 'disconnected-cleanup';
    await env.DB.prepare(`INSERT INTO devices (id, secret_hash, group_id) VALUES ('cleanup-device', 'test', 'cleanup-group')`).run();
    await env.DB.prepare(`INSERT INTO sessions (id, device_id, client_session_id, cwd, platform, state)
        VALUES (?, 'cleanup-device', 'cleanup-local', '/tmp/demo', 'codex', 'thinking')`).bind(id).run();
    const stub = env.SESSION.get(env.SESSION.idFromName(id));
    const connect = async () => {
        const response = await stub.fetch(`https://internal/connect?sessionId=${id}&group_id=cleanup-group&device_id=cleanup-device&device_name=Mac`, { headers: { Upgrade: 'websocket' } });
        const socket = response.webSocket!;
        socket.accept();
        return socket;
    };
    const desktop = await connect();
    desktop.close();
    await expect.poll(async () => (await (await stub.fetch('https://internal/state')).json<any>()).desktop_connected).toBe(false);
    await runInDurableObject(stub, async (instance: SessionDO, state) => {
        await instance.alarm();
        expect(await state.storage.getAlarm()).not.toBeNull();
    });
    expect(await env.DB.prepare('SELECT is_active FROM sessions WHERE id = ?').bind(id).first('is_active')).toBe(1);
    await runInDurableObject(stub, async (_instance, state) => {
        await state.storage.put('disconnectedAt', Date.now() - 61_000);
    });
    await evictDurableObject(stub);
    await runInDurableObject(stub, async (instance: SessionDO) => { await instance.alarm(); });
    expect(await env.DB.prepare('SELECT is_active FROM sessions WHERE id = ?').bind(id).first('is_active')).toBe(0);
    const reconnected = await connect();
    try {
        await expect.poll(async () => env.DB.prepare('SELECT is_active FROM sessions WHERE id = ?').bind(id).first('is_active')).toBe(1);
        await runInDurableObject(stub, async (instance: SessionDO) => { await instance.alarm(); });
        expect(await env.DB.prepare('SELECT is_active FROM sessions WHERE id = ?').bind(id).first('is_active')).toBe(1);
        expect((await (await stub.fetch('https://internal/state')).json<any>()).disconnected_at).toBeNull();
    } finally { reconnected.close(); }
});

it('hides old empty recaps in live events, history, and restored snapshots', async () => {
    const stub = env.SESSION.get(env.SESSION.idFromName('empty-recaps'));
    const response = await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } });
    const desktop = response.webSocket!;
    desktop.accept();
    const connected = await stub.fetch('https://internal/events', { headers: { Upgrade: 'websocket' } });
    const viewer = connected.webSocket!;
    const events: any[] = [];
    viewer.addEventListener('message', event => { events.push(JSON.parse(String(event.data))); });
    viewer.accept();
    const empty = { headline: 'The agent completed the turn without captured transcript output.', generated_at: 1000 };
    const useful = { headline: 'Fixed session loading', generated_at: 2000 };
    const snapshot = async () => (await stub.fetch('https://internal/snapshot')).json<any>();
    try {
        desktop.send(JSON.stringify({ type: 'recap', status: 'ready', latest: empty }));
        desktop.send(JSON.stringify({ type: 'recap_history', history: [useful, empty] }));
        await expect.poll(() => events.some(e => e.type === 'recap' && e.status === 'waiting' && !e.latest)).toBe(true);
        await expect.poll(async () => (await snapshot()).recap_history).toEqual([useful]);
        // Seed the pre-upgrade storage shape to exercise hibernation migration too.
        await runInDurableObject(stub, async (_instance, state) => {
            const stored = await state.storage.get<any>('persistentState');
            await state.storage.put('persistentState', { ...stored, lastRecap: { type: 'recap', status: 'ready', latest: empty }, lastRecapHistory: [empty, useful] });
        });
        await evictDurableObject(stub);
        expect((await snapshot()).recap.latest).toBeUndefined();
        expect((await snapshot()).recap_history).toEqual([useful]);
    } finally { viewer.close(); desktop.close(); }
});
