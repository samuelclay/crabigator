import type { Env } from '../types/env';
import type { RegisterDeviceRequest, RegisterDeviceResponse, HeartbeatResponse } from '../types/api';
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
