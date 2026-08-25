import type { Env } from '../types/env';
import type { SessionPr } from '../types/session';
import { jsonResponse } from '../router';
import { requireDeviceAuth, requireMobileAuth } from '../auth/middleware';
import { watchedPrRows, watchedPlaceholderPr, deleteWatchedPr } from './watched-prs';

interface BoardSessionRow {
    session_id: string;
    platform: 'claude' | 'codex' | null;
    cwd: string | null;
    session_state: string | null;
    is_active: number | null;
    last_seen_at: number | null;
    prompts_changed_at: number | null;
    completions_changed_at: number | null;
    titles: string | null;
    titles_changed_at: number | null;
    recap: string | null;
    repo_owner: string | null;
    repo_name: string | null;
    branch: string | null;
    uncommitted_files: number | null;
    additions: number | null;
    deletions: number | null;
    slack_threads: string | null;
}

interface SessionPrRow extends BoardSessionRow {
    owner: string;
    repo: string;
    number: number;
    data: string;
    updated_at: number;
    is_primary: number;
    disposition: string | null;
}

/** Per-session recap brief stored in sessions.recap (see SessionDO). */
interface SessionRecapBrief {
    headline: string;
    bullets: string[];
    next_prompt_notes: string[];
    artifacts: string[];
    generated_at: number;
    additions: number;
    deletions: number;
}

/** Resolve the caller's group from desktop HMAC or dashboard bearer auth. */
async function boardGroupId(request: Request, env: Env): Promise<string | Response> {
    if (request.headers.get('X-Device-Id')) {
        const result = await requireDeviceAuth(request, env);
        if ('error' in result) return result.error;
        const row = await env.DB.prepare('SELECT group_id FROM devices WHERE id = ?')
            .bind(result.auth.device_id)
            .first<{ group_id: string | null }>();
        return row?.group_id || result.auth.device_id;
    }
    const result = await requireMobileAuth(request, env);
    if ('error' in result) return result.error;
    return result.auth.group_id;
}

/**
 * POST /api/prs/backfill - Populate session_prs for sessions that streamed
 * PRs before the D1 write-through existed. Each SessionDO has durably stored
 * its last PR list since PR streaming shipped; this walks the group's
 * sessions that have no session_prs rows yet, reads that stored list, and
 * writes it through with the session's last activity as the timestamp.
 */
export async function backfillSessionPrs(request: Request, env: Env): Promise<Response> {
    const groupId = await boardGroupId(request, env);
    if (groupId instanceof Response) return groupId;

    const sessions = await env.DB.prepare(
        `SELECT s.id, COALESCE(s.ended_at, s.last_seen_at, s.started_at) AS seen_at
         FROM sessions s
         JOIN devices d ON d.id = s.device_id
         WHERE d.group_id = ?
           AND s.id NOT IN (SELECT DISTINCT session_id FROM session_prs)
         ORDER BY s.started_at DESC
         LIMIT 200`
    )
        .bind(groupId)
        .all<{ id: string; seen_at: number | null }>();

    let sessionsBackfilled = 0;
    let prsWritten = 0;
    const rows = sessions.results ?? [];
    // Small batches keep DO wakes and subrequests well inside limits.
    for (let i = 0; i < rows.length; i += 10) {
        const chunk = rows.slice(i, i + 10);
        await Promise.all(
            chunk.map(async (session) => {
                try {
                    const stub = env.SESSION.get(env.SESSION.idFromName(session.id));
                    const response = await stub.fetch(new Request('https://internal/prs'));
                    if (!response.ok) return;
                    const data: unknown = await response.json();
                    const prs = prsFromPayload(data);
                    if (prs.length === 0) return;
                    const seenAt = session.seen_at || Math.floor(Date.now() / 1000);
                    await env.DB.batch(
                        prs.map((pr) =>
                            env.DB.prepare(
                                `INSERT INTO session_prs (session_id, owner, repo, number, url, state, is_primary, data, updated_at)
                                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
                                 ON CONFLICT (session_id, owner, repo, number) DO NOTHING`
                            ).bind(
                                session.id,
                                pr.owner,
                                pr.repo,
                                pr.number,
                                pr.url || '',
                                pr.state || '',
                                pr.primary ? 1 : 0,
                                JSON.stringify(pr),
                                seenAt
                            )
                        )
                    );
                    sessionsBackfilled += 1;
                    prsWritten += prs.length;
                } catch {
                    // A dead or unreachable DO just contributes nothing.
                }
            })
        );
    }

    return jsonResponse({
        sessions_checked: rows.length,
        sessions_backfilled: sessionsBackfilled,
        prs_written: prsWritten,
    });
}

/** One session's scrollback matches, as SessionDO's /search returns them. */
interface ScrollbackHit {
    session_id: string;
    total: number;
    /** The newest matching line, for the collapsed one-line preview. */
    collapsed: string;
    /** The last few matches with context, for the expanded preview. */
    rows: { text: string; is_match: boolean; gap_before: boolean }[];
}

/** The desktop board needs three characters before it greps transcripts. */
const SEARCH_QUERY_MIN = 3;
/** Enough live sessions to cover any real board without a runaway fan-out. */
const SEARCH_SESSION_LIMIT = 60;

/**
 * GET /api/prs/search?q= - Grep the group's live session scrollback, so the
 * web PR board's search reaches the same text the desktop board greps in each
 * session's local scrollback.log. Only running sessions hold scrollback, so
 * ended sessions contribute nothing.
 */
export async function searchSessionScrollback(request: Request, env: Env): Promise<Response> {
    const groupId = await boardGroupId(request, env);
    if (groupId instanceof Response) return groupId;

    const query = (new URL(request.url).searchParams.get('q') || '').trim();
    if (query.length < SEARCH_QUERY_MIN) return jsonResponse({ results: [] });

    const sessions = await env.DB.prepare(
        `SELECT s.id
         FROM sessions s
         JOIN devices d ON d.id = s.device_id
         WHERE d.group_id = ? AND s.is_active = 1
         ORDER BY s.last_seen_at DESC
         LIMIT ?`
    )
        .bind(groupId, SEARCH_SESSION_LIMIT)
        .all<{ id: string }>();

    const rows = sessions.results ?? [];
    const results: ScrollbackHit[] = [];
    // Small batches keep DO wakes and subrequests well inside limits.
    for (let i = 0; i < rows.length; i += 10) {
        const chunk = rows.slice(i, i + 10);
        const hits = await Promise.all(
            chunk.map(async (session): Promise<ScrollbackHit | null> => {
                try {
                    const stub = env.SESSION.get(env.SESSION.idFromName(session.id));
                    const response = await stub.fetch(
                        new Request(`https://internal/search?q=${encodeURIComponent(query)}`)
                    );
                    if (!response.ok) return null;
                    const data = (await response.json()) as Omit<ScrollbackHit, 'session_id'>;
                    if (!data?.total) return null;
                    return { session_id: session.id, ...data };
                } catch {
                    // A dead or unreachable DO just contributes no matches.
                    return null;
                }
            })
        );
        for (const hit of hits) {
            if (hit) results.push(hit);
        }
    }

    return jsonResponse({ results });
}

/** One aggregated PR on the board: freshest data + per-session engagement. */
interface BoardEntry {
    owner: string;
    repo: string;
    number: number;
    pr: SessionPr;
    updated_at: number;
    /** Sessions created with this PR or currently working on its branch. */
    sessions: {
        session_id: string;
        platform: 'claude' | 'codex';
        dir_name: string;
        repo_owner: string;
        repo_name: string;
        branch: string;
        state: string;
        active: boolean;
        last_seen_at: number;
        /** When the session last received a prompt, as Unix seconds. */
        prompts_changed_at: number;
        /** When the session's completion count last changed, as Unix seconds. */
        completions_changed_at: number;
        /** The session's current terminal title (last of the titles history). */
        title: string;
        /** Unix ms when that title was set; 0 when unknown. */
        title_set_at: number;
        /** Uncommitted files in the session's worktree. */
        uncommitted: number;
        /** Lines added across those uncommitted files. */
        additions: number;
        /** Lines removed across those uncommitted files. */
        deletions: number;
        /** The session's latest recap brief, when one was recorded. */
        recap: SessionRecapBrief | null;
    }[];
}

type BoardSession = BoardEntry['sessions'][number];

function sessionTitle(raw: string | null): string {
    try {
        const titles: unknown = JSON.parse(raw || '[]');
        return Array.isArray(titles) && titles.length > 0
            ? String(titles[titles.length - 1])
            : '';
    } catch {
        return '';
    }
}

function sessionRecap(raw: string | null): SessionRecapBrief | null {
    try {
        const parsed = JSON.parse(raw || 'null');
        if (!parsed?.headline) return null;
        return {
            headline: String(parsed.headline),
            bullets: stringArray(parsed.bullets),
            next_prompt_notes: stringArray(parsed.next_prompt_notes),
            artifacts: stringArray(parsed.artifacts),
            generated_at: parsed.generated_at || 0,
            additions: parsed.additions || 0,
            deletions: parsed.deletions || 0,
        };
    } catch {
        return null;
    }
}

/**
 * A Slack permalink's display identity, as the desktop learned it. The board
 * labels links `#channel · Author`, falling back to the channel ID in the
 * permalink when a session never carried the enriched metadata.
 */
interface BoardSlackThread {
    url: string;
    channel?: string;
    author?: string;
}

/** Collect every session's enriched Slack threads, newest label winning. */
function collectSlackThreads(
    rows: BoardSessionRow[],
    into: Map<string, BoardSlackThread>
): void {
    for (const row of rows) {
        if (!row.slack_threads) continue;
        let parsed: unknown;
        try {
            parsed = JSON.parse(row.slack_threads);
        } catch {
            continue;
        }
        if (!Array.isArray(parsed)) continue;
        for (const thread of parsed) {
            const url = typeof thread?.url === 'string' ? thread.url : '';
            if (!url) continue;
            const existing = into.get(url);
            const channel = typeof thread.channel === 'string' ? thread.channel : '';
            const author = typeof thread.author === 'string' ? thread.author : '';
            into.set(url, {
                url,
                channel: channel || existing?.channel || '',
                author: author || existing?.author || '',
            });
        }
    }
}

function stringArray(value: unknown): string[] {
    return Array.isArray(value)
        ? value.filter((item): item is string => typeof item === 'string' && item.trim() !== '')
        : [];
}

function boardSession(row: BoardSessionRow): BoardSession {
    const cwd = row.cwd || '';
    const dirName = cwd.split('/').filter(Boolean).pop() || cwd;
    return {
        session_id: row.session_id,
        platform: row.platform || 'claude',
        dir_name: dirName,
        repo_owner: row.repo_owner || '',
        repo_name: row.repo_name || dirName,
        branch: row.branch || '',
        state: row.session_state || '',
        active: !!row.is_active,
        last_seen_at: row.last_seen_at || 0,
        prompts_changed_at: row.prompts_changed_at || 0,
        completions_changed_at: row.completions_changed_at || 0,
        title: sessionTitle(row.titles),
        title_set_at: row.titles_changed_at || 0,
        uncommitted: row.uncommitted_files || 0,
        additions: row.additions || 0,
        deletions: row.deletions || 0,
        recap: sessionRecap(row.recap),
    };
}

function isSessionPr(value: unknown): value is SessionPr {
    if (!value || typeof value !== 'object') return false;
    const pr = value as Record<string, unknown>;
    return typeof pr.number === 'number'
        && typeof pr.owner === 'string'
        && typeof pr.repo === 'string'
        && typeof pr.url === 'string'
        && typeof pr.branch === 'string'
        && typeof pr.title === 'string'
        && typeof pr.state === 'string'
        && typeof pr.is_draft === 'boolean'
        && typeof pr.additions === 'number'
        && typeof pr.deletions === 'number'
        && typeof pr.changed_files === 'number'
        && typeof pr.mergeable === 'string'
        && typeof pr.merge_state_status === 'string'
        && typeof pr.checks_passed === 'number'
        && typeof pr.checks_failed === 'number'
        && typeof pr.checks_pending === 'number'
        && typeof pr.checks_total === 'number'
        && typeof pr.created_here === 'boolean'
        && typeof pr.refreshed_at === 'number';
}

function parseSessionPr(data: string): SessionPr | null {
    try {
        const parsed: unknown = JSON.parse(data);
        return isSessionPr(parsed) ? parsed : null;
    } catch {
        return null;
    }
}

function prsFromPayload(payload: unknown): SessionPr[] {
    if (!payload || typeof payload !== 'object') return [];
    const prs = (payload as Record<string, unknown>).prs;
    return Array.isArray(prs) ? prs.filter(isSessionPr) : [];
}

function primarySourceRank(source?: string): number {
    if (source === 'override') return 3;
    if (source === 'session') return 2;
    if (source === 'auto') return 1;
    return 0;
}

function combinedPrimarySource(existing: SessionPr, incoming: SessionPr): string {
    if (existing.primary && incoming.primary) {
        return primarySourceRank(incoming.primary_source) > primarySourceRank(existing.primary_source)
            ? incoming.primary_source || ''
            : existing.primary_source || '';
    }
    if (existing.primary) return existing.primary_source || '';
    if (incoming.primary) return incoming.primary_source || '';
    return '';
}

function mergeSlackUrls(existing?: string[], incoming?: string[]): string[] {
    const merged = [...(existing || [])];
    for (const url of incoming || []) {
        if (!merged.includes(url)) merged.push(url);
    }
    return merged;
}

function foreignWithoutExplicitInterest(pr: SessionPr): boolean {
    return pr.authored_by_viewer === false
        && (pr.user_mentions || 0) === 0
        && pr.primary_source !== 'session'
        && pr.primary_source !== 'override';
}

function sessionRepositoryMatchesPr(row: BoardSessionRow, pr: SessionPr): boolean {
    const cwdRepo = (row.cwd || '').split('/').filter(Boolean).pop() || '';
    const currentRepo = row.repo_name || cwdRepo;
    const ownerMatches = row.repo_owner
        ? row.repo_owner.toLowerCase() === pr.owner.toLowerCase()
        : !!pr.created_here;
    return ownerMatches && currentRepo.toLowerCase() === pr.repo.toLowerCase();
}

// A PR the session opened itself is the session's own work even when it lives
// in a sibling repository of the same organization — a fix paired with the
// checkout's own PR. Other organizations still need a matching checkout.
function sessionCreatedPr(row: BoardSessionRow, pr: SessionPr): boolean {
    if (!pr.created_here) return false;
    if (!row.repo_owner) return sessionRepositoryMatchesPr(row, pr);
    return row.repo_owner.toLowerCase() === (pr.owner || '').toLowerCase();
}

function prAttachedToSession(row: BoardSessionRow, pr: SessionPr): boolean {
    if (!pr.branch || ['main', 'master', 'develop'].includes(pr.branch)) return false;
    if (!sessionRepositoryMatchesPr(row, pr)) return false;
    if (row.branch === pr.branch) return true;
    const worktree = (row.cwd || '').split('/').filter(Boolean).pop() || '';
    return !!worktree && (pr.branch === worktree || pr.branch.endsWith(`/${worktree}`));
}

/** Derive worktree ownership from durable session data written by older clients. */
function effectiveSessionPr(row: SessionPrRow, stored: SessionPr): SessionPr {
    const pr = { ...stored };
    if (row.disposition === 'primary') {
        return { ...pr, primary: true, primary_source: 'override', dismissed: false };
    }
    if (row.disposition === 'secondary') {
        return { ...pr, primary: false, primary_source: 'override', dismissed: false };
    }
    if (row.disposition === 'dismissed') {
        return { ...pr, primary: false, primary_source: 'override', dismissed: true };
    }

    const explicitlySecondary = !pr.primary
        && (pr.primary_source === 'session' || pr.primary_source === 'override');
    if (
        !explicitlySecondary
        && !pr.dismissed
        && pr.state !== 'CLOSED'
        && prAttachedToSession(row, pr)
    ) {
        pr.primary = true;
        pr.primary_source = 'auto';
    }
    return pr;
}

function visiblePr(pr: SessionPr, lingerMs: number, nowMs: number, updatedAt: number): boolean {
    if (pr.dismissed) return false;
    // A classified primary remains actionable while GitHub enrichment
    // retries, and a watched PR is wanted by definition.
    const wanted = !!pr.primary || !!pr.watched;
    if (!pr.refreshed_at) return wanted;
    if (pr.state === 'OPEN') return true;
    if (!wanted || (!pr.watched && foreignWithoutExplicitInterest(pr))) return false;
    const latest = Math.max(
        pr.closed_at || 0,
        pr.last_mentioned_at || 0,
        pr.updated_at || 0,
        updatedAt * 1000
    );
    return !!lingerMs && !!latest && nowMs - latest <= lingerMs;
}

/**
 * GET /api/prs/board - Every PR the group's sessions have tracked, deduped
 * across sessions with the same merge rules the desktop board uses: newest
 * GitHub stats win, engagement counters sum, Slack links union, and stored
 * dispositions override classification (dismissed PRs are omitted).
 * Accepts dashboard bearer auth or desktop HMAC auth (the TUI's all-sessions
 * view). `?days=N` bounds how long finished primary PRs linger (default 1, max 90).
 */
export async function getPrBoard(request: Request, env: Env): Promise<Response> {
    const groupId = await boardGroupId(request, env);
    if (groupId instanceof Response) return groupId;
    try {
        return await buildPrBoard(request, env, groupId);
    } catch (error) {
        // Authed endpoint: the message helps the TUI/dashboard report failures.
        return jsonResponse(
            { error: `PR board failed: ${error instanceof Error ? error.message : error}` },
            500
        );
    }
}

async function buildPrBoard(request: Request, env: Env, groupId: string): Promise<Response> {
    const daysParam = parseInt(new URL(request.url).searchParams.get('days') ?? '1', 10);
    const lingerDays = Number.isFinite(daysParam) ? Math.min(Math.max(daysParam, 0), 90) : 1;

    const rows = await env.DB.prepare(
        `SELECT sp.owner, sp.repo, sp.number, sp.data, sp.updated_at, sp.session_id,
                sp.is_primary,
                s.platform, s.cwd, s.state AS session_state, s.is_active, s.last_seen_at,
                s.prompts_changed_at, s.completions_changed_at,
                s.titles, s.titles_changed_at, s.recap,
                s.repo_owner, s.repo_name, s.branch,
                s.uncommitted_files, s.additions, s.deletions, s.slack_threads,
                o.disposition
         FROM session_prs sp
         JOIN sessions s ON s.id = sp.session_id
         JOIN devices d ON d.id = s.device_id
         LEFT JOIN pr_overrides o
             ON o.group_key = ?1 AND o.owner = sp.owner AND o.repo = sp.repo AND o.number = sp.number
         WHERE d.group_id = ?1
         ORDER BY sp.updated_at DESC
         LIMIT 1000`
    )
        .bind(groupId)
        .all<SessionPrRow>();

    const merged = new Map<string, BoardEntry & { disposition: string | null }>();
    // PRs a session holds a verified claim on, and the sessions the strict
    // ownership gate below left unrepresented paired with the primary PR their
    // own classifier picked (cross-repo reviews, replying to someone else's
    // branch). Both are resolved after the merge pass.
    const representedSessions = new Set<string>();
    const ownedKeys = new Set<string>();
    const fallbacks = new Map<string, { key: string; rank: number; row: SessionPrRow }>();
    for (const row of rows.results ?? []) {
        const storedPr = parseSessionPr(row.data);
        if (!storedPr) continue;
        const pr = effectiveSessionPr(row, storedPr);
        const key = `${row.owner}/${row.repo}#${row.number}`;
        let entry = merged.get(key);
        if (!entry) {
            entry = {
                owner: row.owner,
                repo: row.repo,
                number: row.number,
                // Engagement counters accumulate per contributing session.
                pr: { ...pr, mentions: 0, user_mentions: 0, last_mentioned_at: 0 },
                updated_at: row.updated_at,
                sessions: [],
                disposition: row.disposition,
            };
            merged.set(key, entry);
        }
        const aggregatePrimary = !!entry.pr.primary || !!pr.primary;
        const aggregatePrimarySource = combinedPrimarySource(entry.pr, pr);
        const aggregateDismissed = !!entry.pr.dismissed || !!pr.dismissed;
        if (pr.refreshed_at > entry.pr.refreshed_at) {
            const previous = entry.pr;
            entry.pr = {
                ...pr,
                mentions: previous.mentions,
                user_mentions: previous.user_mentions,
                last_mentioned_at: previous.last_mentioned_at,
                author_login: pr.author_login || previous.author_login,
                authored_by_viewer: pr.authored_by_viewer ?? previous.authored_by_viewer,
                slack_comment_urls: mergeSlackUrls(
                    pr.slack_comment_urls,
                    previous.slack_comment_urls
                ),
            };
            if (!entry.pr.slack_origin_url) {
                entry.pr.slack_origin_url = previous.slack_origin_url || '';
            }
        }
        entry.updated_at = Math.max(entry.updated_at, row.updated_at);
        entry.pr.mentions = (entry.pr.mentions || 0) + (pr.mentions || 0);
        entry.pr.user_mentions = (entry.pr.user_mentions || 0) + (pr.user_mentions || 0);
        entry.pr.last_mentioned_at = Math.max(
            entry.pr.last_mentioned_at || 0,
            pr.last_mentioned_at || 0
        );
        entry.pr.primary = aggregatePrimary;
        entry.pr.primary_source = aggregatePrimarySource;
        entry.pr.dismissed = aggregateDismissed;
        if (!entry.pr.slack_origin_url) entry.pr.slack_origin_url = pr.slack_origin_url || '';
        entry.pr.slack_comment_urls = mergeSlackUrls(
            entry.pr.slack_comment_urls,
            pr.slack_comment_urls
        );
        if (!entry.pr.ai_note && pr.ai_note) {
            entry.pr.ai_note = pr.ai_note;
            entry.pr.ai_confidence = pr.ai_confidence || '';
        }
        const representsSession = !!pr.primary
            && (sessionCreatedPr(row, pr) || prAttachedToSession(row, pr));
        if (representsSession) {
            representedSessions.add(row.session_id);
            ownedKeys.add(key);
            entry.sessions.push(boardSession(row));
        } else if (
            pr.primary
            && !pr.dismissed
            && !!row.repo_owner
            && row.repo_owner.toLowerCase() === (pr.owner || '').toLowerCase()
        ) {
            // Same-org only: a session that merely discusses another org's PR
            // must not migrate into that repository's group.
            const rank = pr.last_mentioned_at || 0;
            const existing = fallbacks.get(row.session_id);
            if (!existing || rank > existing.rank) {
                fallbacks.set(row.session_id, { key, rank, row });
            }
        }
    }

    // A session whose primary PR failed the strict gate would otherwise fall
    // back to a bare peer row even though its own status bar names the PR.
    // Trust the session's classification — but only while no session holds a
    // verified claim on that PR.
    for (const [sessionId, fallback] of fallbacks) {
        if (representedSessions.has(sessionId) || ownedKeys.has(fallback.key)) continue;
        merged.get(fallback.key)?.sessions.push(boardSession(fallback.row));
    }

    // Explicitly watched PRs join the board even when no session tracks
    // them, carrying whatever stats an open prs board relayed last. The
    // watch marks session-tracked copies too, so clients render the watch
    // glyph and keep the PR through the primary-only PR view.
    const watched = await watchedPrRows(env, groupId);
    const watchedKeys = new Set(watched.map((row) => `${row.owner}/${row.repo}#${row.number}`));
    for (const entry of merged.values()) {
        if (watchedKeys.has(`${entry.owner}/${entry.repo}#${entry.number}`)) {
            entry.pr.watched = true;
        }
    }

    const lingerMs = lingerDays * 24 * 3600 * 1000;
    const nowMs = Date.now();
    const prs = [...merged.values()]
        .filter((entry) => {
            if (entry.disposition === 'dismissed') return false;
            if (entry.disposition === 'primary') {
                entry.pr.primary = true;
                entry.pr.primary_source = 'override';
            }
            if (entry.disposition === 'secondary') {
                entry.pr.primary = false;
                entry.pr.primary_source = 'override';
            }
            return visiblePr(entry.pr, lingerMs, nowMs, entry.updated_at);
        })
        .map(({ disposition: _disposition, ...entry }) => entry);

    // Watch-only PRs (no session row) render from their relayed stats; a
    // watch whose PR finished and aged past the linger window clears itself.
    const seenKeys = new Set(prs.map((entry) => `${entry.owner}/${entry.repo}#${entry.number}`));
    for (const row of watched) {
        if (seenKeys.has(`${row.owner}/${row.repo}#${row.number}`)) continue;
        const pr = parseSessionPr(row.data || '') ?? watchedPlaceholderPr(row);
        pr.watched = true;
        if (!visiblePr(pr, lingerMs, nowMs, row.refreshed_at || row.added_at)) {
            await deleteWatchedPr(env, groupId, row.owner, row.repo, row.number);
            continue;
        }
        prs.push({
            owner: row.owner,
            repo: row.repo,
            number: row.number,
            pr,
            updated_at: row.refreshed_at || row.added_at,
            sessions: [],
        });
    }

    // Return every active account session separately. Clients keep any session
    // without a same-repository primary PR as its own peer row.
    const sessionRows = await env.DB.prepare(
        `SELECT s.id AS session_id, s.platform, s.cwd, s.state AS session_state,
                s.is_active, s.last_seen_at,
                s.prompts_changed_at, s.completions_changed_at,
                s.titles, s.titles_changed_at, s.recap,
                s.repo_owner, s.repo_name, s.branch,
                s.uncommitted_files, s.additions, s.deletions, s.slack_threads
         FROM sessions s
         JOIN devices d ON d.id = s.device_id
         WHERE d.group_id = ? AND s.is_active = 1
         ORDER BY s.last_seen_at DESC
         LIMIT 200`
    )
        .bind(groupId)
        .all<BoardSessionRow>();
    const sessions = (sessionRows.results ?? []).map(boardSession);

    const slack = new Map<string, BoardSlackThread>();
    collectSlackThreads(rows.results ?? [], slack);
    collectSlackThreads(sessionRows.results ?? [], slack);

    return jsonResponse({ prs, sessions, slack: [...slack.values()] });
}
