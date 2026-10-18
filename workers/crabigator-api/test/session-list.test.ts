import { env } from 'cloudflare:test';
import { expect, it } from 'vitest';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';

it('keeps the desktop identity in the dashboard session list', async () => {
    const testEnv = env as unknown as Env;
    await ensureAccountSchema(testEnv);
    const stored = { glyph: '◖◆◗', fg: [255, 240, 224], bg: [106, 16, 56] };
    await testEnv.DB.prepare(`
        INSERT INTO devices (id, secret_hash, name, group_id)
        VALUES ('mark-device', 'test-secret', 'Demo Mac', 'mark-group')
    `).run();
    for (const [id, mark] of [['marked', JSON.stringify(stored)], ['legacy', null]]) {
        await testEnv.DB.prepare(`
            INSERT INTO sessions (id, device_id, client_session_id, cwd, platform, session_mark)
            VALUES (?, 'mark-device', ?, '/tmp/demo', 'claude', ?)
        `).bind(id, `local-${id}`, mark).run();
    }

    // This is the durable object used by GET /api/sessions, not the unused
    // listSessions handler, which already returned these fields.
    const stub = testEnv.SESSION_LIST.get(testEnv.SESSION_LIST.idFromName('mark-list-test'));
    const response = await stub.fetch('https://internal/sessions?group_id=mark-group');
    expect(response.status).toBe(200);
    const { sessions } = await response.json() as { sessions: Record<string, unknown>[] };
    expect(sessions.find(s => s.id === 'marked')).toMatchObject({
        client_session_id: 'local-marked',
        session_mark: stored,
    });
    expect(sessions.find(s => s.id === 'legacy')).toMatchObject({
        client_session_id: 'local-legacy',
        session_mark: null,
    });
    for (const session of sessions) {
        expect(session).not.toHaveProperty('device_id');
        expect(session).not.toHaveProperty('group_id');
    }
    await testEnv.DB.prepare(`INSERT INTO sessions
        (id, device_id, client_session_id, cwd, platform, state, last_seen_at)
        VALUES ('dead-desktop', 'mark-device', 'dead-local', '/tmp/demo', 'codex', 'thinking', 1600)`).run();
    // Repeated reads used to delete from memory, then re-add the same dead
    // session from D1 and return it anyway.
    for (let attempt = 0; attempt < 2; attempt++) {
        const roster = await (await stub.fetch('https://internal/sessions?group_id=mark-group')).json<any>();
        expect(roster.sessions.map((s: any) => s.id)).not.toContain('dead-desktop');
        expect(roster.sessions.map((s: any) => s.id)).toContain('marked');
    }

});
