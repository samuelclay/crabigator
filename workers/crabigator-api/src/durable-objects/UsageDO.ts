import type { Env } from '../types/env';

/** Free tier: 10 minutes per day */
const FREE_LIMIT_SECONDS = 10 * 60;

/** How long between heartbeats to consider a viewer active (35s to allow for 5s intervals) */
const HEARTBEAT_TIMEOUT_MS = 35_000;

/** How often to flush usage to D1 (60 seconds) */
const FLUSH_INTERVAL_MS = 60_000;

interface UsageState {
    groupId: string;
    todayDate: string;  // YYYY-MM-DD UTC
    usedSeconds: number;
    /** Map of session_id -> last heartbeat timestamp */
    activeViewers: Map<string, number>;
    /** Unflushed seconds to write to D1 */
    pendingDelta: number;
    /** Cached subscription status */
    isPro: boolean | null;
    /** When subscription status was last checked */
    lastSubscriptionCheck: number;
}

/**
 * Durable Object for tracking dashboard viewing time per group.
 *
 * Usage is tracked via viewer heartbeats from SessionDO:
 * - Each heartbeat records 5 seconds of viewing time
 * - Multiple concurrent viewers accumulate time at 2x, 3x, etc.
 * - Usage resets at midnight UTC
 * - Pro users (active subscription) have unlimited usage
 *
 * Writes to D1 are batched via alarm every 60 seconds to minimize costs.
 */
export class UsageDO implements DurableObject {
    private state: DurableObjectState;
    private env: Env;
    private usageState: UsageState;

    constructor(state: DurableObjectState, env: Env) {
        this.state = state;
        this.env = env;
        this.usageState = {
            groupId: '',
            todayDate: this.getTodayDate(),
            usedSeconds: 0,
            activeViewers: new Map(),
            pendingDelta: 0,
            isPro: null,
            lastSubscriptionCheck: 0,
        };

        // Restore state from storage
        state.blockConcurrencyWhile(async () => {
            const stored = await state.storage.get<{
                groupId: string;
                todayDate: string;
                usedSeconds: number;
                pendingDelta: number;
            }>('usageState');
            if (stored) {
                this.usageState.groupId = stored.groupId;
                this.usageState.todayDate = stored.todayDate;
                this.usageState.usedSeconds = stored.usedSeconds;
                this.usageState.pendingDelta = stored.pendingDelta || 0;
            }
        });
    }

    async fetch(request: Request): Promise<Response> {
        const url = new URL(request.url);

        // Extract group_id from query params if provided
        const groupId = url.searchParams.get('group_id');
        if (groupId && !this.usageState.groupId) {
            this.usageState.groupId = groupId;
            await this.state.storage.put('usageState', {
                groupId: this.usageState.groupId,
                todayDate: this.usageState.todayDate,
                usedSeconds: this.usageState.usedSeconds,
                pendingDelta: this.usageState.pendingDelta,
            });
        }

        switch (url.pathname) {
            case '/heartbeat':
                return this.handleHeartbeat(request);
            case '/usage':
                return this.handleGetUsage();
            case '/sync':
                return this.handleSync();
            case '/reset':
                return this.handleReset();
            default:
                return new Response('Not found', { status: 404 });
        }
    }

    /**
     * Handle alarm for periodic D1 flush
     */
    async alarm(): Promise<void> {
        await this.flushToD1();
    }

    /**
     * Record a viewer heartbeat from a session.
     * Each heartbeat represents 5 seconds of viewing time.
     */
    private async handleHeartbeat(request: Request): Promise<Response> {
        if (request.method !== 'POST') {
            return new Response('Method not allowed', { status: 405 });
        }

        let body: { session_id: string };
        try {
            body = await request.json();
        } catch {
            return new Response(JSON.stringify({ error: 'Invalid JSON' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }

        const { session_id } = body;
        if (!session_id) {
            return new Response(JSON.stringify({ error: 'Missing session_id' }), {
                status: 400,
                headers: { 'Content-Type': 'application/json' },
            });
        }

        const now = Date.now();
        const today = this.getTodayDate();

        // Check for day rollover
        if (today !== this.usageState.todayDate) {
            // Flush any pending usage before resetting
            if (this.usageState.pendingDelta > 0) {
                await this.flushToD1();
            }
            this.usageState.todayDate = today;
            this.usageState.usedSeconds = 0;
            this.usageState.activeViewers.clear();
            this.usageState.isPro = null;  // Re-check subscription on new day
        }

        // Clean up stale viewers
        for (const [sid, lastHb] of this.usageState.activeViewers) {
            if (now - lastHb > HEARTBEAT_TIMEOUT_MS) {
                this.usageState.activeViewers.delete(sid);
            }
        }

        // Calculate time since last heartbeat from this session
        const lastHb = this.usageState.activeViewers.get(session_id);
        let secondsToAdd = 0;

        if (lastHb) {
            // Return early if heartbeat came too quickly (prevent spam)
            if (now - lastHb < 4000) {
                return new Response(JSON.stringify({ ok: true, throttled: true }), {
                    headers: { 'Content-Type': 'application/json' },
                });
            }
            // Add time since last heartbeat (capped at 10s to prevent gaps from counting too much)
            secondsToAdd = Math.min(Math.round((now - lastHb) / 1000), 10);
        } else {
            // First heartbeat from this session - count as 5 seconds
            secondsToAdd = 5;
        }

        // Update viewer's last heartbeat
        this.usageState.activeViewers.set(session_id, now);

        // Add usage if not pro
        if (!await this.checkSubscription()) {
            this.usageState.usedSeconds += secondsToAdd;
            this.usageState.pendingDelta += secondsToAdd;
        }

        // Persist state
        await this.state.storage.put('usageState', {
            groupId: this.usageState.groupId,
            todayDate: this.usageState.todayDate,
            usedSeconds: this.usageState.usedSeconds,
            pendingDelta: this.usageState.pendingDelta,
        });

        // Schedule alarm for D1 flush if not already scheduled
        const currentAlarm = await this.state.storage.getAlarm();
        if (!currentAlarm) {
            await this.state.storage.setAlarm(Date.now() + FLUSH_INTERVAL_MS);
        }

        // Check if limit exceeded
        const isLimited = !this.usageState.isPro && this.usageState.usedSeconds >= FREE_LIMIT_SECONDS;

        return new Response(JSON.stringify({
            ok: true,
            used_seconds: this.usageState.usedSeconds,
            limit_seconds: FREE_LIMIT_SECONDS,
            is_limited: isLimited,
            is_pro: this.usageState.isPro || false,
        }), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Get current usage status
     */
    private async handleGetUsage(): Promise<Response> {
        const today = this.getTodayDate();

        // Check for day rollover
        if (today !== this.usageState.todayDate) {
            // Flush any pending usage before resetting
            if (this.usageState.pendingDelta > 0) {
                await this.flushToD1();
            }
            this.usageState.todayDate = today;
            this.usageState.usedSeconds = 0;
            this.usageState.isPro = null;
        }

        // Sync from D1 to ensure consistency (D1 is source of truth)
        // Only do this if we have pending delta = 0 (no unflushed changes)
        if (this.usageState.pendingDelta === 0 && this.usageState.groupId) {
            const row = await this.env.DB.prepare(
                'SELECT used_seconds FROM daily_usage WHERE group_id = ? AND date = ?'
            ).bind(this.usageState.groupId, today).first<{ used_seconds: number }>();

            const d1Value = row?.used_seconds || 0;
            if (d1Value !== this.usageState.usedSeconds) {
                this.usageState.usedSeconds = d1Value;
                await this.state.storage.put('usageState', {
                    groupId: this.usageState.groupId,
                    todayDate: this.usageState.todayDate,
                    usedSeconds: this.usageState.usedSeconds,
                    pendingDelta: this.usageState.pendingDelta,
                });
            }
        }

        // Check subscription status
        const isPro = await this.checkSubscription();
        const isLimited = !isPro && this.usageState.usedSeconds >= FREE_LIMIT_SECONDS;
        const remainingSeconds = isPro ? Infinity : Math.max(0, FREE_LIMIT_SECONDS - this.usageState.usedSeconds);

        return new Response(JSON.stringify({
            date: today,
            used_seconds: this.usageState.usedSeconds,
            limit_seconds: FREE_LIMIT_SECONDS,
            remaining_seconds: remainingSeconds,
            is_limited: isLimited,
            is_pro: isPro,
        }), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Reset usage to zero (admin/debug endpoint)
     */
    private async handleReset(): Promise<Response> {
        const today = this.getTodayDate();

        // Clear DO state
        this.usageState.todayDate = today;
        this.usageState.usedSeconds = 0;
        this.usageState.pendingDelta = 0;
        this.usageState.activeViewers.clear();
        this.usageState.isPro = null;
        this.usageState.lastSubscriptionCheck = 0;

        // Persist cleared state
        await this.state.storage.put('usageState', {
            groupId: this.usageState.groupId,
            todayDate: this.usageState.todayDate,
            usedSeconds: 0,
            pendingDelta: 0,
        });

        return new Response(JSON.stringify({ ok: true, reset: true }), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Force sync usage from D1 (useful after subscription changes)
     */
    private async handleSync(): Promise<Response> {
        // Clear cached subscription status
        this.usageState.isPro = null;
        this.usageState.lastSubscriptionCheck = 0;

        // Reload usage from D1
        const today = this.getTodayDate();
        const row = await this.env.DB.prepare(
            'SELECT used_seconds FROM daily_usage WHERE group_id = ? AND date = ?'
        ).bind(this.usageState.groupId, today).first<{ used_seconds: number }>();

        if (row) {
            this.usageState.usedSeconds = row.used_seconds;
        } else {
            this.usageState.usedSeconds = 0;
        }
        this.usageState.todayDate = today;
        this.usageState.pendingDelta = 0;

        await this.state.storage.put('usageState', {
            groupId: this.usageState.groupId,
            todayDate: this.usageState.todayDate,
            usedSeconds: this.usageState.usedSeconds,
            pendingDelta: this.usageState.pendingDelta,
        });

        return new Response(JSON.stringify({ ok: true, synced: true }), {
            headers: { 'Content-Type': 'application/json' },
        });
    }

    /**
     * Check if the group has an active subscription
     * Caches result for 5 minutes to reduce D1 queries
     */
    private async checkSubscription(): Promise<boolean> {
        const now = Date.now();
        const CACHE_TTL_MS = 5 * 60 * 1000;  // 5 minutes

        // Return cached value if still valid
        if (this.usageState.isPro !== null && now - this.usageState.lastSubscriptionCheck < CACHE_TTL_MS) {
            return this.usageState.isPro;
        }

        // Query D1 for active subscription
        const row = await this.env.DB.prepare(`
            SELECT id FROM subscriptions
            WHERE group_id = ?
            AND status IN ('active', 'past_due')
            AND (current_period_end IS NULL OR current_period_end > unixepoch())
            LIMIT 1
        `).bind(this.usageState.groupId).first<{ id: string }>();

        this.usageState.isPro = !!row;
        this.usageState.lastSubscriptionCheck = now;

        return this.usageState.isPro;
    }

    /**
     * Flush pending usage to D1
     */
    private async flushToD1(): Promise<void> {
        if (this.usageState.pendingDelta === 0 || !this.usageState.groupId) {
            return;
        }

        const delta = this.usageState.pendingDelta;
        this.usageState.pendingDelta = 0;

        try {
            // Upsert usage record
            await this.env.DB.prepare(`
                INSERT INTO daily_usage (group_id, date, used_seconds, last_updated_at)
                VALUES (?, ?, ?, unixepoch())
                ON CONFLICT (group_id, date) DO UPDATE SET
                    used_seconds = used_seconds + ?,
                    last_updated_at = unixepoch()
            `).bind(
                this.usageState.groupId,
                this.usageState.todayDate,
                delta,
                delta
            ).run();

            // Persist state after successful flush
            await this.state.storage.put('usageState', {
                groupId: this.usageState.groupId,
                todayDate: this.usageState.todayDate,
                usedSeconds: this.usageState.usedSeconds,
                pendingDelta: this.usageState.pendingDelta,
            });
        } catch (error) {
            // Restore pending delta on error to retry next time
            this.usageState.pendingDelta = delta;
            console.error('Failed to flush usage to D1:', error);
        }
    }

    /**
     * Get current UTC date as YYYY-MM-DD
     */
    private getTodayDate(): string {
        const now = new Date();
        return now.toISOString().split('T')[0];
    }
}
