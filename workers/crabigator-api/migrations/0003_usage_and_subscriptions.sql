-- Usage Tracking and Subscriptions Migration
-- Migration: 0003_usage_and_subscriptions
--
-- Implements monetization with 30 minutes free daily dashboard viewing,
-- then $3/month subscription via Stripe or PayPal.
-- User identity = group_id (all paired devices share one usage quota).

-- Daily usage tracking per group (resets at midnight UTC)
-- Tracks dashboard viewing time in seconds
CREATE TABLE IF NOT EXISTS daily_usage (
    group_id TEXT NOT NULL REFERENCES device_groups(id) ON DELETE CASCADE,
    date TEXT NOT NULL,  -- YYYY-MM-DD UTC
    used_seconds INTEGER NOT NULL DEFAULT 0,
    last_updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (group_id, date)
);

CREATE INDEX IF NOT EXISTS idx_daily_usage_date ON daily_usage(date);

-- Subscriptions (Stripe or PayPal)
-- Only ONE active subscription per group (enforced by partial unique index)
CREATE TABLE IF NOT EXISTS subscriptions (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL REFERENCES device_groups(id),
    provider TEXT NOT NULL,  -- 'stripe' | 'paypal'
    provider_customer_id TEXT,
    provider_subscription_id TEXT NOT NULL,
    status TEXT NOT NULL DEFAULT 'pending',  -- pending|active|past_due|canceled
    current_period_start INTEGER,
    current_period_end INTEGER,
    cancel_at_period_end INTEGER NOT NULL DEFAULT 0,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch())
);

CREATE INDEX IF NOT EXISTS idx_subscriptions_group ON subscriptions(group_id);
CREATE UNIQUE INDEX IF NOT EXISTS idx_subscriptions_provider_sub ON subscriptions(provider, provider_subscription_id);
-- Enforce single active subscription per group
CREATE UNIQUE INDEX IF NOT EXISTS idx_subscriptions_active_group
    ON subscriptions(group_id) WHERE status IN ('active', 'past_due');

-- Webhook idempotency log (provider + id is unique, not just id)
CREATE TABLE IF NOT EXISTS webhook_events (
    provider TEXT NOT NULL,  -- 'stripe' | 'paypal'
    event_id TEXT NOT NULL,  -- Event ID from provider
    event_type TEXT NOT NULL,
    processed_at INTEGER NOT NULL DEFAULT (unixepoch()),
    PRIMARY KEY (provider, event_id)
);
