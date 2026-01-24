import type { Env } from '../types/env';
import type { RegisterDeviceRequest, RegisterDeviceResponse, HeartbeatResponse, LinkedDevice } from '../types/api';
import { jsonResponse } from '../router';
import { requireDeviceAuth } from '../auth/middleware';

/**
 * POST /api/devices - Register a new device (idempotent)
 */
export async function registerDevice(
    request: Request,
    env: Env
): Promise<Response> {
    let body: RegisterDeviceRequest;
    try {
        body = await request.json();
    } catch {
        return new Response(
            JSON.stringify({ error: 'Invalid JSON', code: 'INVALID_JSON' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const { device_id, secret_hash, name } = body;

    if (!device_id || !secret_hash) {
        return new Response(
            JSON.stringify({ error: 'Missing device_id or secret_hash', code: 'MISSING_FIELDS' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Validate format
    if (!/^[a-f0-9-]{36}$/.test(device_id)) {
        return new Response(
            JSON.stringify({ error: 'Invalid device_id format', code: 'INVALID_DEVICE_ID' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    if (!/^[a-f0-9]{64}$/.test(secret_hash)) {
        return new Response(
            JSON.stringify({ error: 'Invalid secret_hash format', code: 'INVALID_SECRET_HASH' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const now = Math.floor(Date.now() / 1000);

    const existing = await env.DB.prepare(
        'SELECT secret_hash FROM devices WHERE id = ?'
    ).bind(device_id).first<{ secret_hash: string }>();

    if (existing && existing.secret_hash !== secret_hash) {
        return new Response(
            JSON.stringify({ error: 'Device already registered', code: 'DEVICE_EXISTS' }),
            { status: 409, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Upsert device (idempotent)
    await env.DB.prepare(`
        INSERT INTO devices (id, secret_hash, name, created_at, last_seen_at)
        VALUES (?, ?, ?, ?, ?)
        ON CONFLICT(id) DO UPDATE SET
            last_seen_at = excluded.last_seen_at,
            name = COALESCE(excluded.name, devices.name)
    `).bind(device_id, secret_hash, name || null, now, now).run();

    const response: RegisterDeviceResponse = { ok: true };
    return jsonResponse(response);
}

/**
 * POST /api/devices/heartbeat - Update last_seen timestamp
 */
export async function deviceHeartbeat(
    request: Request,
    env: Env
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }

    // last_seen is already updated by auth middleware
    const response: HeartbeatResponse = {
        ok: true,
        last_seen_at: Math.floor(Date.now() / 1000),
    };
    return jsonResponse(response);
}

/**
 * GET /api/devices/linked - List linked mobile devices
 * Requires device auth
 */
export async function getLinkedDevices(
    request: Request,
    env: Env
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;

    // Get all linked devices for this desktop
    const results = await env.DB.prepare(`
        SELECT mobile_id, mobile_name, paired_at
        FROM linked_devices
        WHERE desktop_id = ? AND revoked_at IS NULL
        ORDER BY paired_at DESC
    `).bind(device_id).all<{ mobile_id: string; mobile_name: string | null; paired_at: number }>();

    const devices: LinkedDevice[] = results.results.map(row => ({
        mobile_id: row.mobile_id,
        mobile_name: row.mobile_name,
        paired_at: row.paired_at,
    }));

    return jsonResponse({ devices });
}

/**
 * DELETE /api/devices/linked/:mobile_id - Revoke a linked mobile device
 * Requires device auth
 */
export async function revokeLinkedDevice(
    request: Request,
    env: Env,
    params: Record<string, string>
): Promise<Response> {
    const authResult = await requireDeviceAuth(request, env);
    if ('error' in authResult) {
        return authResult.error;
    }
    const { device_id } = authResult.auth;
    const mobile_id = params.mobile_id;

    if (!mobile_id) {
        return new Response(
            JSON.stringify({ error: 'Mobile ID required', code: 'MISSING_MOBILE_ID' }),
            { status: 400, headers: { 'Content-Type': 'application/json' } }
        );
    }

    // Get the linked device record to find the token hash
    const link = await env.DB.prepare(`
        SELECT id, mobile_token_hash
        FROM linked_devices
        WHERE desktop_id = ? AND mobile_id = ? AND revoked_at IS NULL
    `).bind(device_id, mobile_id).first<{ id: string; mobile_token_hash: string }>();

    if (!link) {
        return new Response(
            JSON.stringify({ error: 'Linked device not found', code: 'NOT_FOUND' }),
            { status: 404, headers: { 'Content-Type': 'application/json' } }
        );
    }

    const now = Math.floor(Date.now() / 1000);

    // Mark as revoked
    await env.DB.prepare(`
        UPDATE linked_devices SET revoked_at = ? WHERE id = ?
    `).bind(now, link.id).run();

    // Delete the mobile token from KV
    await env.TOKENS.delete(`mobile:${link.mobile_token_hash}`);

    return jsonResponse({ ok: true });
}
