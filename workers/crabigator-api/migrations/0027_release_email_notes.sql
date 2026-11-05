-- The release as GitHub has it (title, link, notes), read by
-- `make announce-release` with the operator's gh login. The Worker's own
-- requests to GitHub share Cloudflare's addresses and hit its rate limit.
ALTER TABLE release_emails ADD COLUMN release_json TEXT;
