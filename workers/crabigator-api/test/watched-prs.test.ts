import { env } from 'cloudflare:test';
import { expect, it } from 'vitest';
import { hmacSign } from '../src/auth/tokens';
import { relayWatchedPrStats } from '../src/handlers/watched-prs';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

it('relays limit changes without replacing newer watched PR details', async () => {
    const testEnv = env as Env;
    await ensureAccountSchema(testEnv);
    await testEnv.DB.prepare(`
        CREATE TABLE IF NOT EXISTS watched_prs (
            group_key TEXT, owner TEXT, repo TEXT, number INTEGER,
            data TEXT, refreshed_at INTEGER DEFAULT 0,
            PRIMARY KEY (group_key, owner, repo, number)
        )
    `).run();
    const secret = 'a'.repeat(64);
    await testEnv.DB.prepare(`
        INSERT INTO devices (id, secret_hash, group_id) VALUES ('limit-device', ?, 'limit-group')
    `).bind(secret).run();
    await testEnv.DB.prepare(`
        INSERT INTO watched_prs (group_key, owner, repo, number)
        VALUES ('limit-group', 'o', 'r', 123)
    `).run();

    const relay = async (refreshed_at: number, fetch_limited: boolean, title = '') => {
        const timestamp = String(Date.now());
        const path = '/api/prs/watched/stats';
        const request = new Request(`https://example.com${path}`, {
            method: 'POST',
            headers: {
                'Content-Type': 'application/json',
                'X-Device-Id': 'limit-device',
                'X-Timestamp': timestamp,
                'X-Signature': await hmacSign(secret, `POST:${path}:${timestamp}`),
            },
            body: JSON.stringify({ prs: [{ owner: 'o', repo: 'r', number: 123,
                refreshed_at, fetch_limited, title }] }),
        });
        const response = await relayWatchedPrStats(request, testEnv);
        expect(response.status).toBe(200);
        return response.json();
    };
    const stored = async () => {
        const data = await testEnv.DB.prepare(`
            SELECT data FROM watched_prs WHERE group_key = 'limit-group'
        `).first<string>('data');
        return JSON.parse(data!);
    };

    expect(await relay(0, true)).toMatchObject({ updated: 1 });
    expect(await stored()).toMatchObject({ fetch_limited: true });
    expect(await relay(0, false)).toMatchObject({ updated: 1 });
    expect(await stored()).toMatchObject({ fetch_limited: false });
    expect(await relay(1000, false, 'Loaded PR')).toMatchObject({ updated: 1 });
    expect(await relay(0, true)).toMatchObject({ updated: 0 });
    expect(await stored()).toMatchObject({ refreshed_at: 1000, title: 'Loaded PR', fetch_limited: false });
});
