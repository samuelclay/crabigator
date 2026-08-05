-- Latest recap per session as a small JSON brief (headline, generated_at,
-- line additions/deletions), written through by SessionDO so the PR board
-- can show what each session was doing — and how stale that picture is —
-- after the session ends.
ALTER TABLE sessions ADD COLUMN recap TEXT;
