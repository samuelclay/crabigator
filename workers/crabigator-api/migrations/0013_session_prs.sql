-- Durable per-session PR records for the cross-session PR board.
-- SessionDO writes these through from the desktop's 'prs' events (diffed and
-- throttled); the dashboard's PR board reads them joined with pr_overrides.
-- Unlike DO storage, these survive session end and DO eviction.
-- Migration: 0013_session_prs

CREATE TABLE IF NOT EXISTS session_prs (
    session_id TEXT NOT NULL REFERENCES sessions(id) ON DELETE CASCADE,
    owner TEXT NOT NULL,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    url TEXT NOT NULL,
    state TEXT,                -- OPEN / MERGED / CLOSED (denormalized for filters)
    is_primary INTEGER DEFAULT 0,
    data TEXT NOT NULL,        -- full SessionPr JSON (titles-column precedent)
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (session_id, owner, repo, number)
);

CREATE INDEX IF NOT EXISTS idx_session_prs_pr ON session_prs(owner, repo, number);
