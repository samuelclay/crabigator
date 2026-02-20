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

const PAIRING_TOKEN_TTL = 60 * 60 * 24 * 365; // 1 year (display only)

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
        JSON.stringify(tokenData)
    );

    // Also store code -> token mapping for web-based pairing
    await env.TOKENS.put(
        `pairing_code:${code}`,
        token
    );

    await addPairingTokenForDevice(env, device_id, token);

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

    // Generate mobile token
    const mobileToken = generateToken(32);
    const mobileTokenHash = await sha256(mobileToken);
    const mobileId = body.mobile_id || generateUUID();
    const mobileName = body.mobile_name || null;

    // Get or create device group for this desktop
    let groupId = await getOrCreateDeviceGroup(env, tokenData.device_id);

    // Check if this mobile already has links to a different group — if so, merge
    const existingLink = await env.DB.prepare(`
        SELECT d.group_id FROM linked_devices ld
        JOIN devices d ON ld.desktop_id = d.id
        WHERE ld.mobile_id = ? AND ld.revoked_at IS NULL AND d.group_id IS NOT NULL AND d.group_id != ?
        LIMIT 1
    `).bind(mobileId, groupId).first<{ group_id: string }>();

    if (existingLink) {
        // Mobile already belongs to another group — merge this desktop into it
        const existingGroupId = existingLink.group_id;
        await env.DB.prepare(
            `UPDATE devices SET group_id = ? WHERE group_id = ?`
        ).bind(existingGroupId, groupId).run();
        groupId = existingGroupId;
    }

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
        JSON.stringify(tokenData)
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

    return new Response(pairingPageHtml(tokenData.code, false, tokenData.claimed), {
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
    <link rel="icon" href="data:image/svg+xml,<svg xmlns='http://www.w3.org/2000/svg' viewBox='0 0 512 512'><path fill='%23ea580c' d='M379.204,266.236c-1.879,0-3.78-0.07-5.648-0.206c-11.05-0.806-19.356-10.418-18.55-21.468c0.807-11.05,10.416-19.352,21.469-18.55c0.9,0.066,1.819,0.099,2.729,0.099c20.173,0,36.585-16.412,36.585-36.585c0-1.337-0.072-2.687-0.215-4.01c-1.193-11.016,6.77-20.913,17.786-22.106c11.014-1.19,20.913,6.77,22.105,17.786c0.298,2.751,0.449,5.553,0.449,8.329C455.914,231.824,421.502,266.236,379.204,266.236z'/><path fill='%23f97316' d='M415.008,97.65v-53.37c4.962-1.106,10.113-1.709,15.408-1.709c38.929,0,70.489,31.558,70.489,70.487s-31.558,70.487-70.489,70.487c-38.929,0-70.487-31.558-70.487-70.487c0-5.295,0.603-10.446,1.709-15.408L415.008,97.65L415.008,97.65z'/><path fill='%23ea580c' d='M491.941,331.928c-5.383,0-10.752-2.153-14.704-6.409c-20.311-21.868-49.921-36.124-83.371-40.14c-11.001-1.321-18.849-11.31-17.528-22.311c1.321-11.001,11.302-18.851,22.311-17.528c42.871,5.148,81.222,23.853,107.989,52.672c7.541,8.119,7.073,20.813-1.046,28.354C501.726,330.153,496.828,331.928,491.941,331.928z'/><path fill='%23ea580c' d='M482.529,411.418c-6.567,0-13-3.219-16.844-9.136c-16.534-25.457-43.78-44.737-76.72-54.289c-10.642-3.086-16.767-14.215-13.68-24.856c3.086-10.642,14.212-16.766,24.856-13.681c42.186,12.234,77.414,37.438,99.195,70.969c6.035,9.292,3.396,21.718-5.897,27.753C490.063,410.369,486.274,411.418,482.529,411.418z'/><path fill='%23ea580c' d='M434.652,469.431c-7.463,0-14.63-4.182-18.087-11.357c-13.569-28.15-40.168-51.891-72.98-65.132c-10.275-4.146-15.242-15.837-11.096-26.112c4.148-10.276,15.839-15.243,26.113-11.096c42.575,17.181,75.996,47.339,94.109,84.92c4.811,9.982,0.619,21.972-9.362,26.783C440.54,468.79,437.573,469.431,434.652,469.431z'/><path fill='%23f97316' d='M132.796,266.236c-42.298,0-76.709-34.412-76.709-76.709c0-2.775,0.151-5.577,0.449-8.329c1.193-11.016,11.097-18.972,22.105-17.786c11.016,1.193,18.979,11.091,17.786,22.106c-0.143,1.323-0.215,2.672-0.215,4.01c0,20.172,16.412,36.585,36.586,36.585c0.91,0,1.828-0.033,2.729-0.099c11.062-0.804,20.663,7.499,21.469,18.55c0.805,11.05-7.499,20.663-18.55,21.468C136.575,266.167,134.675,266.236,132.796,266.236z'/><path fill='%23f97316' d='M20.058,331.928c-4.887,0-9.785-1.775-13.649-5.363c-8.119-7.541-8.587-20.235-1.046-28.354c26.769-28.819,65.12-47.524,107.989-52.672c11.009-1.317,20.989,6.527,22.311,17.528c1.321,11.001-6.527,20.991-17.528,22.311c-33.451,4.017-63.06,18.272-83.373,40.141C30.81,329.775,25.44,331.928,20.058,331.928z'/><path fill='%23f97316' d='M29.47,411.418c-3.746,0-7.534-1.047-10.909-3.239c-9.293-6.035-11.932-18.461-5.897-27.753c21.78-33.531,57.008-58.736,99.195-70.969c10.644-3.087,21.77,3.039,24.856,13.681c3.087,10.641-3.039,21.77-13.681,24.856c-32.938,9.552-60.186,28.832-76.72,54.289C42.472,408.197,36.036,411.418,29.47,411.418z'/><path fill='%23f97316' d='M77.347,469.431c-2.922,0-5.888-0.641-8.696-1.994c-9.982-4.811-14.173-16.803-9.362-26.783c18.112-37.58,51.534-67.739,94.109-84.92c10.277-4.146,21.967,0.821,26.113,11.096c4.146,10.275-0.821,21.966-11.096,26.112c-32.813,13.241-59.412,36.982-72.98,65.132C91.978,465.247,84.81,469.431,77.347,469.431z'/><ellipse fill='%23fb923c' cx='255.996' cy='294.45' rx='144.436' ry='120.976'/><path fill='%23fb923c' d='M96.991,97.65v-53.37c-4.962-1.106-10.113-1.709-15.408-1.709c-38.929,0-70.487,31.558-70.487,70.487s31.558,70.487,70.487,70.487s70.487-31.558,70.487-70.487c0-5.295-0.602-10.446-1.709-15.408L96.991,97.65L96.991,97.65z'/><path fill='%23f97316' d='M400.43,294.451c0-66.813-64.664-120.975-144.431-120.976v241.952C335.767,415.428,400.43,361.265,400.43,294.451z'/></svg>">
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
        ` : `
            <p class="subtitle">Enter this code on your other device</p>
            <div class="code-display">${formattedCode}</div>
            ${claimed ? `
                <div class="status-claimed">
                    <p>Already paired</p>
                    <p>This code can still be reused</p>
                </div>
            ` : `
                <p class="hint">Open drinkcrabigator.com/dashboard and enter this code</p>
            `}
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
        JSON.stringify(tokenData)
    );

    // Store code -> token mapping
    await env.TOKENS.put(
        `pairing_code:${code}`,
        token
    );

    await addPairingTokenForDevice(env, desktop_id, token);

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

async function addPairingTokenForDevice(env: Env, deviceId: string, token: string): Promise<void> {
    try {
        const existing = await env.TOKENS.get(`pairing_device:${deviceId}`, 'json') as string[] | null;
        const tokens = Array.isArray(existing) ? existing : [];
        if (!tokens.includes(token)) {
            tokens.push(token);
            await env.TOKENS.put(`pairing_device:${deviceId}`, JSON.stringify(tokens));
        }
    } catch {
        // Ignore token tracking failures
    }
}
