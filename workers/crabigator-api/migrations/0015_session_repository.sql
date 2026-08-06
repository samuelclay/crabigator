-- Keep repository identity on every session, including sessions with no PR.
ALTER TABLE sessions ADD COLUMN repo_owner TEXT NOT NULL DEFAULT '';
ALTER TABLE sessions ADD COLUMN repo_name TEXT NOT NULL DEFAULT '';
ALTER TABLE sessions ADD COLUMN branch TEXT NOT NULL DEFAULT '';
