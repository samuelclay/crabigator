-- How full each session's context window is, in tokens, from the desktop's
-- live stats. Both stay NULL until the assistant reports them, so sessions
-- from desktops that never send them are unaffected.
ALTER TABLE sessions ADD COLUMN context_used INTEGER;
ALTER TABLE sessions ADD COLUMN context_limit INTEGER;
