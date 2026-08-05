import type { Env } from '../types/env';
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
                    const data = (await response.json()) as { prs: any[] };
                    const prs = (data.prs || []).filter(
                        (pr) => pr && pr.owner && pr.repo && pr.number
                    );
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
    pr: any;
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

/**
 * GET /api/prs/board - Every PR the group's sessions have tracked, deduped
 * across sessions with the same merge rules the desktop board uses: newest
 * GitHub stats win, engagement counters sum, Slack links union, and stored
 * dispositions override classification (dismissed PRs are omitted).
 * Accepts dashboard bearer auth or desktop HMAC auth (the TUI's all-sessions
 * view). `?days=N` bounds how long finished PRs linger (default 1, max 90).
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
        let pr: any;
        try {
            pr = JSON.parse(row.data);
        } catch {
            continue;
        }
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
        if (row.updated_at > entry.updated_at) {
            const { mentions, user_mentions, last_mentioned_at } = entry.pr;
            entry.pr = { ...pr, mentions, user_mentions, last_mentioned_at };
        }
        entry.updated_at = Math.max(entry.updated_at, row.updated_at);
        entry.pr.mentions += pr.mentions || 0;
        entry.pr.user_mentions += pr.user_mentions || 0;
        entry.pr.last_mentioned_at = Math.max(entry.pr.last_mentioned_at, pr.last_mentioned_at || 0);
        entry.pr.primary = entry.pr.primary || !!pr.primary;
        entry.pr.dismissed = entry.pr.dismissed || !!pr.dismissed;
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
            // PRs gh never confirmed are scanning artifacts (doc examples,
            // wrapped shorthand) — same rule as the desktop board.
            if (!entry.pr.refreshed_at) return false;
            // Finished PRs age off after the window (0 = open only), by close
            // time or last mention. Records from before those fields existed
            // (backfilled sessions) fall back to when the row was last written
            // — for a backfill, the session's final activity.
            if (entry.pr.state !== 'OPEN') {
                const latest = Math.max(
                    entry.pr.closed_at || 0,
                    entry.pr.last_mentioned_at || 0,
                    entry.updated_at * 1000
                );
                if (!lingerMs || !latest || nowMs - latest > lingerMs) return false;
            }
            if (entry.disposition === 'dismissed') return false;
            if (entry.disposition === 'primary') entry.pr.primary = true;
            if (entry.disposition === 'secondary') entry.pr.primary = false;
            return !entry.pr.dismissed;
        })
        .map(({ disposition: _disposition, ...entry }) => entry);

    return jsonResponse({ prs });
}
