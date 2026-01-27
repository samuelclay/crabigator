import type { Env } from '../types/env';

/**
 * Scheduled cleanup job for zombie sessions.
 *
 * Runs every 5 minutes to find and clean up sessions that are marked as
 * active in D1 but are clearly stale:
 *
 * 1. Sessions with last_seen_at > 5 minutes ago (desktop disconnected but cleanup failed)
 * 2. Sessions with no last_seen_at and started_at > 10 minutes ago (never connected)
 *
 * For each stale session, we:
 * - Update D1 to mark as inactive
 * - Notify SessionListDO to remove from active list
 */
export async function cleanupZombieSessions(env: Env): Promise<void> {
    const now = Math.floor(Date.now() / 1000);

    // Threshold: sessions not seen for 5 minutes are considered zombie
    const staleThreshold = now - 300; // 5 minutes

    // For sessions that never connected (no last_seen_at), use longer threshold
    const neverConnectedThreshold = now - 600; // 10 minutes

    try {
        // Find zombie sessions:
        // 1. Active sessions with last_seen_at older than threshold
        // 2. Active sessions with no last_seen_at and started_at older than threshold
        const zombies = await env.DB.prepare(`
            SELECT s.id, d.group_id
            FROM sessions s
            JOIN devices d ON d.id = s.device_id
            WHERE s.is_active = 1
            AND (
                (s.last_seen_at IS NOT NULL AND s.last_seen_at < ?)
                OR (s.last_seen_at IS NULL AND s.started_at < ?)
            )
            LIMIT 100
        `).bind(staleThreshold, neverConnectedThreshold).all<{
            id: string;
            group_id: string | null;
        }>();

        if (!zombies.results || zombies.results.length === 0) {
            return;
        }

        console.log(`Cleaning up ${zombies.results.length} zombie session(s)`);

        const sessionIds = zombies.results.map(r => r.id);

        // Batch update D1
        const placeholders = sessionIds.map(() => '?').join(',');
        await env.DB.prepare(`
            UPDATE sessions
            SET is_active = 0, ended_at = ?
            WHERE id IN (${placeholders}) AND is_active = 1
        `).bind(now, ...sessionIds).run();

        // Notify SessionListDO for each session
        const doId = env.SESSION_LIST.idFromName('global');
        const stub = env.SESSION_LIST.get(doId);

        for (const zombie of zombies.results) {
            try {
                await stub.fetch(new Request('https://internal/notify', {
                    method: 'POST',
                    body: JSON.stringify({
                        type: 'updated',
                        session: {
                            id: zombie.id,
                            ended_at: now,
                            is_active: false,
                            group_id: zombie.group_id,
                        },
                    }),
                    headers: { 'Content-Type': 'application/json' },
                }));
            } catch (error) {
                console.error(`Error notifying SessionListDO for ${zombie.id}:`, error);
            }
        }

        console.log(`Successfully cleaned up ${sessionIds.length} zombie sessions`);
    } catch (error) {
        console.error('Error in zombie session cleanup:', error);
    }
}
