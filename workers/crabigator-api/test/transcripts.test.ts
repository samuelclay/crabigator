import { env } from 'cloudflare:test';
import { afterEach, beforeAll, expect, it, vi } from 'vitest';
import { sha256 } from '../src/auth/tokens';
import { deleteVoiceTranscript, handleTranscribe, listVoiceTranscripts } from '../src/handlers/transcribe';
import type { Env } from '../src/types/env';
import { ensureAccountSchema } from './schema';
import migration from '../migrations/0028_voice_transcripts.sql?raw';

const testEnv = { ...(env as unknown as Env), OPENAI_API_KEY: 'test-key' } as Env;

beforeAll(async () => {
    await ensureAccountSchema(testEnv);
    for (const sql of migration.replace(/^--.*$/gm, '').split(';').map(s => s.trim()).filter(Boolean)) await testEnv.DB.prepare(sql).run();
});
afterEach(() => vi.unstubAllGlobals());

async function paired(name: string, group: string) {
    const token = `${name}-credential`;
    const hash = await sha256(token);
    await testEnv.DB.prepare('INSERT INTO devices (id, secret_hash, group_id) VALUES (?, ?, ?)')
        .bind(`${name}-desktop`, 'test-secret', group).run();
    await testEnv.DB.prepare(`INSERT INTO linked_devices (id, desktop_id, mobile_id, mobile_token_hash)
        VALUES (?, ?, ?, ?)`).bind(name, `${name}-desktop`, name, hash).run();
    await testEnv.TOKENS.put(`mobile:${hash}`, JSON.stringify({ desktop_id: `${name}-desktop`, mobile_id: name, group_id: group }));
    return (path: string, init: RequestInit = {}) => new Request(`https://example.com${path}`, {
        ...init, headers: { Authorization: `Bearer ${token}` },
    });
}

function recording(fields: Record<string, string>): FormData {
    const form = new FormData();
    form.append('file', new Blob(['audio'], { type: 'audio/mp4' }), 'recording.m4a');
    for (const [name, value] of Object.entries(fields)) form.append(name, value);
    return form;
}

it('saves each transcript for its account and lets the owner list and delete it', async () => {
    const owner = await paired('voice-owner', 'voice-group');
    const stranger = await paired('voice-stranger', 'other-group');
    let sent: FormData | undefined;
    vi.stubGlobal('fetch', async (_url: string, init: RequestInit) => {
        sent = init.body as FormData;
        return Response.json({ text: ' Ship the fix. ', duration: 91.5 });
    });

    const response = await handleTranscribe(owner('/api/transcribe', { method: 'POST',
        body: recording({ session_id: 'session-1', session_title: 'Fix voice', client: 'ios', recorded_seconds: '92' }) }), testEnv);
    const result = await response.json() as { text: string; id: string; audio_seconds: number };
    expect(response.status).toBe(200);
    expect(sent?.get('response_format')).toBe('verbose_json');
    expect(result).toMatchObject({ text: ' Ship the fix. ', audio_seconds: 91.5 });

    const list = async (request: Request) => (await (await listVoiceTranscripts(request, testEnv)).json() as { transcripts: unknown[] }).transcripts;
    expect(await list(owner('/api/transcripts'))).toEqual([expect.objectContaining({
        id: result.id, session_id: 'session-1', session_title: 'Fix voice', client: 'ios',
        text: 'Ship the fix.', recorded_seconds: 92, audio_seconds: 91.5,
    })]);
    expect(await list(owner('/api/transcripts?session_id=session-1'))).toHaveLength(1);
    expect(await list(owner('/api/transcripts?session_id=session-2'))).toHaveLength(0);
    expect(await list(stranger('/api/transcripts'))).toHaveLength(0);

    await deleteVoiceTranscript(stranger(`/api/transcripts/${result.id}`, { method: 'DELETE' }), testEnv, { id: result.id });
    expect(await list(owner('/api/transcripts'))).toHaveLength(1);
    await deleteVoiceTranscript(owner(`/api/transcripts/${result.id}`, { method: 'DELETE' }), testEnv, { id: result.id });
    expect(await list(owner('/api/transcripts'))).toHaveLength(0);
});

it('saves nothing when no speech was heard', async () => {
    const owner = await paired('voice-silent', 'silent-group');
    vi.stubGlobal('fetch', async () => Response.json({ text: '', duration: 3 }));
    const response = await handleTranscribe(owner('/api/transcribe', { method: 'POST', body: recording({}) }), testEnv);
    expect(await response.json()).toMatchObject({ text: '', id: null });
});
