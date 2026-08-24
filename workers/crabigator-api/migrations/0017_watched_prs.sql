-- PRs the user explicitly asked the boards to track, independent of any
-- session. Cloud-canonical like pr_overrides: one add shows the PR on every
-- board in the group until it merges or closes and ages past the linger
-- window, is dismissed, or is unwatched.
-- Migration: 0017_watched_prs

CREATE TABLE IF NOT EXISTS watched_prs (
    group_key TEXT NOT NULL,   -- devices.group_id, or device_id when ungrouped
    owner TEXT NOT NULL,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    url TEXT NOT NULL,
    added_at INTEGER NOT NULL DEFAULT (unixepoch()),
    added_by TEXT,             -- mobile_id or device_id that added the watch
    -- Latest GitHub stats (a serialized SessionPr), relayed by whichever
    -- open prs board refreshed the PR last. NULL until first enrichment.
    data TEXT,
    refreshed_at INTEGER NOT NULL DEFAULT 0,
    PRIMARY KEY (group_key, owner, repo, number)
);
