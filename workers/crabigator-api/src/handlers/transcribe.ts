import type { Env } from '../types/env';
import { requireMobileAuth } from '../auth/middleware';
import { jsonResponse } from '../router';

/** A short label from the client, or null when it sent none. */
function formLabel(form: FormData, name: string, max: number): string | null {
    const value = form.get(name);
    return typeof value === 'string' && value.trim() ? value.trim().slice(0, max) : null;
}

function formSeconds(form: FormData, name: string): number | null {
    const value = Number(form.get(name));
    return Number.isFinite(value) && value > 0 && value < 86400 ? value : null;
}

export async function handleTranscribe(
    request: Request,
    env: Env,
    _params?: Record<string, string>,
    ctx?: ExecutionContext,
): Promise<Response> {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }

    if (!env.OPENAI_API_KEY) {
        return new Response(
            JSON.stringify({ error: 'Transcription not configured' }),
            { status: 503, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const contentType = request.headers.get('Content-Type') || '';
    if (!contentType.includes('multipart/form-data')) {
        return new Response(
            JSON.stringify({ error: 'Expected multipart/form-data' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    let formData: FormData;
    try {
        formData = await request.formData();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid form data' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const audioFile = formData.get('file');
    if (!audioFile || typeof audioFile === 'string') {
        return new Response(
            JSON.stringify({ error: 'Missing audio file' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // A phone that gives up waiting must not stop the transcript from being saved.
    const answer = transcribeAndSave(env, authResult.auth.group_id, formData, audioFile);
    ctx?.waitUntil(answer);
    return answer;
}

async function transcribeAndSave(env: Env, groupId: string, formData: FormData, audioFile: File): Promise<Response> {
    // Forward to OpenAI Whisper API
    const whisperForm = new FormData();
    whisperForm.append('file', audioFile);
    whisperForm.append('model', 'whisper-1');
    // The verbose format also reports how much audio Whisper received.
    whisperForm.append('response_format', 'verbose_json');

    try {
        const resp = await fetch('https://api.openai.com/v1/audio/transcriptions', {
            method: 'POST',
            headers: {
                'Authorization': `Bearer ${env.OPENAI_API_KEY}`,
            },
            body: whisperForm,
        });

        if (!resp.ok) {
            const err = await resp.text();
            console.error('Whisper API error:', resp.status, err);
            return new Response(
                JSON.stringify({ error: 'Transcription failed' }),
                { status: 502, headers: { 'Content-Type': 'application/json' } }
            );
        }

        const result = await resp.json() as { text: string; duration?: number };
        const text = (result.text || '').trim();
        const audioSeconds = typeof result.duration === 'number' ? result.duration : null;
        const recordedSeconds = formSeconds(formData, 'recorded_seconds');
        const client = formLabel(formData, 'client', 32);
        if (audioSeconds !== null && recordedSeconds !== null && recordedSeconds - audioSeconds > 5) {
            console.error('Voice recording lost audio before upload:', JSON.stringify({
                client, recorded_seconds: recordedSeconds, audio_seconds: audioSeconds, bytes: audioFile.size,
            }));
        }
        // Save before answering, so the transcript survives whatever the client does next.
        let id: string | null = null;
        if (text) {
            const newId = crypto.randomUUID();
            try {
                await env.DB.prepare(`INSERT INTO voice_transcripts
                    (id, group_id, session_id, session_title, client, text, recorded_seconds, audio_seconds, created_at)
                    VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)`)
                    .bind(newId, groupId, formLabel(formData, 'session_id', 64),
                        formLabel(formData, 'session_title', 200), client,
                        text, recordedSeconds, audioSeconds, Math.floor(Date.now() / 1000))
                    .run();
                id = newId;
            } catch (err) {
                console.error('Could not save voice transcript:', err);
            }
        }
        return jsonResponse({ text: result.text, id, audio_seconds: audioSeconds });
    } catch (err) {
        console.error('Whisper API request failed:', err);
        return new Response(
            JSON.stringify({ error: 'Transcription request failed' }),
            { status: 502, headers: { 'Content-Type': 'application/json' } }
        );
    }
}

interface VoiceTranscriptRow {
    cursor: number;
    id: string;
    session_id: string | null;
    session_title: string | null;
    client: string | null;
    text: string;
    recorded_seconds: number | null;
    audio_seconds: number | null;
    created_at: number;
}

const TRANSCRIPT_PAGE_SIZE = 50;

/** The account's saved voice transcripts, newest first, one page at a time. */
export async function listVoiceTranscripts(request: Request, env: Env): Promise<Response> {
    const auth = await requireMobileAuth(request, env);
    if ('error' in auth) return auth.error;
    const params = new URL(request.url).searchParams;
    const cursor = Number(params.get('before') || Number.MAX_SAFE_INTEGER);
    if (!Number.isSafeInteger(cursor) || cursor < 1) return jsonResponse({ error: 'Invalid history cursor' }, 400);
    const sessionId = params.get('session_id');
    const rows = await env.DB.prepare(`SELECT rowid AS cursor, id, session_id, session_title, client, text,
        recorded_seconds, audio_seconds, created_at FROM voice_transcripts
        WHERE group_id = ? AND rowid < ? AND (? IS NULL OR session_id = ?)
        ORDER BY rowid DESC LIMIT ?`)
        .bind(auth.auth.group_id, cursor, sessionId, sessionId, TRANSCRIPT_PAGE_SIZE).all<VoiceTranscriptRow>();
    const transcripts = rows.results.map(({ cursor: _cursor, ...row }) => row);
    const full = rows.results.length === TRANSCRIPT_PAGE_SIZE;
    return jsonResponse({ transcripts, next_cursor: full ? rows.results.at(-1)!.cursor : null });
}

export async function deleteVoiceTranscript(request: Request, env: Env, params: Record<string, string>): Promise<Response> {
    const auth = await requireMobileAuth(request, env);
    if ('error' in auth) return auth.error;
    await env.DB.prepare('DELETE FROM voice_transcripts WHERE id = ? AND group_id = ?')
        .bind(params.id, auth.auth.group_id).run();
    return jsonResponse({ ok: true });
}
