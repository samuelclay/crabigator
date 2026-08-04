-- PR dispositions set by the user (dashboard toggle, TUI action link, or a
-- future admin surface). Cloud-canonical: desktops fetch these and apply them
-- on top of their automatic primary/secondary classification, so one decision
-- holds across every session and device in the group.
-- Migration: 0012_pr_overrides

CREATE TABLE IF NOT EXISTS pr_overrides (
    group_key TEXT NOT NULL,   -- devices.group_id, or device_id when ungrouped
    owner TEXT NOT NULL,
    repo TEXT NOT NULL,
    number INTEGER NOT NULL,
    disposition TEXT NOT NULL CHECK (disposition IN ('primary', 'secondary', 'dismissed')),
    updated_at INTEGER NOT NULL DEFAULT (unixepoch()),
    updated_by TEXT,           -- mobile_id or device_id that made the change
    PRIMARY KEY (group_key, owner, repo, number)
);
