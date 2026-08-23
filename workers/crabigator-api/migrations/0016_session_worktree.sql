-- Keep the working-tree numbers and Slack thread labels the desktop PR board
-- reads from each session's local mirror, so the web board can draw the same
-- session rows: uncommitted file count, the diff those files carry, when the
-- current title was set, and the enriched Slack threads seen in the session.
ALTER TABLE sessions ADD COLUMN uncommitted_files INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN additions INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN deletions INTEGER NOT NULL DEFAULT 0;
ALTER TABLE sessions ADD COLUMN titles_changed_at INTEGER;
ALTER TABLE sessions ADD COLUMN slack_threads TEXT;
