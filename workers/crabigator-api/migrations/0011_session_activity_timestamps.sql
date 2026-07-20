-- Persist prompt/completion recency for dashboard session-list stats.
ALTER TABLE sessions ADD COLUMN prompts_changed_at INTEGER;
ALTER TABLE sessions ADD COLUMN completions_changed_at INTEGER;
