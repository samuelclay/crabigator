import { env } from 'cloudflare:test';
import { expect, it } from 'vitest';
import { hmacSign } from '../src/auth/tokens';
import { getPrBoard } from '../src/handlers/pr-board';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

it('returns session statistics for standalone sessions and ended PR sessions', async () => {
    const testEnv = env as Env;
    await ensureAccountSchema(testEnv);
    await testEnv.DB.batch([
        testEnv.DB.prepare('ALTER TABLE sessions ADD COLUMN work_seconds INTEGER'),
        testEnv.DB.prepare('ALTER TABLE sessions ADD COLUMN slack_threads TEXT'),
        testEnv.DB.prepare('CREATE TABLE pr_overrides (group_key TEXT, owner TEXT, repo TEXT, number INTEGER, scope_key TEXT, disposition TEXT)'),
        testEnv.DB.prepare('CREATE TABLE watched_prs (group_key TEXT, owner TEXT, repo TEXT, number INTEGER, url TEXT, added_at INTEGER, data TEXT, refreshed_at INTEGER)'),
    ]);
    const secret = 'b'.repeat(64);
    await testEnv.DB.prepare("INSERT INTO devices (id, secret_hash, group_id) VALUES ('stats-device', ?, 'stats-group')").bind(secret).run();
    for (const active of [0, 1]) {
        await testEnv.DB.prepare(`INSERT INTO sessions
            (id, device_id, client_session_id, cwd, platform, is_active, started_at, ended_at, last_seen_at,
             work_seconds, thinking_seconds, prompts, completions, prompts_changed_at, completions_changed_at)
            VALUES (?, 'stats-device', ?, '/project', 'claude', ?, 1000, ?, 1600, ?, 123, 7, 6, 1400, 1500)`)
            .bind(`stats-${active}`, `local-${active}`, active, active ? null : 1600, active ? null : 590).run();
    }
    const pr = { owner: 'o', repo: 'r', number: 1, url: 'https://github.com/o/r/pull/1', branch: 'feature', title: 'Work',
        state: 'OPEN', is_draft: false, additions: 1, deletions: 0, changed_files: 1, mergeable: 'MERGEABLE',
        merge_state_status: 'CLEAN', checks_passed: 0, checks_failed: 0, checks_pending: 0, checks_total: 0,
        created_here: true, primary: true, refreshed_at: 1500 };
    await testEnv.DB.prepare(`INSERT INTO session_prs (session_id, owner, repo, number, url, data, updated_at, is_primary)
        VALUES ('stats-0', 'o', 'r', 1, ?, ?, 1500, 1)`).bind(pr.url, JSON.stringify(pr)).run();
    const timestamp = String(Date.now());
    const path = '/api/prs/board';
    const response = await getPrBoard(new Request(`https://example.com${path}`, { headers: {
        'X-Device-Id': 'stats-device', 'X-Timestamp': timestamp,
        'X-Signature': await hmacSign(secret, `GET:${path}:${timestamp}`),
    } }), testEnv);
    const body = await response.json() as any;
    expect(response.status, JSON.stringify(body)).toBe(200);
    expect(body.sessions).toHaveLength(1);
    const shared = { started_at: 1000, last_seen_at: 1600, prompts_changed_at: 1400, completions_changed_at: 1500,
        stats: { thinking_seconds: 123, prompts: 7, completions: 6 } };
    expect(body.sessions[0]).toMatchObject({ ...shared, session_id: 'stats-1', active: true, ended_at: null,
        stats: { ...shared.stats, work_seconds: null } });
    expect(body.prs[0].sessions[0]).toMatchObject({ ...shared, session_id: 'stats-0', active: false, ended_at: 1600,
        stats: { ...shared.stats, work_seconds: 590 } });
});
