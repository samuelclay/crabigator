import type { Env } from '../types/env';
import type {
    GeneratePairingTokenResponse,
    ClaimPairingTokenRequest,
    ClaimPairingTokenResponse,
    PairingStatusResponse,
} from '../types/api';
import { jsonResponse } from '../router';
import { requireDeviceAuth } from '../auth/middleware';
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
