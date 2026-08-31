import type { Env } from '../types/env';
import { getAppConfig } from '../config';

/**
 * NPM API response for download stats
 */
interface NpmDownloadsResponse {
    downloads: Array<{
        day: string;      // YYYY-MM-DD
        downloads: number;
    }>;
    start: string;
    end: string;
    package: string;
}

/**
 * Fetch and store NPM download stats
 * Called by daily cron trigger at 6 AM UTC
 */
export async function fetchNpmStats(env: Env): Promise<void> {
    try {
        // Fetch last 30 days of downloads
        const response = await fetch(
            'https://api.npmjs.org/downloads/range/last-month/crabigator',
            {
                headers: {
                    'User-Agent': 'crabigator-api/1.0'
                }
            }
        );

        if (!response.ok) {
            console.error('NPM API error:', response.status, await response.text());
            return;
        }

        const data: NpmDownloadsResponse = await response.json();

        // Insert/update each day's stats (upsert using INSERT OR REPLACE)
        for (const day of data.downloads) {
            await env.DB.prepare(`
                INSERT OR REPLACE INTO npm_downloads (date, downloads)
                VALUES (?, ?)
            `).bind(day.day, day.downloads).run();
        }

        console.log(`Updated NPM stats for ${data.downloads.length} days`);
    } catch (error) {
        console.error('Error fetching NPM stats:', error);
    }
}

/**
 * Check for traffic anomalies and send email alert
 * Called by daily cron trigger at 6 AM UTC
 *
 * Alert thresholds:
 * - 3x spike: Yesterday's visitors >= 3x the 7-day average
 * - 75% drop: Yesterday's visitors <= 25% of the 7-day average
 */
export async function checkTrafficAnomalies(env: Env): Promise<void> {
    const now = Math.floor(Date.now() / 1000);
    const oneDayAgo = now - 86400;
    const twoDaysAgo = now - 2 * 86400;
    const eightDaysAgo = now - 8 * 86400;

    try {
        // Get yesterday's unique visitors
        const yesterdayResult = await env.DB.prepare(`
            SELECT COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at >= ? AND created_at < ?
        `).bind(twoDaysAgo, oneDayAgo).first<{ count: number }>();

        const yesterdayVisitors = yesterdayResult?.count || 0;

        // Get 7-day average (excluding yesterday)
        const weekResult = await env.DB.prepare(`
            SELECT COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at >= ? AND created_at < ?
        `).bind(eightDaysAgo, twoDaysAgo).first<{ count: number }>();

        const weekVisitors = weekResult?.count || 0;
        const dailyAverage = weekVisitors / 7;

        // Check if we have enough data to make meaningful comparisons
        if (dailyAverage < 1) {
            console.log('Not enough baseline data for traffic anomaly detection');
            return;
        }

        // Calculate ratio
        const ratio = yesterdayVisitors / dailyAverage;
        const percentChange = Math.round((ratio - 1) * 100);

        // Check thresholds
        const isSpike = ratio >= 3.0;
        const isDrop = ratio <= 0.25;

        if (!isSpike && !isDrop) {
            console.log(`Traffic normal: ${yesterdayVisitors} visitors (${percentChange > 0 ? '+' : ''}${percentChange}% vs avg)`);
            return;
        }

        // Get top traffic sources for context
        const sourcesResult = await env.DB.prepare(`
            SELECT
                CASE
                    WHEN referrer_domain IS NULL THEN 'Direct'
                    WHEN referrer_domain LIKE '%google%' OR referrer_domain LIKE '%bing%' THEN 'Search'
                    WHEN referrer_domain LIKE '%twitter%' OR referrer_domain LIKE '%x.com%'
                         OR referrer_domain LIKE '%reddit%' OR referrer_domain LIKE '%linkedin%' THEN 'Social'
                    WHEN referrer_domain LIKE '%openai%' OR referrer_domain LIKE '%claude.ai%'
                         OR referrer_domain LIKE '%perplexity%' THEN 'AI Assistant'
                    ELSE referrer_domain
                END as source,
                COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at >= ? AND created_at < ?
            GROUP BY source
            ORDER BY count DESC
            LIMIT 5
        `).bind(twoDaysAgo, oneDayAgo).all<{ source: string; count: number }>();

        const topSources = sourcesResult.results || [];

        // Get top campaigns if any
        const campaignsResult = await env.DB.prepare(`
            SELECT utm_campaign, COUNT(DISTINCT visitor_id) as count
            FROM page_views
            WHERE created_at >= ? AND created_at < ?
                AND utm_campaign IS NOT NULL
            GROUP BY utm_campaign
            ORDER BY count DESC
            LIMIT 3
        `).bind(twoDaysAgo, oneDayAgo).all<{ utm_campaign: string; count: number }>();

        const topCampaigns = campaignsResult.results || [];

        // Send alert email
        await sendTrafficAlert(env, {
            type: isSpike ? 'spike' : 'drop',
            yesterdayVisitors,
            dailyAverage: Math.round(dailyAverage),
            percentChange,
            topSources,
            topCampaigns
        });

    } catch (error) {
        console.error('Error checking traffic anomalies:', error);
    }
}

interface TrafficAlertData {
    type: 'spike' | 'drop';
    yesterdayVisitors: number;
    dailyAverage: number;
    percentChange: number;
    topSources: Array<{ source: string; count: number }>;
    topCampaigns: Array<{ utm_campaign: string; count: number }>;
}

async function sendTrafficAlert(env: Env, data: TrafficAlertData): Promise<void> {
    const config = getAppConfig(env);
    const origin = config.public_origin;
    const mailgunDomain = config.email?.mailgun_domain;
    const mailgunFrom = config.email?.from;
    const recipient = config.email?.traffic_alert_recipient;
    if (!env.MAILGUN_API_KEY || !mailgunDomain || !mailgunFrom || !recipient || !origin) {
        console.error('Mailgun not configured for traffic alerts');
        return;
    }

    const emoji = data.type === 'spike' ? '📈' : '📉';
    const trend = data.type === 'spike' ? 'spike' : 'drop';
    const color = data.type === 'spike' ? '#3fb950' : '#f85149';

    const sourcesHtml = data.topSources.length > 0
        ? data.topSources.map(s => `<li><strong>${s.source}:</strong> ${s.count} visitors</li>`).join('')
        : '<li>No data</li>';

    const campaignsHtml = data.topCampaigns.length > 0
        ? data.topCampaigns.map(c => `<li><strong>${c.utm_campaign}:</strong> ${c.count} visitors</li>`).join('')
        : '<li>No campaigns tracked</li>';

    const emailHtml = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Traffic ${data.type === 'spike' ? 'Spike' : 'Drop'} Alert</title>
</head>
<body style="margin: 0; padding: 0; background-color: #0d1117; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;">
    <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background-color: #0d1117;">
        <tr>
            <td align="center" style="padding: 40px 20px;">
                <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="max-width: 560px;">
                    <tr>
                        <td style="background: #161b22; border-radius: 12px; border: 1px solid #30363d; padding: 32px;">

                            <h1 style="margin: 0 0 24px 0; font-size: 24px; color: #e6edf3;">
                                ${emoji} Traffic ${data.type === 'spike' ? 'Spike' : 'Drop'} Detected
                            </h1>

                            <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="margin-bottom: 24px;">
                                <tr>
                                    <td style="padding: 16px; background: #21262d; border-radius: 8px; text-align: center;">
                                        <div style="font-size: 36px; font-weight: 700; color: ${color};">
                                            ${data.yesterdayVisitors}
                                        </div>
                                        <div style="font-size: 14px; color: #8b949e; margin-top: 4px;">
                                            visitors yesterday
                                        </div>
                                    </td>
                                    <td width="16"></td>
                                    <td style="padding: 16px; background: #21262d; border-radius: 8px; text-align: center;">
                                        <div style="font-size: 36px; font-weight: 700; color: #8b949e;">
                                            ${data.dailyAverage}
                                        </div>
                                        <div style="font-size: 14px; color: #8b949e; margin-top: 4px;">
                                            7-day average
                                        </div>
                                    </td>
                                    <td width="16"></td>
                                    <td style="padding: 16px; background: #21262d; border-radius: 8px; text-align: center;">
                                        <div style="font-size: 36px; font-weight: 700; color: ${color};">
                                            ${data.percentChange > 0 ? '+' : ''}${data.percentChange}%
                                        </div>
                                        <div style="font-size: 14px; color: #8b949e; margin-top: 4px;">
                                            change
                                        </div>
                                    </td>
                                </tr>
                            </table>

                            <h2 style="margin: 0 0 12px 0; font-size: 16px; color: #e6edf3;">Top Traffic Sources</h2>
                            <ul style="margin: 0 0 24px 0; padding-left: 20px; color: #8b949e; line-height: 1.8;">
                                ${sourcesHtml}
                            </ul>

                            <h2 style="margin: 0 0 12px 0; font-size: 16px; color: #e6edf3;">Top Campaigns</h2>
                            <ul style="margin: 0 0 24px 0; padding-left: 20px; color: #8b949e; line-height: 1.8;">
                                ${campaignsHtml}
                            </ul>

                            <a href="${origin}/staff" style="display: inline-block; padding: 12px 24px; background: #58a6ff; color: #fff; text-decoration: none; border-radius: 8px; font-weight: 600;">
                                View Staff Dashboard
                            </a>

                        </td>
                    </tr>
                </table>
            </td>
        </tr>
    </table>
</body>
</html>
    `.trim();

    const formData = new FormData();
    formData.append('from', mailgunFrom);
    formData.append('to', recipient);
    formData.append('subject', `${emoji} Crabigator Traffic ${trend}: ${data.percentChange > 0 ? '+' : ''}${data.percentChange}%`);
    formData.append('html', emailHtml);

    try {
        const response = await fetch(
            `https://api.mailgun.net/v3/${mailgunDomain}/messages`,
            {
                method: 'POST',
                headers: {
                    'Authorization': 'Basic ' + btoa('api:' + env.MAILGUN_API_KEY)
                },
                body: formData
            }
        );

        if (!response.ok) {
            console.error('Mailgun error:', await response.text());
        } else {
            console.log(`Traffic ${trend} alert sent: ${data.yesterdayVisitors} visitors (${data.percentChange > 0 ? '+' : ''}${data.percentChange}%)`);
        }
    } catch (error) {
        console.error('Error sending traffic alert:', error);
    }
}
