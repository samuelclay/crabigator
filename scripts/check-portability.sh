#!/bin/bash
# Fail when deployment-specific Cloudflare values leak into tracked templates.

set -euo pipefail

ROOT=$(cd "$(dirname "$0")/.." && pwd)
cd "$ROOT"

if rg -n --hidden \
    --glob '!workers/crabigator-api/wrangler.jsonc' \
    --glob '!workers/crabigator-api/wrangler.production.jsonc' \
    --glob '!scripts/check-portability.sh' \
    --glob '!.git/**' \
    'newsblur|20af5d7e521c82550b1ffe8705e981c5|884ae30b-0a93-4716-83f6-89f62a18156a|e35b69000ee445469639878d623139a5' \
    Makefile AGENTS.md README.md scripts workers/crabigator-api; then
    echo "Deployment-specific Cloudflare values found in tracked files."
    exit 1
fi

if [ -e workers/crabigator-api/wrangler.toml ]; then
    echo "Use the tracked wrangler.example.jsonc and ignored wrangler.jsonc instead of wrangler.toml."
    exit 1
fi

if ! git check-ignore -q workers/crabigator-api/wrangler.jsonc; then
    echo "workers/crabigator-api/wrangler.jsonc must stay ignored."
    exit 1
fi

echo "Portability check passed."
