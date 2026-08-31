import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { staffDashboardHtml } from '../staff-dashboard';
import { categorizeReferrer } from './analytics';
import { getAppConfig } from '../config';

/**
 * Telemetry data sent with update checks
 */
interface TelemetryRequest {
    device_id: string;
    machine_name?: string;
    os?: string;
    os_version?: string;
    timezone_offset?: number;
    app_version: string;
    cli_version?: string;
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

    const { device_id, machine_name, os, os_version, timezone_offset, app_version, cli_version } = body;

    if (!device_id || !app_version) {
        return new Response(
            JSON.stringify({ error: 'Missing device_id or app_version', code: 'MISSING_FIELDS' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Store telemetry (best-effort, don't block on failure)
    if (getAppConfig(env).features?.marketing_analytics) {
        try {
            await env.DB.prepare(`
                INSERT INTO telemetry (device_id, machine_name, os, os_version, timezone_offset, app_version, cli_version, created_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?)
            `).bind(
                device_id,
                machine_name || null,
                os || null,
                os_version || null,
                timezone_offset ?? null,
                app_version,
                cli_version || null,
                Math.floor(Date.now() / 1000)
            ).run();
        } catch {
            // Telemetry must never prevent update checks.
        }
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
    os_version: string | null;
    timezone_offset: number | null;
    app_version: string;
    cli_version: string | null;
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

/** Format a date string like "2024-01-15" to "1/15" for chart labels */
function formatDay(d: string): string {
    const dt = new Date(d);
    return (dt.getMonth() + 1) + '/' + dt.getDate();
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
        devices24hResult,
        totalChecksResult,
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

        // Total unique devices (all time)
        env.DB.prepare(`
            SELECT COUNT(DISTINCT device_id) as count FROM telemetry
        `).first<CountResult>(),

        // Unique devices in last 24h
        env.DB.prepare(`
            SELECT COUNT(DISTINCT device_id) as count FROM telemetry
            WHERE created_at > ?
        `).bind(oneDayAgo).first<CountResult>(),

        // Total checks (all time)
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM telemetry
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

    // Find the actual highest semantic version (not just most popular)
    const latestVersion = versionDist.length > 0
        ? versionDist
            .map(v => v.app_version)
            .sort((a, b) => {
                const aParts = a.replace(/^v/, '').split('.').map(Number);
                const bParts = b.replace(/^v/, '').split('.').map(Number);
                for (let i = 0; i < 3; i++) {
                    if ((aParts[i] || 0) !== (bParts[i] || 0)) {
                        return (bParts[i] || 0) - (aParts[i] || 0);
                    }
                }
                return 0;
            })[0]
        : null;

    // Format checks by day for chart
    const checksByDay = checksByDayResult.results || [];
    const dayLabels = checksByDay.map(r => formatDay(r.day));
    const dayValues = checksByDay.map(r => r.count);

    return jsonResponse({
        recent: recentResult.results || [],
        stats: {
            total_devices: totalDevicesResult?.count || 0,
            devices_24h: devices24hResult?.count || 0,
            total_checks: totalChecksResult?.count || 0,
            checks_24h: checks24hResult?.count || 0,
            checks_7d: checks7dResult?.count || 0,
            new_devices_24h: newDevices24hResult?.count || 0,
            top_version: latestVersion
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

// ============================================
// Website Analytics
// ============================================

interface ReferrerRow {
    referrer_domain: string | null;
    count: number;
}

interface CampaignRow {
    utm_campaign: string;
    utm_source: string | null;
    visitors: number;
    pageviews: number;
}

interface DeviceRow {
    device_type: string;
    count: number;
}

interface BrowserRow {
    browser: string;
    count: number;
}

interface CountryRow {
    country: string;
    count: number;
}

interface NpmRow {
    date: string;
    downloads: number;
}

interface FunnelRow {
    stage: string;
    count: number;
}

interface CampaignPerfRow {
    campaign: string;
    source: string | null;
    visitors: number;
    pageviews: number;
    avg_time: number | null;
    signups: number;
}

interface EmailSignupRow {
    id: number;
    email: string;
    utm_source: string | null;
    utm_campaign: string | null;
    referrer_domain: string | null;
    promo_code: string | null;
    created_at: number;
}

/**
 * GET /api/staff/analytics - Get website analytics data
 */
export async function handleStaffAnalytics(
    _request: Request,
    env: Env
): Promise<Response> {
    const now = Math.floor(Date.now() / 1000);
    const oneDayAgo = now - 86400;
    const sevenDaysAgo = now - 7 * 86400;
    const thirtyDaysAgo = now - 30 * 86400;

    // Run all queries in parallel
    const [
        visitors24hResult,
        visitorsAllResult,
        pageviews24hResult,
        avgEngagementResult,
        visitorsByDayResult,
        referrerDomainsResult,
        campaignsResult,
        devicesResult,
        browsersResult,
        countriesResult,
        npmDownloadsResult,
        funnelStatsResult,
        campaignPerfResult,
        emailSignupsResult,
        signups24hResult,
        signupsAllResult
    ] = await Promise.all([
        // Unique visitors (24h)
        env.DB.prepare(`
            SELECT COUNT(DISTINCT visitor_id) as count FROM page_views
            WHERE created_at > ?
        `).bind(oneDayAgo).first<CountResult>(),

        // Unique visitors (all time)
        env.DB.prepare(`
            SELECT COUNT(DISTINCT visitor_id) as count FROM page_views
        `).first<CountResult>(),

        // Page views (24h)
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM page_views
            WHERE created_at > ?
        `).bind(oneDayAgo).first<CountResult>(),

        // Average engagement (24h)
        env.DB.prepare(`
            SELECT
                AVG(time_on_page) as avg_time,
                AVG(scroll_depth) as avg_scroll,
                SUM(CASE WHEN time_on_page < 10 THEN 1 ELSE 0 END) * 100.0 / NULLIF(COUNT(*), 0) as bounce_rate
            FROM page_views
            WHERE created_at > ?
        `).bind(oneDayAgo).first<{ avg_time: number | null; avg_scroll: number | null; bounce_rate: number | null }>(),

        // Visitors by day (30 days)
        env.DB.prepare(`
            SELECT date(created_at, 'unixepoch') as day, COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at > ?
            GROUP BY day
            ORDER BY day ASC
        `).bind(thirtyDaysAgo).all<DayCount>(),

        // Referrer domains (for categorization)
        env.DB.prepare(`
            SELECT referrer_domain, COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at > ?
            GROUP BY referrer_domain
            ORDER BY count DESC
        `).bind(thirtyDaysAgo).all<ReferrerRow>(),

        // UTM campaigns
        env.DB.prepare(`
            SELECT
                utm_campaign,
                utm_source,
                COUNT(DISTINCT visitor_id) as visitors,
                COUNT(*) as pageviews
            FROM page_views
            WHERE created_at > ? AND utm_campaign IS NOT NULL
            GROUP BY utm_campaign, utm_source
            ORDER BY visitors DESC
            LIMIT 20
        `).bind(thirtyDaysAgo).all<CampaignRow>(),

        // Device distribution (7 days)
        env.DB.prepare(`
            SELECT device_type, COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at > ?
            GROUP BY device_type
            ORDER BY count DESC
        `).bind(sevenDaysAgo).all<DeviceRow>(),

        // Browser distribution (7 days)
        env.DB.prepare(`
            SELECT browser, COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at > ?
            GROUP BY browser
            ORDER BY count DESC
            LIMIT 6
        `).bind(sevenDaysAgo).all<BrowserRow>(),

        // Countries (30 days)
        env.DB.prepare(`
            SELECT country, COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at > ? AND country IS NOT NULL
            GROUP BY country
            ORDER BY count DESC
            LIMIT 10
        `).bind(thirtyDaysAgo).all<CountryRow>(),

        // NPM downloads (30 days)
        env.DB.prepare(`
            SELECT date, downloads FROM npm_downloads
            ORDER BY date DESC
            LIMIT 30
        `).all<NpmRow>(),

        // Funnel stats (30 days)
        env.DB.prepare(`
            SELECT stage, COUNT(DISTINCT COALESCE(visitor_id, device_id, group_id)) as count
            FROM funnel_events
            WHERE created_at > ?
            GROUP BY stage
        `).bind(thirtyDaysAgo).all<FunnelRow>(),

        // Campaign performance with conversions
        env.DB.prepare(`
            SELECT
                pv.utm_campaign as campaign,
                pv.utm_source as source,
                COUNT(DISTINCT pv.visitor_id) as visitors,
                COUNT(*) as pageviews,
                AVG(pv.time_on_page) as avg_time,
                (SELECT COUNT(*) FROM email_signups es
                 WHERE es.utm_campaign = pv.utm_campaign) as signups
            FROM page_views pv
            WHERE pv.created_at > ? AND pv.utm_campaign IS NOT NULL
            GROUP BY pv.utm_campaign, pv.utm_source
            ORDER BY visitors DESC
        `).bind(thirtyDaysAgo).all<CampaignPerfRow>(),

        // Recent email signups (last 50)
        env.DB.prepare(`
            SELECT id, email, utm_source, utm_campaign, referrer_domain, promo_code, created_at
            FROM email_signups
            ORDER BY created_at DESC
            LIMIT 50
        `).all<EmailSignupRow>(),

        // Email signups (24h)
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM email_signups
            WHERE created_at > ?
        `).bind(oneDayAgo).first<CountResult>(),

        // Email signups (all time)
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM email_signups
        `).first<CountResult>()
    ]);

    // Categorize referrer domains into traffic sources
    const referrerDomains = referrerDomainsResult.results || [];
    const sourceCategories: Record<string, number> = {};
    const sourceDetails: Record<string, Array<{ domain: string | null; visitors: number }>> = {};
    for (const row of referrerDomains) {
        const category = categorizeReferrer(row.referrer_domain);
        sourceCategories[category] = (sourceCategories[category] || 0) + row.count;
        if (!sourceDetails[category]) sourceDetails[category] = [];
        sourceDetails[category].push({ domain: row.referrer_domain, visitors: row.count });
    }

    // Sort sources by count
    const sortedSources = Object.entries(sourceCategories)
        .sort((a, b) => b[1] - a[1]);

    // Sort domains within each category by visitors
    for (const domains of Object.values(sourceDetails)) {
        domains.sort((a, b) => b.visitors - a.visitors);
    }

    // Format visitors by day for chart
    const visitorsByDay = visitorsByDayResult.results || [];
    const dayLabels = visitorsByDay.map(r => formatDay(r.day));

    // Format NPM downloads for chart (reverse to chronological order)
    const npmDownloads = (npmDownloadsResult.results || []).reverse();

    // Build funnel data
    const funnelStats = funnelStatsResult.results || [];
    const funnel: Record<string, number> = {};
    for (const row of funnelStats) {
        funnel[row.stage] = row.count;
    }

    return jsonResponse({
        summary: {
            visitors_24h: visitors24hResult?.count || 0,
            visitors_all: visitorsAllResult?.count || 0,
            pageviews_24h: pageviews24hResult?.count || 0,
            avg_time_on_page: Math.round(avgEngagementResult?.avg_time || 0),
            avg_scroll_depth: Math.round(avgEngagementResult?.avg_scroll || 0),
            bounce_rate: Math.round(avgEngagementResult?.bounce_rate || 0),
            signups_24h: signups24hResult?.count || 0,
            signups_all: signupsAllResult?.count || 0
        },
        visitors_by_day: {
            labels: dayLabels,
            values: visitorsByDay.map(r => r.count)
        },
        traffic_sources: {
            labels: sortedSources.map(([name]) => name),
            values: sortedSources.map(([, count]) => count)
        },
        traffic_sources_detail: sourceDetails,
        campaigns: campaignsResult.results || [],
        devices: {
            labels: (devicesResult.results || []).map(r => r.device_type || 'Unknown'),
            values: (devicesResult.results || []).map(r => r.count)
        },
        browsers: {
            labels: (browsersResult.results || []).map(r => r.browser || 'Unknown'),
            values: (browsersResult.results || []).map(r => r.count)
        },
        countries: {
            labels: (countriesResult.results || []).map(r => r.country),
            values: (countriesResult.results || []).map(r => r.count)
        },
        npm_downloads: {
            labels: npmDownloads.map(r => r.date.slice(5)), // MM-DD format
            values: npmDownloads.map(r => r.downloads)
        },
        funnel,
        campaign_performance: campaignPerfResult.results || [],
        email_signups: emailSignupsResult.results || []
    });
}

// ============================================
// Session Analytics
// ============================================

function parseRange(range: string | null): number {
    const now = Math.floor(Date.now() / 1000);
    switch (range) {
        case '7d': return now - 7 * 86400;
        case '90d': return now - 90 * 86400;
        case 'all': return 0;
        default: return now - 30 * 86400; // 30d default
    }
}

function computePercentiles(sorted: number[]): Record<string, number> {
    if (sorted.length === 0) return { p50: 0, p75: 0, p90: 0, p95: 0, p99: 0 };
    const p = (pct: number) => sorted[Math.min(Math.floor(sorted.length * pct), sorted.length - 1)];
    return { p50: p(0.50), p75: p(0.75), p90: p(0.90), p95: p(0.95), p99: p(0.99) };
}

interface WeeklyPercentiles {
    labels: string[];
    p50: number[];
    p75: number[];
    p90: number[];
    p95: number[];
    p99: number[];
}

function computeWeeklyPercentiles(
    rows: Array<{ week: string; val: number }>,
    roundTo: number = 0
): WeeklyPercentiles {
    const byWeek = new Map<string, number[]>();
    for (const row of rows) {
        if (row.val == null || row.val <= 0) continue;
        if (!byWeek.has(row.week)) byWeek.set(row.week, []);
        byWeek.get(row.week)!.push(row.val);
    }
    const result: WeeklyPercentiles = { labels: [], p50: [], p75: [], p90: [], p95: [], p99: [] };
    for (const [week, values] of byWeek) {
        values.sort((a, b) => a - b);
        const pct = computePercentiles(values);
        const r = roundTo > 0 ? (v: number) => Math.round(v * roundTo) / roundTo : Math.round;
        result.labels.push(formatDay(week));
        result.p50.push(r(pct.p50));
        result.p75.push(r(pct.p75));
        result.p90.push(r(pct.p90));
        result.p95.push(r(pct.p95));
        result.p99.push(r(pct.p99));
    }
    return result;
}

/**
 * GET /api/staff/session-analytics?range=30d - Aggregate session analytics
 */
export async function handleStaffSessionAnalytics(
    request: Request,
    env: Env
): Promise<Response> {
    const url = new URL(request.url);
    const since = parseRange(url.searchParams.get('range'));

    const [
        // Overview stats
        totalSessionsResult,
        activeUsersResult,
        avgSessionTimeResult,
        avgThinkingTimeResult,

        // Behavior distributions
        promptsPerSessionResult,
        toolsPerSessionResult,
        thinkingPerPromptResult,

        // Distributions
        thinkingValuesResult,
        durationValuesResult,
        toolUsageResult,
        platformUsageResult,
        modelUsageResult,
        modeUsageResult,

        // Repo analytics
        topReposResult,
        repoLongTailResult,
        reposPerUserResult,

        // User patterns
        toolsByUserResult,
        sessionsPerUserResult,

        // Event-level analytics
        perPromptThinkingResult,
        interPromptGapsResult,

        // Weekly trends
        weeklyBehaviorResult,
        weeklyThinkingResult,

        // Titles
        titlesPerSessionResult,
        weeklyTitlesResult,

        // Weekly percentile trends
        weeklySessionRawResult,
        weeklyPerPromptThinkingRawResult,
        weeklyInterPromptGapsRawResult,
    ] = await Promise.all([
        // --- OVERVIEW ---
        env.DB.prepare(`
            SELECT COUNT(*) as count FROM sessions WHERE started_at > ?
        `).bind(since).first<{ count: number }>(),

        env.DB.prepare(`
            SELECT COUNT(DISTINCT device_id) as count FROM sessions WHERE started_at > ?
        `).bind(since).first<{ count: number }>(),

        env.DB.prepare(`
            SELECT AVG(work_seconds) as avg_time FROM sessions
            WHERE started_at > ? AND work_seconds IS NOT NULL AND work_seconds > 0
        `).bind(since).first<{ avg_time: number | null }>(),

        env.DB.prepare(`
            SELECT AVG(thinking_seconds) as avg_time FROM sessions
            WHERE started_at > ? AND thinking_seconds > 0
        `).bind(since).first<{ avg_time: number | null }>(),

        // --- BEHAVIOR DISTRIBUTIONS ---
        env.DB.prepare(`
            SELECT prompts as val FROM sessions
            WHERE started_at > ? AND prompts > 0
            ORDER BY prompts ASC
        `).bind(since).all<{ val: number }>(),

        env.DB.prepare(`
            SELECT tool_calls as val FROM sessions
            WHERE started_at > ? AND tool_calls > 0
            ORDER BY tool_calls ASC
        `).bind(since).all<{ val: number }>(),

        env.DB.prepare(`
            SELECT CAST(thinking_seconds AS REAL) / prompts as val FROM sessions
            WHERE started_at > ? AND thinking_seconds > 0 AND prompts > 0
            ORDER BY val ASC
        `).bind(since).all<{ val: number }>(),

        // --- DISTRIBUTIONS ---
        env.DB.prepare(`
            SELECT thinking_seconds as val FROM sessions
            WHERE started_at > ? AND thinking_seconds > 0
            ORDER BY thinking_seconds ASC
        `).bind(since).all<{ val: number }>(),

        env.DB.prepare(`
            SELECT work_seconds as val FROM sessions
            WHERE started_at > ? AND work_seconds IS NOT NULL AND work_seconds > 0
            ORDER BY work_seconds ASC
        `).bind(since).all<{ val: number }>(),

        env.DB.prepare(`
            SELECT tool_name, SUM(count) as total
            FROM session_tool_usage
            JOIN sessions ON sessions.id = session_tool_usage.session_id
            WHERE sessions.started_at > ?
            GROUP BY tool_name ORDER BY total DESC LIMIT 15
        `).bind(since).all<{ tool_name: string; total: number }>(),

        env.DB.prepare(`
            SELECT platform, COUNT(*) as count FROM sessions
            WHERE started_at > ?
            GROUP BY platform ORDER BY count DESC
        `).bind(since).all<{ platform: string; count: number }>(),

        env.DB.prepare(`
            SELECT model, COUNT(*) as count FROM sessions
            WHERE started_at > ? AND model IS NOT NULL
            GROUP BY model ORDER BY count DESC
        `).bind(since).all<{ model: string; count: number }>(),

        env.DB.prepare(`
            SELECT mode, COUNT(*) as count FROM sessions
            WHERE started_at > ? AND mode IS NOT NULL
            GROUP BY mode ORDER BY count DESC
        `).bind(since).all<{ mode: string; count: number }>(),

        // --- REPO ANALYTICS ---
        env.DB.prepare(`
            SELECT cwd, COUNT(*) as session_count FROM sessions
            WHERE started_at > ?
            GROUP BY cwd ORDER BY session_count DESC LIMIT 10
        `).bind(since).all<{ cwd: string; session_count: number }>(),

        env.DB.prepare(`
            SELECT
                SUM(CASE WHEN cnt = 1 THEN 1 ELSE 0 END) as one_session,
                SUM(CASE WHEN cnt BETWEEN 2 AND 5 THEN 1 ELSE 0 END) as two_to_five,
                SUM(CASE WHEN cnt > 5 THEN 1 ELSE 0 END) as more_than_five
            FROM (
                SELECT cwd, COUNT(*) as cnt FROM sessions
                WHERE started_at > ? GROUP BY cwd
            )
        `).bind(since).first<{ one_session: number; two_to_five: number; more_than_five: number }>(),

        env.DB.prepare(`
            SELECT repo_count, COUNT(*) as user_count FROM (
                SELECT device_id, COUNT(DISTINCT cwd) as repo_count
                FROM sessions WHERE started_at > ? GROUP BY device_id
            ) GROUP BY repo_count ORDER BY repo_count ASC
        `).bind(since).all<{ repo_count: number; user_count: number }>(),

        // --- USER PATTERNS ---
        // Tool usage by top 5 users (join telemetry for machine_name)
        env.DB.prepare(`
            SELECT
                COALESCE(t.machine_name, SUBSTR(s.device_id, 1, 8)) as user_label,
                stu.tool_name,
                SUM(stu.count) as total
            FROM session_tool_usage stu
            JOIN sessions s ON s.id = stu.session_id
            LEFT JOIN (
                SELECT device_id, machine_name,
                       ROW_NUMBER() OVER (PARTITION BY device_id ORDER BY created_at DESC) as rn
                FROM telemetry WHERE machine_name IS NOT NULL
            ) t ON t.device_id = s.device_id AND t.rn = 1
            WHERE s.started_at > ?
            AND s.device_id IN (
                SELECT device_id FROM sessions
                WHERE started_at > ? GROUP BY device_id
                ORDER BY COUNT(*) DESC LIMIT 5
            )
            GROUP BY user_label, stu.tool_name
            ORDER BY total DESC
        `).bind(since, since).all<{ user_label: string; tool_name: string; total: number }>(),

        // Sessions per user distribution
        env.DB.prepare(`
            SELECT session_count, COUNT(*) as user_count FROM (
                SELECT device_id, COUNT(*) as session_count
                FROM sessions WHERE started_at > ? GROUP BY device_id
            ) GROUP BY session_count ORDER BY session_count ASC
        `).bind(since).all<{ session_count: number; user_count: number }>(),

        // --- EVENT-LEVEL ANALYTICS ---
        // Per-prompt thinking time: consecutive UserPromptSubmit→Stop pairs
        env.DB.prepare(`
            SELECT (stop.timestamp_ms - start.timestamp_ms) / 1000 as thinking_sec
            FROM session_events_log start
            JOIN session_events_log stop ON stop.session_id = start.session_id
                AND stop.event_type = 'Stop'
                AND stop.timestamp_ms > start.timestamp_ms
                AND stop.id = (
                    SELECT MIN(id) FROM session_events_log
                    WHERE session_id = start.session_id
                    AND event_type = 'Stop'
                    AND timestamp_ms > start.timestamp_ms
                )
            JOIN sessions s ON s.id = start.session_id
            WHERE start.event_type = 'UserPromptSubmit'
            AND s.started_at > ?
            ORDER BY thinking_sec ASC
        `).bind(since).all<{ thinking_sec: number }>(),

        // Time between consecutive user prompts (inter-prompt gaps)
        env.DB.prepare(`
            SELECT (e2.timestamp_ms - e1.timestamp_ms) / 1000 as gap_sec
            FROM session_events_log e1
            JOIN session_events_log e2 ON e2.session_id = e1.session_id
                AND e2.event_type = 'UserPromptSubmit'
                AND e2.timestamp_ms > e1.timestamp_ms
                AND e2.id = (
                    SELECT MIN(id) FROM session_events_log
                    WHERE session_id = e1.session_id
                    AND event_type = 'UserPromptSubmit'
                    AND timestamp_ms > e1.timestamp_ms
                )
            JOIN sessions s ON s.id = e1.session_id
            WHERE e1.event_type = 'UserPromptSubmit'
            AND s.started_at > ?
            ORDER BY gap_sec ASC
        `).bind(since).all<{ gap_sec: number }>(),

        // --- WEEKLY TRENDS ---
        // Prompts + tools per session (weekly avg)
        env.DB.prepare(`
            SELECT date(started_at, 'unixepoch', 'weekday 0', '-6 days') as week,
                   AVG(prompts) as avg_prompts,
                   AVG(tool_calls) as avg_tools
            FROM sessions
            WHERE started_at > ? AND prompts > 0
            GROUP BY week ORDER BY week ASC
        `).bind(since).all<{ week: string; avg_prompts: number; avg_tools: number }>(),

        // Thinking per session + thinking per prompt (weekly avg)
        env.DB.prepare(`
            SELECT date(started_at, 'unixepoch', 'weekday 0', '-6 days') as week,
                   AVG(thinking_seconds) as avg_thinking,
                   AVG(CAST(thinking_seconds AS REAL) / prompts) as avg_thinking_per_prompt
            FROM sessions
            WHERE started_at > ? AND thinking_seconds > 0 AND prompts > 0
            GROUP BY week ORDER BY week ASC
        `).bind(since).all<{ week: string; avg_thinking: number; avg_thinking_per_prompt: number }>(),

        // --- TITLES ---
        // Unique titles per session (count from JSON array)
        env.DB.prepare(`
            SELECT json_array_length(titles) as val FROM sessions
            WHERE started_at > ? AND titles IS NOT NULL AND titles != '[]'
            ORDER BY val ASC
        `).bind(since).all<{ val: number }>(),

        // Weekly avg unique titles per session
        env.DB.prepare(`
            SELECT date(started_at, 'unixepoch', 'weekday 0', '-6 days') as week,
                   AVG(json_array_length(titles)) as avg_titles
            FROM sessions
            WHERE started_at > ? AND titles IS NOT NULL AND titles != '[]'
            GROUP BY week ORDER BY week ASC
        `).bind(since).all<{ week: string; avg_titles: number }>(),

        // --- WEEKLY PERCENTILE TRENDS ---
        // All session-level values by week (for computing per-week percentiles)
        env.DB.prepare(`
            SELECT
                date(started_at, 'unixepoch', 'weekday 0', '-6 days') as week,
                prompts,
                tool_calls,
                thinking_seconds,
                CASE WHEN prompts > 0 THEN CAST(thinking_seconds AS REAL) / prompts ELSE NULL END as thinking_per_prompt,
                work_seconds,
                CASE WHEN titles IS NOT NULL AND titles != '[]' THEN json_array_length(titles) ELSE NULL END as title_count
            FROM sessions
            WHERE started_at > ?
            ORDER BY week ASC
        `).bind(since).all<{
            week: string; prompts: number; tool_calls: number;
            thinking_seconds: number; thinking_per_prompt: number | null;
            work_seconds: number | null; title_count: number | null;
        }>(),

        // Per-prompt thinking by week (event-level)
        env.DB.prepare(`
            SELECT
                date(s.started_at, 'unixepoch', 'weekday 0', '-6 days') as week,
                (stop.timestamp_ms - start.timestamp_ms) / 1000 as val
            FROM session_events_log start
            JOIN session_events_log stop ON stop.session_id = start.session_id
                AND stop.event_type = 'Stop'
                AND stop.timestamp_ms > start.timestamp_ms
                AND stop.id = (
                    SELECT MIN(id) FROM session_events_log
                    WHERE session_id = start.session_id
                    AND event_type = 'Stop'
                    AND timestamp_ms > start.timestamp_ms
                )
            JOIN sessions s ON s.id = start.session_id
            WHERE start.event_type = 'UserPromptSubmit'
            AND s.started_at > ?
            ORDER BY week ASC
        `).bind(since).all<{ week: string; val: number }>(),

        // Inter-prompt gap by week (event-level)
        env.DB.prepare(`
            SELECT
                date(s.started_at, 'unixepoch', 'weekday 0', '-6 days') as week,
                (e2.timestamp_ms - e1.timestamp_ms) / 1000 as val
            FROM session_events_log e1
            JOIN session_events_log e2 ON e2.session_id = e1.session_id
                AND e2.event_type = 'UserPromptSubmit'
                AND e2.timestamp_ms > e1.timestamp_ms
                AND e2.id = (
                    SELECT MIN(id) FROM session_events_log
                    WHERE session_id = e1.session_id
                    AND event_type = 'UserPromptSubmit'
                    AND timestamp_ms > e1.timestamp_ms
                )
            JOIN sessions s ON s.id = e1.session_id
            WHERE e1.event_type = 'UserPromptSubmit'
            AND s.started_at > ?
            ORDER BY week ASC
        `).bind(since).all<{ week: string; val: number }>(),
    ]);

    // Compute percentiles
    const thinkingValues = (thinkingValuesResult.results || []).map(r => r.val);
    const durationValues = (durationValuesResult.results || []).map(r => r.val);
    const perPromptValues = (perPromptThinkingResult.results || []).map(r => r.thinking_sec).filter(v => v > 0);
    const interPromptValues = (interPromptGapsResult.results || []).map(r => r.gap_sec).filter(v => v > 0);

    // Behavior distributions
    const promptsPerSession = (promptsPerSessionResult.results || []).map(r => r.val);
    const toolsPerSession = (toolsPerSessionResult.results || []).map(r => r.val);
    const thinkingPerPrompt = (thinkingPerPromptResult.results || []).map(r => Math.round(r.val));

    // Build tools-by-user pivot for stacked bar
    const toolsByUser = toolsByUserResult.results || [];
    const userToolPivot: Record<string, Record<string, number>> = {};
    const allToolNames = new Set<string>();
    for (const row of toolsByUser) {
        if (!userToolPivot[row.user_label]) userToolPivot[row.user_label] = {};
        userToolPivot[row.user_label][row.tool_name] = row.total;
        allToolNames.add(row.tool_name);
    }
    // Top 5 tools by total usage across these users
    const topToolNames = [...allToolNames]
        .map(t => ({ name: t, total: toolsByUser.filter(r => r.tool_name === t).reduce((s, r) => s + r.total, 0) }))
        .sort((a, b) => b.total - a.total)
        .slice(0, 5)
        .map(t => t.name);

    // Extract repo name from full path
    const topRepos = (topReposResult.results || []).map(r => ({
        name: r.cwd.split('/').pop() || r.cwd,
        full_path: r.cwd,
        count: r.session_count,
    }));

    // Weekly trends
    const weeklyBehavior = weeklyBehaviorResult.results || [];
    const weeklyThinking = weeklyThinkingResult.results || [];

    // Titles
    const titlesPerSession = (titlesPerSessionResult.results || []).map(r => r.val);
    const weeklyTitles = weeklyTitlesResult.results || [];

    // Weekly percentile trends from bulk session data
    const sessionRaw = weeklySessionRawResult.results || [];
    const weeklyPromptsPercentiles = computeWeeklyPercentiles(
        sessionRaw.filter(r => r.prompts > 0).map(r => ({ week: r.week, val: r.prompts }))
    );
    const weeklyToolsPercentiles = computeWeeklyPercentiles(
        sessionRaw.filter(r => r.tool_calls > 0).map(r => ({ week: r.week, val: r.tool_calls }))
    );
    const weeklyThinkingPerPromptPercentiles = computeWeeklyPercentiles(
        sessionRaw.filter(r => r.thinking_per_prompt != null && r.thinking_per_prompt > 0)
            .map(r => ({ week: r.week, val: r.thinking_per_prompt! })), 10
    );
    const weeklyThinkingPercentiles = computeWeeklyPercentiles(
        sessionRaw.filter(r => r.thinking_seconds > 0).map(r => ({ week: r.week, val: r.thinking_seconds }))
    );
    const weeklyDurationPercentiles = computeWeeklyPercentiles(
        sessionRaw.filter(r => r.work_seconds != null && r.work_seconds > 0)
            .map(r => ({ week: r.week, val: r.work_seconds! }))
    );
    const weeklyTitlesPercentiles = computeWeeklyPercentiles(
        sessionRaw.filter(r => r.title_count != null && r.title_count > 0)
            .map(r => ({ week: r.week, val: r.title_count! }))
    );
    const weeklyPerPromptThinkingPercentiles = computeWeeklyPercentiles(
        (weeklyPerPromptThinkingRawResult.results || []).filter(r => r.val > 0)
    );
    const weeklyInterPromptGapPercentiles = computeWeeklyPercentiles(
        (weeklyInterPromptGapsRawResult.results || []).filter(r => r.val > 0)
    );

    return jsonResponse({
        overview: {
            total_sessions: totalSessionsResult?.count || 0,
            active_users: activeUsersResult?.count || 0,
            avg_session_time: Math.round(avgSessionTimeResult?.avg_time || 0),
            avg_thinking_time: Math.round(avgThinkingTimeResult?.avg_time || 0),
        },
        prompts_per_session_percentiles: computePercentiles(promptsPerSession),
        tools_per_session_percentiles: computePercentiles(toolsPerSession),
        thinking_per_prompt_percentiles: computePercentiles(thinkingPerPrompt),
        thinking_percentiles: computePercentiles(thinkingValues),
        session_duration_percentiles: computePercentiles(durationValues),
        per_prompt_thinking_percentiles: computePercentiles(perPromptValues),
        inter_prompt_gap_percentiles: computePercentiles(interPromptValues),
        tool_usage: {
            labels: (toolUsageResult.results || []).map(r => r.tool_name),
            values: (toolUsageResult.results || []).map(r => r.total),
        },
        platform_usage: {
            labels: (platformUsageResult.results || []).map(r => r.platform),
            values: (platformUsageResult.results || []).map(r => r.count),
        },
        model_usage: {
            labels: (modelUsageResult.results || []).map(r => r.model),
            values: (modelUsageResult.results || []).map(r => r.count),
        },
        mode_usage: {
            labels: (modeUsageResult.results || []).map(r => r.mode),
            values: (modeUsageResult.results || []).map(r => r.count),
        },
        top_repos: topRepos,
        repo_long_tail: repoLongTailResult || { one_session: 0, two_to_five: 0, more_than_five: 0 },
        repos_per_user: {
            labels: (reposPerUserResult.results || []).map(r => String(r.repo_count)),
            values: (reposPerUserResult.results || []).map(r => r.user_count),
        },
        tools_by_user: {
            users: Object.keys(userToolPivot),
            tools: topToolNames,
            data: userToolPivot,
        },
        sessions_per_user: {
            labels: (sessionsPerUserResult.results || []).map(r => String(r.session_count)),
            values: (sessionsPerUserResult.results || []).map(r => r.user_count),
        },
        weekly_behavior: {
            labels: weeklyBehavior.map(r => formatDay(r.week)),
            avg_prompts: weeklyBehavior.map(r => Math.round(r.avg_prompts * 10) / 10),
            avg_tools: weeklyBehavior.map(r => Math.round(r.avg_tools)),
        },
        weekly_thinking: {
            labels: weeklyThinking.map(r => formatDay(r.week)),
            avg_thinking: weeklyThinking.map(r => Math.round(r.avg_thinking)),
            avg_thinking_per_prompt: weeklyThinking.map(r => Math.round(r.avg_thinking_per_prompt)),
        },
        titles_per_session_percentiles: computePercentiles(titlesPerSession),
        weekly_titles: {
            labels: weeklyTitles.map(r => formatDay(r.week)),
            avg_titles: weeklyTitles.map(r => Math.round(r.avg_titles * 10) / 10),
        },
        // Weekly percentile trends
        weekly_prompts_percentiles: weeklyPromptsPercentiles,
        weekly_tools_percentiles: weeklyToolsPercentiles,
        weekly_thinking_per_prompt_percentiles: weeklyThinkingPerPromptPercentiles,
        weekly_thinking_percentiles: weeklyThinkingPercentiles,
        weekly_duration_percentiles: weeklyDurationPercentiles,
        weekly_titles_percentiles: weeklyTitlesPercentiles,
        weekly_per_prompt_thinking_percentiles: weeklyPerPromptThinkingPercentiles,
        weekly_inter_prompt_gap_percentiles: weeklyInterPromptGapPercentiles,
    });
}
