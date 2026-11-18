---
name: "source-command-commit-push-deploy"
description: "Use when the user asks to commit Crabigator changes, push them to origin/main, and deploy the Cloudflare Worker."
---

# source-command-commit-push-deploy

Use this skill when the user asks to run the migrated source command `commit-push-deploy`.

## Command Template

Commit all changes (split into logical commits per AGENTS.md), push to origin/main, and deploy the Cloudflare Worker.

Steps:
1. Run `git status` and `git diff` to understand changes
2. Split changes into logical commits (Rust vs Worker, separate features/fixes)
3. Push to origin/main
4. Run `WRANGLER_CONFIG=wrangler.production.jsonc WRANGLER_PROFILE=newsblur make deploy` to deploy the official Cloudflare Worker
5. Verify deployment succeeded
