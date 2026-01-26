import type { SessionEvent, SessionState, CloudToDesktopMessage, CloudPromptData, KeyStep } from '../types/session';
import type { Env } from '../types/env';

/**
 * Persistent state - written to storage only on meaningful changes
 * (state transitions, prompt changes, title changes)
 */
interface PersistentState {
    sessionId: string;
    state: SessionState;
    currentPrompt: CloudPromptData | null;
    lastTitle: string | null;
}

/**
 * Ephemeral state - kept in memory only, rebuilt from desktop on reconnect
 * This is the high-frequency data that was causing excessive storage writes
 */
interface EphemeralState {
    lastScrollbackLine: number;
    /** Accumulated scrollback content (capped at ~500KB) */
    scrollbackContent: string;
    lastScreen: string | null;
    lastTitleHistory: string[] | null;
    eventSequence: number;
}

/** Viewer activity timeout - 35s to allow for 5s heartbeat intervals */
const VIEWER_ACTIVITY_TIMEOUT_MS = 35_000;

interface SessionInfo {
    id: string;
    cwd: string;
    platform: string;
    state: string;
    started_at: number;
    device_id?: string;
    group_id?: string | null;
}

/**
 * Durable Object for managing a single crabigator session
 *
 * Handles:
 * - Desktop WebSocket connection (receives events, sends answers)
 * - SSE streams for mobile/web viewers
 * - Event persistence (only critical state)
 * - Late-joiner state catchup
 *
 * Cost optimization:
 * - Splits state into persistent (state/prompt/title) and ephemeral (screen/scrollback)
 * - Only persists on meaningful state changes, not every event
 * - Tracks viewer activity to notify desktop for streaming optimization
 */
export class SessionDO implements DurableObject {
    private state: DurableObjectState;
    private env: Env;
    private desktopWs: WebSocket | null = null;
    private sseClients: Set<WritableStreamDefaultWriter<Uint8Array>> = new Set();
    private persistentState: PersistentState;
    private ephemeralState: EphemeralState;
    private sessionInfo: SessionInfo | null = null;
    private encoder = new TextEncoder();
    /** Last time a viewer (dashboard/phone) signaled activity */
    private lastViewerActivity: number = 0;
    /** Whether we've notified desktop that viewers are active */
    private desktopNotifiedViewerActive: boolean = false;
    /** Last time we notified SessionListDO of activity (throttled) */
    private lastSeenNotifiedAt: number = 0;

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;
        this.persistentState = {
            sessionId: '',
            state: 'ready',
            currentPrompt: null,
            lastTitle: null,
        };
        this.ephemeralState = {
            lastScrollbackLine: 0,
            scrollbackContent: '',
            lastScreen: null,
            lastTitleHistory: null,
            eventSequence: 0,
        };

        // Restore persistent state from storage
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<PersistentState>('persistentState');
            if (stored) {
                this.persistentState = stored;
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
        if (sessionId && !this.persistentState.sessionId) {
            this.persistentState.sessionId = sessionId;
            await this.state.storage.put('persistentState', this.persistentState);
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
            case '/key-sequence':
                return this.handleKeySequence(request);
            case '/draft':
                if (request.method === 'POST') {
                    return this.handleSaveDraft(request);
                } else {
                    return this.handleGetDraft();
                }
            case '/state':
                return this.handleGetState();
            case '/viewer-active':
                return this.handleViewerActive();
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
        const deviceId = url.searchParams.get('device_id') || undefined;
        const groupId = url.searchParams.get('group_id') || undefined;

        // Store session info for disconnect notification
        if (this.persistentState.sessionId) {
            this.sessionInfo = {
                id: this.persistentState.sessionId,
                cwd,
                platform,
                state: this.persistentState.state,
                started_at: startedAt || Math.floor(Date.now() / 1000),
                device_id: deviceId,
                group_id: groupId || null,
            };
            await this.state.storage.put('sessionInfo', this.sessionInfo);

            // Notify SessionListDO that desktop connected
            await this.notifySessionList('connect', this.sessionInfo);

            // Notify desktop of current viewer status
            if (this.hasActiveViewers()) {
                this.notifyDesktopViewerStatus(true);
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
     * Check if enough time has passed to send another last_seen update
     * Throttles to every 10 seconds to avoid excessive updates
     */
    private shouldUpdateLastSeen(): boolean {
        return Date.now() - this.lastSeenNotifiedAt > 10_000; // 10s throttle
    }

    /**
     * Notify SessionListDO of session activity (for deploy grace period)
     *
     * This updates the last_seen timestamp in SessionListDO, which prevents
     * the session from being cleaned up during deploy validation.
     */
    private notifySessionLastSeen(): void {
        this.lastSeenNotifiedAt = Date.now();
        const doId = this.env.SESSION_LIST.idFromName('global');
        const stub = this.env.SESSION_LIST.get(doId);
        stub.fetch(new Request('https://internal/touch', {
            method: 'POST',
            body: JSON.stringify({ id: this.sessionInfo!.id }),
            headers: { 'Content-Type': 'application/json' },
        })).catch(() => {
            // Ignore errors - this is a best-effort update
        });
    }

    /**
     * Handle incoming event from desktop
     */
    private async handleEvent(event: SessionEvent): Promise<void> {
        // Track whether persistent state changed (requires storage write)
        let persistentChanged = false;

        // Update local state based on event type
        switch (event.type) {
            case 'state':
                if (this.persistentState.state !== event.state) {
                    this.persistentState.state = event.state;
                    persistentChanged = true;
                    // Notify SessionListDO so /api/sessions returns correct state
                    if (this.sessionInfo) {
                        this.notifySessionStateUpdate(this.sessionInfo.id, event.state);
                    }
                }
                break;
            case 'scrollback':
                // Ephemeral state - no storage write
                this.ephemeralState.lastScrollbackLine = event.total_lines;
                // Accumulate scrollback content (cap at ~500KB to avoid memory issues)
                const MAX_SCROLLBACK_SIZE = 500 * 1024;
                if (event.diff) {
                    this.ephemeralState.scrollbackContent += event.diff;
                    // Trim from the beginning if too large
                    if (this.ephemeralState.scrollbackContent.length > MAX_SCROLLBACK_SIZE) {
                        // Find a good break point (newline) near the trim point
                        const trimPoint = this.ephemeralState.scrollbackContent.length - MAX_SCROLLBACK_SIZE;
                        const newlineAfterTrim = this.ephemeralState.scrollbackContent.indexOf('\n', trimPoint);
                        if (newlineAfterTrim > 0) {
                            this.ephemeralState.scrollbackContent = this.ephemeralState.scrollbackContent.slice(newlineAfterTrim + 1);
                        } else {
                            this.ephemeralState.scrollbackContent = this.ephemeralState.scrollbackContent.slice(trimPoint);
                        }
                    }
                }
                break;
            case 'screen':
                // Ephemeral state - no storage write
                this.ephemeralState.lastScreen = event.content;
                break;
            case 'title':
                if (this.persistentState.lastTitle !== event.title) {
                    this.persistentState.lastTitle = event.title;
                    persistentChanged = true;
                }
                break;
            case 'title_history':
                // Ephemeral state - no storage write
                this.ephemeralState.lastTitleHistory = event.history;
                break;
            case 'prompt':
                // Compare prompts - this is critical for dashboard interaction
                const currentPromptJson = JSON.stringify(this.persistentState.currentPrompt);
                const newPromptJson = JSON.stringify(event.prompt);
                if (currentPromptJson !== newPromptJson) {
                    this.persistentState.currentPrompt = event.prompt;
                    persistentChanged = true;
                }
                break;
        }

        // Increment ephemeral sequence
        this.ephemeralState.eventSequence++;

        // Only persist to storage on meaningful state changes
        // This is the key optimization: screen/scrollback updates (high frequency)
        // no longer trigger storage writes
        if (persistentChanged) {
            await this.state.storage.put('persistentState', this.persistentState);
        }

        // Broadcast to all SSE clients (still immediate for real-time feel)
        await this.broadcast(event);

        // Notify SessionListDO of activity (throttled to every 10s)
        // This keeps sessions alive during deploys via the grace period
        if (this.sessionInfo && this.shouldUpdateLastSeen()) {
            this.notifySessionLastSeen();
        }
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

        // Send accumulated scrollback history first (so it appears before screen)
        // Note: This is ephemeral state - may be empty after DO hibernation
        if (this.ephemeralState.scrollbackContent) {
            const scrollbackHistoryEvent: SessionEvent = {
                type: 'scrollback_history',
                content: this.ephemeralState.scrollbackContent,
            };
            await this.sendSSE(writer, scrollbackHistoryEvent);
        }

        // Send screen snapshot (for immediate visual)
        // Note: This is ephemeral state - may be empty after DO hibernation
        if (this.ephemeralState.lastScreen) {
            const screenEvent: SessionEvent = {
                type: 'screen',
                content: this.ephemeralState.lastScreen,
            };
            await this.sendSSE(writer, screenEvent);
        }

        // Send current state (persistent)
        const stateEvent: SessionEvent = {
            type: 'state',
            state: this.persistentState.state,
            timestamp: Date.now(),
        };
        await this.sendSSE(writer, stateEvent);

        // Send current title if available (persistent)
        if (this.persistentState.lastTitle) {
            const titleEvent: SessionEvent = {
                type: 'title',
                title: this.persistentState.lastTitle,
            };
            await this.sendSSE(writer, titleEvent);
        }

        // Send title history if available (ephemeral)
        if (this.ephemeralState.lastTitleHistory && this.ephemeralState.lastTitleHistory.length > 0) {
            const titleHistoryEvent: SessionEvent = {
                type: 'title_history',
                history: this.ephemeralState.lastTitleHistory,
            };
            await this.sendSSE(writer, titleHistoryEvent);
        }

        // Send current prompt if any (persistent - for interactive dashboard)
        if (this.persistentState.currentPrompt) {
            const promptEvent: SessionEvent = {
                type: 'prompt',
                prompt: this.persistentState.currentPrompt,
            };
            await this.sendSSE(writer, promptEvent);
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
     * Handle key sequence from dashboard, forward to desktop
     * Used for Tab instructions (navigate + tab + type + enter)
     */
    private async handleKeySequence(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        let body: { steps: KeyStep[] };
        try {
            body = await request.json();
        } catch {
            return new Response(
                JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
                { status: 400, headers: { 'Content-Type': 'application/json' } }
            );
        }

        if (!body.steps || !Array.isArray(body.steps) || body.steps.length === 0) {
            return new Response(
                JSON.stringify({ error: 'Missing or empty steps', code: 'MISSING_STEPS' }),
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
            type: 'key_sequence',
            steps: body.steps,
        };

        try {
            this.desktopWs.send(JSON.stringify(message));
        } catch (error) {
            console.error('Error sending key sequence to desktop:', error);
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
     * Save draft input text (for persistence across deploys)
     */
    private async handleSaveDraft(request: Request): Promise<Response> {
        try {
            const body = await request.json() as { text?: string };
            const text = body.text || '';
            await this.state.storage.put('draft', text);
            return new Response(JSON.stringify({ ok: true }), {
                headers: { 'Content-Type': 'application/json' }
            });
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid request' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' }
            });
        }
    }

    /**
     * Get draft input text
     */
    private async handleGetDraft(): Promise<Response> {
        const text = await this.state.storage.get<string>('draft') || '';
        return new Response(JSON.stringify({ text }), {
            headers: { 'Content-Type': 'application/json' }
        });
    }

    /**
     * Get current session state (for debugging)
     */
    private handleGetState(): Response {
        return new Response(
            JSON.stringify({
                state: this.persistentState.state,
                scrollback_lines: this.ephemeralState.lastScrollbackLine,
                has_screen: this.ephemeralState.lastScreen !== null,
                title: this.persistentState.lastTitle,
                event_sequence: this.ephemeralState.eventSequence,
                desktop_connected: this.desktopWs !== null,
                sse_clients: this.sseClients.size,
                has_active_viewers: this.hasActiveViewers(),
            }),
            { headers: { 'Content-Type': 'application/json' } }
        );
    }

    /**
     * Handle viewer activity heartbeat from dashboard
     * Viewers send this every 5s while active, stops after 30s inactivity
     */
    private handleViewerActive(): Response {
        this.lastViewerActivity = Date.now();

        // Always notify desktop that viewers are active
        // This ensures desktop knows even if previous notifications were missed
        if (this.desktopWs) {
            this.notifyDesktopViewerStatus(true);
        }

        // Note: Usage tracking is now handled by /api/usage/heartbeat (one per browser)
        // NOT per-session, to avoid multiplying usage when viewing multiple sessions

        return new Response(JSON.stringify({ ok: true }), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Record viewer heartbeat in UsageDO for usage tracking
     * Fire-and-forget to avoid blocking the response
     */
    private recordUsageHeartbeat(): void {
        if (!this.sessionInfo?.group_id || !this.persistentState.sessionId) {
            return;
        }

        const groupId = this.sessionInfo.group_id;
        const sessionId = this.persistentState.sessionId;

        // Fire and forget - don't await
        const doId = this.env.USAGE.idFromName(groupId);
        const stub = this.env.USAGE.get(doId);
        stub.fetch(new Request(`https://internal/heartbeat?group_id=${groupId}`, {
            method: 'POST',
            body: JSON.stringify({ session_id: sessionId }),
            headers: { 'Content-Type': 'application/json' },
        })).catch((error) => {
            console.error('Error recording usage heartbeat:', error);
        });
    }

    /**
     * Check if there are active viewers (heartbeat within timeout)
     */
    private hasActiveViewers(): boolean {
        return Date.now() - this.lastViewerActivity < VIEWER_ACTIVITY_TIMEOUT_MS;
    }

    /**
     * Notify desktop WebSocket about viewer status change
     */
    private notifyDesktopViewerStatus(active: boolean): void {
        if (!this.desktopWs) return;

        // For inactive notifications, still track to avoid spam
        // For active notifications, always send (heartbeats are throttled to 5s anyway)
        if (!active && this.desktopNotifiedViewerActive === active) return;
        this.desktopNotifiedViewerActive = active;

        const message = {
            type: 'viewer_status',
            active,
        };

        try {
            this.desktopWs.send(JSON.stringify(message));
        } catch {
            // Connection may have failed, ignore
        }
    }
}
