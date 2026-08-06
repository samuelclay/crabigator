import type { Env } from '../types/env';
import type { SessionPr } from '../types/session';
import { jsonResponse } from '../router';
import { requireDeviceAuth, requireMobileAuth } from '../auth/middleware';

interface SessionPrRow {
    owner: string;
    repo: string;
    number: number;
    data: string;
    updated_at: number;
    session_id: string;
    cwd: string | null;
    session_state: string | null;
    is_active: number | null;
    last_seen_at: number | null;
    titles: string | null;
    recap: string | null;
    disposition: string | null;
}

/** Per-session recap brief stored in sessions.recap (see SessionDO). */
interface SessionRecapBrief {
    headline: string;
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

/** One aggregated PR on the board: freshest data + per-session engagement. */
interface BoardEntry {
    owner: string;
    repo: string;
    number: number;
    pr: SessionPr;
    updated_at: number;
    sessions: {
        session_id: string;
        dir_name: string;
        state: string;
        active: boolean;
        last_seen_at: number;
        /** The session's current terminal title (last of the titles history). */
        title: string;
        /** The session's latest recap brief, when one was recorded. */
        recap: SessionRecapBrief | null;
    }[];
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

function foreignWithoutExplicitInterest(pr: SessionPr): boolean {
    return pr.authored_by_viewer === false
        && (pr.user_mentions || 0) === 0
        && pr.primary_source !== 'session'
        && pr.primary_source !== 'override';
}

function visiblePr(pr: SessionPr, lingerMs: number, nowMs: number, updatedAt: number): boolean {
    if (pr.dismissed) return false;
    // A classified primary remains actionable while GitHub enrichment retries.
    if (!pr.refreshed_at) return !!pr.primary;
    if (pr.state === 'OPEN') return true;
    if (!pr.primary || foreignWithoutExplicitInterest(pr)) return false;
    const latest = Math.max(
        pr.closed_at || 0,
        pr.last_mentioned_at || 0,
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
                s.cwd, s.state AS session_state, s.is_active, s.last_seen_at,
                s.titles, s.recap,
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
    for (const row of rows.results ?? []) {
        const pr = parseSessionPr(row.data);
        if (!pr) continue;
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
        if (row.updated_at > entry.updated_at) {
            const previous = entry.pr;
            entry.pr = {
                ...pr,
                mentions: previous.mentions,
                user_mentions: previous.user_mentions,
                last_mentioned_at: previous.last_mentioned_at,
                author_login: pr.author_login || previous.author_login,
                authored_by_viewer: pr.authored_by_viewer ?? previous.authored_by_viewer,
            };
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
        const slackUrls: string[] = entry.pr.slack_comment_urls || [];
        for (const url of pr.slack_comment_urls || []) {
            if (!slackUrls.includes(url)) slackUrls.push(url);
        }
        entry.pr.slack_comment_urls = slackUrls;
        if (!entry.pr.ai_note && pr.ai_note) {
            entry.pr.ai_note = pr.ai_note;
            entry.pr.ai_confidence = pr.ai_confidence || '';
        }
        const cwd = row.cwd || '';
        let title = '';
        try {
            const titles = JSON.parse(row.titles || '[]');
            if (Array.isArray(titles) && titles.length > 0) title = String(titles[titles.length - 1]);
        } catch {
            // Unparseable titles column contributes nothing.
        }
        let recap: SessionRecapBrief | null = null;
        try {
            const parsed = JSON.parse(row.recap || 'null');
            if (parsed?.headline) {
                recap = {
                    headline: String(parsed.headline),
                    generated_at: parsed.generated_at || 0,
                    additions: parsed.additions || 0,
                    deletions: parsed.deletions || 0,
                };
            }
        } catch {
            // Unparseable recap column contributes nothing.
        }
        entry.sessions.push({
            session_id: row.session_id,
            dir_name: cwd.split('/').filter(Boolean).pop() || cwd,
            state: row.session_state || '',
            active: !!row.is_active,
            last_seen_at: row.last_seen_at || 0,
            title,
            recap,
        });
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

    return jsonResponse({ prs });
}
