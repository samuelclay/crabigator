import { env, evictDurableObject } from 'cloudflare:test';
import { expect, it } from 'vitest';
import type { CloudToDesktopMessage, SessionEvent } from '../src/types/session';

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
        await expect.poll(async () => (await snapshot()).scrollback).toBe('A complete replacement\n');
        const result = await stub.fetch('https://internal/events', { headers: { Upgrade: 'websocket' } });
        viewer = result.webSocket!;
        const events: SessionEvent[] = [];
        viewer.addEventListener('message', e => { events.push(JSON.parse(String(e.data))); });
        viewer.accept();
        await expect.poll(() => events).toContainEqual({ type: 'scrollback_history', content: 'A complete replacement\n' });
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
