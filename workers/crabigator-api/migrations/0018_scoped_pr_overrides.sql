-- Scope PR dispositions. A dismissal (or promote/demote) made from inside a
-- session should not reach the group's other sessions: overrides now carry a
-- scope_key — '' for the whole group (the old behavior, kept for board-level
-- actions and watched PRs), 'session:<id>' for one session, or 'path:<cwd>'
-- for every session in one worktree directory (sticky across new sessions
-- there). SQLite cannot alter a primary key, so the table is rebuilt and the
-- existing rows migrate as group-wide.
-- Migration: 0018_scoped_pr_overrides

CREATE TABLE pr_overrides_new (
    group_key TEXT NOT NULL,   -- devices.group_id, or device_id when ungrouped
    owner TEXT NOT NULL,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    scope_key TEXT NOT NULL DEFAULT '',  -- '', 'session:<id>', or 'path:<cwd>'
    disposition TEXT NOT NULL CHECK (disposition IN ('primary', 'secondary', 'dismissed')),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_by TEXT,           -- mobile_id or device_id that made the change
    PRIMARY KEY (group_key, owner, repo, number, scope_key)
);

INSERT INTO pr_overrides_new (group_key, owner, repo, number, scope_key, disposition, updated_at, updated_by)
SELECT group_key, owner, repo, number, '', disposition, updated_at, updated_by
FROM pr_overrides;

DROP TABLE pr_overrides;
ALTER TABLE pr_overrides_new RENAME TO pr_overrides;

-- The scope a session's own dismissals should use, published by the desktop:
-- 'path:<cwd>' when the session runs in a linked git worktree, NULL otherwise
-- (boards fall back to 'session:<id>'). Lets the boards build correctly
-- scoped action links for sessions they did not run inside.
ALTER TABLE sessions ADD COLUMN pr_scope TEXT;
