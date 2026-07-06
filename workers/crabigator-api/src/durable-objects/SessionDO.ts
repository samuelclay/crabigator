import type {
    SessionEvent,
    SessionState,
    CloudToDesktopMessage,
    CloudPromptData,
    KeyStep,
    GitEvent,
    GitCommitInfo,
    ChangesEvent,
    StatsEvent,
} from '../types/session';
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
    lastTitleHistory: string[] | null;
    /** Most recent recap event (status + latest finished recap). */
    lastRecap: any | null;
    /** Full recap history for this session, oldest first. */
    lastRecapHistory: any[] | null;
    /** Commits detected after this web session established a baseline, oldest first. */
    lastCommitHistory: GitCommitInfo[] | null;
    /** Last git HEAD hash seen by the web side. */
    lastCommitHeadHash: string | null;
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
    lastGit: GitEvent | null;
    lastChanges: ChangesEvent | null;
    lastStats: StatsEvent | null;
    eventSequence: number;
}

/** Viewer activity timeout - 35s to allow for 5s heartbeat intervals */
const VIEWER_ACTIVITY_TIMEOUT_MS = 35_000;
/** Desktop heartbeat cadence is 2h; allow multiple missed beats before culling. */
const SESSION_HEARTBEAT_TIMEOUT_MS = 6 * 60 * 60 * 1000;

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
    /** Last time we updated D1 last_seen_at (throttled separately, 60s) */
    private lastD1SeenUpdate: number = 0;

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;
        this.persistentState = {
            sessionId: '',
            state: 'ready',
            currentPrompt: null,
            lastTitle: null,
            lastTitleHistory: null,
            lastRecap: null,
            lastRecapHistory: null,
            lastCommitHistory: null,
            lastCommitHeadHash: null,
        };
        this.ephemeralState = {
            lastScrollbackLine: 0,
            scrollbackContent: '',
            lastScreen: null,
            lastGit: null,
            lastChanges: null,
            lastStats: null,
            eventSequence: 0,
        };

        // Restore persistent state from storage
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<PersistentState>('persistentState');
            if (stored) {
                this.persistentState = { ...this.persistentState, ...stored };
            }
            const storedInfo = await state.storage.get<SessionInfo>('sessionInfo');
            if (storedInfo) {
                this.sessionInfo = storedInfo;
            }
            // Backfill title history from D1 if missing from DO storage
            if (!this.persistentState.lastTitleHistory && this.persistentState.sessionId) {
                const row = await this.env.DB.prepare(
                    'SELECT titles FROM sessions WHERE id = ?'
                ).bind(this.persistentState.sessionId).first<{ titles: string | null }>();
                if (row?.titles) {
                    try {
                        this.persistentState.lastTitleHistory = JSON.parse(row.titles);
                        await state.storage.put('persistentState', this.persistentState);
                    } catch {}
                }
            }

            // Restore desktop WebSocket reference after hibernation wake-up
            const existingDesktopWs = state.getWebSockets('desktop');
            if (existingDesktopWs.length > 0) {
                this.desktopWs = existingDesktopWs[0];
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
            case '/spawn':
                return this.handleSpawn(request);
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

            // Mark D1 active for zombie detection and recovery after transient
            // disconnects or a prior stale cleanup.
            this.updateD1LastSeen(true);

            // Notify desktop of current viewer status
            if (this.hasActiveViewers()) {
                this.notifyDesktopViewerStatus(true);
            }
        }

        const pair = new WebSocketPair();
        const [client, server] = [pair[0], pair[1]];

        // Use Hibernation API - DO can sleep between messages, reducing billed duration
        this.state.acceptWebSocket(server, ['desktop']);
        this.desktopWs = server;

        // Event handling is done via webSocketMessage/webSocketClose/webSocketError methods
        // instead of addEventListener, enabling the DO to hibernate between messages

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
     * Check if enough time has passed to send another D1 last_seen_at update
     * Throttles to every 60 seconds to minimize D1 writes
     */
    private shouldUpdateD1LastSeen(): boolean {
        return Date.now() - this.lastD1SeenUpdate > 60_000; // 60s throttle
    }

    /**
     * Update last_seen_at in D1 for zombie session detection
     *
     * This is used by the scheduled cleanup job to identify sessions that
     * have stopped sending activity. Fire-and-forget to avoid blocking.
     */
    private updateD1LastSeen(reactivate = false): void {
        if (!this.sessionInfo) return;
        this.lastD1SeenUpdate = Date.now();
        const now = Math.floor(Date.now() / 1000);

        const query = reactivate
            ? `UPDATE sessions SET last_seen_at = ?, is_active = 1, ended_at = NULL WHERE id = ?`
            : `UPDATE sessions SET last_seen_at = ? WHERE id = ? AND is_active = 1`;

        this.env.DB.prepare(query).bind(now, this.sessionInfo.id).run().catch((error) => {
            console.error('Error updating last_seen_at:', error);
        });
    }

    /**
     * Build web-only commit history from the bounded recent log desktop sends
     * with git status. The first observed HEAD is a baseline; later HEAD changes
     * are commits that happened while the session was visible to the web layer.
     */
    private updateCommitHistoryFromGit(event: GitEvent): {
        persistentChanged: boolean;
        historyEvent: SessionEvent | null;
    } {
        const recentCommits = event.recent_commits || [];
        const currentHead = recentCommits[0]?.hash;
        if (!currentHead) {
            return { persistentChanged: false, historyEvent: null };
        }

        let persistentChanged = false;
        let historyChanged = false;
        const previousHead =
            this.persistentState.lastCommitHeadHash ||
            this.ephemeralState.lastGit?.recent_commits?.[0]?.hash ||
            null;

        if (previousHead && previousHead !== currentHead) {
            const previousIndex = recentCommits.findIndex(commit => commit.hash === previousHead);
            const newCommits = previousIndex === -1
                ? recentCommits.slice(0, 1)
                : recentCommits.slice(0, previousIndex);

            historyChanged = this.appendCommitHistory(newCommits.slice().reverse());
            persistentChanged = persistentChanged || historyChanged;
        }

        if (this.persistentState.lastCommitHeadHash !== currentHead) {
            this.persistentState.lastCommitHeadHash = currentHead;
            persistentChanged = true;
        }

        if (!historyChanged) {
            return { persistentChanged, historyEvent: null };
        }

        return {
            persistentChanged,
            historyEvent: {
                type: 'commit_history',
                history: this.persistentState.lastCommitHistory || [],
            },
        };
    }

    private appendCommitHistory(commits: GitCommitInfo[]): boolean {
        if (commits.length === 0) return false;

        const existing = this.persistentState.lastCommitHistory || [];
        const seenHashes = new Set(existing.map(commit => commit.hash));
        const merged = [...existing];

        for (const commit of commits) {
            if (!commit.hash || seenHashes.has(commit.hash)) continue;
            merged.push(commit);
            seenHashes.add(commit.hash);
        }

        if (merged.length === existing.length) return false;

        this.persistentState.lastCommitHistory = merged.slice(-100);
        return true;
    }

    /**
     * Handle incoming event from desktop
     */
    private async handleEvent(event: SessionEvent): Promise<void> {
        if (event.type === 'heartbeat') {
            if (this.sessionInfo) {
                await this.notifySessionList('connect', this.sessionInfo);
                this.updateD1LastSeen(true);
            }
            return;
        }

        // Track whether persistent state changed (requires storage write)
        let persistentChanged = false;
        const postBroadcastEvents: SessionEvent[] = [];

        // Update local state based on event type
        switch (event.type) {
            case 'state':
                if (this.persistentState.state !== event.state) {
                    this.persistentState.state = event.state;
                    persistentChanged = true;
                    // Clear prompt when leaving interactive states (permission/question)
                    // This prevents stale prompts from being sent to reconnecting viewers
                    if (event.state !== 'permission' && event.state !== 'question') {
                        this.persistentState.currentPrompt = null;
                    }
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
                this.persistentState.lastTitleHistory = event.history;
                persistentChanged = true;
                // Persist to D1 for analytics
                if (event.history && event.history.length > 0 && this.persistentState.sessionId) {
                    this.env.DB.prepare('UPDATE sessions SET titles = ? WHERE id = ?')
                        .bind(JSON.stringify(event.history), this.persistentState.sessionId)
                        .run()
                        .catch(() => {});
                }
                break;
            case 'recap': {
                // Store the most recent recap state for late-joining SSE viewers.
                const incoming = JSON.stringify(event);
                const stored = JSON.stringify(this.persistentState.lastRecap);
                if (stored !== incoming) {
                    this.persistentState.lastRecap = event;
                    persistentChanged = true;
                }
                break;
            }
            case 'recap_history': {
                this.persistentState.lastRecapHistory = (event as any).history || [];
                persistentChanged = true;
                break;
            }
            case 'git':
                {
                    const commitUpdate = this.updateCommitHistoryFromGit(event as GitEvent);
                    persistentChanged = persistentChanged || commitUpdate.persistentChanged;
                    if (commitUpdate.historyEvent) {
                        postBroadcastEvents.push(commitUpdate.historyEvent);
                    }
                }
                // Git status itself is ephemeral; commit history above is web-only persistent state.
                this.ephemeralState.lastGit = event as GitEvent;
                break;
            case 'changes':
                // Ephemeral state - no storage write
                this.ephemeralState.lastChanges = event as ChangesEvent;
                break;
            case 'stats':
                // Ephemeral state - cache for late-joining viewers
                this.ephemeralState.lastStats = event as StatsEvent;
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
        for (const followupEvent of postBroadcastEvents) {
            await this.broadcast(followupEvent);
        }

        // Notify SessionListDO of activity (throttled to every 10s)
        // This keeps sessions alive during deploys via the grace period
        if (this.sessionInfo && this.shouldUpdateLastSeen()) {
            this.notifySessionLastSeen();
        }

        // Update D1 last_seen_at for zombie detection (throttled to every 60s)
        if (this.sessionInfo && this.shouldUpdateD1LastSeen()) {
            this.updateD1LastSeen();
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

        // Send title history if available (persistent)
        if (this.persistentState.lastTitleHistory && this.persistentState.lastTitleHistory.length > 0) {
            const titleHistoryEvent: SessionEvent = {
                type: 'title_history',
                history: this.persistentState.lastTitleHistory,
            };
            await this.sendSSE(writer, titleHistoryEvent);
        }

        // Send the latest recap (and its full history) so a freshly-loaded
        // dashboard tab paints the recap card without waiting for the next turn.
        if (this.persistentState.lastRecap) {
            await this.sendSSE(writer, this.persistentState.lastRecap as SessionEvent);
        }
        if (this.persistentState.lastRecapHistory && this.persistentState.lastRecapHistory.length > 0) {
            const historyEvent = {
                type: 'recap_history' as const,
                history: this.persistentState.lastRecapHistory,
            };
            await this.sendSSE(writer, historyEvent as SessionEvent);
        }
        if (this.persistentState.lastCommitHistory && this.persistentState.lastCommitHistory.length > 0) {
            const historyEvent = {
                type: 'commit_history' as const,
                history: this.persistentState.lastCommitHistory,
            };
            await this.sendSSE(writer, historyEvent as SessionEvent);
        }

        // Send git state if available (ephemeral)
        if (this.ephemeralState.lastGit) {
            await this.sendSSE(writer, this.ephemeralState.lastGit);
        }

        // Send changes state if available (ephemeral)
        if (this.ephemeralState.lastChanges) {
            await this.sendSSE(writer, this.ephemeralState.lastChanges);
        }

        // Send stats if available (ephemeral - cached from last desktop push)
        if (this.ephemeralState.lastStats) {
            await this.sendSSE(writer, this.ephemeralState.lastStats);
        }

        // Send current prompt if any (persistent - for interactive dashboard)
        // Only send if session is in interactive state (permission/question)
        const isInteractiveState = this.persistentState.state === 'permission' || this.persistentState.state === 'question';
        if (this.persistentState.currentPrompt && isInteractiveState) {
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
     * Handle spawn request from dashboard, forward to desktop
     * Desktop will open a new terminal window with crabigator
     */
    private async handleSpawn(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        let body: { cwd: string; platform?: string };
        try {
            body = await request.json();
        } catch {
            return new Response(
                JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
                { status: 400, headers: { 'Content-Type': 'application/json' } }
            );
        }

        if (!body.cwd) {
            return new Response(
                JSON.stringify({ error: 'Missing cwd', code: 'MISSING_CWD' }),
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
            type: 'spawn',
            cwd: body.cwd,
            ...(body.platform ? { platform: body.platform } : {}),
        };

        try {
            this.desktopWs.send(JSON.stringify(message));
        } catch (error) {
            console.error('Error sending spawn to desktop:', error);
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

    /**
     * WebSocket Hibernation API - called when a message arrives on an accepted WebSocket.
     * The DO wakes from hibernation if needed, allowing it to sleep between messages.
     */
    async webSocketMessage(ws: WebSocket, message: string | ArrayBuffer): Promise<void> {
        try {
            const data = JSON.parse(message as string) as SessionEvent;
            await this.handleEvent(data);
        } catch (error) {
            console.error('Error handling WebSocket message:', error);
        }
    }

    /**
     * WebSocket Hibernation API - called when an accepted WebSocket closes.
     */
    async webSocketClose(ws: WebSocket, code: number, reason: string, wasClean: boolean): Promise<void> {
        if (this.desktopWs === ws || this.desktopWs === null) {
            this.desktopWs = null;
            this.broadcastDesktopStatus(false);
            if (this.sessionInfo) {
                this.notifySessionList('disconnect', { id: this.sessionInfo.id });
            }
            // Schedule missed-heartbeat cleanup. Normal wrapper exits end the
            // session via HTTP; disconnects alone can be transient.
            this.state.storage.setAlarm(Date.now() + SESSION_HEARTBEAT_TIMEOUT_MS);
        }
    }

    /**
     * WebSocket Hibernation API - called when an accepted WebSocket errors.
     */
    async webSocketError(ws: WebSocket, error: unknown): Promise<void> {
        console.error('WebSocket error:', error);
        if (this.desktopWs === ws || this.desktopWs === null) {
            this.desktopWs = null;
            this.broadcastDesktopStatus(false);
            if (this.sessionInfo) {
                this.notifySessionList('disconnect', { id: this.sessionInfo.id });
            }
            this.state.storage.setAlarm(Date.now() + SESSION_HEARTBEAT_TIMEOUT_MS);
        }
    }

    /**
     * Alarm handler for cleaning up disconnected sessions.
     *
     * Called after the missed-heartbeat window. If desktop has reconnected
     * (e.g., after a deploy), do nothing. If the session is still active but
     * has not been seen recently, mark it ended and notify SessionListDO.
     */
    async alarm(): Promise<void> {
        // If desktop reconnected (e.g., after deploy), nothing to clean up
        if (this.desktopWs !== null) {
            return;
        }

        // Desktop is still disconnected and has missed the heartbeat window.
        if (!this.sessionInfo) {
            return;
        }

        const sessionId = this.sessionInfo.id;
        const endedAt = Math.floor(Date.now() / 1000);

        const row = await this.env.DB.prepare(`
            SELECT started_at, last_seen_at, is_active
            FROM sessions
            WHERE id = ?
        `).bind(sessionId).first<{
            started_at: number;
            last_seen_at: number | null;
            is_active: number;
        }>();

        if (!row || row.is_active !== 1) {
            return;
        }

        const lastSeen = row.last_seen_at || row.started_at;
        const cleanupAt = (lastSeen * 1000) + SESSION_HEARTBEAT_TIMEOUT_MS;
        if (Date.now() < cleanupAt) {
            this.state.storage.setAlarm(cleanupAt);
            return;
        }

        // Update D1 to mark session as ended
        try {
            await this.env.DB.prepare(`
                UPDATE sessions
                SET is_active = 0, ended_at = ?
                WHERE id = ? AND is_active = 1
            `).bind(endedAt, sessionId).run();
        } catch (error) {
            console.error('Error updating session in D1:', error);
        }

        // Notify SessionListDO to remove from active sessions
        try {
            const doId = this.env.SESSION_LIST.idFromName('global');
            const stub = this.env.SESSION_LIST.get(doId);
            await stub.fetch(new Request('https://internal/notify', {
                method: 'POST',
                body: JSON.stringify({
                    type: 'updated',
                    session: {
                        id: sessionId,
                        ended_at: endedAt,
                        is_active: false,
                        group_id: this.sessionInfo.group_id,
                    },
                }),
                headers: { 'Content-Type': 'application/json' },
            }));
        } catch (error) {
            console.error('Error notifying SessionListDO:', error);
        }
    }
}
