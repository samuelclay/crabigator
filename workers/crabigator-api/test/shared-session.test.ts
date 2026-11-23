import { env, evictDurableObject, SELF } from 'cloudflare:test';
import { expect, it } from 'vitest';
import { ensureAccountSchema } from './schema';
import type { Env } from '../src/types/env';

it('serves one read-only archive by its share token after its desktop and object are gone', async () => {
    await ensureAccountSchema(env as unknown as Env);
    const id = crypto.randomUUID(), token = crypto.randomUUID();
    await env.DB.prepare("INSERT INTO devices (id, secret_hash, group_id) VALUES ('archive-device', 'demo', 'archive-group')").run();
    await env.DB.prepare("INSERT INTO sessions (id, share_token, cwd, platform, device_id, client_session_id) VALUES (?, ?, '/work/harbor', 'codex', 'archive-device', 'local')").bind(id, token).run();
    const base = 'https://example.com/share/' + id;
    const legacy = base.replace(id, token);
    for (const suffix of ['', '/data?generation=old&cursor=3']) {
        const redirect = await SELF.fetch(legacy + suffix, { redirect: 'manual' });
        expect(redirect.status).toBe(308);
        expect(redirect.headers.get('Location')).toBe(base + suffix);
    }
    const stub = env.SESSION.get(env.SESSION.idFromName(id));
    const ws = (await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } })).webSocket!;
    ws.accept();
    const send = (event: object) => ws.send(JSON.stringify(event));
    // Cross both the live buffer's 500 KiB limit and an archive page boundary.
    const history = '🦀 working\n'.repeat(60_000);
    send({ type: 'scrollback_history', content: 'old screen history' });
    send({ type: 'scrollback_history', content: history });
    send({ type: 'screen', content: 'Finished' });
    send({ type: 'changes', by_language: [] });
    send({ type: 'git', files: [], branch: 'harbor' });
    send({ type: 'title', title: 'Harbor' });
    await (await stub.fetch('https://internal/draft', { method: 'POST', body: JSON.stringify({ text: 'unsubmitted' }) })).text();
    const purgeRootCache = async () => {
        await caches.default.delete(new Request(base + '/data'));
        await caches.default.delete(new Request('https://example.com/__share-cache/meta/' + id));
    };
    const data = async (query = '') => (await SELF.fetch(base + '/data' + query)).json<any>();
    await expect.poll(async () => {
        await purgeRootCache();
        return (await data()).screen;
    }).toBe('Finished');
    ws.close();
    await expect.poll(async () => {
        await purgeRootCache();
        return (await data()).connected;
    }).toBe(false);
    await env.DB.prepare('UPDATE sessions SET ended_at = unixepoch(), is_active = 0 WHERE id = ?').bind(id).run();
    await evictDurableObject(stub);
    await purgeRootCache();
    const page = await SELF.fetch(base);
    expect(page.status).toBe(200);
    expect(page.headers.get('Referrer-Policy')).toBe('no-referrer');
    expect(page.headers.get('Cache-Control')).toBe('public, max-age=60, s-maxage=300');
    const firstData = await SELF.fetch(base + '/data');
    expect(firstData.headers.get('Cache-Control')).toBe('public, max-age=30, s-maxage=60');
    let snapshot = await firstData.json<any>();
    expect(snapshot).toMatchObject({ id: id.slice(0, 8), ended: true, screen: 'Finished', title: 'Harbor',
        git: { branch: 'harbor' }, changes: { by_language: [] } });
    expect(snapshot).not.toHaveProperty('draft');
    expect(snapshot).not.toHaveProperty('conductor_command');
    let transcript = snapshot.history.content;
    expect(snapshot.history.more).toBe(true);
    while (snapshot.history.more) {
        snapshot = await data('?generation=' + snapshot.history.generation + '&cursor=' + snapshot.history.cursor);
        transcript += snapshot.history.content;
    }
    expect(transcript === history).toBe(true);
    for (const invalid of [crypto.randomUUID(), id.slice(0, 8), token.slice(0, 8)]) {
        const missing = await SELF.fetch('https://example.com/share/' + invalid + '/data');
        expect(missing.status).toBe(404);
        expect(missing.headers.get('Cache-Control')).toBe('private, no-store');
    }
    expect((await SELF.fetch(base + '/data', { method: 'POST' })).status).toBe(404);
    for (const action of ['key', 'answer', 'share']) {
        const response = await SELF.fetch('https://example.com/api/sessions/' + id + '/' + action,
            { method: 'POST', headers: { Authorization: 'Bearer ' + token } });
        expect(response.status).toBe(401);
    }
});

it('caches shared responses at the edge and normalizes cache-busting queries', async () => {
    await ensureAccountSchema(env as unknown as Env);
    const id = crypto.randomUUID(), legacyToken = crypto.randomUUID();
    await env.DB.prepare("INSERT INTO devices (id, secret_hash, group_id) VALUES ('cache-device', 'demo', 'cache-group')").run();
    await env.DB.prepare("INSERT INTO sessions (id, share_token, cwd, platform, device_id, client_session_id) VALUES (?, ?, '/work/edge', 'claude', 'cache-device', 'edge-live')").bind(id, legacyToken).run();

    const base = 'https://example.com/share/' + id;
    const legacy = 'https://example.com/share/' + legacyToken;
    const stub = env.SESSION.get(env.SESSION.idFromName(id));
    const ws = (await stub.fetch('https://internal/connect', { headers: { Upgrade: 'websocket' } })).webSocket!;
    ws.accept();
    ws.send(JSON.stringify({ type: 'scrollback_history', content: 'line 1\n' }));
    ws.send(JSON.stringify({ type: 'screen', content: 'First frame' }));

    // Wait for DO to record the first frame directly without polluting edge cache.
    await expect.poll(async () => (await (await stub.fetch('https://internal/shared-view')).json<any>()).screen).toBe('First frame');

    // Populate edge cache for the live snapshot.
    const liveRes = await SELF.fetch(base + '/data');
    expect(liveRes.status).toBe(200);
    expect(liveRes.headers.get('Cache-Control')).toBe('public, max-age=0, s-maxage=2');
    expect(liveRes.headers.get('Referrer-Policy')).toBe('no-referrer');
    const firstBody = await liveRes.json<any>();
    expect(firstBody.screen).toBe('First frame');
    const generation = firstBody.history.generation;

    // Advance the DO state to a second frame.
    ws.send(JSON.stringify({ type: 'screen', content: 'Second frame' }));
    await expect.poll(async () => (await (await stub.fetch('https://internal/shared-view')).json<any>()).screen).toBe('Second frame');

    // Repeated fetches, extra query params, cache-bypass headers, malformed cursors, and
    // spoofed generations must all hit the normalized edge cache entry instead of the DO.
    const variants = [
        base + '/data',
        base + '/data?utm_source=slack&rand=9999',
        base + '/data?generation=' + generation + '&cursor=0',
        base + '/data?generation=' + generation + '&cursor=-5',
        base + '/data?generation=' + generation + '&cursor=01',
        base + '/data?generation=' + crypto.randomUUID() + '&cursor=1',
    ];
    for (const url of variants) {
        const res = await SELF.fetch(url, {
            headers: { 'Cache-Control': 'no-cache', Pragma: 'no-cache', Cookie: 'session=abc', Authorization: 'Bearer nope' },
        });
        expect(res.status).toBe(200);
        expect((await res.json<any>()).screen).toBe('First frame');
    }

    // Out-of-range cursors populate the tail snapshot once, then serve subsequent arbitrary
    // cursors from cache without hitting the DO again.
    const firstFar = await SELF.fetch(base + '/data?generation=' + generation + '&cursor=999999');
    expect(firstFar.status).toBe(200);
    expect(await firstFar.json<any>()).toMatchObject({
        screen: 'Second frame',
        history: { generation, after: 999999, cursor: 999999, more: false, content: '' },
    });

    ws.send(JSON.stringify({ type: 'screen', content: 'Third frame' }));
    await expect.poll(async () => (await (await stub.fetch('https://internal/shared-view')).json<any>()).screen).toBe('Third frame');

    const secondFar = await SELF.fetch(base + '/data?generation=' + generation + '&cursor=999998');
    expect(secondFar.status).toBe(200);
    expect(await secondFar.json<any>()).toMatchObject({
        screen: 'Second frame',
        history: { generation, after: 999998, cursor: 999998, more: false, content: '' },
    });

    // Populate HTML and legacy redirect caches, then delete the D1 row to prove cache hits skip D1.
    const page1 = await SELF.fetch(base);
    expect(page1.status).toBe(200);
    const redir1 = await SELF.fetch(legacy + '/data?generation=old&cursor=3', { redirect: 'manual' });
    expect(redir1.status).toBe(308);
    expect(redir1.headers.get('Location')).toBe(base + '/data?generation=old&cursor=3');
    expect(redir1.headers.get('Cache-Control')).toBe('public, max-age=300, s-maxage=300');

    // End a second session and cache its ended data snapshot before deleting from D1.
    const endedId = crypto.randomUUID();
    await env.DB.prepare("INSERT INTO sessions (id, share_token, cwd, platform, device_id, client_session_id, ended_at, is_active) VALUES (?, ?, '/work/done', 'codex', 'cache-device', 'edge-done', unixepoch(), 0)").bind(endedId, endedId).run();
    const endedRes1 = await SELF.fetch('https://example.com/share/' + endedId + '/data');
    expect(endedRes1.status).toBe(200);
    expect(endedRes1.headers.get('Cache-Control')).toBe('public, max-age=30, s-maxage=60');
    expect((await endedRes1.json<any>()).ended).toBe(true);

    ws.close();
    await env.DB.prepare('DELETE FROM sessions WHERE id IN (?, ?)').bind(id, endedId).run();

    const page2 = await SELF.fetch(base + '?fbclid=123', { headers: { 'Cache-Control': 'no-cache' } });
    expect(page2.status).toBe(200);
    expect(page2.headers.get('Referrer-Policy')).toBe('no-referrer');
    expect(await page2.text()).toContain('Shared session');

    const redir2 = await SELF.fetch(legacy + '/data?generation=other&cursor=99', { redirect: 'manual' });
    expect(redir2.status).toBe(308);
    expect(redir2.headers.get('Location')).toBe(base + '/data?generation=other&cursor=99');

    const endedRes2 = await SELF.fetch('https://example.com/share/' + endedId + '/data?utm_campaign=launch');
    expect(endedRes2.status).toBe(200);
    expect((await endedRes2.json<any>()).ended).toBe(true);
});

it('keeps the same link when a desktop retries registration', async () => {
    const { hmacSign } = await import('../src/auth/tokens');
    await ensureAccountSchema(env as unknown as Env);
    const device = crypto.randomUUID(), secret = crypto.randomUUID();
    await env.DB.prepare("INSERT INTO devices (id, secret_hash, group_id) VALUES (?, ?, 'share-test')").bind(device, secret).run();
    const ts = String(Date.now());
    const headers = { 'X-Device-Id': device, 'X-Timestamp': ts, 'X-Signature': await hmacSign(secret, 'POST:/api/sessions:' + ts) };
    const register = async () => SELF.fetch('https://example.com/api/sessions', {
        method: 'POST', headers, body: JSON.stringify({ client_session_id: 'retry', cwd: '/demo', platform: 'codex' }),
    });
    const registrations = await Promise.all([register(), register()]);
    expect(registrations.map(r => r.status).sort()).toEqual([200, 201]);
    const [first, retry] = await Promise.all(registrations.map(r => r.json<{ id: string; share_url: string }>()));
    expect(first).toEqual(retry);
    expect(first.share_url).toBe('https://example.com/share/' + first.id);
    const route = '/api/sessions/' + first.id + '/share';
    const owner = await SELF.fetch('https://example.com' + route, { method: 'POST',
        headers: { ...headers, 'X-Signature': await hmacSign(secret, 'POST:' + route + ':' + ts) } });
    expect(await owner.json()).toEqual({ url: first.share_url });
});
