import type { Env } from '../types/env';
import { generateUUID } from './tokens';

/**
 * Get or create a device group for a desktop device.
 */
export async function getOrCreateDeviceGroup(env: Env, deviceId: string): Promise<string> {
    const device = await env.DB.prepare(
        'SELECT group_id FROM devices WHERE id = ?',
    ).bind(deviceId).first<{ group_id: string | null }>();

    if (device?.group_id) {
        return device.group_id;
    }

    const groupId = generateUUID();
    const now = Math.floor(Date.now() / 1000);

    await env.DB.prepare(`
        INSERT INTO device_groups (id, created_at)
        VALUES (?, ?)
    `).bind(groupId, now).run();

    await env.DB.prepare(`
        UPDATE devices SET group_id = ? WHERE id = ?
    `).bind(groupId, deviceId).run();

    return groupId;
}
