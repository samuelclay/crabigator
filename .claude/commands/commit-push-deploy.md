---
name: crabigator-commit-push-deploy
description: Use when the user asks to commit Crabigator changes, push them to origin/main, and deploy the Cloudflare Worker.
allowed-tools: Bash
---
Commit all changes (split into logical commits per CLAUDE.md), push to origin/main, and deploy the Cloudflare Worker.

Steps:
1. Run `git status` and `git diff` to understand changes
2. Split changes into logical commits (Rust vs Worker, separate features/fixes)
3. Push to origin/main
4. Run `WRANGLER_CONFIG=wrangler.production.jsonc WRANGLER_PROFILE=newsblur make deploy` to deploy the official Cloudflare Worker
5. Verify deployment succeeded
