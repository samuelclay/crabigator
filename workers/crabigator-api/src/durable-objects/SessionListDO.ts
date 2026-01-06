interface ActiveSession {
    id: string;
    cwd: string;
    platform: string;
    state: string;
    started_at: number;
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
    private sseClients: Set<WritableStreamDefaultWriter<Uint8Array>> = new Set();
    private encoder = new TextEncoder();
    private activeSessions: Map<string, ActiveSession> = new Map();

    constructor(state: DurableObjectState) {
        this.state = state;

        // Restore active sessions from storage
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<[string, ActiveSession][]>('activeSessions');
            if (stored) {
                this.activeSessions = new Map(stored);
            }
        });
    }

    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);

        switch (url.pathname) {
            case '/subscribe':
                return this.handleSubscribe();
            case '/notify':
                return this.handleNotify(request);
            case '/sessions':
                return this.handleGetSessions();
            case '/connect':
                return this.handleConnect(request);
            case '/disconnect':
                return this.handleDisconnect(request);
            default:
                return new Response('Not found', { status: 404 });
        }
    }

    /**
     * Handle SSE subscription from dashboard
     */
    private handleSubscribe(): Response {
        const { readable, writable } = new TransformStream<Uint8Array, Uint8Array>();
        const writer = writable.getWriter();

        this.sseClients.add(writer);

        // Send initial connected event
        this.sendSSE(writer, { type: 'connected', clients: this.sseClients.size });

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
        const data = `data: ${JSON.stringify(event)}\n\n`;
        const encoded = this.encoder.encode(data);

        const deadClients: WritableStreamDefaultWriter<Uint8Array>[] = [];

        for (const writer of this.sseClients) {
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
    private handleGetSessions(): Response {
        const sessions = Array.from(this.activeSessions.values());
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
                await this.broadcast({ type: 'deleted', session: { id } });
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
}
