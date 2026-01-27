import type { Env } from '../types/env';
import { jsonResponse } from '../router';
import { staffDashboardHtml } from '../staff-dashboard';
import { categorizeReferrer } from './analytics';

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

interface VisitorsByDayRow {
    day: string;
    count: number;
}

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
        emailSignupsResult
    ] = await Promise.all([
        // Unique visitors (24h)
        env.DB.prepare(`
            SELECT COUNT(DISTINCT visitor_id) as count FROM page_views
            WHERE created_at > ?
        `).bind(oneDayAgo).first<CountResult>(),

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
        `).bind(thirtyDaysAgo).all<VisitorsByDayRow>(),

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
        `).all<EmailSignupRow>()
    ]);

    // Categorize referrer domains into traffic sources
    const referrerDomains = referrerDomainsResult.results || [];
    const sourceCategories: Record<string, number> = {};
    for (const row of referrerDomains) {
        const category = categorizeReferrer(row.referrer_domain);
        sourceCategories[category] = (sourceCategories[category] || 0) + row.count;
    }

    // Sort sources by count
    const sortedSources = Object.entries(sourceCategories)
        .sort((a, b) => b[1] - a[1]);

    // Format visitors by day for chart
    const visitorsByDay = visitorsByDayResult.results || [];
    const dayLabels = visitorsByDay.map(r => {
        const d = new Date(r.day);
        return (d.getMonth() + 1) + '/' + d.getDate();
    });

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
            pageviews_24h: pageviews24hResult?.count || 0,
            avg_time_on_page: Math.round(avgEngagementResult?.avg_time || 0),
            avg_scroll_depth: Math.round(avgEngagementResult?.avg_scroll || 0),
            bounce_rate: Math.round(avgEngagementResult?.bounce_rate || 0)
        },
        visitors_by_day: {
            labels: dayLabels,
            values: visitorsByDay.map(r => r.count)
        },
        traffic_sources: {
            labels: sortedSources.map(([name]) => name),
            values: sortedSources.map(([, count]) => count)
        },
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
