import type { SessionEvent, SessionState, CloudToDesktopMessage } from '../types/session';

interface SessionDOState {
    sessionId: string;
    state: SessionState;
    lastScrollbackLine: number;
    lastScreen: string | null;
    lastTitle: string | null;
    eventSequence: number;
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
    private desktopWs: WebSocket | null = null;
    private sseClients: Set<WritableStreamDefaultWriter<Uint8Array>> = new Set();
    private sessionState: SessionDOState;
    private encoder = new TextEncoder();

    constructor(state: DurableObjectState) {
        this.state = state;
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
            case '/state':
                return this.handleGetState();
            default:
                return new Response('Not found', { status: 404 });
        }
    }

    /**
     * Handle WebSocket connection from desktop crabigator
     */
    private handleDesktopWebSocket(request: Request): Response {
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
            }
        });

        server.addEventListener('error', (error) => {
            console.error('WebSocket error:', error);
            if (this.desktopWs === server) {
                this.desktopWs = null;
                this.broadcastDesktopStatus(false);
            }
        });

        return new Response(null, {
            status: 101,
            webSocket: client,
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
