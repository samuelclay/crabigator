import type { Env } from './types/env';

/** Allow reconnects after a network change without keeping dead sessions live for hours. */
export const SESSION_DISCONNECT_GRACE_MS = 60_000;

/** D1 records the last reported state; only the session object knows if it is still live. */
export async function disconnectedSessions(
    env: Env,
    sessions: { id: string; lastSeen: number }[],
): Promise<Set<string>> {
    const disconnected = new Set<string>();
    await Promise.all(sessions.map(async ({ id, lastSeen }) => {
        try {
            const response = await env.SESSION.get(env.SESSION.idFromName(id)).fetch('https://internal/state');
            if (!response.ok) return;
            const state = await response.json() as { desktop_connected?: boolean; disconnected_at?: number | null };
            if (state.desktop_connected === false
                && Date.now() - (state.disconnected_at ?? lastSeen * 1000) >= SESSION_DISCONNECT_GRACE_MS) {
                disconnected.add(id);
            }
        } catch {
            // An unavailable server is not evidence that the desktop stopped.
        }
    }));
    return disconnected;
}
