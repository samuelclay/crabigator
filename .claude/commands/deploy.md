Commit all changes (split into logical commits per CLAUDE.md), push to origin/main, and deploy the Cloudflare Worker.

Steps:
1. Run `git status` and `git diff` to understand changes
2. Split changes into logical commits (Rust vs Worker, separate features/fixes)
3. Push to origin/main
4. Run `make deploy` to deploy the Cloudflare Worker
5. Verify deployment succeeded

Allowed bash commands: git status, git diff, git add, git commit, git push, git log, make deploy
