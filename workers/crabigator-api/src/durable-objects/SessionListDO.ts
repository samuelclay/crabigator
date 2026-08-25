import type { Env } from '../types/env';
import type { SessionInfo } from '../types/session';

interface ActiveSession {
    id: string;
    cwd: string;
    platform: string;
    state: string;
    started_at: number;
    device_id?: string;
    device_name?: string;
    group_id?: string | null;
    last_seen?: number;  // Timestamp of last desktop activity
    last_activity_at?: number;  // Unix timestamp of recent session activity
    stats?: SessionInfo['stats'];
}

interface ViewerAttachment {
    group_id: string;
}

/**
 * Durable Object for broadcasting session list changes to dashboard viewers
 *
 * Maintains the authoritative list of active/recoverable sessions.
 * Sessions are added when desktop connects, removed when the session ends.
 * COMPLETE sessions are kept across transient disconnects (e.g., deploys).
 */
export class SessionListDO implements DurableObject {
    private state: DurableObjectState;
    private env: Env;
    private activeSessions: Map<string, ActiveSession> = new Map();

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;

        // Restore active sessions from storage. Validation happens on /sessions,
        // outside the constructor, so hibernation wake-ups stay cheap.
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<[string, ActiveSession][]>('activeSessions');
            if (stored) {
                this.activeSessions = new Map(stored);
            }
        });
    }

    /**
     * Validate all sessions by checking if their desktops are still connected.
     * Removes stale sessions that no longer have active desktop connections
     * AND have been inactive for more than the grace period.
     *
     * The grace period allows desktops to reconnect after deploys break WebSockets.
     */
    private async validateSessions(): Promise<void> {
        const GRACE_PERIOD_MS = 60_000; // 60 seconds
        const now = Date.now();
        const stale: string[] = [];

        console.log(`validateSessions: checking ${this.activeSessions.size} sessions`);
        for (const [id, session] of this.activeSessions) {
            console.log(`validateSessions: session ${id} state="${session.state}" last_seen=${session.last_seen}`);

            try {
                const doId = this.env.SESSION.idFromName(id);
                const stub = this.env.SESSION.get(doId);
                const resp = await stub.fetch(new Request('https://internal/state'));
                const state = (await resp.json()) as { desktop_connected?: boolean };

                if (!state.desktop_connected) {
                    // Only stale if disconnected AND past grace period
                    const lastSeen = session.last_seen || 0;
                    if (now - lastSeen > GRACE_PERIOD_MS) {
                        stale.push(id);
                    }
                }
            } catch {
                // Can't reach SessionDO - check grace period before removing
                const lastSeen = session.last_seen || 0;
                if (now - lastSeen > GRACE_PERIOD_MS) {
                    stale.push(id);
                }
            }
        }

        if (stale.length > 0) {
            for (const id of stale) {
                const session = this.activeSessions.get(id);
                console.log(`validateSessions: removing stale session ${id.slice(0, 8)} cwd=${session?.cwd} last_seen=${session?.last_seen}`);
                this.activeSessions.delete(id);
            }
            await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));
            console.log(`Cleaned up ${stale.length} stale session(s): ${stale.join(', ')}`);
        }
    }

    /**
     * Refresh missing device/group metadata for stored sessions.
     */
    private async refreshMissingSessionMetadata(): Promise<void> {
        let updated = false;
        for (const [id, session] of this.activeSessions) {
            if (session.device_id && session.group_id && session.device_name) {
                continue;
            }
            const row = await this.env.DB.prepare(`
                SELECT sessions.device_id as device_id, devices.group_id as group_id, devices.name as device_name
                FROM sessions
                JOIN devices ON devices.id = sessions.device_id
                WHERE sessions.id = ?
            `).bind(id).first<{ device_id: string; group_id: string | null; device_name: string | null }>();
            if (row) {
                session.device_id = session.device_id || row.device_id;
                session.group_id = session.group_id || row.group_id || null;
                session.device_name = session.device_name || row.device_name || undefined;
                this.activeSessions.set(id, session);
                updated = true;
            }
        }

        if (updated) {
            await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));
        }
    }

    private sanitizeSession(session: ActiveSession): Omit<ActiveSession, 'device_id' | 'group_id'> {
        const { device_id: _deviceId, group_id: _groupId, ...rest } = session;
        return rest;
    }

    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);

        switch (url.pathname) {
            case '/subscribe':
                return this.handleSubscribe(request);
            case '/notify':
                return this.handleNotify(request);
            case '/sessions':
                return this.handleGetSessions(request);
            case '/connect':
                return this.handleConnect(request);
            case '/disconnect':
                return this.handleDisconnect(request);
            case '/update':
                return this.handleUpdate(request);
            case '/touch':
                return this.handleTouch(request);
            case '/debug':
                return this.handleDebug(request);
            default:
                return new Response('Not found', { status: 404 });
        }
    }

    /**
     * Handle a hibernatable WebSocket subscription from the dashboard.
     */
    private handleSubscribe(request: Request): Response {
        const url = new URL(request.url);
        const version = url.searchParams.get('version') || 'unknown';
        const groupId = url.searchParams.get('group_id');
        if (!groupId) {
            return new Response('Missing group_id', { status: 400 });
        }
        if (request.headers.get('Upgrade')?.toLowerCase() !== 'websocket') {
            return new Response('Expected WebSocket', { status: 426 });
        }

        const pair = new WebSocketPair();
        const [client, server] = [pair[0], pair[1]];
        this.state.acceptWebSocket(server, ['viewer']);
        server.serializeAttachment({ group_id: groupId } satisfies ViewerAttachment);

        const clients = this.state.getWebSockets('viewer').length;
        server.send(JSON.stringify({ type: 'connected', clients, version }));

        return new Response(null, { status: 101, webSocket: client });
    }

    /**
     * Handle notification from worker about session changes
     */
    private async handleNotify(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const event = await request.json() as {
                type?: string;
                session?: Partial<ActiveSession> & { ended_at?: number | null; is_active?: boolean };
            };
            await this.broadcast(event);

            const sessionId = event.session?.id;
            const endedAt = event.session?.ended_at;
            const isInactive = event.session?.is_active === false;
            const shouldRemove =
                event.type === 'deleted'
                || (event.type === 'updated' && (endedAt != null || isInactive));
            if (shouldRemove && sessionId) {
                const existing = this.activeSessions.get(sessionId);
                if (existing) {
                    this.activeSessions.delete(sessionId);
                    await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));

                    if (existing.device_id) {
                        const remainingForDevice = Array.from(this.activeSessions.values())
                            .some(active => active.device_id === existing.device_id);
                        if (!remainingForDevice) {
                            await this.cleanupPairingTokens(existing.device_id);
                        }
                    }
                }
            }

            return new Response(JSON.stringify({
                ok: true,
                clients: this.state.getWebSockets('viewer').length,
            }), {
                headers: { 'Content-Type': 'application/json' },
            });
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid JSON' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }
    }

    /**
     * Broadcast an event to the matching hibernatable viewer WebSockets.
     */
    private async broadcast(event: unknown): Promise<void> {
        const eventObj = event as { type?: string; session?: ActiveSession | Partial<ActiveSession> };
        let groupId: string | null = null;
        let deviceName: string | undefined;

        if (eventObj.session?.group_id) {
            groupId = eventObj.session.group_id;
            deviceName = eventObj.session.device_name;
        } else if (eventObj.session?.id) {
            const existing = this.activeSessions.get(eventObj.session.id);
            if (existing?.group_id) {
                groupId = existing.group_id;
                deviceName = existing.device_name;
            } else if (eventObj.session.id) {
                const row = await this.env.DB.prepare(`
                    SELECT devices.group_id as group_id, devices.name as device_name
                    FROM sessions
                    JOIN devices ON devices.id = sessions.device_id
                    WHERE sessions.id = ?
                `).bind(eventObj.session.id).first<{ group_id: string | null; device_name: string | null }>();
                groupId = row?.group_id || null;
                deviceName = row?.device_name || undefined;
            }
        }
        if (groupId && !deviceName && eventObj.session?.id) {
            const row = await this.env.DB.prepare(`
                SELECT devices.name as device_name
                FROM sessions
                JOIN devices ON devices.id = sessions.device_id
                WHERE sessions.id = ?
            `).bind(eventObj.session.id).first<{ device_name: string | null }>();
            deviceName = row?.device_name || undefined;
        }

        if (!groupId) {
            // Can't safely determine ownership - skip broadcast
            return;
        }

        let payload = eventObj;
        if (eventObj.session && typeof eventObj.session === 'object') {
            const { group_id: _groupId, device_id: _deviceId, ...rest } = eventObj.session as Record<string, unknown>;
            if (deviceName && rest.device_name === undefined) {
                rest.device_name = deviceName;
            }
            payload = { ...eventObj, session: rest };
        }

        const data = JSON.stringify(payload);
        for (const viewer of this.state.getWebSockets('viewer')) {
            const attachment = viewer.deserializeAttachment();
            if (!this.isViewerAttachment(attachment) || attachment.group_id !== groupId) {
                continue;
            }
            try {
                viewer.send(data);
            } catch {
                // The runtime removes closed sockets from getWebSockets().
            }
        }
    }

    private isViewerAttachment(value: unknown): value is ViewerAttachment {
        return typeof value === 'object'
            && value !== null
            && 'group_id' in value
            && typeof value.group_id === 'string';
    }

    webSocketMessage(_ws: WebSocket, _message: string | ArrayBuffer): void {}

    webSocketClose(
        _ws: WebSocket,
        _code: number,
        _reason: string,
        _wasClean: boolean
    ): void {}

    webSocketError(_ws: WebSocket, _error: unknown): void {}

    /**
     * Get list of currently connected sessions
     */
    private async handleGetSessions(request: Request): Promise<Response> {
        const url = new URL(request.url);
        const groupId = url.searchParams.get('group_id');
        if (!groupId) {
            return new Response(JSON.stringify({ error: 'Missing group_id' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }

        // Validate sessions before returning - removes stale disconnected sessions
        await this.validateSessions();
        await this.refreshMissingSessionMetadata();

        // Always reconcile with D1 to catch sessions that might have reconnected
        // without notifying SessionListDO (e.g., after a deploy or DO migration)
        // D1 is the source of truth for active sessions - merge into memory
        const d1Sessions = await this.fetchActiveSessionsFromD1(groupId);
        console.log(`handleGetSessions: D1 returned ${d1Sessions.length} sessions, memory has ${this.activeSessions.size} total`);
        let updated = false;
        for (const session of d1Sessions) {
            const existing = this.activeSessions.get(session.id);
            if (!existing) {
                console.log(`handleGetSessions: adding missing session ${session.id.slice(0, 8)} from D1`);
                this.activeSessions.set(session.id, session);
                updated = true;
            } else if (!existing.group_id && session.group_id) {
                // Update existing session with missing group_id
                existing.group_id = session.group_id;
                existing.device_id = existing.device_id || session.device_id;
                this.activeSessions.set(session.id, existing);
                updated = true;
            }
        }
        if (updated) {
            await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));
        }

        // Return D1 sessions directly - they are authoritative for is_active=1
        // Memory state is used for real-time updates but D1 is source of truth
        const sessions = d1Sessions.map(session => this.sanitizeSession(session));
        console.log(`handleGetSessions: returning ${sessions.length} sessions for group ${groupId.slice(0, 8)}`);

        return new Response(JSON.stringify({ sessions }), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Fetch active sessions from D1 as a fallback
     */
    private async fetchActiveSessionsFromD1(groupId: string): Promise<ActiveSession[]> {
        const results = await this.env.DB.prepare(`
            SELECT sessions.id, sessions.cwd, sessions.platform, sessions.state, sessions.started_at,
                   sessions.last_seen_at, sessions.device_id, devices.group_id, devices.name as device_name,
                   sessions.prompts, sessions.completions, sessions.tool_calls, sessions.thinking_seconds,
                   sessions.prompts_changed_at, sessions.completions_changed_at
            FROM sessions
            JOIN devices ON devices.id = sessions.device_id
            WHERE devices.group_id = ? AND sessions.is_active = 1
            ORDER BY COALESCE(sessions.last_seen_at, sessions.started_at) DESC
            LIMIT 50
        `).bind(groupId).all<{
            id: string;
            cwd: string;
            platform: string;
            state: string;
            started_at: number;
            last_seen_at: number | null;
            device_id: string;
            group_id: string | null;
            device_name: string | null;
            prompts: number;
            completions: number;
            tool_calls: number;
            thinking_seconds: number;
            prompts_changed_at: number | null;
            completions_changed_at: number | null;
        }>();

        return (results.results || []).map(row => ({
            id: row.id,
            cwd: row.cwd,
            platform: row.platform,
            state: row.state,
            started_at: row.started_at,
            last_activity_at: row.last_seen_at || row.started_at,
            device_id: row.device_id,
            device_name: row.device_name || undefined,
            group_id: row.group_id,
            stats: {
                prompts: row.prompts,
                completions: row.completions,
                tool_calls: row.tool_calls,
                thinking_seconds: row.thinking_seconds,
                prompts_changed_at: row.prompts_changed_at || undefined,
                completions_changed_at: row.completions_changed_at || undefined,
            },
        }));
    }

    /**
     * Register a session as connected (called when desktop WebSocket opens)
     */
    private async handleConnect(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const session = await request.json() as ActiveSession;
            if (!session.group_id || !session.device_id || !session.device_name) {
                const row = await this.env.DB.prepare(`
                    SELECT sessions.device_id as device_id, devices.group_id as group_id, devices.name as device_name
                    FROM sessions
                    JOIN devices ON devices.id = sessions.device_id
                    WHERE sessions.id = ?
                `).bind(session.id).first<{ device_id: string; group_id: string | null; device_name: string | null }>();
                if (row) {
                    session.device_id = session.device_id || row.device_id;
                    session.group_id = session.group_id || row.group_id || null;
                    session.device_name = session.device_name || row.device_name || undefined;
                }
            }
            const now = Date.now();
            const sessionWithLastSeen = {
                ...session,
                last_seen: now,
                last_activity_at: Math.floor(now / 1000),
            };
            console.log(`handleConnect: storing session ${session.id} with state=${session.state}`);
            this.activeSessions.set(session.id, sessionWithLastSeen);
            await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));

            // Broadcast to dashboard viewers
            await this.broadcast({ type: 'created', session: sessionWithLastSeen });

            return new Response(JSON.stringify({ ok: true }), {
                headers: { 'Content-Type': 'application/json' },
            });
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid JSON' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }
    }

    /**
     * Handle desktop WebSocket disconnect.
     *
     * Don't delete immediately - the disconnect might be temporary (e.g., deploy).
     * Just update last_seen so validateSessions can clean up truly stale sessions.
     * Sessions are only removed when validateSessions finds them disconnected
     * past the grace period, or when explicitly ended via /notify.
     */
    private async handleDisconnect(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const { id } = await request.json() as { id: string };
            const session = this.activeSessions.get(id);

            if (session) {
                // Update last_seen so grace period starts from disconnect time
                session.last_seen = Date.now();
                this.activeSessions.set(id, session);
                await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));
            }

            return new Response(JSON.stringify({ ok: true }), {
                headers: { 'Content-Type': 'application/json' },
            });
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid JSON' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }
    }

    /**
     * Update session state (called when session state changes)
     *
     * State changes are infrequent (few per minute) so persistence is cheap.
     * This ensures state survives DO hibernation.
     */
    private async handleUpdate(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const { id, state } = await request.json() as { id: string; state: string };
            const session = this.activeSessions.get(id);

            if (session) {
                const now = Date.now();
                session.state = state;
                session.last_seen = now;  // Refresh on any update
                session.last_activity_at = Math.floor(now / 1000);
                this.activeSessions.set(id, session);
                await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));

                // Broadcast state update to dashboard viewers
                await this.broadcast({ type: 'updated', session: { id, state, last_activity_at: session.last_activity_at, group_id: session.group_id } });
            }

            return new Response(JSON.stringify({ ok: true }), {
                headers: { 'Content-Type': 'application/json' },
            });
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid JSON' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }
    }

    /**
     * Debug endpoint to inspect activeSessions state
     */
    private handleDebug(request: Request): Response {
        const url = new URL(request.url);
        const full = url.searchParams.get('full') === '1';
        const groupFilter = url.searchParams.get('group_id');
        const deviceFilter = url.searchParams.get('device_id');
        const now = Date.now();

        const entries = Array.from(this.activeSessions.entries()).filter(([, session]) =>
            (!groupFilter || session.group_id === groupFilter) &&
            (!deviceFilter || session.device_id === deviceFilter)
        );

        const sessions = entries.map(([id, s]) => {
            const lastSeen = s.last_seen ?? null;
            const lastSeenAgeMs = lastSeen ? now - lastSeen : null;
            return {
                id: full ? id : id.slice(0, 8),
                cwd: s.cwd,
                platform: s.platform,
                state: s.state,
                started_at: s.started_at,
                last_activity_at: s.last_activity_at ?? null,
                last_seen: lastSeen,
                last_seen_age_ms: lastSeenAgeMs,
                device_id: s.device_id ? (full ? s.device_id : s.device_id.slice(0, 8)) : undefined,
                device_name: s.device_name ?? null,
                group_id: s.group_id ? (full ? s.group_id : s.group_id.slice(0, 8)) : null,
            };
        });

        const byState: Record<string, number> = {};
        for (const session of sessions) {
            const key = session.state || 'unknown';
            byState[key] = (byState[key] || 0) + 1;
        }

        return new Response(JSON.stringify({
            now,
            count: sessions.length,
            by_state: byState,
            sessions,
        }, null, 2), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Update last_seen timestamp for a session (called periodically from SessionDO).
     *
     * This keeps sessions alive during deploys by updating the grace period timer.
     * It is only liveness metadata, so it should not broadcast dashboard updates or
     * advance last_activity_at; otherwise heartbeats continuously reorder the UI.
     */
    private async handleTouch(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const { id } = await request.json() as { id: string };
            const session = this.activeSessions.get(id);
            if (session) {
                const now = Date.now();
                session.last_seen = now;
                this.activeSessions.set(id, session);
                // Persist so deploys see the updated last_seen (10s writes are acceptable)
                await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));
            }
            return new Response(JSON.stringify({ ok: true }), {
                headers: { 'Content-Type': 'application/json' },
            });
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid JSON' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }
    }

    private async cleanupPairingTokens(deviceId: string): Promise<void> {
        try {
            const tokenList = await this.env.TOKENS.get(`pairing_device:${deviceId}`, 'json') as string[] | null;
            if (!Array.isArray(tokenList) || tokenList.length === 0) {
                return;
            }

            for (const token of tokenList) {
                const tokenDataStr = await this.env.TOKENS.get(`pairing:${token}`);
                if (tokenDataStr) {
                    try {
                        const data = JSON.parse(tokenDataStr) as { code?: string };
                        if (data.code) {
                            await this.env.TOKENS.delete(`pairing_code:${data.code}`);
                        }
                    } catch {
                        // Ignore JSON parse errors
                    }
                }
                await this.env.TOKENS.delete(`pairing:${token}`);
            }

            await this.env.TOKENS.delete(`pairing_device:${deviceId}`);
        } catch (error) {
            console.error('Error cleaning up pairing tokens:', error);
        }
    }
}
