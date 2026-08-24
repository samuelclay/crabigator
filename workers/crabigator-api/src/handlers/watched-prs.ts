import type { Env } from '../types/env';
import type { SessionPr } from '../types/session';
import { jsonResponse } from '../router';
import { requireDeviceAuth, requireMobileAuth } from '../auth/middleware';

export interface WatchedPrRow {
    owner: string;
    repo: string;
    number: number;
    url: string;
    added_at: number;
    data: string | null;
    refreshed_at: number;
}

const PR_NAME = /^[A-Za-z0-9_.-]+$/;

/** Resolve the caller's group from desktop HMAC or dashboard bearer auth. */
async function watchGroupId(request: Request, env: Env): Promise<string | Response> {
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

/** The group's watched PRs, for the board merge and the GET endpoint. */
export async function watchedPrRows(env: Env, groupKey: string): Promise<WatchedPrRow[]> {
    const rows = await env.DB.prepare(
        `SELECT owner, repo, number, url, added_at, data, refreshed_at
         FROM watched_prs WHERE group_key = ?
         ORDER BY added_at DESC LIMIT 200`
    )
        .bind(groupKey)
        .all<WatchedPrRow>();
    return rows.results ?? [];
}

/** Delete one watch. Shared with pr-overrides: dismissing a PR ends its watch. */
export async function deleteWatchedPr(
    env: Env,
    groupKey: string,
    owner: string,
    repo: string,
    number: number
): Promise<void> {
    await env.DB.prepare(
        'DELETE FROM watched_prs WHERE group_key = ? AND owner = ? AND repo = ? AND number = ?'
    )
        .bind(groupKey, owner, repo, number)
        .run();
}

/** A full SessionPr shape for a watch that has never been enriched, so every
 * client can parse the board entry without special cases. */
export function watchedPlaceholderPr(row: WatchedPrRow): SessionPr {
    return {
        number: row.number,
        owner: row.owner,
        repo: row.repo,
        url: row.url,
        branch: '',
        title: '',
        state: '',
        is_draft: false,
        additions: 0,
        deletions: 0,
        changed_files: 0,
        mergeable: '',
        merge_state_status: '',
        checks_passed: 0,
        checks_failed: 0,
        checks_pending: 0,
        checks_total: 0,
        created_here: false,
        refreshed_at: 0,
    };
}

/**
 * GET /api/prs/watched - The group's explicitly watched PRs, with the last
 * GitHub stats an open board relayed (null until first enrichment).
 * Accepts desktop HMAC auth (the prs board) or dashboard bearer auth.
 */
export async function getWatchedPrs(request: Request, env: Env): Promise<Response> {
    const groupKey = await watchGroupId(request, env);
    if (groupKey instanceof Response) return groupKey;

    const watched = (await watchedPrRows(env, groupKey)).map((row) => ({
        owner: row.owner,
        repo: row.repo,
        number: row.number,
        url: row.url,
        added_at: row.added_at,
        refreshed_at: row.refreshed_at,
        pr: parseStoredPr(row.data),
    }));
    return jsonResponse({ watched });
}

function parseStoredPr(data: string | null): SessionPr | null {
    if (!data) return null;
    try {
        const parsed: unknown = JSON.parse(data);
        return parsed && typeof parsed === 'object' ? (parsed as SessionPr) : null;
    } catch {
        return null;
    }
}

/**
 * POST /api/prs/watched - Add or remove one watched PR.
 * Body: { owner, repo, number, url?, remove?: true }
 * Adding is idempotent and keeps an existing watch's stats; the URL is
 * derived from owner/repo/number when the caller doesn't send one.
 */
export async function setWatchedPr(request: Request, env: Env): Promise<Response> {
    const groupKey = await watchGroupId(request, env);
    if (groupKey instanceof Response) return groupKey;

    let body: { owner?: string; repo?: string; number?: number; url?: string; remove?: boolean };
    try {
        body = await request.json();
    } catch {
        return jsonResponse({ error: 'Invalid JSON', code: 'INVALID_JSON' }, 400);
    }

    const { owner, repo, number } = body;
    if (
        typeof owner !== 'string' || !PR_NAME.test(owner) ||
        typeof repo !== 'string' || !PR_NAME.test(repo) ||
        typeof number !== 'number' || !Number.isInteger(number) || number <= 0
    ) {
        return jsonResponse({ error: 'Missing or invalid owner/repo/number', code: 'INVALID_PR' }, 400);
    }

    if (body.remove) {
        await deleteWatchedPr(env, groupKey, owner, repo, number);
        return jsonResponse({ ok: true });
    }

    const url = typeof body.url === 'string' && body.url.startsWith('https://github.com/')
        ? body.url
        : `https://github.com/${owner}/${repo}/pull/${number}`;
    await env.DB.prepare(
        `INSERT INTO watched_prs (group_key, owner, repo, number, url, added_at, added_by)
         VALUES (?, ?, ?, ?, ?, unixepoch(), ?)
         ON CONFLICT (group_key, owner, repo, number) DO NOTHING`
    )
        .bind(groupKey, owner, repo, number, url, callerId(request))
        .run();
    return jsonResponse({ ok: true });
}

function callerId(request: Request): string {
    return request.headers.get('X-Device-Id') || 'dashboard';
}

/**
 * POST /api/prs/watched/stats - An open prs board relays the GitHub stats it
 * fetched for watched PRs, so the web board and other machines see them.
 * Body: { prs: SessionPr[] } — only PRs already watched are updated.
 */
export async function relayWatchedPrStats(request: Request, env: Env): Promise<Response> {
    const groupKey = await watchGroupId(request, env);
    if (groupKey instanceof Response) return groupKey;

    let body: { prs?: unknown };
    try {
        body = await request.json();
    } catch {
        return jsonResponse({ error: 'Invalid JSON', code: 'INVALID_JSON' }, 400);
    }
    const prs = Array.isArray(body.prs) ? body.prs : [];
    const updates = prs.filter((pr): pr is SessionPr => {
        if (!pr || typeof pr !== 'object') return false;
        const candidate = pr as Record<string, unknown>;
        return typeof candidate.owner === 'string'
            && typeof candidate.repo === 'string'
            && typeof candidate.number === 'number'
            && typeof candidate.refreshed_at === 'number';
    }).slice(0, 200);

    if (updates.length > 0) {
        await env.DB.batch(
            updates.map((pr) =>
                env.DB.prepare(
                    `UPDATE watched_prs SET data = ?, refreshed_at = unixepoch()
                     WHERE group_key = ? AND owner = ? AND repo = ? AND number = ?`
                ).bind(JSON.stringify(pr), groupKey, pr.owner, pr.repo, pr.number)
            )
        );
    }
    return jsonResponse({ ok: true, updated: updates.length });
}
