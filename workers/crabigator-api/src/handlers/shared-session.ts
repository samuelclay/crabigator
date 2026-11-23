import { requireSessionAccess } from '../auth/middleware';
import { parseStoredSessionMark, sessionMarkFromSeed } from '../session-mark';
import { sharedSessionHtml } from '../dashboard/shared-session';
import type { Env } from '../types/env';

const UUID_V4 = /^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/;

const securityHeaders = {
    'Referrer-Policy': 'no-referrer',
    'X-Robots-Tag': 'noindex, nofollow, noarchive',
    'X-Content-Type-Options': 'nosniff',
};

const privateHeaders = {
    'Cache-Control': 'private, no-store',
    ...securityHeaders,
};

const sharedHtmlCsp = "default-src 'none'; script-src 'self' 'wasm-unsafe-eval'; style-src 'unsafe-inline' https://fonts.googleapis.com; font-src https://fonts.gstatic.com; connect-src 'self'; img-src 'self'; base-uri 'none'; form-action 'none'; frame-ancestors 'none'";

const HTML_BROWSER_TTL = 60;
const HTML_EDGE_TTL = 300;
const REDIRECT_TTL = 300;
const LIVE_DATA_TTL = 2;
const ENDED_DATA_BROWSER_TTL = 30;
const ENDED_DATA_EDGE_TTL = 60;

interface SharedSessionRow {
    id: string;
    client_session_id: string;
    cwd: string;
    platform: string;
    started_at: number;
    ended_at: number | null;
    is_active: number;
    session_mark: string | null;
}

interface SharedViewPayload {
    connected?: boolean;
    history?: {
        generation: string;
        after: number;
        cursor: number;
        more: boolean;
        content: string;
    };
    [key: string]: unknown;
}

type ShareCacheMeta =
    | { redirectTo: string }
    | { generation: string; latestCursor: number };

export function shareUrl(request: Request, sessionId: string): string {
    return new URL('/share/' + sessionId, request.url).href;
}

function pageCacheKey(origin: string, token: string): Request {
    return new Request(`${origin}/share/${token}`, { method: 'GET' });
}

function dataCacheKey(origin: string, token: string, generation: string, cursor: number): Request {
    const href = cursor > 0
        ? `${origin}/share/${token}/data?generation=${generation}&cursor=${cursor}`
        : `${origin}/share/${token}/data`;
    return new Request(href, { method: 'GET' });
}

function metaCacheKey(origin: string, token: string): Request {
    return new Request(`${origin}/__share-cache/meta/${token}`, { method: 'GET' });
}

function parseArchiveCursor(searchParams: URLSearchParams): { generation: string; cursor: number } {
    const rawGen = searchParams.get('generation') || '';
    const rawCur = searchParams.get('cursor') || '';
    if ((rawGen === 'initial' || UUID_V4.test(rawGen)) && /^[1-9]\d*$/.test(rawCur)) {
        const cursor = Number(rawCur);
        if (Number.isSafeInteger(cursor) && cursor > 0) {
            return { generation: rawGen, cursor };
        }
    }
    return { generation: '', cursor: 0 };
}

function cachePut(ctx: ExecutionContext | undefined, key: Request, response: Response): void {
    const write = caches.default.put(key, response).catch(() => {});
    if (ctx) ctx.waitUntil(write);
}

function cacheMeta(ctx: ExecutionContext | undefined, origin: string, token: string, meta: ShareCacheMeta, ttl: number): void {
    cachePut(
        ctx,
        metaCacheKey(origin, token),
        Response.json(meta, { headers: { 'Cache-Control': `public, max-age=${ttl}, s-maxage=${ttl}` } }),
    );
}

function redirectResponse(url: URL, targetId: string, isData: boolean): Response {
    const target = new URL('/share/' + targetId + (isData ? '/data' : '') + url.search, url.origin);
    return new Response(null, {
        status: 308,
        headers: {
            ...securityHeaders,
            'Cache-Control': `public, max-age=${REDIRECT_TTL}, s-maxage=${REDIRECT_TTL}`,
            Location: target.href,
        },
    });
}

async function findSharedSession(env: Env, token: string): Promise<SharedSessionRow | null> {
    return env.DB.prepare(`SELECT id, client_session_id, cwd, platform, started_at, ended_at, is_active, session_mark
        FROM sessions WHERE share_token IS NOT NULL AND (id = ? OR share_token = ?)`)
        .bind(token, token)
        .first<SharedSessionRow>();
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

export async function sharedSession(
    request: Request,
    env: Env,
    params: Record<string, string>,
    ctx?: ExecutionContext,
): Promise<Response> {
    if (!UUID_V4.test(params.token)) {
        return new Response('Shared session not found.', { status: 404, headers: privateHeaders });
    }

    const url = new URL(request.url);
    const isData = url.pathname.endsWith('/data');

    if (!isData) {
        const key = pageCacheKey(url.origin, params.token);
        const cached = await caches.default.match(key);
        if (cached) {
            if (cached.status === 308 && url.search) {
                const location = cached.headers.get('Location');
                if (location) {
                    const target = new URL(location);
                    target.search = url.search;
                    const headers = new Headers(cached.headers);
                    headers.set('Location', target.href);
                    return new Response(null, { status: 308, headers });
                }
            }
            return cached;
        }

        const row = await findSharedSession(env, params.token);
        if (!row) return new Response('Shared session not found.', { status: 404, headers: privateHeaders });

        if (params.token !== row.id) {
            const cleanTarget = new URL('/share/' + params.token, url.origin);
            cachePut(ctx, key, redirectResponse(cleanTarget, row.id, false));
            cacheMeta(ctx, url.origin, params.token, { redirectTo: row.id }, REDIRECT_TTL);
            return redirectResponse(url, row.id, false);
        }

        const pageHeaders = {
            ...securityHeaders,
            'Cache-Control': `public, max-age=${HTML_BROWSER_TTL}, s-maxage=${HTML_EDGE_TTL}`,
            'Content-Type': 'text/html; charset=utf-8',
            'Content-Security-Policy': sharedHtmlCsp,
        };
        cachePut(ctx, key, new Response(sharedSessionHtml, { headers: pageHeaders }));
        return new Response(sharedSessionHtml, { headers: pageHeaders });
    }

    // Normalize archive cursor and ignore extra query params or client headers so
    // viewers share edge cache entries and cache-busting queries cannot hit the DO.
    const { generation, cursor } = parseArchiveCursor(url.searchParams);
    const key = dataCacheKey(url.origin, params.token, generation, cursor);
    const cached = await caches.default.match(key);
    if (cached) return cached;

    const metaRes = await caches.default.match(metaCacheKey(url.origin, params.token));
    const meta = metaRes ? await metaRes.json<ShareCacheMeta>() : null;
    if (meta && 'redirectTo' in meta) {
        return redirectResponse(url, meta.redirectTo, true);
    }
    if (meta && cursor > 0) {
        if (generation !== meta.generation) {
            const rootCached = await caches.default.match(dataCacheKey(url.origin, params.token, '', 0));
            if (rootCached) return rootCached;
        } else if (cursor > meta.latestCursor) {
            const tailKey = dataCacheKey(
                url.origin,
                params.token,
                meta.latestCursor > 0 ? meta.generation : '',
                meta.latestCursor,
            );
            const tailCached = await caches.default.match(tailKey);
            if (tailCached) {
                const body = await tailCached.json<Record<string, unknown>>();
                body.history = { generation: meta.generation, after: cursor, cursor, more: false, content: '' };
                const headers = new Headers(securityHeaders);
                const cacheControl = tailCached.headers.get('Cache-Control');
                if (cacheControl) headers.set('Cache-Control', cacheControl);
                return Response.json(body, { headers });
            }
        }
    }

    const row = await findSharedSession(env, params.token);
    if (!row) return new Response('Shared session not found.', { status: 404, headers: privateHeaders });

    if (params.token !== row.id) {
        cacheMeta(ctx, url.origin, params.token, { redirectTo: row.id }, REDIRECT_TTL);
        return redirectResponse(url, row.id, true);
    }

    const internal = new URL('https://internal/shared-view');
    internal.searchParams.set('generation', generation);
    internal.searchParams.set('cursor', String(cursor));
    const response = await env.SESSION.get(env.SESSION.idFromName(row.id)).fetch(internal.href);
    if (!response.ok) return new Response('Session unavailable', { status: 503, headers: privateHeaders });

    const view = await response.json<SharedViewPayload>();
    const ended = !row.is_active || row.ended_at !== null;
    const immutable = ended && !view.connected;
    const edgeTtl = immutable ? ENDED_DATA_EDGE_TTL : LIVE_DATA_TTL;
    const cacheControl = immutable
        ? `public, max-age=${ENDED_DATA_BROWSER_TTL}, s-maxage=${ENDED_DATA_EDGE_TTL}`
        : `public, max-age=0, s-maxage=${LIVE_DATA_TTL}`;
    const headers = {
        ...securityHeaders,
        'Cache-Control': cacheControl,
        'Content-Type': 'application/json',
    };

    const payload = {
        ...view,
        id: row.id.slice(0, 8),
        project: row.cwd.split('/').filter(Boolean).pop() || row.cwd,
        platform: row.platform,
        started_at: row.started_at,
        ended_at: row.ended_at,
        ended,
        mark: parseStoredSessionMark(row.session_mark) ?? sessionMarkFromSeed(row.client_session_id),
    };
    const json = JSON.stringify(payload);

    const archiveGeneration = response.headers.get('X-Archive-Generation') || view.history?.generation || '';
    const parsedLatest = Number(response.headers.get('X-Archive-Latest'));
    const latestCursor = Number.isSafeInteger(parsedLatest) && parsedLatest >= 0
        ? parsedLatest
        : (view.history && !view.history.more && view.history.after === 0 ? view.history.cursor : 0);

    if (archiveGeneration) {
        cacheMeta(ctx, url.origin, row.id, { generation: archiveGeneration, latestCursor }, edgeTtl);
    }

    const actualAfter = view.history?.after ?? 0;
    if (actualAfter === 0) {
        cachePut(ctx, dataCacheKey(url.origin, row.id, '', 0), new Response(json, { headers }));
    } else if (cursor > latestCursor && archiveGeneration) {
        const tailPayload = {
            ...payload,
            history: { generation: archiveGeneration, after: latestCursor, cursor: latestCursor, more: false, content: '' },
        };
        const tailKey = dataCacheKey(url.origin, row.id, latestCursor > 0 ? archiveGeneration : '', latestCursor);
        cachePut(ctx, tailKey, new Response(JSON.stringify(tailPayload), { headers }));
    } else if (archiveGeneration) {
        cachePut(ctx, dataCacheKey(url.origin, row.id, archiveGeneration, actualAfter), new Response(json, { headers }));
    }

    return new Response(json, { headers });
}
