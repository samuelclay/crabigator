-- Migration: 0009_session_analytics
-- Adds session-level analytics columns and per-tool usage tracking

-- New columns on sessions table for analytics
ALTER TABLE sessions ADD COLUMN work_seconds INTEGER;
ALTER TABLE sessions ADD COLUMN model TEXT;
ALTER TABLE sessions ADD COLUMN compressions INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN mode TEXT;

-- Per-tool breakdown for each session
-- Allows answering: "What tools does user X prefer?" and "Which tools are most popular?"
CREATE TABLE IF NOT EXISTS session_tool_usage (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    tool_name TEXT NOT NULL,
    count INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (session_id, tool_name)
);

CREATE INDEX IF NOT EXISTS idx_tool_usage_tool ON session_tool_usage(tool_name);

-- State transition event log for each session
-- Enables: per-prompt thinking time distributions, time-between-interruptions, state duration analysis
CREATE TABLE IF NOT EXISTS session_events_log (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    event_type TEXT NOT NULL,              -- 'UserPromptSubmit', 'Stop', 'PermissionRequest', etc.
    timestamp_ms INTEGER NOT NULL,         -- Unix timestamp in milliseconds
    state_before TEXT,                     -- State before this event
    state_after TEXT                       -- State after this event
);

CREATE INDEX IF NOT EXISTS idx_events_log_session ON session_events_log(session_id, timestamp_ms);
CREATE INDEX IF NOT EXISTS idx_events_log_type ON session_events_log(event_type, timestamp_ms);
