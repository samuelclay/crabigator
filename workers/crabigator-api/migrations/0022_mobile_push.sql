CREATE TABLE IF NOT EXISTS mobile_push_tokens (
    mobile_id TEXT NOT NULL,
    token TEXT NOT NULL UNIQUE,
    token_hash TEXT NOT NULL,
    platform TEXT NOT NULL CHECK(platform = 'android'),
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (mobile_id)
);
