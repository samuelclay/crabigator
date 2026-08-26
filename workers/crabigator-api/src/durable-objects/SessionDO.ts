import type {
    SessionEvent,
    SessionState,
    CloudToDesktopMessage,
    CloudPromptData,
    KeyStep,
    GitEvent,
    GitCommitInfo,
    ChangesEvent,
    SlackThread,
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
    /** Slack permalinks pasted during this session. */
    lastSlackThreads: SlackThread[] | null;
    /** Enriched metadata for Slack permalinks attached to tracked PRs. */
    lastPrSlackThreads: SlackThread[] | null;
    /** PRs created/updated during this session (recap panel). */
    lastPrs: any[] | null;
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

/**
 * The slice of a recap event the PR board needs, as the JSON string stored
 * in sessions.recap: display text, generation time, and the turn's line delta.
 * Null when the event carries no finished recap.
 */
function recapBrief(event: any): string | null {
    const latest = event?.latest;
    if (!latest?.headline) return null;
    return JSON.stringify({
        headline: latest.headline,
        bullets: Array.isArray(latest.bullets) ? latest.bullets : [],
        next_prompt_notes: Array.isArray(latest.next_prompt_notes)
            ? latest.next_prompt_notes
            : [],
        artifacts: Array.isArray(latest.artifacts) ? latest.artifacts : [],
        generated_at: latest.generated_at || 0,
        additions: latest.line_delta?.additions || 0,
        deletions: latest.line_delta?.deletions || 0,
    });
}

/**
 * Scrollback search, matching the desktop board's transcript search: one or
 * two letters hit nearly every line, so a query needs three characters; the
 * expanded preview shows the last few matches with a little context each.
 */
const SCROLLBACK_QUERY_MIN = 3;
const SCROLLBACK_PREVIEW_MATCHES = 3;
const SCROLLBACK_PREVIEW_CONTEXT = 2;

/** One line of a scrollback preview, before the client styles it. */
interface ScrollbackPreviewRow {
    text: string;
    is_match: boolean;
    /** This row opens a new context group — the client draws a `⋯` first. */
    gap_before: boolean;
}

/**
 * Terminal styling has no meaning in a text search, so drop it first: OSC
 * strings (including hyperlinks), then CSI and other escape sequences.
 */
function stripAnsi(text: string): string {
    return text
        .replace(/\x1b\][^\x07\x1b]*(?:\x07|\x1b\\)/g, '')
        .replace(/\x1b\[[0-9;?]*[ -/]*[@-~]/g, '')
        .replace(/\x1b[@-Z\\-_]/g, '')
        .replace(/\r/g, '');
}

/** JSON body for the DO's internal search route. */
function searchResponse(data: unknown): Response {
    return new Response(JSON.stringify(data), {
        headers: { 'Content-Type': 'application/json' },
    });
}

/**
 * Context windows around the last few matches, merged where they touch —
 * the same shape the desktop board's expanded preview uses.
 */
function previewRows(lines: string[], matches: number[]): ScrollbackPreviewRow[] {
    const recent = matches.slice(-SCROLLBACK_PREVIEW_MATCHES);
    const ranges: [number, number][] = [];
    for (const index of recent) {
        const start = Math.max(0, index - SCROLLBACK_PREVIEW_CONTEXT);
        const end = Math.min(lines.length - 1, index + SCROLLBACK_PREVIEW_CONTEXT);
        const last = ranges[ranges.length - 1];
        if (last && start <= last[1] + 1) last[1] = Math.max(last[1], end);
        else ranges.push([start, end]);
    }
    const matched = new Set(matches);
    const rows: ScrollbackPreviewRow[] = [];
    ranges.forEach(([start, end], group) => {
        for (let index = start; index <= end; index++) {
            rows.push({
                text: lines[index],
                is_match: matched.has(index),
                gap_before: group > 0 && index === start,
            });
        }
    });
    return rows;
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
    device_name?: string;
    group_id?: string | null;
}

/**
 * Durable Object for managing a single crabigator session
 *
 * Handles:
 * - Desktop WebSocket connection (receives events, sends answers)
 * - Hibernatable WebSocket streams for mobile/web viewers
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
    private persistentState: PersistentState;
    private ephemeralState: EphemeralState;
    private sessionInfo: SessionInfo | null = null;
    /** Last time a viewer (dashboard/phone) signaled activity */
    private lastViewerActivity: number = 0;
    /** Whether we've notified desktop that viewers are active */
    private desktopNotifiedViewerActive: boolean = false;
    /** Last time we notified SessionListDO of activity (throttled) */
    private lastSeenNotifiedAt: number = 0;
    /** Last time we updated D1 last_seen_at (throttled separately, 60s) */
    private lastD1SeenUpdate: number = 0;
    /** Last time session_prs rows were written to D1 (60s throttle) */
    private lastD1PrsUpdate: number = 0;
    /** Serialized form of the last session_prs write, to skip no-op writes */
    private lastD1PrsSnapshot: string = '';
    /** Last repository identity and worktree numbers written to the session row. */
    private lastD1RepositorySnapshot: string = '';
    /** Last Slack thread list written to the session row. */
    private lastD1SlackSnapshot: string = '';
    /** Serializes stats persistence so external D1 I/O cannot reorder updates. */
    private statsUpdateQueue: Promise<void> = Promise.resolve();

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
            lastSlackThreads: null,
            lastPrSlackThreads: null,
            lastPrs: null,
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
                return this.handleViewerWebSocket(request);
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
            case '/prs':
                // Dump the stored PR list — used to backfill session_prs in
                // D1 for sessions that predate the write-through.
                return new Response(
                    JSON.stringify({ prs: this.persistentState.lastPrs || [] }),
                    { headers: { 'Content-Type': 'application/json' } }
                );
            case '/search':
                // Grep this session's accumulated scrollback — the web PR
                // board's search, standing in for the desktop board's read of
                // the session's local scrollback.log.
                return this.handleScrollbackSearch(request);
            case '/viewer-active':
                return this.handleViewerActive();
            case '/pr-overrides-changed':
                return this.handlePrOverridesChanged();
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
        const deviceName = url.searchParams.get('device_name') || undefined;
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
                device_name: deviceName,
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
     * Write the session's PR list through to D1 for the cross-session PR
     * board. Diffed (no-op events skipped) and throttled to one write per
     * minute per session — prs events fire on every enrichment poll and the
     * write is fire-and-forget, matching the titles precedent.
     */
    private persistPrsToD1(prs: any[]): void {
        const sessionId = this.persistentState.sessionId || this.sessionInfo?.id;
        if (!sessionId || prs.length === 0) return;
        const snapshot = JSON.stringify(prs);
        if (snapshot === this.lastD1PrsSnapshot) return;
        const firstWrite = this.lastD1PrsSnapshot === '';
        if (!firstWrite && Date.now() - this.lastD1PrsUpdate < 60_000) return;
        this.lastD1PrsUpdate = Date.now();
        this.lastD1PrsSnapshot = snapshot;

        const now = Math.floor(Date.now() / 1000);
        const statements = prs
            .filter((pr) => pr && pr.owner && pr.repo && pr.number)
            .map((pr) =>
                this.env.DB.prepare(
                    `INSERT INTO session_prs (session_id, owner, repo, number, url, state, is_primary, data, updated_at)
                     VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                     ON CONFLICT (session_id, owner, repo, number)
                     DO UPDATE SET url = excluded.url, state = excluded.state,
                                   is_primary = excluded.is_primary, data = excluded.data,
                                   updated_at = excluded.updated_at`
                ).bind(
                    sessionId,
                    pr.owner,
                    pr.repo,
                    pr.number,
                    pr.url || '',
                    pr.state || '',
                    pr.primary ? 1 : 0,
                    JSON.stringify(pr),
                    now
                )
            );
        if (statements.length > 0) {
            this.env.DB.batch(statements).catch(() => {});
        }
    }

    /**
     * Search this session's scrollback and return the same preview rows the
     * desktop board builds: the newest matching line on its own, plus the
     * last few matches with surrounding context for the expanded view.
     */
    private handleScrollbackSearch(request: Request): Response {
        const query = (new URL(request.url).searchParams.get('q') || '').toLowerCase();
        const empty = { total: 0, collapsed: '', rows: [] as ScrollbackPreviewRow[] };
        if (query.length < SCROLLBACK_QUERY_MIN || !this.ephemeralState.scrollbackContent) {
            return searchResponse(empty);
        }
        const lines = stripAnsi(this.ephemeralState.scrollbackContent).split('\n');
        const matches: number[] = [];
        for (let i = 0; i < lines.length; i++) {
            if (lines[i].toLowerCase().includes(query)) matches.push(i);
        }
        if (matches.length === 0) return searchResponse(empty);
        return searchResponse({
            total: matches.length,
            collapsed: lines[matches[matches.length - 1]],
            rows: previewRows(lines, matches),
        });
    }

    /**
     * Keep the Slack threads seen in this session beside its durable record.
     * The desktop enriches each permalink with its channel name and author
     * from local Slack metadata; storing the enriched thread lets the web PR
     * board label Slack links the same way instead of showing channel IDs.
     */
    /**
     * Pasted permalinks plus the PR-attached ones, deduped by URL. Pasted
     * threads come first and win, since the session enriches them earliest.
     */
    private mergedSlackThreads(): SlackThread[] {
        const merged: SlackThread[] = [];
        const seen = new Set<string>();
        for (const thread of [
            ...(this.persistentState.lastSlackThreads || []),
            ...(this.persistentState.lastPrSlackThreads || []),
        ]) {
            if (!thread?.url || seen.has(thread.url)) continue;
            seen.add(thread.url);
            merged.push(thread);
        }
        return merged;
    }

    private persistSlackThreadsToD1(threads: SlackThread[]): void {
        const sessionId = this.persistentState.sessionId || this.sessionInfo?.id;
        if (!sessionId || threads.length === 0) return;
        const snapshot = JSON.stringify(threads);
        if (snapshot === this.lastD1SlackSnapshot) return;
        this.env.DB.prepare('UPDATE sessions SET slack_threads = ? WHERE id = ?')
            .bind(snapshot, sessionId)
            .run()
            .then(() => {
                this.lastD1SlackSnapshot = snapshot;
            })
            .catch(() => {});
    }

    /**
     * Keep the session's repository, branch, and working-tree numbers beside
     * its durable session record. This lets the PR board retain active
     * sessions before they have created or mentioned a pull request, and draw
     * their uncommitted diff the way the desktop board does.
     */
    private persistRepositoryToD1(event: GitEvent): void {
        const sessionId = this.persistentState.sessionId || this.sessionInfo?.id;
        if (!sessionId) return;
        const files = event.files || [];
        const additions = files.reduce((sum, file) => sum + (file.additions || 0), 0);
        const deletions = files.reduce((sum, file) => sum + (file.deletions || 0), 0);
        const snapshot = JSON.stringify([
            event.repo_owner || '',
            event.repo_name || '',
            event.branch || '',
            files.length,
            additions,
            deletions,
        ]);
        if (snapshot === this.lastD1RepositorySnapshot) return;
        this.env.DB.prepare(
            `UPDATE sessions
             SET repo_owner = ?, repo_name = ?, branch = ?,
                 uncommitted_files = ?, additions = ?, deletions = ?
             WHERE id = ?`
        ).bind(
            event.repo_owner || '',
            event.repo_name || '',
            event.branch || '',
            files.length,
            additions,
            deletions,
            sessionId
        ).run()
            .then(() => {
                this.lastD1RepositorySnapshot = snapshot;
            })
            .catch((error) => {
                console.error('Error persisting session repository:', error);
            });
    }

    /**
     * Build web-only commit history from the bounded recent log desktop sends
     * with git status. If no history has been recorded yet, backfill commits
     * from this session's start time so late Worker/client upgrades don't
     * silently discard commits that already happened in the session.
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
        const hasCommitHistory = (this.persistentState.lastCommitHistory?.length || 0) > 0;
        const previousHead =
            this.persistentState.lastCommitHeadHash ||
            this.ephemeralState.lastGit?.recent_commits?.[0]?.hash ||
            null;

        let newCommits: GitCommitInfo[] = [];
        if (!hasCommitHistory) {
            newCommits = this.commitsSinceSessionStart(recentCommits);
        }

        if (newCommits.length === 0 && previousHead && previousHead !== currentHead) {
            const previousIndex = recentCommits.findIndex(commit => commit.hash === previousHead);
            newCommits = previousIndex === -1
                ? recentCommits.slice(0, 1)
                : recentCommits.slice(0, previousIndex);
        }

        if (newCommits.length > 0) {
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

    private commitsSinceSessionStart(recentCommits: GitCommitInfo[]): GitCommitInfo[] {
        const sessionStartedAt = this.sessionInfo?.started_at || 0;
        if (!sessionStartedAt) return [];

        return recentCommits.filter(commit => commit.timestamp >= sessionStartedAt);
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
            case 'title_history': {
                const history = JSON.stringify(event.history || []);
                const changed = history !== JSON.stringify(this.persistentState.lastTitleHistory);
                this.persistentState.lastTitleHistory = event.history;
                persistentChanged = true;
                // Persist to D1 for analytics. The change time rides along so
                // the PR board can pick the newest title among a PR's sessions
                // the way the desktop board does — which means only a real
                // change may move it.
                if (changed && event.history?.length && this.persistentState.sessionId) {
                    this.env.DB.prepare(
                        'UPDATE sessions SET titles = ?, titles_changed_at = ? WHERE id = ?'
                    )
                        .bind(history, Date.now(), this.persistentState.sessionId)
                        .run()
                        .catch(() => {});
                }
                break;
            }
            case 'recap': {
                // Store the most recent recap state for late-joining viewers.
                const incoming = JSON.stringify(event);
                const stored = JSON.stringify(this.persistentState.lastRecap);
                if (stored !== incoming) {
                    const previousBrief = recapBrief(this.persistentState.lastRecap);
                    this.persistentState.lastRecap = event;
                    persistentChanged = true;
                    // Persist a small brief to D1 so the PR board can show
                    // what each session was doing — and how stale that
                    // picture is — after it ends (titles precedent).
                    const brief = recapBrief(event);
                    if (brief && brief !== previousBrief && this.persistentState.sessionId) {
                        this.env.DB.prepare('UPDATE sessions SET recap = ? WHERE id = ?')
                            .bind(brief, this.persistentState.sessionId)
                            .run()
                            .catch(() => {});
                    }
                }
                break;
            }
            case 'recap_history': {
                this.persistentState.lastRecapHistory = (event as any).history || [];
                persistentChanged = true;
                break;
            }
            case 'slack_threads': {
                this.persistentState.lastSlackThreads = event.threads;
                persistentChanged = true;
                this.persistSlackThreadsToD1(this.mergedSlackThreads());
                break;
            }
            case 'pr_slack_threads': {
                this.persistentState.lastPrSlackThreads = (event as any).threads || [];
                persistentChanged = true;
                this.persistSlackThreadsToD1(this.mergedSlackThreads());
                break;
            }
            case 'prs': {
                this.persistentState.lastPrs = (event as any).prs || [];
                persistentChanged = true;
                this.persistPrsToD1(this.persistentState.lastPrs || []);
                break;
            }
            case 'git':
                {
                    const gitEvent = event as GitEvent;
                    const commitUpdate = this.updateCommitHistoryFromGit(gitEvent);
                    persistentChanged = persistentChanged || commitUpdate.persistentChanged;
                    if (commitUpdate.historyEvent) {
                        postBroadcastEvents.push(commitUpdate.historyEvent);
                    }
                    this.persistRepositoryToD1(gitEvent);
                }
                // Git status itself is ephemeral; commit history above is web-only persistent state.
                this.ephemeralState.lastGit = event as GitEvent;
                break;
            case 'changes':
                // Ephemeral state - no storage write
                this.ephemeralState.lastChanges = event as ChangesEvent;
                break;
            case 'stats': {
                const stats = event as StatsEvent;
                this.statsUpdateQueue = this.statsUpdateQueue.then(async () => {
                    // Cache for late-joining viewers and persist low-frequency
                    // prompt/completion timestamps for the D1-backed session list.
                    const previousStats = this.ephemeralState.lastStats;
                    const promptTimestampChanged = stats.prompts_changed_at !== undefined
                        && stats.prompts_changed_at !== previousStats?.prompts_changed_at;
                    const completionTimestampChanged = stats.completions_changed_at !== undefined
                        && stats.completions_changed_at !== previousStats?.completions_changed_at;
                    let recencyPersisted = true;

                    if ((promptTimestampChanged || completionTimestampChanged)
                        && this.persistentState.sessionId) {
                        try {
                            await this.env.DB.prepare(`
                                UPDATE sessions
                                SET prompts_changed_at = COALESCE(?, prompts_changed_at),
                                    completions_changed_at = COALESCE(?, completions_changed_at)
                                WHERE id = ?
                            `).bind(
                                stats.prompts_changed_at ?? null,
                                stats.completions_changed_at ?? null,
                                this.persistentState.sessionId,
                            ).run();
                        } catch (error) {
                            recencyPersisted = false;
                            console.error('Failed to persist session recency timestamps', {
                                sessionId: this.persistentState.sessionId,
                                error,
                            });
                        }
                    }

                    this.ephemeralState.lastStats = recencyPersisted
                        ? stats
                        : {
                            ...stats,
                            prompts_changed_at: previousStats?.prompts_changed_at,
                            completions_changed_at: previousStats?.completions_changed_at,
                        };
                });
                await this.statsUpdateQueue;
                break;
            }
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

        // Broadcast to all dashboard viewers (still immediate for real-time feel)
        this.broadcast(event);
        for (const followupEvent of postBroadcastEvents) {
            this.broadcast(followupEvent);
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
     * Handle a hibernatable WebSocket connection from a mobile/web viewer.
     */
    private handleViewerWebSocket(request: Request): Response {
        if (request.headers.get('Upgrade')?.toLowerCase() !== 'websocket') {
            return new Response('Expected WebSocket', { status: 426 });
        }

        const pair = new WebSocketPair();
        const [client, server] = [pair[0], pair[1]];
        this.state.acceptWebSocket(server, ['viewer']);

        try {
            this.sendCurrentState(server);
        } catch (error) {
            console.error('Error sending initial viewer state:', error);
            server.close(1011, 'Initial state failed');
        }

        return new Response(null, { status: 101, webSocket: client });
    }

    /**
     * Send current state to a newly connected viewer.
     */
    private sendCurrentState(viewer: WebSocket): void {
        // Send desktop connection status first
        // This allows dashboard to immediately remove cards for disconnected sessions
        const desktopStatusEvent: SessionEvent = {
            type: 'desktop_status',
            connected: this.desktopWs !== null,
            timestamp: Date.now(),
        };
        this.sendViewerEvent(viewer, desktopStatusEvent);

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
            this.sendViewerEvent(viewer, scrollbackHistoryEvent);
        }

        // Send screen snapshot (for immediate visual)
        // Note: This is ephemeral state - may be empty after DO hibernation
        if (this.ephemeralState.lastScreen) {
            const screenEvent: SessionEvent = {
                type: 'screen',
                content: this.ephemeralState.lastScreen,
            };
            this.sendViewerEvent(viewer, screenEvent);
        }

        // Send current state (persistent)
        const stateEvent: SessionEvent = {
            type: 'state',
            state: this.persistentState.state,
            timestamp: Date.now(),
        };
        this.sendViewerEvent(viewer, stateEvent);

        // Send current title if available (persistent)
        if (this.persistentState.lastTitle) {
            const titleEvent: SessionEvent = {
                type: 'title',
                title: this.persistentState.lastTitle,
            };
            this.sendViewerEvent(viewer, titleEvent);
        }

        // Send title history if available (persistent)
        if (this.persistentState.lastTitleHistory && this.persistentState.lastTitleHistory.length > 0) {
            const titleHistoryEvent: SessionEvent = {
                type: 'title_history',
                history: this.persistentState.lastTitleHistory,
            };
            this.sendViewerEvent(viewer, titleHistoryEvent);
        }

        // Send the latest recap (and its full history) so a freshly-loaded
        // dashboard tab paints the recap card without waiting for the next turn.
        if (this.persistentState.lastRecap) {
            this.sendViewerEvent(viewer, this.persistentState.lastRecap as SessionEvent);
        }
        if (this.persistentState.lastRecapHistory && this.persistentState.lastRecapHistory.length > 0) {
            const historyEvent = {
                type: 'recap_history' as const,
                history: this.persistentState.lastRecapHistory,
            };
            this.sendViewerEvent(viewer, historyEvent as SessionEvent);
        }
        if (this.persistentState.lastSlackThreads && this.persistentState.lastSlackThreads.length > 0) {
            const slackThreadsEvent = {
                type: 'slack_threads' as const,
                threads: this.persistentState.lastSlackThreads,
            };
            this.sendViewerEvent(viewer, slackThreadsEvent as SessionEvent);
        }
        if (this.persistentState.lastPrs && this.persistentState.lastPrs.length > 0) {
            const prsEvent = {
                type: 'prs' as const,
                prs: this.persistentState.lastPrs,
            };
            this.sendViewerEvent(viewer, prsEvent as SessionEvent);
        }
        if (this.persistentState.lastCommitHistory && this.persistentState.lastCommitHistory.length > 0) {
            const historyEvent = {
                type: 'commit_history' as const,
                history: this.persistentState.lastCommitHistory,
            };
            this.sendViewerEvent(viewer, historyEvent as SessionEvent);
        }

        // Send git state if available (ephemeral)
        if (this.ephemeralState.lastGit) {
            this.sendViewerEvent(viewer, this.ephemeralState.lastGit);
        }

        // Send changes state if available (ephemeral)
        if (this.ephemeralState.lastChanges) {
            this.sendViewerEvent(viewer, this.ephemeralState.lastChanges);
        }

        // Send stats if available (ephemeral - cached from last desktop push)
        if (this.ephemeralState.lastStats) {
            this.sendViewerEvent(viewer, this.ephemeralState.lastStats);
        }

        // Send current prompt if any (persistent - for interactive dashboard)
        // Only send if session is in interactive state (permission/question)
        const isInteractiveState = this.persistentState.state === 'permission' || this.persistentState.state === 'question';
        if (this.persistentState.currentPrompt && isInteractiveState) {
            const promptEvent: SessionEvent = {
                type: 'prompt',
                prompt: this.persistentState.currentPrompt,
            };
            this.sendViewerEvent(viewer, promptEvent);
        }
    }

    /**
     * Send one event to a dashboard viewer.
     */
    private sendViewerEvent(viewer: WebSocket, event: SessionEvent): void {
        viewer.send(JSON.stringify(event));
    }

    /**
     * Broadcast an event to all hibernatable viewer WebSockets.
     */
    private broadcast(event: SessionEvent): void {
        const data = JSON.stringify(event);
        for (const viewer of this.state.getWebSockets('viewer')) {
            try {
                viewer.send(data);
            } catch {
                // The runtime removes closed sockets from getWebSockets().
            }
        }
    }

    /**
     * Broadcast desktop connection status to dashboard viewers.
     */
    private broadcastDesktopStatus(connected: boolean): void {
        const event: SessionEvent = {
            type: 'desktop_status',
            connected,
            timestamp: Date.now(),
        };
        this.broadcast(event);
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
                viewer_websockets: this.state.getWebSockets('viewer').length,
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
     * Tell the desktop the group's PR dispositions changed so it refetches
     * them now instead of on its next poll. Sent after a ★/☆, ↑/↓, or ✕
     * click on the dashboard or an action link.
     */
    private handlePrOverridesChanged(): Response {
        const message: CloudToDesktopMessage = { type: 'pr_overrides_changed' };
        let delivered = false;
        if (this.desktopWs) {
            try {
                this.desktopWs.send(JSON.stringify(message));
                delivered = true;
            } catch {
                // Connection may have failed, ignore
            }
        }
        return new Response(JSON.stringify({ ok: true, delivered }), {
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
        if (!this.state.getTags(ws).includes('desktop')) {
            return;
        }

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
        if (!this.state.getTags(ws).includes('desktop')) {
            return;
        }
        if (this.desktopWs && this.desktopWs !== ws) {
            return;
        }

        this.desktopWs = null;
        this.broadcastDesktopStatus(false);
        if (this.sessionInfo) {
            await this.notifySessionList('disconnect', { id: this.sessionInfo.id });
        }
        // Schedule missed-heartbeat cleanup. Normal wrapper exits end the
        // session via HTTP; disconnects alone can be transient.
        await this.state.storage.setAlarm(Date.now() + SESSION_HEARTBEAT_TIMEOUT_MS);
    }

    /**
     * WebSocket Hibernation API - called when an accepted WebSocket errors.
     */
    async webSocketError(ws: WebSocket, error: unknown): Promise<void> {
        if (!this.state.getTags(ws).includes('desktop')) {
            return;
        }
        if (this.desktopWs && this.desktopWs !== ws) {
            return;
        }

        console.error('Desktop WebSocket error:', error);
        this.desktopWs = null;
        this.broadcastDesktopStatus(false);
        if (this.sessionInfo) {
            await this.notifySessionList('disconnect', { id: this.sessionInfo.id });
        }
        await this.state.storage.setAlarm(Date.now() + SESSION_HEARTBEAT_TIMEOUT_MS);
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
