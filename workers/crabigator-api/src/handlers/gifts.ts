import type { Env } from '../types/env';
import { jsonResponse } from '../router';

/**
 * Create an error response
 */
function errorResponse(error: string, code: string, status: number): Response {
    return new Response(
        JSON.stringify({ error, code }),
        { status, headers: { 'Content-Type': 'application/json' } }
    );
}

/**
 * Duration types and their corresponding seconds
 */
const DURATION_SECONDS: Record<string, number | null> = {
    day: 86400,
    week: 604800,
    month: 2592000,      // 30 days
    year: 31536000,      // 365 days
    forever: null        // NULL = no expiry
};

/**
 * Generate a random 8-character gift code
 * Uses characters that are unambiguous (no I/O/0/1)
 */
function generateGiftCode(): string {
    const chars = 'ABCDEFGHJKLMNPQRSTUVWXYZ23456789';
    const array = new Uint8Array(8);
    crypto.getRandomValues(array);
    let code = '';
    for (let i = 0; i < 8; i++) {
        code += chars[array[i] % chars.length];
    }
    return code;
}

/**
 * Generate a UUID v4
 */
function generateUUID(): string {
    return crypto.randomUUID();
}

/**
 * Format seconds into human-readable duration
 */
function formatDuration(seconds: number): string {
    if (seconds < 60) return `${seconds}s`;
    if (seconds < 3600) return `${Math.floor(seconds / 60)}m`;
    if (seconds < 86400) return `${Math.floor(seconds / 3600)}h ${Math.floor((seconds % 3600) / 60)}m`;
    const days = Math.floor(seconds / 86400);
    const hours = Math.floor((seconds % 86400) / 3600);
    if (days < 7) return hours > 0 ? `${days}d ${hours}h` : `${days}d`;
    return `${days}d`;
}

// ============================================
// Types
// ============================================

interface CreateGiftRequest {
    duration_type: string;
    recipient_email?: string;
}

interface Gift {
    id: string;
    duration_type: string;
    duration_seconds: number | null;
    created_at: number;
    recipient_email: string | null;
    email_sent_at: number | null;
    claimed_at: number | null;
    claimed_by_group_id: string | null;
    claimed_by_cookie_id: string | null;
    subscription_id: string | null;
}

interface GiftWithStats extends Gift {
    session_count: number;
    total_duration_seconds: number;
    avg_duration_seconds: number | null;
}

interface ClaimGiftRequest {
    cookie_id?: string;
}

// ============================================
// Staff Endpoints
// ============================================

/**
 * POST /api/staff/gifts - Create a new gift code
 */
export async function handleCreateGift(
    request: Request,
    env: Env
): Promise<Response> {
    let body: CreateGiftRequest;
    try {
        body = await request.json();
    } catch {
        return errorResponse('Invalid JSON', 'INVALID_JSON', 400);
    }

    const { duration_type, recipient_email } = body;

    if (!duration_type || !(duration_type in DURATION_SECONDS)) {
        return errorResponse(
            'Invalid duration_type. Must be: day, week, month, year, or forever',
            'INVALID_DURATION',
            400
        );
    }

    // Generate unique gift code (retry on collision)
    let giftCode: string;
    let attempts = 0;
    while (attempts < 5) {
        giftCode = generateGiftCode();
        const existing = await env.DB.prepare(
            'SELECT id FROM gifts WHERE id = ?'
        ).bind(giftCode).first();
        if (!existing) break;
        attempts++;
    }

    if (attempts >= 5) {
        return errorResponse('Failed to generate unique gift code', 'CODE_GENERATION_FAILED', 500);
    }

    const durationSeconds = DURATION_SECONDS[duration_type];
    const now = Math.floor(Date.now() / 1000);

    await env.DB.prepare(`
        INSERT INTO gifts (id, duration_type, duration_seconds, created_at, recipient_email)
        VALUES (?, ?, ?, ?, ?)
    `).bind(
        giftCode!,
        duration_type,
        durationSeconds,
        now,
        recipient_email || null
    ).run();

    const giftUrl = `https://drinkcrabigator.com/dashboard?gift=${giftCode!}`;

    return jsonResponse({
        id: giftCode!,
        duration_type,
        duration_seconds: durationSeconds,
        url: giftUrl,
        recipient_email: recipient_email || null,
        created_at: now
    });
}

/**
 * GET /api/staff/gifts - List all gifts with usage stats
 */
export async function handleListGifts(
    _request: Request,
    env: Env
): Promise<Response> {
    // Query gifts with usage stats
    const result = await env.DB.prepare(`
        SELECT
            g.id,
            g.duration_type,
            g.duration_seconds,
            g.created_at,
            g.recipient_email,
            g.email_sent_at,
            g.claimed_at,
            g.claimed_by_group_id,
            g.claimed_by_cookie_id,
            g.subscription_id,
            COUNT(DISTINCT s.id) as session_count,
            COALESCE(SUM(CASE
                WHEN s.ended_at IS NOT NULL THEN s.ended_at - s.started_at
                ELSE 0
            END), 0) as total_duration_seconds,
            AVG(CASE
                WHEN s.ended_at IS NOT NULL THEN s.ended_at - s.started_at
                ELSE NULL
            END) as avg_duration_seconds
        FROM gifts g
        LEFT JOIN subscriptions sub ON g.subscription_id = sub.id
        LEFT JOIN devices d ON d.group_id = sub.group_id
        LEFT JOIN sessions s ON s.device_id = d.id
        GROUP BY g.id
        ORDER BY g.created_at DESC
    `).all<GiftWithStats>();

    const gifts = (result.results || []).map(gift => ({
        ...gift,
        url: `https://drinkcrabigator.com/dashboard?gift=${gift.id}`,
        total_duration_formatted: formatDuration(gift.total_duration_seconds || 0),
        avg_duration_formatted: gift.avg_duration_seconds
            ? formatDuration(Math.round(gift.avg_duration_seconds))
            : null,
        status: gift.claimed_by_group_id
            ? 'claimed'
            : gift.claimed_by_cookie_id
                ? 'pending'
                : 'unclaimed'
    }));

    return jsonResponse({ gifts });
}

/**
 * POST /api/staff/gifts/:id/send-email - Send gift email via Mailgun
 */
export async function handleSendGiftEmail(
    _request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const giftId = params.id;

    // Get the gift
    const gift = await env.DB.prepare(
        'SELECT * FROM gifts WHERE id = ?'
    ).bind(giftId).first<Gift>();

    if (!gift) {
        return errorResponse('Gift not found', 'NOT_FOUND', 404);
    }

    if (!gift.recipient_email) {
        return errorResponse('Gift has no recipient email', 'NO_EMAIL', 400);
    }

    if (!env.MAILGUN_API_KEY || !env.MAILGUN_DOMAIN) {
        return errorResponse('Mailgun not configured', 'MAILGUN_NOT_CONFIGURED', 500);
    }

    const giftUrl = `https://drinkcrabigator.com/dashboard?gift=${gift.id}`;
    const durationText = gift.duration_type === 'forever'
        ? 'lifetime access'
        : gift.duration_type === 'year'
            ? 'a year of access'
            : gift.duration_type === 'month'
                ? 'a month of access'
                : gift.duration_type === 'week'
                    ? 'a week of access'
                    : 'a day of access';
    const ctaText = gift.duration_type === 'forever'
        ? 'Claim Your Free Lifetime Access'
        : gift.duration_type === 'year'
            ? 'Claim Your Free Year'
            : gift.duration_type === 'month'
                ? 'Claim Your Free Month'
                : gift.duration_type === 'week'
                    ? 'Claim Your Free Week'
                    : 'Claim Your Free Day';
    const subjectDuration = gift.duration_type === 'forever'
        ? 'lifetime access to'
        : gift.duration_type === 'year'
            ? 'a year of'
            : gift.duration_type === 'month'
                ? 'a month of'
                : gift.duration_type === 'week'
                    ? 'a week of'
                    : 'a day of';

    // Build email HTML - using tables for email compatibility
    const emailHtml = `
<!DOCTYPE html>
<html>
<head>
    <meta charset="utf-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>You've been gifted Crabigator Pro!</title>
</head>
<body style="margin: 0; padding: 0; background-color: #0d1117; font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, Helvetica, Arial, sans-serif;">
    <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background-color: #0d1117;">
        <tr>
            <td align="center" style="padding: 40px 20px;">

                <!-- Main Container -->
                <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="max-width: 560px;">

                    <!-- Hero Card -->
                    <tr>
                        <td style="background: linear-gradient(180deg, #1c2128 0%, #161b22 100%); border-radius: 16px; border: 1px solid #30363d; padding: 48px 40px; text-align: center;">

                            <!-- Logo -->
                            <img src="https://drinkcrabigator.com/assets/logo.svg" alt="Crabigator" width="80" height="80" style="display: block; margin: 0 auto 24px auto;">

                            <!-- Gift Badge -->
                            <table role="presentation" cellspacing="0" cellpadding="0" style="margin: 0 auto 20px auto;">
                                <tr>
                                    <td style="background: linear-gradient(135deg, rgba(88, 166, 255, 0.15) 0%, rgba(163, 113, 247, 0.15) 100%); border: 1px solid rgba(88, 166, 255, 0.3); border-radius: 20px; padding: 6px 16px;">
                                        <span style="color: #58a6ff; font-size: 13px; font-weight: 500; letter-spacing: 0.5px;">GIFT SUBSCRIPTION</span>
                                    </td>
                                </tr>
                            </table>

                            <!-- Headline -->
                            <h1 style="margin: 0 0 12px 0; font-size: 28px; font-weight: 700; color: #e6edf3; line-height: 1.3;">
                                You've been gifted<br>Crabigator Pro!
                            </h1>

                            <!-- Subheadline -->
                            <p style="margin: 0 0 32px 0; font-size: 16px; color: #8b949e; line-height: 1.5;">
                                Please enjoy <strong style="color: #a371f7;">${durationText}</strong><br>to control Claude Code from anywhere.
                            </p>

                            <!-- CTA Button -->
                            <table role="presentation" cellspacing="0" cellpadding="0" style="margin: 0 auto;">
                                <tr>
                                    <td style="border-radius: 10px; background: linear-gradient(135deg, #58a6ff 0%, #a371f7 100%);">
                                        <a href="${giftUrl}" style="display: inline-block; padding: 16px 40px; font-size: 16px; font-weight: 600; color: #ffffff; text-decoration: none;">
                                            ${ctaText}
                                        </a>
                                    </td>
                                </tr>
                            </table>

                            <!-- Link below CTA -->
                            <p style="margin: 24px 0 0 0; font-size: 13px; color: #484f58;">
                                Or copy this link:
                            </p>
                            <p style="margin: 4px 0 0 0; font-size: 13px;">
                                <a href="${giftUrl}" style="color: #58a6ff; text-decoration: none; word-break: break-all;">${giftUrl}</a>
                            </p>

                        </td>
                    </tr>

                    <!-- Spacer -->
                    <tr><td style="height: 24px;"></td></tr>

                    <!-- Personal Note -->
                    <tr>
                        <td style="padding: 0 8px;">
                            <table role="presentation" width="100%" cellspacing="0" cellpadding="0" style="background: linear-gradient(135deg, rgba(88, 166, 255, 0.05) 0%, rgba(163, 113, 247, 0.05) 100%); border-radius: 12px; border: 1px solid #21262d;">
                                <tr>
                                    <td style="padding: 24px 28px;">
                                        <p style="margin: 0 0 12px 0; font-size: 14px; color: #8b949e; line-height: 1.6; font-style: italic;">
                                            "I built Crabigator because I kept missing Claude's permission prompts while grabbing coffee. Now I can approve them from my phone and keep the momentum going. Hope you find it as useful as I do!"
                                        </p>
                                        <p style="margin: 0; font-size: 14px;">
                                            <span style="color: #e6edf3; font-weight: 500;">&mdash;</span>
                                            <a href="https://x.com/samuelclay" style="color: #58a6ff; text-decoration: none; font-weight: 500;">Samuel Clay</a>
                                            <span style="color: #484f58; font-size: 13px;">&nbsp;&nbsp;Creator of Crabigator</span>
                                        </p>
                                    </td>
                                </tr>
                            </table>
                        </td>
                    </tr>

                    <!-- Spacer -->
                    <tr><td style="height: 24px;"></td></tr>

                    <!-- What is Crabigator Section -->
                    <tr>
                        <td style="background: #161b22; border-radius: 12px; border: 1px solid #30363d; padding: 32px;">

                            <h2 style="margin: 0 0 16px 0; font-size: 18px; font-weight: 600; color: #e6edf3;">
                                What is Crabigator?
                            </h2>

                            <p style="margin: 0 0 24px 0; font-size: 15px; color: #8b949e; line-height: 1.6;">
                                Crabigator lets you control Claude Code sessions from your phone or any browser. Answer permissions, approve plans, and respond to questions&mdash;all without being at your computer.
                            </p>

                            <!-- Features Grid -->
                            <table role="presentation" width="100%" cellspacing="0" cellpadding="0">
                                <!-- Feature 1 -->
                                <tr>
                                    <td style="padding: 12px 0; border-top: 1px solid #21262d;">
                                        <table role="presentation" cellspacing="0" cellpadding="0">
                                            <tr>
                                                <td width="44" valign="top">
                                                    <div style="width: 36px; height: 36px; background: rgba(88, 166, 255, 0.1); border-radius: 8px; text-align: center; line-height: 36px;">
                                                        <img src="https://drinkcrabigator.com/assets/icon-phone.svg" alt="" width="18" height="18" style="vertical-align: middle;">
                                                    </div>
                                                </td>
                                                <td>
                                                    <div style="font-size: 14px; font-weight: 600; color: #e6edf3; margin-bottom: 2px;">Remote Control</div>
                                                    <div style="font-size: 13px; color: #8b949e;">Approve permissions and answer questions from your phone</div>
                                                </td>
                                            </tr>
                                        </table>
                                    </td>
                                </tr>
                                <!-- Feature 2 -->
                                <tr>
                                    <td style="padding: 12px 0; border-top: 1px solid #21262d;">
                                        <table role="presentation" cellspacing="0" cellpadding="0">
                                            <tr>
                                                <td width="44" valign="top">
                                                    <div style="width: 36px; height: 36px; background: rgba(163, 113, 247, 0.1); border-radius: 8px; text-align: center; line-height: 36px;">
                                                        <img src="https://drinkcrabigator.com/assets/icon-cloud.svg" alt="" width="18" height="18" style="vertical-align: middle;">
                                                    </div>
                                                </td>
                                                <td>
                                                    <div style="font-size: 14px; font-weight: 600; color: #e6edf3; margin-bottom: 2px;">Real-time Streaming</div>
                                                    <div style="font-size: 13px; color: #8b949e;">Watch Claude work live with session stats and git changes</div>
                                                </td>
                                            </tr>
                                        </table>
                                    </td>
                                </tr>
                                <!-- Feature 3 -->
                                <tr>
                                    <td style="padding: 12px 0; border-top: 1px solid #21262d;">
                                        <table role="presentation" cellspacing="0" cellpadding="0">
                                            <tr>
                                                <td width="44" valign="top">
                                                    <div style="width: 36px; height: 36px; background: rgba(63, 185, 80, 0.1); border-radius: 8px; text-align: center; line-height: 36px;">
                                                        <img src="https://drinkcrabigator.com/assets/icon-bolt.svg" alt="" width="18" height="18" style="vertical-align: middle;">
                                                    </div>
                                                </td>
                                                <td>
                                                    <div style="font-size: 14px; font-weight: 600; color: #e6edf3; margin-bottom: 2px;">Instant Setup</div>
                                                    <div style="font-size: 13px; color: #8b949e;">Install via npm, pair in seconds, works with any terminal</div>
                                                </td>
                                            </tr>
                                        </table>
                                    </td>
                                </tr>
                            </table>

                        </td>
                    </tr>

                    <!-- Spacer -->
                    <tr><td style="height: 24px;"></td></tr>

                    <!-- Footer -->
                    <tr>
                        <td style="text-align: center; padding: 20px; border-top: 1px solid #21262d;">
                            <p style="margin: 0 0 4px 0; font-size: 12px; color: #484f58;">
                                <a href="https://drinkcrabigator.com" style="color: #58a6ff; text-decoration: none;">drinkcrabigator.com</a>
                            </p>
                            <p style="margin: 0; font-size: 11px; color: #3d4450;">
                                Built in San Francisco
                            </p>
                        </td>
                    </tr>

                </table>

            </td>
        </tr>
    </table>
</body>
</html>
    `.trim();

    // Send via Mailgun
    const formData = new FormData();
    formData.append('from', env.MAILGUN_FROM || `Crabigator <noreply@${env.MAILGUN_DOMAIN}>`);
    formData.append('to', gift.recipient_email);
    formData.append('subject', `You've been gifted ${subjectDuration} Crabigator Pro!`);
    formData.append('html', emailHtml);

    const mailgunResponse = await fetch(
        `https://api.mailgun.net/v3/${env.MAILGUN_DOMAIN}/messages`,
        {
            method: 'POST',
            headers: {
                'Authorization': 'Basic ' + btoa('api:' + env.MAILGUN_API_KEY)
            },
            body: formData
        }
    );

    if (!mailgunResponse.ok) {
        const error = await mailgunResponse.text();
        console.error('Mailgun error:', error);
        return errorResponse('Failed to send email', 'EMAIL_FAILED', 500);
    }

    // Update gift with email_sent_at
    const now = Math.floor(Date.now() / 1000);
    await env.DB.prepare(
        'UPDATE gifts SET email_sent_at = ? WHERE id = ?'
    ).bind(now, giftId).run();

    return jsonResponse({ ok: true, sent_to: gift.recipient_email });
}

// ============================================
// Public Endpoints
// ============================================

/**
 * GET /api/gifts/:code - Get gift info (for validation before claiming)
 */
export async function handleGetGift(
    _request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const code = params.code.toUpperCase();

    const gift = await env.DB.prepare(
        'SELECT id, duration_type, duration_seconds, claimed_at, claimed_by_group_id FROM gifts WHERE id = ?'
    ).bind(code).first<Gift>();

    if (!gift) {
        return errorResponse('Gift not found', 'NOT_FOUND', 404);
    }

    const isClaimable = !gift.claimed_at && !gift.claimed_by_group_id;
    const durationText = gift.duration_type === 'forever'
        ? 'Forever'
        : gift.duration_type.charAt(0).toUpperCase() + gift.duration_type.slice(1);

    return jsonResponse({
        id: gift.id,
        duration_type: gift.duration_type,
        duration_text: durationText,
        is_claimable: isClaimable,
        already_claimed: !isClaimable
    });
}

/**
 * POST /api/gifts/:code/claim - Claim a gift
 *
 * If user has mobile auth (group_id), creates subscription immediately.
 * If no auth, stores pending claim with cookie_id for later resolution.
 */
export async function handleClaimGift(
    request: Request,
    env: Env,
    params: Record<string, string>,
    auth?: { group_id?: string }
): Promise<Response> {
    const code = params.code.toUpperCase();

    // Get the gift
    const gift = await env.DB.prepare(
        'SELECT * FROM gifts WHERE id = ?'
    ).bind(code).first<Gift>();

    if (!gift) {
        return errorResponse('Gift not found', 'NOT_FOUND', 404);
    }

    // Check if already claimed
    if (gift.claimed_at || gift.claimed_by_group_id) {
        return errorResponse('Gift has already been claimed', 'ALREADY_CLAIMED', 400);
    }

    const now = Math.floor(Date.now() / 1000);

    // If user has group_id (is paired), create subscription immediately
    if (auth?.group_id) {
        // Check if group already has an active subscription
        const existingSub = await env.DB.prepare(`
            SELECT id FROM subscriptions
            WHERE group_id = ? AND status IN ('active', 'past_due')
        `).bind(auth.group_id).first();

        if (existingSub) {
            // They already have pro - still mark gift as claimed but don't create new sub
            await env.DB.prepare(`
                UPDATE gifts SET claimed_at = ?, claimed_by_group_id = ? WHERE id = ?
            `).bind(now, auth.group_id, code).run();

            return jsonResponse({
                ok: true,
                status: 'already_pro',
                message: 'Gift claimed! You already have an active subscription.'
            });
        }

        // Create the subscription
        const subscriptionId = generateUUID();
        const periodEnd = gift.duration_seconds
            ? now + gift.duration_seconds
            : null; // NULL for forever

        await env.DB.prepare(`
            INSERT INTO subscriptions (
                id, group_id, provider, provider_subscription_id, status,
                current_period_start, current_period_end, created_at, updated_at
            ) VALUES (?, ?, 'gift', ?, 'active', ?, ?, ?, ?)
        `).bind(
            subscriptionId,
            auth.group_id,
            code,  // Use gift code as provider_subscription_id
            now,
            periodEnd,
            now,
            now
        ).run();

        // Update gift as claimed
        await env.DB.prepare(`
            UPDATE gifts SET claimed_at = ?, claimed_by_group_id = ?, subscription_id = ?
            WHERE id = ?
        `).bind(now, auth.group_id, subscriptionId, code).run();

        // Sync the UsageDO to reflect new pro status
        try {
            const doId = env.USAGE.idFromName(auth.group_id);
            const stub = env.USAGE.get(doId);
            await stub.fetch(new Request(`https://internal/sync?group_id=${auth.group_id}`));
        } catch {
            // Best effort
        }

        const durationText = gift.duration_type === 'forever'
            ? 'lifetime'
            : `${gift.duration_type}`;

        return jsonResponse({
            ok: true,
            status: 'activated',
            message: `Gift claimed! You now have ${durationText} of Crabigator Pro.`,
            subscription_id: subscriptionId,
            expires_at: periodEnd
        });
    }

    // No group_id - create pending claim with cookie
    let body: ClaimGiftRequest = {};
    try {
        body = await request.json();
    } catch {
        // OK if no body
    }

    let cookieId = body.cookie_id;
    if (!cookieId) {
        cookieId = generateUUID();
    }

    // Check if this cookie already has a pending claim
    const existingPending = await env.DB.prepare(
        'SELECT gift_id FROM pending_gift_claims WHERE cookie_id = ?'
    ).bind(cookieId).first();

    if (existingPending) {
        return errorResponse('You already have a pending gift claim', 'ALREADY_PENDING', 400);
    }

    // Create pending claim
    await env.DB.prepare(`
        INSERT INTO pending_gift_claims (cookie_id, gift_id, claimed_at)
        VALUES (?, ?, ?)
    `).bind(cookieId, code, now).run();

    // Update gift with cookie_id
    await env.DB.prepare(`
        UPDATE gifts SET claimed_at = ?, claimed_by_cookie_id = ?
        WHERE id = ?
    `).bind(now, cookieId, code).run();

    const durationText = gift.duration_type === 'forever'
        ? 'lifetime'
        : `${gift.duration_type}`;

    return jsonResponse({
        ok: true,
        status: 'pending',
        message: `Gift claimed! Pair your device to activate ${durationText} of Crabigator Pro.`,
        cookie_id: cookieId
    });
}

/**
 * POST /api/gifts/resolve-pending - Resolve pending gift after pairing
 *
 * Called after user successfully pairs a device to check if they have
 * a pending gift claim that should now be activated.
 */
export async function handleResolvePendingGift(
    request: Request,
    env: Env,
    auth: { group_id: string }
): Promise<Response> {
    let body: { cookie_id: string };
    try {
        body = await request.json();
    } catch {
        return errorResponse('Invalid JSON', 'INVALID_JSON', 400);
    }

    const { cookie_id } = body;
    if (!cookie_id) {
        return errorResponse('Missing cookie_id', 'MISSING_COOKIE_ID', 400);
    }

    // Get pending claim
    const pending = await env.DB.prepare(
        'SELECT gift_id FROM pending_gift_claims WHERE cookie_id = ?'
    ).bind(cookie_id).first<{ gift_id: string }>();

    if (!pending) {
        return jsonResponse({ ok: true, resolved: false, message: 'No pending gift found' });
    }

    // Get the gift
    const gift = await env.DB.prepare(
        'SELECT * FROM gifts WHERE id = ?'
    ).bind(pending.gift_id).first<Gift>();

    if (!gift) {
        // Gift was deleted? Clean up pending
        await env.DB.prepare(
            'DELETE FROM pending_gift_claims WHERE cookie_id = ?'
        ).bind(cookie_id).run();
        return jsonResponse({ ok: true, resolved: false, message: 'Gift no longer exists' });
    }

    // Check if group already has an active subscription
    const existingSub = await env.DB.prepare(`
        SELECT id FROM subscriptions
        WHERE group_id = ? AND status IN ('active', 'past_due')
    `).bind(auth.group_id).first();

    const now = Math.floor(Date.now() / 1000);

    if (existingSub) {
        // They already have pro - update gift to show claimed but don't create sub
        await env.DB.prepare(`
            UPDATE gifts SET claimed_by_group_id = ?, claimed_by_cookie_id = NULL
            WHERE id = ?
        `).bind(auth.group_id, gift.id).run();

        await env.DB.prepare(
            'DELETE FROM pending_gift_claims WHERE cookie_id = ?'
        ).bind(cookie_id).run();

        return jsonResponse({
            ok: true,
            resolved: true,
            status: 'already_pro',
            message: 'Gift applied! You already have an active subscription.'
        });
    }

    // Create the subscription
    const subscriptionId = generateUUID();
    const periodEnd = gift.duration_seconds
        ? now + gift.duration_seconds
        : null;

    await env.DB.prepare(`
        INSERT INTO subscriptions (
            id, group_id, provider, provider_subscription_id, status,
            current_period_start, current_period_end, created_at, updated_at
        ) VALUES (?, ?, 'gift', ?, 'active', ?, ?, ?, ?)
    `).bind(
        subscriptionId,
        auth.group_id,
        gift.id,
        now,
        periodEnd,
        now,
        now
    ).run();

    // Update gift record
    await env.DB.prepare(`
        UPDATE gifts SET claimed_by_group_id = ?, claimed_by_cookie_id = NULL, subscription_id = ?
        WHERE id = ?
    `).bind(auth.group_id, subscriptionId, gift.id).run();

    // Delete pending claim
    await env.DB.prepare(
        'DELETE FROM pending_gift_claims WHERE cookie_id = ?'
    ).bind(cookie_id).run();

    // Sync the UsageDO
    try {
        const doId = env.USAGE.idFromName(auth.group_id);
        const stub = env.USAGE.get(doId);
        await stub.fetch(new Request(`https://internal/sync?group_id=${auth.group_id}`));
    } catch {
        // Best effort
    }

    const durationText = gift.duration_type === 'forever'
        ? 'lifetime'
        : `${gift.duration_type}`;

    return jsonResponse({
        ok: true,
        resolved: true,
        status: 'activated',
        message: `Gift activated! You now have ${durationText} of Crabigator Pro.`,
        subscription_id: subscriptionId,
        expires_at: periodEnd
    });
}
