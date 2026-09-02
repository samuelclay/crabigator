---
name: crabigator-deploy
description: Use when the user asks to deploy Crabigator's Cloudflare Worker or dashboard without committing or releasing code.
allowed-tools: Bash
---
Deploy the Cloudflare Worker (no commit/push).

Steps:
1. Run `WRANGLER_CONFIG=wrangler.production.jsonc WRANGLER_PROFILE=newsblur make deploy` to deploy the official Cloudflare Worker (the tracked production config only deploys with the NewsBlur profile)
2. Verify deployment succeeded
