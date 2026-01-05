import type { Env } from '../types/env';
import type { AuthContext, DeviceAuth, MobileAuth, ShareAuth } from '../types/api';
import { sha256, hmacVerify } from './tokens';

const TIMESTAMP_TOLERANCE_MS = 5 * 60 * 1000; // 5 minutes

/**
 * Verify device signature from headers
 * Headers required:
 * - X-Device-Id: device_id
 * - X-Timestamp: unix timestamp (ms)
 * - X-Signature: HMAC-SHA256(secret_hash, "{method}:{path}:{timestamp}")
 */
export async function verifyDeviceSignature(
    request: Request,
    env: Env
): Promise<DeviceAuth | null> {
    const deviceId = request.headers.get('X-Device-Id');
    const timestamp = request.headers.get('X-Timestamp');
    const signature = request.headers.get('X-Signature');

    if (!deviceId || !timestamp || !signature) {
        return null;
    }

    // Check timestamp is recent
    const ts = parseInt(timestamp, 10);
    const now = Date.now();
    if (isNaN(ts) || Math.abs(now - ts) > TIMESTAMP_TOLERANCE_MS) {
        return null;
    }

    // Get device from DB
    const device = await env.DB.prepare(
        'SELECT id, secret_hash FROM devices WHERE id = ?'
    ).bind(deviceId).first<{ id: string; secret_hash: string }>();

    if (!device) {
        return null;
    }

    // Verify signature
    const url = new URL(request.url);
    const message = `${request.method}:${url.pathname}:${timestamp}`;
    const isValid = await hmacVerify(device.secret_hash, message, signature);

    if (!isValid) {
        return null;
    }

    // Update last_seen
    await env.DB.prepare(
        'UPDATE devices SET last_seen_at = ? WHERE id = ?'
    ).bind(Math.floor(now / 1000), deviceId).run();

    return { type: 'device', device_id: deviceId };
}

/**
 * Verify mobile token from Authorization header
 * Header: Authorization: Bearer {mobile_token}
 */
export async function verifyMobileToken(
    request: Request,
    env: Env
): Promise<MobileAuth | null> {
    const authHeader = request.headers.get('Authorization');
    if (!authHeader?.startsWith('Bearer ')) {
        return null;
    }

    const token = authHeader.slice(7);
    const tokenHash = await sha256(token);

    // Look up in KV
    const data = await env.TOKENS.get(`mobile:${tokenHash}`, 'json') as {
        desktop_id: string;
        mobile_id: string;
    } | null;

    if (!data) {
        return null;
    }

    // Check if link is still valid (not revoked)
    const link = await env.DB.prepare(
        'SELECT id FROM linked_devices WHERE desktop_id = ? AND mobile_id = ? AND revoked_at IS NULL'
    ).bind(data.desktop_id, data.mobile_id).first();

    if (!link) {
        // Link was revoked, clean up KV
        await env.TOKENS.delete(`mobile:${tokenHash}`);
        return null;
    }

    return { type: 'mobile', desktop_id: data.desktop_id, mobile_id: data.mobile_id };
}

/**
 * Verify share token from query param or header
 */
export async function verifyShareToken(
    sessionId: string,
    token: string,
    env: Env
): Promise<ShareAuth | null> {
    const session = await env.DB.prepare(
        'SELECT id, share_token FROM sessions WHERE id = ? AND share_token = ?'
    ).bind(sessionId, token).first<{ id: string; share_token: string }>();

    if (!session) {
        return null;
    }

    return { type: 'share', session_id: session.id };
}

/**
 * Authenticate request - tries device signature first, then mobile token
 */
export async function authenticate(
    request: Request,
    env: Env
): Promise<AuthContext | null> {
    // Try device signature
    const deviceAuth = await verifyDeviceSignature(request, env);
    if (deviceAuth) {
        return deviceAuth;
    }

    // Try mobile token
    const mobileAuth = await verifyMobileToken(request, env);
    if (mobileAuth) {
        return mobileAuth;
    }

    return null;
}

/**
 * Require authentication - returns error response if not authenticated
 */
export async function requireAuth(
    request: Request,
    env: Env
): Promise<{ auth: AuthContext } | { error: Response }> {
    const auth = await authenticate(request, env);
    if (!auth) {
        return {
            error: new Response(
                JSON.stringify({ error: 'Unauthorized', code: 'UNAUTHORIZED' }),
                { status: 401, headers: { 'Content-Type': 'application/json' } }
            ),
        };
    }
    return { auth };
}

/**
 * Require device authentication specifically
 */
export async function requireDeviceAuth(
    request: Request,
    env: Env
): Promise<{ auth: DeviceAuth } | { error: Response }> {
    const auth = await verifyDeviceSignature(request, env);
    if (!auth) {
        return {
            error: new Response(
                JSON.stringify({ error: 'Device authentication required', code: 'DEVICE_AUTH_REQUIRED' }),
                { status: 401, headers: { 'Content-Type': 'application/json' } }
            ),
        };
    }
    return { auth };
}
