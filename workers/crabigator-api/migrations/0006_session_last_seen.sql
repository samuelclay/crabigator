-- Add last_seen_at column for zombie session detection
-- Migration: 0006_session_last_seen
--
-- This column tracks the last time the desktop sent activity to the cloud.
-- Used by the scheduled cleanup job to identify zombie sessions that were
-- never properly cleaned up (e.g., desktop crashed before WebSocket connected,
-- or network issues prevented disconnect detection).

ALTER TABLE sessions ADD COLUMN last_seen_at INTEGER;

-- Set initial value for active sessions to started_at
-- This ensures existing sessions won't be immediately marked as stale
UPDATE sessions SET last_seen_at = started_at WHERE is_active = 1;

-- Create index for efficient staleness queries
CREATE INDEX IF NOT EXISTS idx_sessions_last_seen ON sessions(is_active, last_seen_at);
