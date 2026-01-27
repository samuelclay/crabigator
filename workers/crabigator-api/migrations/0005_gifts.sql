-- Gift Subscription System Migration
-- Migration: 0005_gifts
--
-- Allows staff to create gift codes that grant pro status for a specified duration.
-- Supports claiming by users who haven't paired a device yet (cookie-based pending claims).

-- Gift codes created by staff
CREATE TABLE IF NOT EXISTS gifts (
    id TEXT PRIMARY KEY,                    -- 8-char code like 'GIFT7X9K'
    duration_type TEXT NOT NULL,            -- 'day', 'week', 'month', 'year', 'forever'
    duration_seconds INTEGER,               -- Computed seconds (NULL for forever)
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    recipient_email TEXT,                   -- Optional: email if sent via Mailgun
    email_sent_at INTEGER,                  -- When email was sent

    -- Claim tracking
    claimed_at INTEGER,                     -- NULL if unclaimed
    claimed_by_group_id TEXT,               -- Group that claimed (if paired)
    claimed_by_cookie_id TEXT,              -- Cookie ID (if not yet paired)
    subscription_id TEXT                    -- References subscriptions.id when created
);

CREATE INDEX IF NOT EXISTS idx_gifts_claimed ON gifts(claimed_at);
CREATE INDEX IF NOT EXISTS idx_gifts_subscription ON gifts(subscription_id);
CREATE INDEX IF NOT EXISTS idx_gifts_cookie ON gifts(claimed_by_cookie_id);

-- Pending gift claims (for users without group_id yet)
-- When they pair a device, we check this table and create their subscription
CREATE TABLE IF NOT EXISTS pending_gift_claims (
    cookie_id TEXT PRIMARY KEY,             -- Browser cookie ID
    gift_id TEXT NOT NULL,                  -- References gifts.id
    claimed_at INTEGER NOT NULL DEFAULT (unixepoch()),
    FOREIGN KEY (gift_id) REFERENCES gifts(id)
);
