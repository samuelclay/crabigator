import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { staffDashboardHtml } from '../staff-dashboard';

/**
 * Telemetry data sent with update checks
 */
interface TelemetryRequest {
    device_id: string;
    machine_name?: string;
    os?: string;
    timezone_offset?: number;
    app_version: string;
}

/**
 * Response from GitHub Releases API (simplified)
 */
interface GitHubRelease {
    tag_name: string;
    html_url: string;
}

/**
 * POST /api/update-check - Check for updates and record telemetry
 *
 * Proxies the GitHub Releases API while recording anonymous telemetry.
 * No auth required - anyone can check for updates.
 */
export async function handleUpdateCheck(
    request: Request,
    env: Env
): Promise<Response> {
    let body: TelemetryRequest;
    try {
        body = await request.json();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const { device_id, machine_name, os, timezone_offset, app_version } = body;

    if (!device_id || !app_version) {
        return new Response(
            JSON.stringify({ error: 'Missing device_id or app_version', code: 'MISSING_FIELDS' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Store telemetry (best-effort, don't block on failure)
    try {
        await env.DB.prepare(`
            INSERT INTO telemetry (device_id, machine_name, os, timezone_offset, app_version, created_at)
            VALUES (?, ?, ?, ?, ?, ?)
        `).bind(
            device_id,
            machine_name || null,
            os || null,
            timezone_offset ?? null,
            app_version,
            Math.floor(Date.now() / 1000)
        ).run();
    } catch {
        // Silently ignore telemetry failures
    }

    // Fetch latest release from GitHub
    try {
        const githubResponse = await fetch(
            'https://api.github.com/repos/samuelclay/crabigator/releases/latest',
            {
                headers: {
                    'Accept': 'application/vnd.github.v3+json',
                    'User-Agent': 'crabigator-api'
                }
            }
        );

        if (!githubResponse.ok) {
            return new Response(
                JSON.stringify({ error: 'GitHub API error', code: 'GITHUB_ERROR' }),
                { status: 502, headers: { 'Content-Type': 'application/json' } }
            );
        }

        const release: GitHubRelease = await githubResponse.json();

        return jsonResponse({
            tag_name: release.tag_name,
            html_url: release.html_url
        });
    } catch (error) {
        return new Response(
            JSON.stringify({ error: 'Failed to fetch release', code: 'FETCH_ERROR' }),
            { status: 502, headers: { 'Content-Type': 'application/json' } }
        );
    }
}

/**
 * GET /staff - Staff telemetry dashboard page
 */
export async function handleStaffDashboard(): Promise<Response> {
    return new Response(staffDashboardHtml, {
        headers: {
            'Content-Type': 'text/html; charset=utf-8',
            'Cache-Control': 'no-cache'
        }
    });
}

interface TelemetryRow {
    id: number;
    device_id: string;
    machine_name: string | null;
    os: string | null;
    timezone_offset: number | null;
    app_version: string;
    created_at: number;
}

interface CountResult {
    count: number;
}

interface VersionCount {
    app_version: string;
    count: number;
}

interface DayCount {
    day: string;
    count: number;
}

/**
 * POST /api/staff/sync-usage - Force sync a group's usage DO
 */
export async function handleStaffSyncUsage(
    request: Request,
    env: Env
): Promise<Response> {
    let body: { group_id: string };
    try {
        body = await request.json();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const { group_id } = body;
    if (!group_id) {
        return new Response(
            JSON.stringify({ error: 'Missing group_id', code: 'MISSING_FIELDS' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Sync the UsageDO
    const doId = env.USAGE.idFromName(group_id);
    const stub = env.USAGE.get(doId);
    await stub.fetch(new Request(`https://internal/sync?group_id=${group_id}`));

    return jsonResponse({ ok: true, synced: group_id });
}

/**
 * GET /api/staff/telemetry - Get telemetry data for dashboard
 */
export async function handleStaffTelemetry(
    _request: Request,
    env: Env
): Promise<Response> {
    const now = Math.floor(Date.now() / 1000);
    const oneDayAgo = now - 86400;
    const sevenDaysAgo = now - 7 * 86400;

    // Run all queries in parallel
    const [
        recentResult,
        totalDevicesResult,
        checks24hResult,
        checks7dResult,
        newDevices24hResult,
        versionDistResult,
        checksByDayResult
    ] = await Promise.all([
        // Recent telemetry (last 100)
        env.DB.prepare(`
            SELECT * FROM telemetry
            ORDER BY created_at DESC
            LIMIT 100
        `).all<TelemetryRow>(),

        // Total unique devices
        env.DB.prepare(`
            SELECT COUNT(DISTINCT device_id) as count FROM telemetry
        `).first<CountResult>(),

        // Checks in last 24h
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM telemetry
            WHERE created_at > ?
        `).bind(oneDayAgo).first<CountResult>(),

        // Checks in last 7 days
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM telemetry
            WHERE created_at > ?
        `).bind(sevenDaysAgo).first<CountResult>(),

        // New devices in last 24h (first seen today)
        env.DB.prepare(`
            SELECT COUNT(DISTINCT device_id) as count FROM telemetry t1
            WHERE created_at > ?
            AND NOT EXISTS (
                SELECT 1 FROM telemetry t2
                WHERE t2.device_id = t1.device_id
                AND t2.created_at <= ?
            )
        `).bind(oneDayAgo, oneDayAgo).first<CountResult>(),

        // Version distribution (last 7 days)
        env.DB.prepare(`
            SELECT app_version, COUNT(DISTINCT device_id) as count
            FROM telemetry
            WHERE created_at > ?
            GROUP BY app_version
            ORDER BY count DESC
            LIMIT 6
        `).bind(sevenDaysAgo).all<VersionCount>(),

        // Checks by day (last 14 days)
        env.DB.prepare(`
            SELECT date(created_at, 'unixepoch') as day, COUNT(*) as count
            FROM telemetry
            WHERE created_at > ?
            GROUP BY day
            ORDER BY day ASC
        `).bind(now - 14 * 86400).all<DayCount>()
    ]);

    // Format version distribution for chart
    const versionDist = versionDistResult.results || [];
    const topVersion = versionDist.length > 0 ? versionDist[0].app_version : null;

    // Format checks by day for chart
    const checksByDay = checksByDayResult.results || [];
    const dayLabels = checksByDay.map(r => {
        const d = new Date(r.day);
        return (d.getMonth() + 1) + '/' + d.getDate();
    });
    const dayValues = checksByDay.map(r => r.count);

    return jsonResponse({
        recent: recentResult.results || [],
        stats: {
            total_devices: totalDevicesResult?.count || 0,
            checks_24h: checks24hResult?.count || 0,
            checks_7d: checks7dResult?.count || 0,
            new_devices_24h: newDevices24hResult?.count || 0,
            top_version: topVersion
        },
        version_distribution: {
            labels: versionDist.map(v => v.app_version),
            values: versionDist.map(v => v.count)
        },
        checks_by_day: {
            labels: dayLabels,
            values: dayValues
        }
    });
}
