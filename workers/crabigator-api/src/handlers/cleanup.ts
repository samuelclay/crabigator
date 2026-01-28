import type { Env } from '../types/env';

/**
 * Scheduled cleanup job for zombie sessions.
 *
 * Runs every 5 minutes to find and clean up sessions that are marked as
 * active in D1 but have stale timestamps. Before marking any session as
 * inactive, we verify with the SessionDO that the desktop is actually
 * disconnected - this prevents false positives from D1 update failures.
 *
 * Candidates for cleanup:
 * 1. Sessions with last_seen_at > 5 minutes ago
 * 2. Sessions with no last_seen_at and started_at > 10 minutes ago
 *
 * For each confirmed zombie (desktop actually disconnected), we:
 * - Update D1 to mark as inactive
 * - Notify SessionListDO to remove from active list
 */
export async function cleanupZombieSessions(env: Env): Promise<void> {
    const now = Math.floor(Date.now() / 1000);

    // Threshold: sessions not seen for 5 minutes are candidates for cleanup
    const staleThreshold = now - 300; // 5 minutes

    // For sessions that never connected (no last_seen_at), use longer threshold
    const neverConnectedThreshold = now - 600; // 10 minutes

    try {
        // Find candidate zombie sessions based on D1 timestamps
        const candidates = await env.DB.prepare(`
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

        if (!candidates.results || candidates.results.length === 0) {
            return;
        }

        console.log(`Found ${candidates.results.length} candidate zombie session(s), verifying...`);

        // Verify each candidate by checking if desktop is actually disconnected
        const confirmedZombies: Array<{ id: string; group_id: string | null }> = [];

        for (const candidate of candidates.results) {
            try {
                const doId = env.SESSION.idFromName(candidate.id);
                const stub = env.SESSION.get(doId);
                const resp = await stub.fetch(new Request('https://internal/state'));
                const state = await resp.json() as { desktop_connected?: boolean };

                if (!state.desktop_connected) {
                    // Desktop is actually disconnected - this is a real zombie
                    confirmedZombies.push(candidate);
                } else {
                    // Desktop is connected but D1 is stale - update D1 timestamp
                    console.log(`Session ${candidate.id.slice(0, 8)} has stale D1 but desktop is connected, updating timestamp`);
                    await env.DB.prepare(`
                        UPDATE sessions SET last_seen_at = ? WHERE id = ?
                    `).bind(now, candidate.id).run();
                }
            } catch (error) {
                // Can't reach SessionDO - assume it's a zombie
                console.log(`Session ${candidate.id.slice(0, 8)} SessionDO unreachable, marking as zombie`);
                confirmedZombies.push(candidate);
            }
        }

        if (confirmedZombies.length === 0) {
            console.log('No confirmed zombies after verification');
            return;
        }

        console.log(`Cleaning up ${confirmedZombies.length} confirmed zombie session(s)`);

        const sessionIds = confirmedZombies.map(r => r.id);

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

        for (const zombie of confirmedZombies) {
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
