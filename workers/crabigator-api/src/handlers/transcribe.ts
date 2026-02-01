import type { Env } from '../types/env';
import { requireMobileAuth } from '../auth/middleware';

export async function handleTranscribe(request: Request, env: Env): Promise<Response> {
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

    // Forward to OpenAI Whisper API
    const whisperForm = new FormData();
    whisperForm.append('file', audioFile as Blob);
    whisperForm.append('model', 'whisper-1');

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

        const result = await resp.json() as { text: string };
        return new Response(
            JSON.stringify({ text: result.text }),
            { headers: { 'Content-Type': 'application/json' } }
        );
    } catch (err) {
        console.error('Whisper API request failed:', err);
        return new Response(
            JSON.stringify({ error: 'Transcription request failed' }),
            { status: 502, headers: { 'Content-Type': 'application/json' } }
        );
    }
}
