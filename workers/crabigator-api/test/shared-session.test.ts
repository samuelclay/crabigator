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
    const data = async (query = '') => (await SELF.fetch(base + '/data' + query)).json<any>();
    await expect.poll(async () => (await data()).screen).toBe('Finished');
    ws.close();
    await expect.poll(async () => (await data()).connected).toBe(false);
    await env.DB.prepare('UPDATE sessions SET ended_at = unixepoch(), is_active = 0 WHERE id = ?').bind(id).run();
    await evictDurableObject(stub);
    const page = await SELF.fetch(base);
    expect(page.status).toBe(200);
    expect(page.headers.get('Referrer-Policy')).toBe('no-referrer');
    let snapshot = await data();
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
    for (const invalid of [crypto.randomUUID(), id.slice(0, 8), token.slice(0, 8)]) expect((await SELF.fetch('https://example.com/share/' + invalid + '/data')).status).toBe(404);
    expect((await SELF.fetch(base + '/data', { method: 'POST' })).status).toBe(404);
    for (const action of ['key', 'answer', 'share']) {
        const response = await SELF.fetch('https://example.com/api/sessions/' + id + '/' + action,
            { method: 'POST', headers: { Authorization: 'Bearer ' + token } });
        expect(response.status).toBe(401);
    }
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
