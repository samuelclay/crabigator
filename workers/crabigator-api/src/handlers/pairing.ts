import type { Env } from '../types/env';
import type {
    GeneratePairingTokenResponse,
    ClaimPairingTokenRequest,
    ClaimPairingTokenResponse,
    PairingStatusResponse,
} from '../types/api';
import { jsonResponse } from '../router';
import { requireDeviceAuth, verifyMobileToken } from '../auth/middleware';
import { generateToken, generatePairingCode, generateUUID, sha256 } from '../auth/tokens';

const PAIRING_TOKEN_TTL = 5 * 60; // 5 minutes

interface PairingTokenData {
    device_id: string;
    code: string;
    expires_at: number;
    claimed: boolean;
    mobile_id?: string;
    mobile_name?: string;
}

/**
 * POST /api/pairing/generate - Generate a pairing token for mobile linking
 * Requires device auth
 */
export async function generatePairingToken(
    request: Request,
    env: Env
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;

    // Generate pairing token and code
    const token = generateToken(32);
    const code = generatePairingCode();
    const expiresAt = Math.floor(Date.now() / 1000) + PAIRING_TOKEN_TTL;

    // Store token data in KV
    const tokenData: PairingTokenData = {
        device_id,
        code,
        expires_at: expiresAt,
        claimed: false,
    };
    await env.TOKENS.put(
        `pairing:${token}`,
        JSON.stringify(tokenData),
        { expirationTtl: PAIRING_TOKEN_TTL }
    );

    // Also store code -> token mapping for web-based pairing
    await env.TOKENS.put(
        `pairing_code:${code}`,
        token,
        { expirationTtl: PAIRING_TOKEN_TTL }
    );

    // Generate QR data URL
    const qrData = `crabigator://pair?t=${token}&d=${device_id}`;

    const response: GeneratePairingTokenResponse = {
        token,
        expires_at: expiresAt,
        qr_data: qrData,
        code,
    };

    return jsonResponse(response);
}

/**
 * POST /api/pairing/claim - Mobile claims a pairing token
 * No auth required (this is how mobile devices get authenticated)
 */
export async function claimPairingToken(
    request: Request,
    env: Env
): Promise<Response> {
    let body: ClaimPairingTokenRequest;
    try {
        body = await request.json();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Support both token and code-based claims
    let token: string | null = body.pairing_token;

    // If it's a code format (AAA-BBB-CCC), look up the actual token
    if (token?.match(/^[A-Z0-9]{3}-[A-Z0-9]{3}-[A-Z0-9]{3}$/)) {
        const lookupToken = await env.TOKENS.get(`pairing_code:${token}`);
        if (lookupToken) {
            token = lookupToken;
        } else {
            token = null; // Code not found, will trigger error below
        }
    }

    if (!token) {
        return new Response(
            JSON.stringify({ error: 'Invalid or expired pairing code', code: 'INVALID_CODE' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Get token data from KV
    const tokenDataStr = await env.TOKENS.get(`pairing:${token}`);
    if (!tokenDataStr) {
        return new Response(
            JSON.stringify({ error: 'Invalid or expired pairing token', code: 'INVALID_TOKEN' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const tokenData: PairingTokenData = JSON.parse(tokenDataStr);

    if (tokenData.claimed) {
        return new Response(
            JSON.stringify({ error: 'Token already claimed', code: 'ALREADY_CLAIMED' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Generate mobile token
    const mobileToken = generateToken(32);
    const mobileTokenHash = await sha256(mobileToken);
    const mobileId = body.mobile_id || generateUUID();
    const mobileName = body.mobile_name || null;

    // Get or create device group for this desktop
    let groupId = await getOrCreateDeviceGroup(env, tokenData.device_id);

    // Store mobile token in KV (maps to group, not individual desktop)
    await env.TOKENS.put(
        `mobile:${mobileTokenHash}`,
        JSON.stringify({
            group_id: groupId,
            desktop_id: tokenData.device_id, // Keep for backwards compatibility
            mobile_id: mobileId,
        }),
        { expirationTtl: 60 * 60 * 24 * 365 } // 1 year
    );

    // Insert linked_devices record
    const linkId = generateUUID();
    const now = Math.floor(Date.now() / 1000);
    await env.DB.prepare(`
        INSERT INTO linked_devices (id, desktop_id, mobile_id, mobile_name, mobile_token_hash, paired_at)
        VALUES (?, ?, ?, ?, ?, ?)
        ON CONFLICT(desktop_id, mobile_id) DO UPDATE SET
            mobile_token_hash = excluded.mobile_token_hash,
            paired_at = excluded.paired_at,
            revoked_at = NULL
    `).bind(linkId, tokenData.device_id, mobileId, mobileName, mobileTokenHash, now).run();

    // Mark token as claimed (so desktop knows pairing succeeded)
    tokenData.claimed = true;
    tokenData.mobile_id = mobileId;
    tokenData.mobile_name = mobileName || undefined;
    await env.TOKENS.put(
        `pairing:${token}`,
        JSON.stringify(tokenData),
        { expirationTtl: 60 } // Keep for 1 minute so desktop can see it was claimed
    );

    // Get desktop name for response
    const device = await env.DB.prepare(
        'SELECT name FROM devices WHERE id = ?'
    ).bind(tokenData.device_id).first<{ name: string | null }>();

    const response: ClaimPairingTokenResponse = {
        mobile_token: mobileToken,
        desktop_name: device?.name || null,
    };

    return jsonResponse(response);
}

/**
 * GET /api/pairing/:token/status - Check if pairing token was claimed
 * Requires device auth (only the device that generated the token can check)
 */
export async function getPairingStatus(
    request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;

    const token = params.token;
    if (!token) {
        return new Response(
            JSON.stringify({ error: 'Token required', code: 'MISSING_TOKEN' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const tokenDataStr = await env.TOKENS.get(`pairing:${token}`);
    if (!tokenDataStr) {
        const response: PairingStatusResponse = {
            linked: [],
        };
        return jsonResponse({ paired: false, expired: true, ...response });
    }

    const tokenData: PairingTokenData = JSON.parse(tokenDataStr);

    // Verify this token belongs to the requesting device
    if (tokenData.device_id !== device_id) {
        return new Response(
            JSON.stringify({ error: 'Token not found', code: 'NOT_FOUND' }),
            { status: 404, headers: { 'Content-Type': 'application/json' } }
        );
    }

    if (tokenData.claimed) {
        // Pairing complete - return linked device info
        return jsonResponse({
            paired: true,
            expired: false,
            mobile_name: tokenData.mobile_name || null,
        });
    }

    // Not yet claimed
    return jsonResponse({
        paired: false,
        expired: false,
    });
}

/**
 * GET /pair/:token - Display pairing code page
 * No auth required - shows the human-readable code for the token
 */
export async function getPairingCodePage(
    _request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const token = params.token;
    if (!token) {
        return new Response('Token required', { status: 400 });
    }

    // Look up token data
    const tokenDataStr = await env.TOKENS.get(`pairing:${token}`);
    if (!tokenDataStr) {
        return new Response(pairingPageHtml('EXPIRED', true), {
            status: 200,
            headers: { 'Content-Type': 'text/html; charset=utf-8' }
        });
    }

    const tokenData: PairingTokenData = JSON.parse(tokenDataStr);

    if (tokenData.claimed) {
        return new Response(pairingPageHtml('CLAIMED', false, true), {
            status: 200,
            headers: { 'Content-Type': 'text/html; charset=utf-8' }
        });
    }

    return new Response(pairingPageHtml(tokenData.code, false, false), {
        status: 200,
        headers: { 'Content-Type': 'text/html; charset=utf-8' }
    });
}

/**
 * Generate HTML for the pairing code page
 */
function pairingPageHtml(code: string, expired: boolean, claimed: boolean = false): string {
    // Format code into segments for display
    const codeSegments = code.split('-');
    const formattedCode = codeSegments.map(seg =>
        `<span class="code-segment">${seg}</span>`
    ).join('<span class="code-dash">-</span>');

    return `<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Pair Device - Crabigator</title>
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 100 100'><text y='.9em' font-size='90'>🦀</text></svg>">
    <style>
        * { margin: 0; padding: 0; box-sizing: border-box; }

        body {
            font-family: -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            background: linear-gradient(145deg, #0a0a0a 0%, #1a1a2e 100%);
            color: #e5e5e5;
            min-height: 100vh;
            display: flex;
            flex-direction: column;
            align-items: center;
            justify-content: center;
            padding: 2rem;
        }

        .card {
            background: rgba(30, 30, 40, 0.8);
            border: 1px solid rgba(167, 139, 250, 0.2);
            border-radius: 1rem;
            padding: 2.5rem 3rem;
            text-align: center;
            max-width: 420px;
            width: 100%;
            box-shadow: 0 20px 40px rgba(0, 0, 0, 0.4);
        }

        .logo {
            font-size: 3rem;
            margin-bottom: 0.5rem;
        }

        h1 {
            font-size: 1.25rem;
            font-weight: 600;
            color: #a78bfa;
            margin-bottom: 0.5rem;
        }

        .subtitle {
            color: #71717a;
            font-size: 0.9rem;
            margin-bottom: 2rem;
        }

        .code-display {
            background: rgba(0, 0, 0, 0.3);
            border: 1px solid rgba(251, 191, 36, 0.3);
            border-radius: 0.75rem;
            padding: 1.25rem 1.5rem;
            margin: 1.5rem 0;
            display: flex;
            align-items: center;
            justify-content: center;
            gap: 0.25rem;
            white-space: nowrap;
        }

        .code-segment {
            font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
            font-size: 2rem;
            font-weight: 700;
            color: #fbbf24;
            letter-spacing: 0.15em;
        }

        .code-dash {
            font-family: 'SF Mono', 'Fira Code', 'Consolas', monospace;
            font-size: 2rem;
            font-weight: 400;
            color: #71717a;
            margin: 0 0.25rem;
        }

        .status-expired, .status-claimed {
            padding: 1.5rem;
            border-radius: 0.75rem;
            margin: 1.5rem 0;
        }

        .status-expired {
            background: rgba(248, 113, 113, 0.1);
            border: 1px solid rgba(248, 113, 113, 0.3);
        }

        .status-expired p:first-child {
            color: #f87171;
            font-weight: 500;
            margin-bottom: 0.5rem;
        }

        .status-expired p:last-child {
            color: #71717a;
            font-size: 0.9rem;
        }

        .status-claimed {
            background: rgba(74, 222, 128, 0.1);
            border: 1px solid rgba(74, 222, 128, 0.3);
        }

        .status-claimed p:first-child {
            color: #4ade80;
            font-weight: 500;
            margin-bottom: 0.5rem;
        }

        .status-claimed p:last-child {
            color: #71717a;
            font-size: 0.9rem;
        }

        .dashboard-link {
            display: inline-block;
            margin-top: 1.5rem;
            padding: 0.75rem 2rem;
            background: linear-gradient(135deg, #7c3aed 0%, #a78bfa 100%);
            color: white;
            text-decoration: none;
            border-radius: 0.5rem;
            font-weight: 500;
            font-size: 0.95rem;
            transition: transform 0.2s, box-shadow 0.2s;
        }

        .dashboard-link:hover {
            transform: translateY(-2px);
            box-shadow: 0 8px 20px rgba(124, 58, 237, 0.4);
        }

        .hint {
            margin-top: 2rem;
            padding-top: 1.5rem;
            border-top: 1px solid rgba(255, 255, 255, 0.1);
            color: #52525b;
            font-size: 0.8rem;
        }

        @media (max-width: 480px) {
            .card { padding: 2rem 1.5rem; }
            .code-segment { font-size: 1.5rem; }
            .code-dash { font-size: 1.5rem; }
        }
    </style>
</head>
<body>
    <div class="card">
        <div class="logo">🦀</div>
        <h1>Crabigator Pairing</h1>
        ${expired ? `
            <p class="subtitle">Link expired</p>
            <div class="status-expired">
                <p>This pairing link has expired</p>
                <p>Generate a new code from the desktop app</p>
            </div>
        ` : claimed ? `
            <p class="subtitle">Already paired</p>
            <div class="status-claimed">
                <p>This device has been paired!</p>
                <p>You can now view sessions on the dashboard</p>
            </div>
        ` : `
            <p class="subtitle">Enter this code on your other device</p>
            <div class="code-display">${formattedCode}</div>
            <p class="hint">Open drinkcrabigator.com/dashboard and enter this code</p>
        `}
        <a href="/dashboard" class="dashboard-link">Go to Dashboard</a>
    </div>
</body>
</html>`;
}

/**
 * POST /api/pairing/invite - Generate an invite code for pairing another device
 * Uses mobile_token auth (for dashboard use)
 * Returns a pairing code that another mobile device can use to join the same group
 */
export async function generateInviteCode(
    request: Request,
    env: Env
): Promise<Response> {
    // Verify mobile token
    const mobileAuth = await verifyMobileToken(request, env);
    if (!mobileAuth) {
        return new Response(
            JSON.stringify({ error: 'Mobile authentication required', code: 'MOBILE_AUTH_REQUIRED' }),
            { status: 401, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const { desktop_id } = mobileAuth;

    // Generate pairing token and code
    const token = generateToken(32);
    const code = generatePairingCode();
    const expiresAt = Math.floor(Date.now() / 1000) + PAIRING_TOKEN_TTL;

    // Store token data in KV (same structure as desktop-generated tokens)
    const tokenData: PairingTokenData = {
        device_id: desktop_id,
        code,
        expires_at: expiresAt,
        claimed: false,
    };
    await env.TOKENS.put(
        `pairing:${token}`,
        JSON.stringify(tokenData),
        { expirationTtl: PAIRING_TOKEN_TTL }
    );

    // Store code -> token mapping
    await env.TOKENS.put(
        `pairing_code:${code}`,
        token,
        { expirationTtl: PAIRING_TOKEN_TTL }
    );

    // Return code and URL
    const response: GeneratePairingTokenResponse = {
        token,
        expires_at: expiresAt,
        qr_data: `crabigator://pair?t=${token}&d=${desktop_id}`,
        code,
    };

    return jsonResponse(response);
}

/**
 * Get or create a device group for a desktop device
 */
async function getOrCreateDeviceGroup(env: Env, deviceId: string): Promise<string> {
    // Check if device already has a group
    const device = await env.DB.prepare(
        'SELECT group_id FROM devices WHERE id = ?'
    ).bind(deviceId).first<{ group_id: string | null }>();

    if (device?.group_id) {
        return device.group_id;
    }

    // Create new group
    const groupId = generateUUID();
    const now = Math.floor(Date.now() / 1000);

    await env.DB.prepare(`
        INSERT INTO device_groups (id, created_at)
        VALUES (?, ?)
    `).bind(groupId, now).run();

    // Associate device with group
    await env.DB.prepare(`
        UPDATE devices SET group_id = ? WHERE id = ?
    `).bind(groupId, deviceId).run();

    return groupId;
}
