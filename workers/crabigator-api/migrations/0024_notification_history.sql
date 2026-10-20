-- Retain history independently of session/device deletion. Never store push tokens.
CREATE TABLE notification_deliveries (
    id TEXT PRIMARY KEY,
    group_id TEXT NOT NULL,
    session_id TEXT NOT NULL,
    mobile_id TEXT NOT NULL,
    platform TEXT NOT NULL,
    environment TEXT NOT NULL,
    notification_key TEXT NOT NULL,
    kind TEXT NOT NULL,
    prompt_revision INTEGER NOT NULL,
    session_title TEXT,
    prompt_snapshot TEXT,
    status TEXT NOT NULL DEFAULT 'reserved',
    created_at INTEGER NOT NULL,
    completed_at INTEGER,
    UNIQUE(group_id, session_id, mobile_id, notification_key, kind)
);
CREATE INDEX notification_deliveries_group_time ON notification_deliveries(group_id, created_at DESC, id DESC);
CREATE INDEX notification_deliveries_session ON notification_deliveries(session_id, created_at DESC);
CREATE TABLE notification_attempts (
    id TEXT PRIMARY KEY,
    delivery_id TEXT NOT NULL,
    started_at INTEGER NOT NULL,
    completed_at INTEGER,
    status TEXT NOT NULL DEFAULT 'sending',
    http_status INTEGER,
    provider_id TEXT,
    provider_reason TEXT,
    payload TEXT NOT NULL
);
CREATE INDEX notification_attempts_delivery ON notification_attempts(delivery_id, started_at);
