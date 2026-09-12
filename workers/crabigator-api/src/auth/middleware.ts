import type { Env } from '../types/env';
import type { AuthContext, DeviceAuth, MobileAuth, ShareAuth } from '../types/api';
import { sha256, hmacVerify } from './tokens';

const TIMESTAMP_TOLERANCE_MS = 5 * 60 * 1000; // 5 minutes
const MOBILE_TOKEN_TTL = 60 * 60 * 24 * 365; // 1 year
const VIEWER_COOKIE = 'crabigator_viewer';

interface ViewerTokenData {
    account_id?: string;
    group_id?: string;
    desktop_id: string;
    mobile_id: string;
}

function jsonError(error: string, code: string, status: number): Response {
    return new Response(
        JSON.stringify({ error, code }),
        {
            status,
            headers: {
                'Content-Type': 'application/json',
                'X-Error-Code': code,
            },
        },
    );
}

/**
 * Extract a bearer token from a header, query parameter, or viewer cookie.
 */
export function extractToken(request: Request): string | null {
    const authHeader = request.headers.get('Authorization');
    if (authHeader?.startsWith('Bearer ')) {
        return authHeader.slice(7);
    }
    const url = new URL(request.url);
    const queryToken = url.searchParams.get('token');
    if (queryToken) return queryToken;
    return readCookie(request, VIEWER_COOKIE);
}

export function viewerCookie(token: string, secure: boolean): string {
    const parts = [
        `${VIEWER_COOKIE}=${token}`,
        'Path=/',
        'HttpOnly',
        'SameSite=Lax',
        `Max-Age=${MOBILE_TOKEN_TTL}`,
    ];
    if (secure) parts.push('Secure');
    return parts.join('; ');
}

function readCookie(request: Request, name: string): string | null {
    const cookie = request.headers.get('Cookie') || '';
    const match = cookie.match(new RegExp(`(?:^|; )${name}=([^;]+)`));
    return match ? decodeURIComponent(match[1]) : null;
}

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
    const token = extractToken(request);
    if (!token) {
        return null;
    }

    const tokenHash = await sha256(token);

    const data = await env.TOKENS.get(`mobile:${tokenHash}`, 'json') as ViewerTokenData | null;

    if (!data) {
        return null;
    }

    if (data.account_id) {
        const account = await env.DB.prepare(
            'SELECT group_id FROM accounts WHERE id = ?',
        ).bind(data.account_id).first<{ group_id: string | null }>();
        if (!account) {
            await env.TOKENS.delete(`mobile:${tokenHash}`);
            return null;
        }
        return {
            type: 'mobile',
            desktop_id: data.desktop_id || '',
            mobile_id: data.mobile_id,
            group_id: account.group_id || undefined,
            account_id: data.account_id,
        };
    }

    const link = await env.DB.prepare(`
        SELECT ld.id, d.group_id
        FROM linked_devices ld
        JOIN devices d ON d.id = ld.desktop_id
        WHERE ld.desktop_id = ? AND ld.mobile_id = ? AND ld.revoked_at IS NULL
    `).bind(data.desktop_id, data.mobile_id).first<{ id: string; group_id: string | null }>();

    if (!link) {
        await env.TOKENS.delete(`mobile:${tokenHash}`);
        return null;
    }

    const groupId = link.group_id || data.group_id || null;
    if (link.group_id && data.group_id !== link.group_id) {
        await env.TOKENS.put(
            `mobile:${tokenHash}`,
            JSON.stringify({ ...data, group_id: link.group_id }),
            { expirationTtl: MOBILE_TOKEN_TTL }
        );
    }

    return { type: 'mobile', desktop_id: data.desktop_id, mobile_id: data.mobile_id, group_id: groupId || undefined };
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
    const deviceAuth = await verifyDeviceSignature(request, env);
    if (deviceAuth) {
        return deviceAuth;
    }

    const mobileAuth = await verifyMobileToken(request, env);
    if (mobileAuth) {
        return mobileAuth;
    }

    return null;
}

export async function requireAuth(
    request: Request,
    env: Env
): Promise<{ auth: AuthContext } | { error: Response }> {
    const auth = await authenticate(request, env);
    if (!auth) {
        return { error: jsonError('Unauthorized', 'UNAUTHORIZED', 401) };
    }
    return { auth };
}

export async function requireDeviceAuth(
    request: Request,
    env: Env
): Promise<{ auth: DeviceAuth } | { error: Response }> {
    const auth = await verifyDeviceSignature(request, env);
    if (!auth) {
        return { error: jsonError('Device authentication required', 'DEVICE_AUTH_REQUIRED', 401) };
    }
    return { auth };
}

export async function requireMobileAuth(
    request: Request,
    env: Env
): Promise<{ auth: MobileAuth & { group_id: string } } | { error: Response }> {
    const auth = await verifyMobileToken(request, env);
    if (!auth) {
        return { error: jsonError('Mobile authentication required', 'MOBILE_AUTH_REQUIRED', 401) };
    }
    if (!auth.group_id) {
        if (auth.account_id) {
            return {
                error: jsonError(
                    'No desktops linked. Run crabigator pair and enter the code.',
                    'NO_DESKTOPS',
                    403,
                ),
            };
        }
        return { error: jsonError('Mobile authentication required', 'MOBILE_AUTH_REQUIRED', 401) };
    }
    return { auth: auth as MobileAuth & { group_id: string } };
}

export async function requireSessionAccess(
    request: Request,
    env: Env,
    sessionId: string
): Promise<{ auth: MobileAuth & { group_id: string } } | { error: Response }> {
    const authResult = await requireMobileAuth(request, env);
    if ('error' in authResult) {
        return authResult;
    }

    const session = await env.DB.prepare(`
        SELECT devices.group_id as group_id
        FROM sessions
        JOIN devices ON devices.id = sessions.device_id
        WHERE sessions.id = ?
    `).bind(sessionId).first<{ group_id: string | null }>();

    if (!session) {
        return { error: jsonError('Session not found', 'NOT_FOUND', 404) };
    }

    if (!session.group_id || session.group_id !== authResult.auth.group_id) {
        return { error: jsonError('Forbidden', 'FORBIDDEN', 403) };
    }

    return authResult;
}
