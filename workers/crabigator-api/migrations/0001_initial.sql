-- Crabigator Cloud Infrastructure Schema
-- Migration: 0001_initial

-- Desktop devices (source of identity)
-- Each desktop running crabigator gets a unique device_id
CREATE TABLE IF NOT EXISTS devices (
    id TEXT PRIMARY KEY,                      -- device_id (UUID)
    secret_hash TEXT NOT NULL,                -- SHA-256 hash of device_secret
    name TEXT,                                -- Optional device name (e.g., "MacBook Pro")
    created_at INTEGER NOT NULL DEFAULT (unixepoch()),
    last_seen_at INTEGER
);

-- Linked mobile devices
-- Mobile devices pair with desktop via QR code
CREATE TABLE IF NOT EXISTS linked_devices (
    id TEXT PRIMARY KEY,                      -- link_id (UUID)
    desktop_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    mobile_id TEXT NOT NULL,                  -- Mobile device identifier
    mobile_name TEXT,                         -- e.g., "iPhone 15"
    mobile_token_hash TEXT NOT NULL,          -- SHA-256 hash of mobile_token
    paired_at INTEGER NOT NULL DEFAULT (unixepoch()),
    revoked_at INTEGER,                       -- NULL if active, timestamp if revoked
    UNIQUE(desktop_id, mobile_id)
);

CREATE INDEX IF NOT EXISTS idx_linked_desktop ON linked_devices(desktop_id, revoked_at);
CREATE INDEX IF NOT EXISTS idx_linked_mobile_token ON linked_devices(mobile_token_hash);

-- Sessions
-- Each crabigator session (launch) creates a cloud session
CREATE TABLE IF NOT EXISTS sessions (
    id TEXT PRIMARY KEY,                      -- Cloud session ID (UUID)
    device_id TEXT NOT NULL REFERENCES devices(id) ON DELETE CASCADE,
    client_session_id TEXT NOT NULL,          -- Local crabigator session ID
    cwd TEXT NOT NULL,                        -- Working directory
    platform TEXT NOT NULL,                   -- 'claude' | 'codex'
    state TEXT NOT NULL DEFAULT 'ready',      -- Session state
    started_at INTEGER NOT NULL DEFAULT (unixepoch()),
    ended_at INTEGER,                         -- NULL if active
    is_active INTEGER NOT NULL DEFAULT 1,     -- 1 if active, 0 if ended
    share_token TEXT UNIQUE,                  -- For shareable links (NULL if not shared)

    -- Denormalized stats for quick queries
    prompts INTEGER NOT NULL DEFAULT 0,
    completions INTEGER NOT NULL DEFAULT 0,
    tool_calls INTEGER NOT NULL DEFAULT 0,
    thinking_seconds INTEGER NOT NULL DEFAULT 0,

    UNIQUE(device_id, client_session_id)
);

CREATE INDEX IF NOT EXISTS idx_sessions_device ON sessions(device_id, is_active, started_at DESC);
CREATE INDEX IF NOT EXISTS idx_sessions_share_token ON sessions(share_token) WHERE share_token IS NOT NULL;

-- Session events (for archive replay and late-joiner catchup)
-- Events are persisted here, then archived to R2 when session ends
CREATE TABLE IF NOT EXISTS session_events (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,                 -- 'scrollback' | 'state' | 'git' | 'changes' | 'stats' | 'screen'
    timestamp INTEGER NOT NULL,               -- Unix timestamp (ms)
    payload TEXT NOT NULL,                    -- JSON payload
    sequence INTEGER NOT NULL                 -- Monotonic sequence number per session
);

CREATE INDEX IF NOT EXISTS idx_events_session ON session_events(session_id, sequence);
