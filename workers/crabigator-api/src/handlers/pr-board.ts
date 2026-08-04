import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { requireMobileAuth } from '../auth/middleware';

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
    disposition: string | null;
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
    }[];
}

/**
 * GET /api/prs/board - Every PR the group's sessions have tracked, deduped
 * across sessions with the same merge rules the desktop board uses: newest
 * GitHub stats win, engagement counters sum, Slack links union, and stored
 * dispositions override classification (dismissed PRs are omitted).
 */
export async function getPrBoard(request: Request, env: Env): Promise<Response> {
    const result = await requireMobileAuth(request, env);
    if ('error' in result) return result.error;
    const groupId = result.auth.group_id;

    const rows = await env.DB.prepare(
        `SELECT sp.owner, sp.repo, sp.number, sp.data, sp.updated_at, sp.session_id,
                s.cwd, s.state AS session_state, s.is_active, s.last_seen_at,
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
            entry.updated_at = row.updated_at;
        }
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
        entry.sessions.push({
            session_id: row.session_id,
            dir_name: cwd.split('/').filter(Boolean).pop() || cwd,
            state: row.session_state || '',
            active: !!row.is_active,
            last_seen_at: row.last_seen_at || 0,
        });
    }

    const prs = [...merged.values()]
        .filter((entry) => {
            if (entry.disposition === 'dismissed') return false;
            if (entry.disposition === 'primary') entry.pr.primary = true;
            if (entry.disposition === 'secondary') entry.pr.primary = false;
            return !entry.pr.dismissed;
        })
        .map(({ disposition: _disposition, ...entry }) => entry);

    return jsonResponse({ prs });
}
