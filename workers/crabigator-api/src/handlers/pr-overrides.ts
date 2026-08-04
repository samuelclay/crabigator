import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { requireDeviceAuth, requireMobileAuth } from '../auth/middleware';

const DISPOSITIONS = ['primary', 'secondary', 'dismissed'] as const;
type Disposition = (typeof DISPOSITIONS)[number];

interface PrOverrideRow {
    owner: string;
    repo: string;
    number: number;
    disposition: Disposition;
    updated_at: number;
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
        `SELECT owner, repo, number, disposition, updated_at
         FROM pr_overrides WHERE group_key = ?`
    )
        .bind(groupKey)
        .all<PrOverrideRow>();

    return jsonResponse({ overrides: rows.results ?? [] });
}

/**
 * POST /api/pr-overrides - Set or clear one PR's disposition.
 * Body: { owner, repo, number, disposition: 'primary' | 'secondary' | 'dismissed' | 'auto' }
 * 'auto' deletes the override, returning the PR to automatic classification.
 */
export async function setPrOverride(request: Request, env: Env): Promise<Response> {
    const result = await requireMobileAuth(request, env);
    if ('error' in result) return result.error;

    let body: { owner?: string; repo?: string; number?: number; disposition?: string };
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

    const groupKey = result.auth.group_id;
    if (disposition === 'auto') {
        await env.DB.prepare(
            'DELETE FROM pr_overrides WHERE group_key = ? AND owner = ? AND repo = ? AND number = ?'
        )
            .bind(groupKey, owner, repo, number)
            .run();
    } else {
        await env.DB.prepare(
            `INSERT INTO pr_overrides (group_key, owner, repo, number, disposition, updated_at, updated_by)
             VALUES (?, ?, ?, ?, ?, unixepoch(), ?)
             ON CONFLICT (group_key, owner, repo, number)
             DO UPDATE SET disposition = excluded.disposition,
                           updated_at = excluded.updated_at,
                           updated_by = excluded.updated_by`
        )
            .bind(groupKey, owner, repo, number, disposition, result.auth.mobile_id)
            .run();
    }

    return jsonResponse({ ok: true });
}
