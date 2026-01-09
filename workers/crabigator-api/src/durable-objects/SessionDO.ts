import type { SessionEvent, SessionState, CloudToDesktopMessage } from '../types/session';
import type { Env } from '../types/env';

interface SessionDOState {
    sessionId: string;
    state: SessionState;
    lastScrollbackLine: number;
    lastScreen: string | null;
    lastTitle: string | null;
    eventSequence: number;
}

interface SessionInfo {
    id: string;
    cwd: string;
    platform: string;
    state: string;
    started_at: number;
}

/**
 * Durable Object for managing a single crabigator session
 *
 * Handles:
 * - Desktop WebSocket connection (receives events, sends answers)
 * - SSE streams for mobile/web viewers
 * - Event persistence
 * - Late-joiner state catchup
 */
export class SessionDO implements DurableObject {
    private state: DurableObjectState;
    private env: Env;
    private desktopWs: WebSocket | null = null;
    private sseClients: Set<WritableStreamDefaultWriter<Uint8Array>> = new Set();
    private sessionState: SessionDOState;
    private sessionInfo: SessionInfo | null = null;
    private encoder = new TextEncoder();

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;
        this.sessionState = {
            sessionId: '',
            state: 'ready',
            lastScrollbackLine: 0,
            lastScreen: null,
            lastTitle: null,
            eventSequence: 0,
        };

        // Restore state from storage
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<SessionDOState>('sessionState');
            if (stored) {
                this.sessionState = stored;
            }
            const storedInfo = await state.storage.get<SessionInfo>('sessionInfo');
            if (storedInfo) {
                this.sessionInfo = storedInfo;
            }
        });
    }

    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);

        // Extract session ID from query params if provided
        const sessionId = url.searchParams.get('sessionId');
        if (sessionId && !this.sessionState.sessionId) {
            this.sessionState.sessionId = sessionId;
            await this.state.storage.put('sessionState', this.sessionState);
        }

        switch (url.pathname) {
            case '/connect':
                return this.handleDesktopWebSocket(request);
            case '/events':
                return this.handleSSE(request);
            case '/answer':
                return this.handleAnswer(request);
            case '/key':
                return this.handleKey(request);
            case '/state':
                return this.handleGetState();
            default:
                return new Response('Not found', { status: 404 });
        }
    }

    /**
     * Handle WebSocket connection from desktop crabigator
     */
    private async handleDesktopWebSocket(request: Request): Promise<Response> {
        // Check for WebSocket upgrade
        const upgradeHeader = request.headers.get('Upgrade');
        if (upgradeHeader !== 'websocket') {
            return new Response('Expected WebSocket', { status: 426 });
        }

        // Close existing connection if any
        if (this.desktopWs) {
            try {
                this.desktopWs.close(1000, 'New connection');
            } catch {
                // Ignore errors closing old connection
            }
        }

        // Get session info from query params
        const url = new URL(request.url);
        const cwd = url.searchParams.get('cwd') || '';
        const platform = url.searchParams.get('platform') || 'claude';
        const startedAt = parseInt(url.searchParams.get('started_at') || '0', 10);

        // Store session info for disconnect notification
        if (this.sessionState.sessionId) {
            this.sessionInfo = {
                id: this.sessionState.sessionId,
                cwd,
                platform,
                state: this.sessionState.state,
                started_at: startedAt || Math.floor(Date.now() / 1000),
            };
            await this.state.storage.put('sessionInfo', this.sessionInfo);

            // Notify SessionListDO that desktop connected
            await this.notifySessionList('connect', this.sessionInfo);
        }

        const pair = new WebSocketPair();
        const [client, server] = [pair[0], pair[1]];

        server.accept();
        this.desktopWs = server;

        server.addEventListener('message', async (event) => {
            try {
                const data = JSON.parse(event.data as string) as SessionEvent;
                await this.handleEvent(data);
            } catch (error) {
                console.error('Error handling WebSocket message:', error);
            }
        });

        server.addEventListener('close', () => {
            if (this.desktopWs === server) {
                this.desktopWs = null;
                // Notify SSE clients that desktop disconnected
                this.broadcastDesktopStatus(false);
                // Notify SessionListDO that desktop disconnected
                if (this.sessionInfo) {
                    this.notifySessionList('disconnect', { id: this.sessionInfo.id });
                }
            }
        });

        server.addEventListener('error', (error) => {
            console.error('WebSocket error:', error);
            if (this.desktopWs === server) {
                this.desktopWs = null;
                this.broadcastDesktopStatus(false);
                // Notify SessionListDO that desktop disconnected
                if (this.sessionInfo) {
                    this.notifySessionList('disconnect', { id: this.sessionInfo.id });
                }
            }
        });

        return new Response(null, {
            status: 101,
            webSocket: client,
        });
    }

    /**
     * Notify SessionListDO about connect/disconnect
     */
    private async notifySessionList(action: 'connect' | 'disconnect', data: unknown): Promise<void> {
        try {
            const doId = this.env.SESSION_LIST.idFromName('global');
            const stub = this.env.SESSION_LIST.get(doId);
            await stub.fetch(new Request(`https://internal/${action}`, {
                method: 'POST',
                body: JSON.stringify(data),
                headers: { 'Content-Type': 'application/json' },
            }));
        } catch (error) {
            console.error('Error notifying SessionListDO:', error);
        }
    }

    /**
     * Notify SessionListDO about state changes
     */
    private notifySessionStateUpdate(sessionId: string, state: string): void {
        // Fire and forget - don't await to avoid blocking event handling
        const doId = this.env.SESSION_LIST.idFromName('global');
        const stub = this.env.SESSION_LIST.get(doId);
        stub.fetch(new Request('https://internal/update', {
            method: 'POST',
            body: JSON.stringify({ id: sessionId, state }),
            headers: { 'Content-Type': 'application/json' },
        })).catch((error) => {
            console.error('Error updating session state in SessionListDO:', error);
        });
    }

    /**
     * Handle incoming event from desktop
     */
    private async handleEvent(event: SessionEvent): Promise<void> {
        // Update local state based on event type
        switch (event.type) {
            case 'state':
                this.sessionState.state = event.state;
                // Notify SessionListDO so /api/sessions returns correct state
                if (this.sessionInfo) {
                    this.notifySessionStateUpdate(this.sessionInfo.id, event.state);
                }
                break;
            case 'scrollback':
                this.sessionState.lastScrollbackLine = event.total_lines;
                break;
            case 'screen':
                this.sessionState.lastScreen = event.content;
                break;
            case 'title':
                this.sessionState.lastTitle = event.title;
                break;
        }

        // Increment sequence and persist state
        this.sessionState.eventSequence++;
        await this.state.storage.put('sessionState', this.sessionState);

        // Broadcast to all SSE clients
        await this.broadcast(event);

        // Note: Event persistence to D1 would be done here in production
        // For MVP, we rely on in-memory state
    }

    /**
     * Handle SSE connection from mobile/web viewer
     */
    private handleSSE(_request: Request): Response {
        const { readable, writable } = new TransformStream<Uint8Array, Uint8Array>();
        const writer = writable.getWriter();

        // Add to clients set
        this.sseClients.add(writer);

        // Send initial state to late joiner
        this.sendCurrentState(writer).catch((err) => {
            console.error('Error sending initial state:', err);
            this.sseClients.delete(writer);
        });

        // Client disconnect is detected when write fails in sendSSE/broadcast
        // No need to consume the readable stream here

        return new Response(readable, {
            headers: {
                'Content-Type': 'text/event-stream',
                'Cache-Control': 'no-cache',
                'Connection': 'keep-alive',
            },
        });
    }

    /**
     * Send current state to a newly connected SSE client
     */
    private async sendCurrentState(writer: WritableStreamDefaultWriter<Uint8Array>): Promise<void> {
        // Send desktop connection status first
        // This allows dashboard to immediately remove cards for disconnected sessions
        const desktopStatusEvent: SessionEvent = {
            type: 'desktop_status',
            connected: this.desktopWs !== null,
            timestamp: Date.now(),
        };
        await this.sendSSE(writer, desktopStatusEvent);

        // If desktop is disconnected, no need to send other state
        if (!this.desktopWs) {
            return;
        }

        // Send screen snapshot (for immediate visual)
        if (this.sessionState.lastScreen) {
            const screenEvent: SessionEvent = {
                type: 'screen',
                content: this.sessionState.lastScreen,
            };
            await this.sendSSE(writer, screenEvent);
        }

        // Send current state
        const stateEvent: SessionEvent = {
            type: 'state',
            state: this.sessionState.state,
            timestamp: Date.now(),
        };
        await this.sendSSE(writer, stateEvent);

        // Send current title if available
        if (this.sessionState.lastTitle) {
            const titleEvent: SessionEvent = {
                type: 'title',
                title: this.sessionState.lastTitle,
            };
            await this.sendSSE(writer, titleEvent);
        }
    }

    /**
     * Send SSE event to a single client
     */
    private async sendSSE(
        writer: WritableStreamDefaultWriter<Uint8Array>,
        event: SessionEvent
    ): Promise<void> {
        const data = `data: ${JSON.stringify(event)}\n\n`;
        try {
            await writer.write(this.encoder.encode(data));
        } catch {
            // Client disconnected
            this.sseClients.delete(writer);
        }
    }

    /**
     * Broadcast event to all SSE clients
     */
    private async broadcast(event: SessionEvent): Promise<void> {
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

        // Clean up dead clients
        for (const writer of deadClients) {
            this.sseClients.delete(writer);
        }
    }

    /**
     * Broadcast desktop connection status to SSE clients
     */
    private async broadcastDesktopStatus(connected: boolean): Promise<void> {
        const event: SessionEvent = {
            type: 'desktop_status',
            connected,
            timestamp: Date.now(),
        };
        await this.broadcast(event);
    }

    /**
     * Handle answer from mobile, forward to desktop
     */
    private async handleAnswer(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        let body: { text: string };
        try {
            body = await request.json();
        } catch {
            return new Response(
                JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
                { status: 400, headers: { 'Content-Type': 'application/json' } }
            );
        }

        if (!body.text) {
            return new Response(
                JSON.stringify({ error: 'Missing text', code: 'MISSING_TEXT' }),
                { status: 400, headers: { 'Content-Type': 'application/json' } }
            );
        }

        if (!this.desktopWs) {
            return new Response(
                JSON.stringify({ error: 'Desktop not connected', code: 'DESKTOP_OFFLINE' }),
                { status: 503, headers: { 'Content-Type': 'application/json' } }
            );
        }

        const message: CloudToDesktopMessage = {
            type: 'answer',
            text: body.text,
        };

        try {
            this.desktopWs.send(JSON.stringify(message));
        } catch (error) {
            console.error('Error sending to desktop:', error);
            return new Response(
                JSON.stringify({ error: 'Failed to send', code: 'SEND_FAILED' }),
                { status: 500, headers: { 'Content-Type': 'application/json' } }
            );
        }

        return new Response(
            JSON.stringify({ ok: true }),
            { headers: { 'Content-Type': 'application/json' } }
        );
    }

    /**
     * Handle key command from dashboard, forward to desktop
     * Used for mode switching via Shift+Tab
     */
    private async handleKey(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        let body: { key: string };
        try {
            body = await request.json();
        } catch {
            return new Response(
                JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
                { status: 400, headers: { 'Content-Type': 'application/json' } }
            );
        }

        if (!body.key) {
            return new Response(
                JSON.stringify({ error: 'Missing key', code: 'MISSING_KEY' }),
                { status: 400, headers: { 'Content-Type': 'application/json' } }
            );
        }

        if (!this.desktopWs) {
            return new Response(
                JSON.stringify({ error: 'Desktop not connected', code: 'DESKTOP_OFFLINE' }),
                { status: 503, headers: { 'Content-Type': 'application/json' } }
            );
        }

        const message: CloudToDesktopMessage = {
            type: 'key',
            key: body.key,
        };

        try {
            this.desktopWs.send(JSON.stringify(message));
        } catch (error) {
            console.error('Error sending key to desktop:', error);
            return new Response(
                JSON.stringify({ error: 'Failed to send', code: 'SEND_FAILED' }),
                { status: 500, headers: { 'Content-Type': 'application/json' } }
            );
        }

        return new Response(
            JSON.stringify({ ok: true }),
            { headers: { 'Content-Type': 'application/json' } }
        );
    }

    /**
     * Get current session state
     */
    private handleGetState(): Response {
        return new Response(
            JSON.stringify({
                state: this.sessionState.state,
                scrollback_lines: this.sessionState.lastScrollbackLine,
                has_screen: this.sessionState.lastScreen !== null,
                title: this.sessionState.lastTitle,
                event_sequence: this.sessionState.eventSequence,
                desktop_connected: this.desktopWs !== null,
                sse_clients: this.sseClients.size,
            }),
            { headers: { 'Content-Type': 'application/json' } }
        );
    }
}
