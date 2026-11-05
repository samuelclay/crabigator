#!/bin/bash
# Queue a release's email to the newsletter (see `make announce-release`):
#   scripts/announce-release.sh vX.Y.Z [test-address]
# Reads the release with your gh login, since GitHub rate-limits the Worker's
# own requests, and adds it to release_emails. The Worker's five-minute cron
# writes the email and sends it. A test address gets it alone.
set -euo pipefail
tag="${1:-}"
to="${2:-}"
if [[ ! "$tag" =~ ^v[0-9]+\.[0-9]+\.[0-9]+$ ]]; then
    echo "Usage: make announce-release TAG=vX.Y.Z [TO=you@example.com]" >&2
    exit 1
fi
if [[ -n "$to" && ! "$to" =~ ^[^@[:space:]]+@[^@[:space:]]+\.[A-Za-z]+$ ]]; then
    echo "TO must be one email address" >&2
    exit 1
fi
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
gh api -H "Accept: application/vnd.github.full+json" "repos/samuelclay/crabigator/releases/tags/$tag" > "$work/release.json"
python3 - "$tag" "$to" "$work/release.json" > "$work/queue.sql" <<'PY'
import json, sys
tag, to, path = sys.argv[1:]
release = json.load(open(path))
kept = json.dumps({key: release.get(key) for key in ("tag_name", "name", "html_url", "body", "body_html")})
quote = lambda value: "NULL" if not value else "'" + value.replace("'", "''") + "'"
print(f"INSERT OR IGNORE INTO release_emails (tag, test_recipient, release_json) VALUES ({quote(tag)}, {quote(to)}, {quote(kept)});")
PY
cd "$(dirname "$0")/../workers/crabigator-api"
npx wrangler d1 execute "${D1_DATABASE:-DB}" --remote --config "${WRANGLER_CONFIG:-wrangler.jsonc}" \
    ${WRANGLER_PROFILE:+--profile "$WRANGLER_PROFILE"} --file "$work/queue.sql"
echo "Queued $tag${to:+ for $to}. The Worker sends it within five minutes."
