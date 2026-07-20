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

/** Desktop heartbeats are sent every 2h; wait for multiple misses before culling. */
const SESSION_HEARTBEAT_TIMEOUT_SECONDS = 6 * 60 * 60;

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
        await env.DB.prepare(`
            UPDATE sessions
            SET cwd = ?,
                platform = ?,
                state = CASE WHEN is_active = 1 THEN state ELSE 'ready' END,
                ended_at = NULL,
                is_active = 1,
                last_seen_at = ?
            WHERE id = ?
        `).bind(cwd, platform, now, existing.id).run();

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
            last_activity_at: now,
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
               sessions.prompts, sessions.completions, sessions.tool_calls, sessions.thinking_seconds,
               sessions.prompts_changed_at, sessions.completions_changed_at,
               devices.name as device_name
        FROM sessions
        JOIN devices ON devices.id = sessions.device_id
    `;
    const params: (string | number)[] = [];

    if (groupId) {
        query += ' WHERE devices.group_id = ?';
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
        prompts_changed_at: number | null;
        completions_changed_at: number | null;
        device_name: string | null;
    }>();

    // Validate active sessions against SessionDO state (cleanup on read)
    // This catches zombie sessions that D1 thinks are active but aren't
    const staleSessionIds: string[] = [];
    const now = Math.floor(Date.now() / 1000);
    const staleThreshold = now - SESSION_HEARTBEAT_TIMEOUT_SECONDS;

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
            device_name: row.device_name || undefined,
            stats: {
                prompts: row.prompts,
                completions: row.completions,
                tool_calls: row.tool_calls,
                thinking_seconds: row.thinking_seconds,
                prompts_changed_at: row.prompts_changed_at || undefined,
                completions_changed_at: row.completions_changed_at || undefined,
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
               sessions.prompts_changed_at, sessions.completions_changed_at,
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
        prompts_changed_at: number | null;
        completions_changed_at: number | null;
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
            prompts_changed_at: session.prompts_changed_at || undefined,
            completions_changed_at: session.completions_changed_at || undefined,
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
    const session = await env.DB.prepare(`
        SELECT id, prompts, completions, tool_calls, thinking_seconds,
               prompts_changed_at, completions_changed_at
        FROM sessions
        WHERE id = ? AND device_id = ?
    `).bind(sessionId, device_id).first<{
        id: string;
        prompts: number;
        completions: number;
        tool_calls: number;
        thinking_seconds: number;
        prompts_changed_at: number | null;
        completions_changed_at: number | null;
    }>();

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
    const now = Math.floor(Date.now() / 1000);
    let promptsChangedAt = session.prompts_changed_at;
    let completionsChangedAt = session.completions_changed_at;

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
        if (body.stats.prompts !== undefined && body.stats.prompts !== session.prompts) {
            promptsChangedAt = now;
            updates.push('prompts_changed_at = ?');
            values.push(promptsChangedAt);
        }
        if (body.stats.completions !== undefined && body.stats.completions !== session.completions) {
            completionsChangedAt = now;
            updates.push('completions_changed_at = ?');
            values.push(completionsChangedAt);
        }

        const statFields = [
            'prompts', 'completions', 'tool_calls', 'thinking_seconds',
            'work_seconds', 'model', 'compressions', 'mode',
        ] as const;
        for (const field of statFields) {
            if (body.stats[field] !== undefined) {
                updates.push(`${field} = ?`);
                values.push(body.stats[field] as string | number);
            }
        }
        if (body.stats.titles && body.stats.titles.length > 0) {
            updates.push('titles = ?');
            values.push(JSON.stringify(body.stats.titles));
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
                last_activity_at: now,
                ended_at: body.ended_at,
                is_active: body.ended_at ? false : undefined,
                stats: body.stats ? {
                    prompts: body.stats.prompts ?? session.prompts,
                    completions: body.stats.completions ?? session.completions,
                    tool_calls: body.stats.tool_calls ?? session.tool_calls,
                    thinking_seconds: body.stats.thinking_seconds ?? session.thinking_seconds,
                    prompts_changed_at: promptsChangedAt || undefined,
                    completions_changed_at: completionsChangedAt || undefined,
                } : undefined,
            },
        });
    }

    // Store per-tool breakdown (fire-and-forget)
    if (body.stats?.tool_breakdown) {
        const toolEntries = Object.entries(body.stats.tool_breakdown);
        if (toolEntries.length > 0) {
            const stmts = toolEntries.map(([toolName, count]) =>
                env.DB.prepare(
                    'INSERT OR REPLACE INTO session_tool_usage (session_id, tool_name, count) VALUES (?, ?, ?)'
                ).bind(sessionId, toolName, count)
            );
            env.DB.batch(stmts).catch((error: unknown) => {
                console.error('Error storing tool usage:', error);
            });
        }
    }

    // Store event history (fire-and-forget)
    if (body.stats?.event_history && body.stats.event_history.length > 0) {
        const stmts = body.stats.event_history.map((event) =>
            env.DB.prepare(
                'INSERT INTO session_events_log (session_id, event_type, timestamp_ms, state_before, state_after) VALUES (?, ?, ?, ?, ?)'
            ).bind(sessionId, event.event_type, event.timestamp_ms, event.state_before || null, event.state_after || null)
        );
        env.DB.batch(stmts).catch((error: unknown) => {
            console.error('Error storing event history:', error);
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
