-- GitHub / Google accounts that own a device group.
-- Pairing still attaches desktops; social login identifies the person.

CREATE TABLE IF NOT EXISTS accounts (
    id TEXT PRIMARY KEY,
    group_id TEXT UNIQUE REFERENCES device_groups(id),
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_login_at INTEGER
);

CREATE TABLE IF NOT EXISTS account_identities (
    id TEXT PRIMARY KEY,
    account_id TEXT NOT NULL REFERENCES accounts(id) ON DELETE CASCADE,
    provider TEXT NOT NULL,
    provider_user_id TEXT NOT NULL,
    email TEXT,
    name TEXT,
    username TEXT,
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    UNIQUE(provider, provider_user_id)
);

CREATE INDEX IF NOT EXISTS idx_account_identities_account
    ON account_identities(account_id);
