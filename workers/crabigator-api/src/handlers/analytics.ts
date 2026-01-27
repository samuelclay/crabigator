import type { Env } from '../types/env';

/**
 * User-Agent parsing to extract device, browser, and OS
 */
function parseUserAgent(ua: string): { device: string; browser: string; os: string } {
    // Device detection
    const device = /Mobile|Android|iPhone/.test(ua)
        ? (/iPad|Tablet/.test(ua) ? 'tablet' : 'mobile')
        : 'desktop';

    // Browser detection
    let browser = 'Other';
    if (ua.includes('Edg/')) browser = 'Edge';
    else if (ua.includes('Chrome')) browser = 'Chrome';
    else if (ua.includes('Safari')) browser = 'Safari';
    else if (ua.includes('Firefox')) browser = 'Firefox';
    else if (ua.includes('Opera') || ua.includes('OPR')) browser = 'Opera';

    // OS detection
    let os = 'Other';
    if (ua.includes('iPhone') || ua.includes('iPad')) os = 'iOS';
    else if (ua.includes('Android')) os = 'Android';
    else if (ua.includes('Mac OS') || ua.includes('Macintosh')) os = 'macOS';
    else if (ua.includes('Windows')) os = 'Windows';
    else if (ua.includes('Linux')) os = 'Linux';

    return { device, browser, os };
}

/**
 * Extract domain from referrer URL, returning null for empty/invalid
 */
function extractDomain(url: string | null | undefined): string | null {
    if (!url) return null;
    try {
        return new URL(url).hostname.replace(/^www\./, '');
    } catch {
        return null;
    }
}

/**
 * Categorize a referrer domain into source type
 */
export function categorizeReferrer(domain: string | null): string {
    if (!domain) return 'Direct';

    const d = domain.toLowerCase();

    // AI Assistants
    if (d.includes('chat.openai.com') || d.includes('chatgpt.com') ||
        d.includes('claude.ai') || d.includes('perplexity.ai') ||
        d.includes('copilot.microsoft.com') || d.includes('you.com') ||
        d.includes('poe.com') || d.includes('bard.google.com') ||
        d.includes('gemini.google.com')) {
        return 'AI Assistant';
    }

    // Search engines
    if (d.includes('google.') || d === 'bing.com' || d === 'duckduckgo.com' ||
        d === 'yahoo.com' || d === 'baidu.com' || d === 'yandex.') {
        return 'Search';
    }

    // Social media
    if (d === 'twitter.com' || d === 'x.com' || d === 'linkedin.com' ||
        d === 'reddit.com' || d === 'facebook.com' || d === 'instagram.com' ||
        d === 'youtube.com' || d === 'tiktok.com') {
        return 'Social';
    }

    // Developer sites
    if (d === 'github.com' || d === 'stackoverflow.com' || d === 'dev.to' ||
        d === 'news.ycombinator.com' || d === 'hackernews.com' ||
        d === 'lobste.rs' || d === 'medium.com') {
        return 'Dev';
    }

    return 'Other';
}

/**
 * Request body for page view beacon
 */
interface PageViewRequest {
    visitor_id: string;
    session_id: string;
    page?: string;
    referrer?: string;
    utm_source?: string;
    utm_medium?: string;
    utm_campaign?: string;
    utm_content?: string;
    utm_term?: string;
    promo_code?: string;
}

/**
 * Request body for engagement update beacon
 */
interface EngagementRequest {
    visitor_id: string;
    session_id: string;
    scroll_depth?: number;
    time_on_page?: number;
}

/**
 * Request body for analytics event
 */
interface EventRequest {
    visitor_id: string;
    session_id: string;
    event_type: string;
    event_target?: string;
    event_value?: string;
    page?: string;
    utm_source?: string;
    utm_medium?: string;
    utm_campaign?: string;
    promo_code?: string;
    referrer?: string;
}

/**
 * POST /api/analytics/beacon - Record page view or engagement update
 *
 * Query param ?type=engagement for updating scroll/time metrics
 * Otherwise records a new page view
 */
export async function handleAnalyticsBeacon(
    request: Request,
    env: Env
): Promise<Response> {
    const url = new URL(request.url);
    const isEngagement = url.searchParams.get('type') === 'engagement';

    let body: PageViewRequest | EngagementRequest;
    try {
        body = await request.json();
    } catch {
        // Silently accept malformed requests (analytics should not block)
        return new Response('', { status: 204 });
    }

    const { visitor_id, session_id } = body;

    if (!visitor_id || !session_id) {
        return new Response('', { status: 204 });
    }

    if (isEngagement) {
        // Update existing page view with engagement metrics
        const engagement = body as EngagementRequest;
        try {
            await env.DB.prepare(`
                UPDATE page_views
                SET scroll_depth = MAX(scroll_depth, ?),
                    time_on_page = MAX(time_on_page, ?)
                WHERE id = (
                    SELECT id FROM page_views
                    WHERE visitor_id = ? AND session_id = ?
                    ORDER BY created_at DESC
                    LIMIT 1
                )
            `).bind(
                engagement.scroll_depth || 0,
                engagement.time_on_page || 0,
                visitor_id,
                session_id
            ).run();
        } catch {
            // Silently ignore failures
        }
    } else {
        // Record new page view
        const pv = body as PageViewRequest;

        // Parse request metadata
        const ua = request.headers.get('User-Agent') || '';
        const { device, browser, os } = parseUserAgent(ua);

        // Geo from Cloudflare headers
        const country = request.headers.get('CF-IPCountry') || null;
        const region = request.headers.get('CF-Region') || null;
        const city = request.headers.get('CF-City') || null;

        const referrerDomain = extractDomain(pv.referrer);

        try {
            // Insert page view
            await env.DB.prepare(`
                INSERT INTO page_views (
                    visitor_id, session_id, page, referrer, referrer_domain,
                    utm_source, utm_medium, utm_campaign, utm_content, utm_term,
                    promo_code, device_type, browser, os, country, region, city
                ) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
            `).bind(
                visitor_id,
                session_id,
                pv.page || '/',
                pv.referrer || null,
                referrerDomain,
                pv.utm_source || null,
                pv.utm_medium || null,
                pv.utm_campaign || null,
                pv.utm_content || null,
                pv.utm_term || null,
                pv.promo_code || null,
                device,
                browser,
                os,
                country,
                region,
                city
            ).run();

            // Record funnel event for first visit from this visitor
            // INSERT OR IGNORE ensures we only count first visit
            await env.DB.prepare(`
                INSERT OR IGNORE INTO funnel_events (
                    visitor_id, stage, utm_source, utm_medium, utm_campaign,
                    promo_code, referrer_domain
                )
                SELECT ?, 'visit', ?, ?, ?, ?, ?
                WHERE NOT EXISTS (
                    SELECT 1 FROM funnel_events
                    WHERE visitor_id = ? AND stage = 'visit'
                )
            `).bind(
                visitor_id,
                pv.utm_source || null,
                pv.utm_medium || null,
                pv.utm_campaign || null,
                pv.promo_code || null,
                referrerDomain,
                visitor_id
            ).run();
        } catch {
            // Silently ignore failures
        }
    }

    return new Response('', { status: 204 });
}

/**
 * POST /api/analytics/event - Record click/interaction event
 *
 * Also handles email signup events specially by storing in email_signups table
 */
export async function handleAnalyticsEvent(
    request: Request,
    env: Env
): Promise<Response> {
    let body: EventRequest;
    try {
        body = await request.json();
    } catch {
        return new Response('', { status: 204 });
    }

    const { visitor_id, session_id, event_type } = body;

    if (!visitor_id || !session_id || !event_type) {
        return new Response('', { status: 204 });
    }

    try {
        // Insert analytics event
        await env.DB.prepare(`
            INSERT INTO analytics_events (
                visitor_id, session_id, event_type, event_target, event_value, page
            ) VALUES (?, ?, ?, ?, ?, ?)
        `).bind(
            visitor_id,
            session_id,
            event_type,
            body.event_target || null,
            body.event_value || null,
            body.page || '/'
        ).run();

        // Handle email signup specially
        if (event_type === 'signup' && body.event_value) {
            const email = body.event_value.trim().toLowerCase();
            const referrerDomain = extractDomain(body.referrer);

            await env.DB.prepare(`
                INSERT OR IGNORE INTO email_signups (
                    email, visitor_id, utm_source, utm_medium, utm_campaign,
                    promo_code, referrer_domain
                ) VALUES (?, ?, ?, ?, ?, ?, ?)
            `).bind(
                email,
                visitor_id,
                body.utm_source || null,
                body.utm_medium || null,
                body.utm_campaign || null,
                body.promo_code || null,
                referrerDomain
            ).run();
        }
    } catch {
        // Silently ignore failures
    }

    return new Response('', { status: 204 });
}
