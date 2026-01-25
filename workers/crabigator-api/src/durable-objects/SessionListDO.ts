import type { Env } from '../types/env';

interface ActiveSession {
    id: string;
    cwd: string;
    platform: string;
    state: string;
    started_at: number;
    device_id?: string;
    group_id?: string | null;
}

/**
 * Durable Object for broadcasting session list changes to dashboard viewers
 *
 * Maintains the authoritative list of currently-connected sessions.
 * Sessions are added when desktop connects, removed when desktop disconnects.
 * This ensures /api/sessions only returns sessions with active desktop connections.
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
     * Removes stale sessions that no longer have active desktop connections.
     */
    private async validateSessions(): Promise<void> {
        const stale: string[] = [];

        for (const [id] of this.activeSessions) {
            try {
                const doId = this.env.SESSION.idFromName(id);
                const stub = this.env.SESSION.get(doId);
                const resp = await stub.fetch(new Request('https://internal/state'));
                const state = (await resp.json()) as { desktop_connected?: boolean };

                if (!state.desktop_connected) {
                    stale.push(id);
                }
            } catch {
                // If we can't reach SessionDO, assume stale
                stale.push(id);
            }
        }

        if (stale.length > 0) {
            for (const id of stale) {
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
            const event = await request.json();
            await this.broadcast(event);
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

        await this.refreshMissingSessionMetadata();

        const sessions = Array.from(this.activeSessions.values())
            .filter(session => session.group_id === groupId)
            .map(session => this.sanitizeSession(session));
        return new Response(JSON.stringify({ sessions }), {
            headers: { 'Content-Type': 'application/json' },
        });
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
            this.activeSessions.set(session.id, session);
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
     * Unregister a session as disconnected (called when desktop WebSocket closes)
     */
    private async handleDisconnect(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        try {
            const { id } = await request.json() as { id: string };
            const session = this.activeSessions.get(id);

            if (session) {
                this.activeSessions.delete(id);
                await this.state.storage.put('activeSessions', Array.from(this.activeSessions.entries()));

                // Broadcast to dashboard viewers
                await this.broadcast({ type: 'deleted', session: { id, group_id: session.group_id } });

                if (session.device_id) {
                    const remainingForDevice = Array.from(this.activeSessions.values())
                        .some(active => active.device_id === session.device_id);
                    if (!remainingForDevice) {
                        await this.cleanupPairingTokens(session.device_id);
                    }
                }
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
