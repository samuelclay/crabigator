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
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS sessions (
                id TEXT PRIMARY KEY,
                device_id TEXT NOT NULL,
                client_session_id TEXT NOT NULL,
                cwd TEXT NOT NULL,
                platform TEXT NOT NULL,
                state TEXT NOT NULL DEFAULT 'ready',
                started_at INTEGER NOT NULL DEFAULT (unixepoch()),
                ended_at INTEGER,
                last_seen_at INTEGER,
                is_active INTEGER NOT NULL DEFAULT 1,
                prompts INTEGER NOT NULL DEFAULT 0,
                completions INTEGER NOT NULL DEFAULT 0,
                tool_calls INTEGER NOT NULL DEFAULT 0,
                thinking_seconds INTEGER NOT NULL DEFAULT 0,
                prompts_changed_at INTEGER,
                completions_changed_at INTEGER,
                titles TEXT,
                titles_changed_at INTEGER,
                recap TEXT,
                repo_owner TEXT,
                repo_name TEXT,
                branch TEXT,
                uncommitted_files INTEGER,
                additions INTEGER,
                deletions INTEGER,
                pr_scope TEXT
            )
        `),
        env.DB.prepare(`
            CREATE TABLE IF NOT EXISTS session_prs (
                session_id TEXT NOT NULL,
                owner TEXT NOT NULL,
                repo TEXT NOT NULL,
                number INTEGER NOT NULL,
                url TEXT NOT NULL,
                state TEXT,
                is_primary INTEGER DEFAULT 0,
                data TEXT NOT NULL,
                updated_at INTEGER NOT NULL,
                PRIMARY KEY (session_id, owner, repo, number)
            )
        `),
    ]);
}
