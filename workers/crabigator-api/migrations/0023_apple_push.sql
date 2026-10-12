-- Retain every Android registration while allowing native Apple device tokens.
CREATE TABLE mobile_push_tokens_next (
    mobile_id TEXT NOT NULL PRIMARY KEY,
    token TEXT NOT NULL UNIQUE,
    token_hash TEXT NOT NULL,
    platform TEXT NOT NULL CHECK(platform IN ('android', 'ios')),
    environment TEXT NOT NULL DEFAULT 'production' CHECK(environment IN ('production', 'sandbox')),
    sound INTEGER NOT NULL DEFAULT 0 CHECK(sound IN (0, 1)),
    updated_at INTEGER NOT NULL
);
INSERT INTO mobile_push_tokens_next (mobile_id, token, token_hash, platform, updated_at)
SELECT mobile_id, token, token_hash, platform, updated_at FROM mobile_push_tokens;
DROP TABLE mobile_push_tokens;
ALTER TABLE mobile_push_tokens_next RENAME TO mobile_push_tokens;
