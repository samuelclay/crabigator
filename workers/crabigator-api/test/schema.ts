import type { Env } from '../src/types/env';

export async function ensureAccountSchema(env: Env): Promise<void> {
    await env.DB.batch([
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS device_groups (
                id TEXT PRIMARY KEY,
                created_at INTEGER NOT NULL DEFAULT (unixepoch())
            )
        `),
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS devices (
                id TEXT PRIMARY KEY,
                secret_hash TEXT NOT NULL,
                name TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                last_seen_at INTEGER,
                group_id TEXT
            )
        `),
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS linked_devices (
                id TEXT PRIMARY KEY,
                desktop_id TEXT NOT NULL,
                mobile_id TEXT NOT NULL,
                mobile_name TEXT,
                mobile_token_hash TEXT NOT NULL,
                paired_at INTEGER NOT NULL DEFAULT (unixepoch()),
                revoked_at INTEGER,
                UNIQUE(desktop_id, mobile_id)
            )
        `),
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS accounts (
                id TEXT PRIMARY KEY,
                group_id TEXT UNIQUE,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                last_login_at INTEGER
            )
        `),
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS account_identities (
                id TEXT PRIMARY KEY,
                account_id TEXT NOT NULL,
                provider TEXT NOT NULL,
                provider_user_id TEXT NOT NULL,
                email TEXT,
                name TEXT,
                username TEXT,
                created_at INTEGER NOT NULL DEFAULT (unixepoch()),
                UNIQUE(provider, provider_user_id)
            )
        `),
    ]);
}
