-- MCP event subscriptions. ChatGPT (and any client that speaks MCP events)
-- registers a webhook here; session activity delivers a signed POST.
-- expires_at and verified_until are unix milliseconds.

CREATE TABLE IF NOT EXISTS mcp_event_subscriptions (
    id TEXT PRIMARY KEY,
    principal TEXT NOT NULL,
    account_id TEXT,
    group_id TEXT NOT NULL,
    event_name TEXT NOT NULL,
    arguments_json TEXT NOT NULL,
    callback_url TEXT NOT NULL,
    secret TEXT NOT NULL,
    previous_secret TEXT,
    previous_secret_until INTEGER,
    expires_at INTEGER NOT NULL,
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_mcp_event_subscriptions_delivery
    ON mcp_event_subscriptions (group_id, event_name, expires_at);

CREATE TABLE IF NOT EXISTS mcp_callback_verifications (
    principal TEXT NOT NULL,
    callback_url TEXT NOT NULL,
    verified_until INTEGER NOT NULL,
    PRIMARY KEY (principal, callback_url)
);
