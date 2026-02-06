import type { Env } from '../types/env';
import type {
    CreateSessionRequest,
    CreateSessionResponse,
    ListSessionsResponse,
    GetSessionResponse,
    UpdateSessionRequest,
    UpdateSessionResponse,
} from '../types/api';
import type { SessionInfo, SessionState } from '../types/session';
import { jsonResponse } from '../router';
import { requireAuth, requireDeviceAuth } from '../auth/middleware';
import { generateUUID } from '../auth/tokens';

/**
 * Notify the SessionListDO about session changes for real-time dashboard updates.
 * Fire-and-forget - don't await, don't block the response.
 */
function notifySessionListChange(
    env: Env,
    event: { type: 'created' | 'updated' | 'deleted'; session: Partial<SessionInfo> }
): void {
    const doId = env.SESSION_LIST.idFromName('global');
    const stub = env.SESSION_LIST.get(doId);
    // Fire and forget - don't await
    stub.fetch(new Request('https://internal/notify', {
        method: 'POST',
        body: JSON.stringify(event),
        headers: { 'Content-Type': 'application/json' },
    })).catch(() => {
        // Ignore errors - dashboard updates are best-effort
    });
}

/**
 * POST /api/sessions - Create a new session
 */
export async function createSession(
    request: Request,
    env: Env
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;

    let body: CreateSessionRequest;
    try {
        body = await request.json();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const { client_session_id, cwd, platform } = body;

    if (!client_session_id || !cwd || !platform) {
        return new Response(
            JSON.stringify({ error: 'Missing required fields', code: 'MISSING_FIELDS' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    if (platform !== 'claude' && platform !== 'codex') {
        return new Response(
            JSON.stringify({ error: 'Invalid platform', code: 'INVALID_PLATFORM' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const sessionId = generateUUID();
    const now = Math.floor(Date.now() / 1000);

    // Check if session already exists (resume case)
    const existing = await env.DB.prepare(
        'SELECT id FROM sessions WHERE device_id = ? AND client_session_id = ?'
    ).bind(device_id, client_session_id).first<{ id: string }>();

    if (existing) {
        // Session already exists, return existing ID
        const url = new URL(request.url);
        const wsProtocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
        const wsUrl = `${wsProtocol}//${url.host}/api/sessions/${existing.id}/connect`;

        const response: CreateSessionResponse = {
            id: existing.id,
            ws_url: wsUrl,
        };
        return jsonResponse(response);
    }

    // Create new session
    await env.DB.prepare(`
        INSERT INTO sessions (id, device_id, client_session_id, cwd, platform, state, started_at, is_active)
        VALUES (?, ?, ?, ?, ?, 'ready', ?, 1)
    `).bind(sessionId, device_id, client_session_id, cwd, platform, now).run();

    // Unhide project if it was manually hidden
    const device = await env.DB.prepare('SELECT group_id FROM devices WHERE id = ?').bind(device_id).first<{ group_id: string | null }>();
    if (device?.group_id) {
        const key = `hidden-projects:${device.group_id}`;
        const hiddenJson = await env.TOKENS.get(key);
        if (hiddenJson) {
            const hidden: string[] = JSON.parse(hiddenJson);
            const filtered = hidden.filter((p: string) => p !== cwd);
            if (filtered.length !== hidden.length) {
                if (filtered.length > 0) {
                    await env.TOKENS.put(key, JSON.stringify(filtered), { expirationTtl: 60 * 60 * 24 * 30 });
                } else {
                    await env.TOKENS.delete(key);
                }
            }
        }
    }

    // Notify dashboard of new session (fire-and-forget)
    notifySessionListChange(env, {
        type: 'created',
        session: {
            id: sessionId,
            client_session_id,
            cwd,
            platform,
            state: 'ready',
            started_at: now,
            is_active: true,
            stats: { prompts: 0, completions: 0, tool_calls: 0, thinking_seconds: 0 },
        },
    });

    const url = new URL(request.url);
    const wsProtocol = url.protocol === 'https:' ? 'wss:' : 'ws:';
    const wsUrl = `${wsProtocol}//${url.host}/api/sessions/${sessionId}/connect`;

    const response: CreateSessionResponse = {
        id: sessionId,
        ws_url: wsUrl,
    };
    return jsonResponse(response, 201);
}

/**
 * GET /api/sessions - List sessions for authenticated device or mobile
 */
export async function listSessions(
    request: Request,
    env: Env
): Promise<Response> {
    const authResult = await requireAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const auth = authResult.auth;

    // Determine scope based on auth type
    let deviceId: string | null = null;
    let groupId: string | null = null;
    if (auth.type === 'device') {
        deviceId = auth.device_id;
    } else if (auth.type === 'mobile') {
        groupId = auth.group_id || null;
        if (!groupId) {
            deviceId = auth.desktop_id;
        }
    } else {
        return new Response(
            JSON.stringify({ error: 'Cannot list sessions with share token', code: 'INVALID_AUTH' }),
            { status: 403, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const url = new URL(request.url);
    const activeOnly = url.searchParams.get('active') !== 'false';
    const limit = Math.min(parseInt(url.searchParams.get('limit') || '50', 10), 100);
    const offset = parseInt(url.searchParams.get('offset') || '0', 10);

    let query = `
        SELECT sessions.id, sessions.client_session_id, sessions.cwd, sessions.platform, sessions.state,
               sessions.started_at, sessions.ended_at, sessions.is_active, sessions.last_seen_at,
               sessions.prompts, sessions.completions, sessions.tool_calls, sessions.thinking_seconds
        FROM sessions
    `;
    const params: (string | number)[] = [];

    if (groupId) {
        query += ' JOIN devices ON devices.id = sessions.device_id WHERE devices.group_id = ?';
        params.push(groupId);
    } else if (deviceId) {
        query += ' WHERE sessions.device_id = ?';
        params.push(deviceId);
    }

    if (activeOnly) {
        query += ' AND is_active = 1';
    }

    query += ' ORDER BY started_at DESC LIMIT ? OFFSET ?';
    params.push(limit, offset);

    const results = await env.DB.prepare(query).bind(...params).all<{
        id: string;
        client_session_id: string;
        cwd: string;
        platform: 'claude' | 'codex';
        state: SessionState;
        started_at: number;
        ended_at: number | null;
        is_active: number;
        last_seen_at: number | null;
        prompts: number;
        completions: number;
        tool_calls: number;
        thinking_seconds: number;
    }>();

    // Validate active sessions against SessionDO state (cleanup on read)
    // This catches zombie sessions that D1 thinks are active but aren't
    const staleSessionIds: string[] = [];
    const now = Math.floor(Date.now() / 1000);
    const staleThreshold = now - 120; // 2 minutes - allows for deploy reconnection

    if (activeOnly && results.results) {
        const activeSessions = results.results.filter(row => row.is_active === 1);

        // Check each active session's actual state (parallel for performance)
        const checks = activeSessions.map(async (row) => {
            try {
                const doId = env.SESSION.idFromName(row.id);
                const stub = env.SESSION.get(doId);
                const resp = await stub.fetch(new Request('https://internal/state'));
                const state = (await resp.json()) as { desktop_connected?: boolean };

                if (!state.desktop_connected) {
                    // Desktop disconnected - check if stale
                    const lastSeen = row.last_seen_at || row.started_at;
                    if (lastSeen < staleThreshold) {
                        staleSessionIds.push(row.id);
                    }
                }
            } catch {
                // Can't reach DO - check last_seen_at threshold
                const lastSeen = row.last_seen_at || row.started_at;
                if (lastSeen < staleThreshold) {
                    staleSessionIds.push(row.id);
                }
            }
        });

        await Promise.all(checks);
    }

    // Clean up stale sessions in background (fire and forget)
    if (staleSessionIds.length > 0) {
        const endedAt = now;

        // Batch update D1
        const placeholders = staleSessionIds.map(() => '?').join(',');
        env.DB.prepare(`
            UPDATE sessions
            SET is_active = 0, ended_at = ?
            WHERE id IN (${placeholders}) AND is_active = 1
        `).bind(endedAt, ...staleSessionIds).run().catch((error) => {
            console.error('Error cleaning up stale sessions:', error);
        });

        // Notify SessionListDO (fire and forget)
        for (const sessionId of staleSessionIds) {
            notifySessionListChange(env, {
                type: 'updated',
                session: { id: sessionId, ended_at: endedAt, is_active: false },
            });
        }
    }

    // Filter out stale sessions from results
    const sessions: SessionInfo[] = (results.results || [])
        .filter(row => !staleSessionIds.includes(row.id))
        .map(row => ({
            id: row.id,
            client_session_id: row.client_session_id,
            cwd: row.cwd,
            platform: row.platform,
            state: row.state,
            started_at: row.started_at,
            ended_at: row.ended_at,
            is_active: row.is_active === 1,
            stats: {
                prompts: row.prompts,
                completions: row.completions,
                tool_calls: row.tool_calls,
                thinking_seconds: row.thinking_seconds,
            },
        }));

    const response: ListSessionsResponse = { sessions };
    return jsonResponse(response);
}

/**
 * GET /api/sessions/:id - Get session details
 */
export async function getSession(
    request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const authResult = await requireAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const auth = authResult.auth;
    let deviceId: string | null = null;
    let groupId: string | null = null;
    if (auth.type === 'device') {
        deviceId = auth.device_id;
    } else if (auth.type === 'mobile') {
        groupId = auth.group_id || null;
        if (!groupId) {
            deviceId = auth.desktop_id;
        }
    } else {
        return new Response(
            JSON.stringify({ error: 'Cannot get session with share token', code: 'INVALID_AUTH' }),
            { status: 403, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const sessionId = params.id;

    let sessionQuery = `
        SELECT sessions.id, sessions.client_session_id, sessions.cwd, sessions.platform, sessions.state,
               sessions.started_at, sessions.ended_at, sessions.is_active,
               sessions.prompts, sessions.completions, sessions.tool_calls, sessions.thinking_seconds,
               sessions.share_token
        FROM sessions
    `;
    const sessionParams: (string | number)[] = [sessionId];
    if (groupId) {
        sessionQuery += `
            JOIN devices ON devices.id = sessions.device_id
            WHERE sessions.id = ? AND devices.group_id = ?
        `;
        sessionParams.push(groupId);
    } else {
        sessionQuery += ' WHERE sessions.id = ? AND sessions.device_id = ?';
        sessionParams.push(deviceId || '');
    }

    const session = await env.DB.prepare(sessionQuery).bind(...sessionParams).first<{
        id: string;
        client_session_id: string;
        cwd: string;
        platform: 'claude' | 'codex';
        state: SessionState;
        started_at: number;
        ended_at: number | null;
        is_active: number;
        prompts: number;
        completions: number;
        tool_calls: number;
        thinking_seconds: number;
        share_token: string | null;
    }>();

    if (!session) {
        return new Response(
            JSON.stringify({ error: 'Session not found', code: 'NOT_FOUND' }),
            { status: 404, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const url = new URL(request.url);
    const shareUrl = session.share_token
        ? `https://${url.host}/api/shared/${session.share_token}`
        : null;

    const response: GetSessionResponse = {
        id: session.id,
        client_session_id: session.client_session_id,
        cwd: session.cwd,
        platform: session.platform,
        state: session.state,
        started_at: session.started_at,
        ended_at: session.ended_at,
        is_active: session.is_active === 1,
        stats: {
            prompts: session.prompts,
            completions: session.completions,
            tool_calls: session.tool_calls,
            thinking_seconds: session.thinking_seconds,
        },
        share_url: shareUrl,
    };
    return jsonResponse(response);
}

/**
 * PATCH /api/sessions/:id - Update session
 */
export async function updateSession(
    request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;
    const sessionId = params.id;

    // Verify session belongs to device
    const session = await env.DB.prepare(
        'SELECT id FROM sessions WHERE id = ? AND device_id = ?'
    ).bind(sessionId, device_id).first();

    if (!session) {
        return new Response(
            JSON.stringify({ error: 'Session not found', code: 'NOT_FOUND' }),
            { status: 404, headers: { 'Content-Type': 'application/json' } }
        );
    }

    let body: UpdateSessionRequest;
    try {
        body = await request.json();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const updates: string[] = [];
    const values: (string | number | null)[] = [];

    if (body.ended_at !== undefined) {
        updates.push('ended_at = ?');
        values.push(body.ended_at);
        updates.push('is_active = 0');
    }

    if (body.state !== undefined) {
        updates.push('state = ?');
        values.push(body.state);
    }

    if (body.stats) {
        if (body.stats.prompts !== undefined) {
            updates.push('prompts = ?');
            values.push(body.stats.prompts);
        }
        if (body.stats.completions !== undefined) {
            updates.push('completions = ?');
            values.push(body.stats.completions);
        }
        if (body.stats.tool_calls !== undefined) {
            updates.push('tool_calls = ?');
            values.push(body.stats.tool_calls);
        }
        if (body.stats.thinking_seconds !== undefined) {
            updates.push('thinking_seconds = ?');
            values.push(body.stats.thinking_seconds);
        }
    }

    if (updates.length > 0) {
        values.push(sessionId);
        await env.DB.prepare(
            `UPDATE sessions SET ${updates.join(', ')} WHERE id = ?`
        ).bind(...values).run();

        // Notify dashboard of session update (fire-and-forget)
        notifySessionListChange(env, {
            type: 'updated',
            session: {
                id: sessionId,
                state: body.state,
                ended_at: body.ended_at,
                is_active: body.ended_at ? false : undefined,
            },
        });
    }

    const response: UpdateSessionResponse = { ok: true };
    return jsonResponse(response);
}

/**
 * DELETE /api/sessions/:id - Delete session
 */
export async function deleteSession(
    request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;
    const sessionId = params.id;

    // Delete session (cascade will delete events)
    const result = await env.DB.prepare(
        'DELETE FROM sessions WHERE id = ? AND device_id = ?'
    ).bind(sessionId, device_id).run();

    if (result.meta.changes === 0) {
        return new Response(
            JSON.stringify({ error: 'Session not found', code: 'NOT_FOUND' }),
            { status: 404, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Notify dashboard of session deletion (fire-and-forget)
    notifySessionListChange(env, {
        type: 'deleted',
        session: { id: sessionId },
    });

    return jsonResponse({ ok: true });
}
