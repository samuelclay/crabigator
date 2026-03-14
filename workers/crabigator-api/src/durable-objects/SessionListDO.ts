import type { Env } from '../types/env';

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
    private sseClients: Map<WritableStreamDefaultWriter<Uint8Array>, { group_id: string }> = new Map();
    private encoder = new TextEncoder();
    private activeSessions: Map<string, ActiveSession> = new Map();

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;

        // Restore active sessions from storage and validate them
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<[string, ActiveSession][]>('activeSessions');
            if (stored) {
                this.activeSessions = new Map(stored);
                // Validate each session - remove stale ones
                await this.validateSessions();
                // Refresh any missing group/device metadata
                await this.refreshMissingSessionMetadata();
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
            if (session.device_id && session.group_id) {
                continue;
            }
            const row = await this.env.DB.prepare(`
                SELECT sessions.device_id as device_id, devices.group_id as group_id
                FROM sessions
                JOIN devices ON devices.id = sessions.device_id
                WHERE sessions.id = ?
            `).bind(id).first<{ device_id: string; group_id: string | null }>();
            if (row) {
                session.device_id = session.device_id || row.device_id;
                session.group_id = session.group_id || row.group_id || null;
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
     * Handle SSE subscription from dashboard
     */
    private handleSubscribe(request: Request): Response {
        const url = new URL(request.url);
        const version = url.searchParams.get('version') || 'unknown';
        const groupId = url.searchParams.get('group_id');
        if (!groupId) {
            return new Response('Missing group_id', { status: 400 });
        }

        const { readable, writable } = new TransformStream<Uint8Array, Uint8Array>();
        const writer = writable.getWriter();

        this.sseClients.set(writer, { group_id: groupId });

        // Send initial connected event with build version
        this.sendSSE(writer, { type: 'connected', clients: this.sseClients.size, version });

        return new Response(readable, {
            headers: {
                'Content-Type': 'text/event-stream',
                'Cache-Control': 'no-cache',
                'Connection': 'keep-alive',
            },
        });
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

            return new Response(JSON.stringify({ ok: true, clients: this.sseClients.size }), {
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
     * Send SSE event to a single client
     */
    private async sendSSE(
        writer: WritableStreamDefaultWriter<Uint8Array>,
        event: unknown
    ): Promise<void> {
        const data = `data: ${JSON.stringify(event)}\n\n`;
        try {
            await writer.write(this.encoder.encode(data));
        } catch {
            this.sseClients.delete(writer);
        }
    }

    /**
     * Broadcast event to all SSE clients
     */
    private async broadcast(event: unknown): Promise<void> {
        const eventObj = event as { type?: string; session?: ActiveSession | Partial<ActiveSession> };
        let groupId: string | null = null;

        if (eventObj.session?.group_id) {
            groupId = eventObj.session.group_id;
        } else if (eventObj.session?.id) {
            const existing = this.activeSessions.get(eventObj.session.id);
            if (existing?.group_id) {
                groupId = existing.group_id;
            } else if (eventObj.session.id) {
                const row = await this.env.DB.prepare(`
                    SELECT devices.group_id as group_id
                    FROM sessions
                    JOIN devices ON devices.id = sessions.device_id
                    WHERE sessions.id = ?
                `).bind(eventObj.session.id).first<{ group_id: string | null }>();
                groupId = row?.group_id || null;
            }
        }

        if (!groupId) {
            // Can't safely determine ownership - skip broadcast
            return;
        }

        let payload = eventObj;
        if (eventObj.session && typeof eventObj.session === 'object') {
            const { group_id: _groupId, device_id: _deviceId, ...rest } = eventObj.session as Record<string, unknown>;
            payload = { ...eventObj, session: rest };
        }

        const data = `data: ${JSON.stringify(payload)}\n\n`;
        const encoded = this.encoder.encode(data);

        const deadClients: WritableStreamDefaultWriter<Uint8Array>[] = [];

        for (const [writer, meta] of this.sseClients.entries()) {
            if (meta.group_id !== groupId) {
                continue;
            }
            try {
                await writer.write(encoded);
            } catch {
                deadClients.push(writer);
            }
        }

        for (const writer of deadClients) {
            this.sseClients.delete(writer);
        }
    }

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
                   sessions.device_id, devices.group_id, devices.name as device_name
            FROM sessions
            JOIN devices ON devices.id = sessions.device_id
            WHERE devices.group_id = ? AND sessions.is_active = 1
            ORDER BY sessions.started_at DESC
            LIMIT 50
        `).bind(groupId).all<{
            id: string;
            cwd: string;
            platform: string;
            state: string;
            started_at: number;
            device_id: string;
            group_id: string | null;
            device_name: string | null;
        }>();

        return (results.results || []).map(row => ({
            id: row.id,
            cwd: row.cwd,
            platform: row.platform,
            state: row.state,
            started_at: row.started_at,
            device_id: row.device_id,
            device_name: row.device_name || undefined,
            group_id: row.group_id,
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
            if (!session.group_id || !session.device_id) {
                const row = await this.env.DB.prepare(`
                    SELECT sessions.device_id as device_id, devices.group_id as group_id
                    FROM sessions
                    JOIN devices ON devices.id = sessions.device_id
                    WHERE sessions.id = ?
                `).bind(session.id).first<{ device_id: string; group_id: string | null }>();
                if (row) {
                    session.device_id = session.device_id || row.device_id;
                    session.group_id = session.group_id || row.group_id || null;
                }
            }
            const sessionWithLastSeen = {
                ...session,
                last_seen: Date.now(),
            };
            console.log(`handleConnect: storing session ${session.id} with state=${session.state}`);
            this.activeSessions.set(session.id, sessionWithLastSeen);
            await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));

            // Broadcast to dashboard viewers
            await this.broadcast({ type: 'created', session });

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
                session.state = state;
                session.last_seen = Date.now();  // Refresh on any update
                this.activeSessions.set(id, session);
                await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));

                // Broadcast state update to dashboard viewers
                await this.broadcast({ type: 'updated', session: { id, state, group_id: session.group_id } });
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
                last_seen: lastSeen,
                last_seen_age_ms: lastSeenAgeMs,
                device_id: s.device_id ? (full ? s.device_id : s.device_id.slice(0, 8)) : undefined,
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
     * SessionDO calls this every ~10s during activity. We persist to storage so
     * that after a deploy, the DO can reload and see the recent last_seen.
     */
    private async handleTouch(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const { id } = await request.json() as { id: string };
            const session = this.activeSessions.get(id);
            if (session) {
                session.last_seen = Date.now();
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
