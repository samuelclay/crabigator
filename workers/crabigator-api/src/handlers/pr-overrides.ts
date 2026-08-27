import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { requireDeviceAuth, requireMobileAuth } from '../auth/middleware';
import { deleteWatchedPr } from './watched-prs';

const DISPOSITIONS = ['primary', 'secondary', 'dismissed'] as const;
type Disposition = (typeof DISPOSITIONS)[number];

/**
 * Where an override applies: '' is the whole group (board-level actions and
 * watched PRs), 'session:<id>' one session, 'path:<cwd>' every session in one
 * worktree directory — sticky for future sessions there. Scoped rows beat the
 * group row when both exist.
 */
function validScopeKey(scope: unknown): scope is string {
    if (typeof scope !== 'string') return false;
    return scope === ''
        || /^session:[A-Za-z0-9_.-]{1,64}$/.test(scope)
        || (/^path:\//.test(scope) && scope.length <= 512);
}

/** [in progress, done] wording for the action page; also the set of
 * dispositions the page accepts. */
const ACTION_WORDING = new Map<string, [string, string]>([
    ['primary', ['Marking as primary', 'Marked as primary']],
    ['secondary', ['Marking as secondary', 'Marked as secondary']],
    ['dismissed', ['Dismissing', 'Dismissed']],
    ['auto', ['Resetting to automatic', 'Reset to automatic']],
    ['unwatched', ['Removing the watch on', 'Stopped watching']],
]);

interface PrOverrideRow {
    owner: string;
    repo: string;
    number: number;
    scope_key: string;
    disposition: Disposition;
    updated_at: number;
}

/** Nudge every live session in the group over its desktop WebSocket so the
 * status strips refetch dispositions now rather than on their next poll. The
 * group key is a group id, or a lone device id for desktops that never paired. */
async function notifyGroupSessions(env: Env, groupKey: string): Promise<void> {
    const rows = await env.DB.prepare(
        `SELECT s.id FROM sessions s
         JOIN devices d ON d.id = s.device_id
         WHERE (d.group_id = ? OR d.id = ?) AND s.is_active = 1
         LIMIT 200`
    )
        .bind(groupKey, groupKey)
        .all<{ id: string }>();
    await Promise.all(
        (rows.results ?? []).map(async (row) => {
            try {
                const stub = env.SESSION.get(env.SESSION.idFromName(row.id));
                await stub.fetch(new Request('https://internal/pr-overrides-changed', { method: 'POST' }));
            } catch {
                // A session whose Durable Object is unreachable keeps its poll.
            }
        })
    );
}

/** Overrides are scoped like dashboards: by device group, falling back to the
 * lone device id for desktops that never paired. */
async function deviceGroupKey(env: Env, deviceId: string): Promise<string> {
    const row = await env.DB.prepare('SELECT group_id FROM devices WHERE id = ?')
        .bind(deviceId)
        .first<{ group_id: string | null }>();
    return row?.group_id || deviceId;
}

/**
 * GET /api/pr-overrides - All PR dispositions for the caller's group.
 * Accepts desktop HMAC auth (the tracker applies these during classification)
 * or dashboard bearer auth (the UI reflects canonical state on load).
 */
export async function getPrOverrides(request: Request, env: Env): Promise<Response> {
    let groupKey: string;
    if (request.headers.get('X-Device-Id')) {
        const result = await requireDeviceAuth(request, env);
        if ('error' in result) return result.error;
        groupKey = await deviceGroupKey(env, result.auth.device_id);
    } else {
        const result = await requireMobileAuth(request, env);
        if ('error' in result) return result.error;
        groupKey = result.auth.group_id;
    }

    const rows = await env.DB.prepare(
        `SELECT owner, repo, number, scope_key, disposition, updated_at
         FROM pr_overrides WHERE group_key = ?`
    )
        .bind(groupKey)
        .all<PrOverrideRow>();

    return jsonResponse({ overrides: rows.results ?? [] });
}

/**
 * GET /pr-action - Tiny self-closing page behind the TUI's action links.
 * A cmd-click in the terminal lands here; the page posts the override with
 * the dashboard auth already stored in this browser, shows the result, and
 * closes its tab. Unpaired browsers are pointed at the dashboard instead.
 */
export async function getPrActionPage(request: Request): Promise<Response> {
    const url = new URL(request.url);
    const owner = url.searchParams.get('owner') ?? '';
    const repo = url.searchParams.get('repo') ?? '';
    const number = parseInt(url.searchParams.get('number') ?? '', 10);
    const disposition = url.searchParams.get('disposition') ?? '';
    const scope = url.searchParams.get('scope') ?? '';

    const wording = ACTION_WORDING.get(disposition);
    const invalid =
        !wording ||
        !/^[A-Za-z0-9_.-]+$/.test(owner) ||
        !/^[A-Za-z0-9_.-]+$/.test(repo) ||
        !Number.isInteger(number) || number <= 0 ||
        !validScopeKey(scope);
    if (invalid) {
        return new Response('Invalid PR action', { status: 400 });
    }

    const [verb, done] = wording;
    const prLabel = `${repo} #${number}`;
    // Unwatching targets the watch list; every other action stores an override.
    const endpoint = disposition === 'unwatched' ? '/api/prs/watched' : '/api/pr-overrides';
    // The scope is free-form text (it can carry a filesystem path); <
    // keeps a hostile value from closing the <script> tag it is embedded in.
    const payload = JSON.stringify(
        disposition === 'unwatched'
            ? { owner, repo, number, remove: true }
            : { owner, repo, number, disposition, scope }
    ).replace(/</g, '\\u003c');

    const html = `<!DOCTYPE html>
<html><head><meta charset="utf-8"><title>${verb} ${prLabel}</title>
<style>
body { background: #0d1117; color: #c9d1d9; font: 14px -apple-system, sans-serif;
       display: flex; align-items: center; justify-content: center; height: 100vh; margin: 0; }
.card { text-align: center; }
.ok { color: #87d787; } .err { color: #f85149; }
a { color: #c4a7f7; }
</style></head><body><div class="card" id="card">${verb} <b>${prLabel}</b>…</div>
<script>
(async () => {
    const card = document.getElementById('card');
    const token = localStorage.getItem('crabigator_mobile_token');
    if (!token) {
        card.innerHTML = 'This browser is not paired. <a href="/dashboard">Open the dashboard</a> to pair, then try again.';
        return;
    }
    try {
        const res = await fetch(${JSON.stringify(endpoint)}, {
            method: 'POST',
            headers: { 'Content-Type': 'application/json', 'Authorization': 'Bearer ' + token },
            body: ${JSON.stringify(payload)},
        });
        if (!res.ok) throw new Error(await res.text());
        card.innerHTML = '<span class="ok">✓</span> ${done} <b>${prLabel}</b>';
        setTimeout(() => window.close(), 900);
        setTimeout(() => { card.innerHTML += '<br><small>You can close this tab.</small>'; }, 1500);
    } catch (e) {
        card.innerHTML = '<span class="err">✗</span> Failed: ' + e.message;
    }
})();
</script></body></html>`;

    return new Response(html, { headers: { 'Content-Type': 'text/html; charset=utf-8' } });
}

/**
 * POST /api/pr-overrides - Set or clear one PR's disposition.
 * Body: { owner, repo, number, disposition: 'primary' | 'secondary' | 'dismissed' | 'auto',
 *         scope?: '' | 'session:<id>' | 'path:<cwd>' }
 * Omitted or empty scope is the whole group. 'auto' deletes the scope's
 * override — with no scope given it clears every row for the PR, returning it
 * fully to automatic classification.
 */
export async function setPrOverride(request: Request, env: Env): Promise<Response> {
    const result = await requireMobileAuth(request, env);
    if ('error' in result) return result.error;

    let body: {
        owner?: string;
        repo?: string;
        number?: number;
        disposition?: string;
        scope?: string;
    };
    try {
        body = await request.json();
    } catch {
        return jsonResponse({ error: 'Invalid JSON', code: 'INVALID_JSON' }, 400);
    }

    const { owner, repo, number, disposition } = body;
    if (
        typeof owner !== 'string' || !owner ||
        typeof repo !== 'string' || !repo ||
        typeof number !== 'number' || !Number.isInteger(number) || number <= 0
    ) {
        return jsonResponse({ error: 'Missing or invalid owner/repo/number', code: 'INVALID_PR' }, 400);
    }
    if (disposition !== 'auto' && !DISPOSITIONS.includes(disposition as Disposition)) {
        return jsonResponse({ error: 'Invalid disposition', code: 'INVALID_DISPOSITION' }, 400);
    }
    const scope = body.scope ?? '';
    if (!validScopeKey(scope)) {
        return jsonResponse({ error: 'Invalid scope', code: 'INVALID_SCOPE' }, 400);
    }

    const groupKey = result.auth.group_id;
    // A group-wide dismissal also ends any explicit watch on the PR — nothing
    // should resurrect a PR the user asked to go away everywhere. A scoped
    // dismissal only hides it from that session or worktree, so the watch
    // (a group-level fact) survives.
    if (disposition === 'dismissed' && scope === '') {
        await deleteWatchedPr(env, groupKey, owner, repo, number);
    }
    if (disposition === 'auto') {
        const scoped = body.scope !== undefined;
        await env.DB.prepare(
            `DELETE FROM pr_overrides
             WHERE group_key = ? AND owner = ? AND repo = ? AND number = ?
               ${scoped ? 'AND scope_key = ?' : ''}`
        )
            .bind(...(scoped
                ? [groupKey, owner, repo, number, scope]
                : [groupKey, owner, repo, number]))
            .run();
    } else {
        await env.DB.prepare(
            `INSERT INTO pr_overrides (group_key, owner, repo, number, scope_key, disposition, updated_at, updated_by)
             VALUES (?, ?, ?, ?, ?, ?, unixepoch(), ?)
             ON CONFLICT (group_key, owner, repo, number, scope_key)
             DO UPDATE SET disposition = excluded.disposition,
                           updated_at = excluded.updated_at,
                           updated_by = excluded.updated_by`
        )
            .bind(groupKey, owner, repo, number, scope, disposition, result.auth.mobile_id)
            .run();
    }
    await notifyGroupSessions(env, groupKey);

    return jsonResponse({ ok: true });
}
