import { requireSessionAccess } from '../auth/middleware';
import { parseStoredSessionMark, sessionMarkFromSeed } from '../session-mark';
import { sharedSessionHtml } from '../dashboard/shared-session';
import type { Env } from '../types/env';

const privateHeaders = {
    'Cache-Control': 'private, no-store',
    'Referrer-Policy': 'no-referrer',
    'X-Robots-Tag': 'noindex, nofollow, noarchive',
    'X-Content-Type-Options': 'nosniff',
};

export function shareUrl(request: Request, sessionId: string): string {
    return new URL('/share/' + sessionId, request.url).href;
}

// Existing sessions acquire their link on the first owner click. Concurrent
// clicks and desktop re-registration must all resolve to the same token.
export async function ownerShareLink(request: Request, env: Env, params: Record<string, string>) {
    const access = await requireSessionAccess(request, env, params.id);
    if ('error' in access) return access.error;
    const row = await env.DB.prepare('UPDATE sessions SET share_token = COALESCE(share_token, id) WHERE id = ? RETURNING id')
        .bind(params.id).first<{ id: string }>();
    if (!row) return new Response('Session not found', { status: 404 });
    return Response.json({ url: shareUrl(request, row.id) }, { headers: privateHeaders });
}

export async function sharedSession(request: Request, env: Env, params: Record<string, string>) {
    const row = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/.test(params.token)
        && await env.DB.prepare(`SELECT id, client_session_id, cwd, platform, started_at, ended_at, is_active, session_mark
            FROM sessions WHERE share_token IS NOT NULL AND (id = ? OR share_token = ?)`).bind(params.token, params.token).first<{
                id: string; client_session_id: string; cwd: string; platform: string; started_at: number;
                ended_at: number | null; is_active: number; session_mark: string | null;
            }>();
    if (!row) return new Response('Shared session not found.', { status: 404, headers: privateHeaders });

    const url = new URL(request.url);
    if (params.token !== row.id) {
        url.pathname = '/share/' + row.id + (url.pathname.endsWith('/data') ? '/data' : '');
        return new Response(null, { status: 308, headers: { ...privateHeaders, Location: url.href } });
    }
    if (!url.pathname.endsWith('/data')) {
        return new Response(sharedSessionHtml, { headers: { ...privateHeaders, 'Content-Type': 'text/html; charset=utf-8',
            'Content-Security-Policy': "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'unsafe-inline'; connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'" } });
    }
    const internal = new URL('https://internal/shared-view');
    for (const key of ['generation', 'cursor']) internal.searchParams.set(key, url.searchParams.get(key) || '');
    const response = await env.SESSION.get(env.SESSION.idFromName(row.id)).fetch(internal.href);
    if (!response.ok) return new Response('Session unavailable', { status: 503, headers: privateHeaders });
    return Response.json({
        ...await response.json<object>(), id: row.id.slice(0, 8), project: row.cwd.split('/').filter(Boolean).pop() || row.cwd,
        platform: row.platform, started_at: row.started_at, ended_at: row.ended_at, ended: !row.is_active || row.ended_at !== null,
        mark: parseStoredSessionMark(row.session_mark) ?? sessionMarkFromSeed(row.client_session_id),
    }, { headers: privateHeaders });
}
