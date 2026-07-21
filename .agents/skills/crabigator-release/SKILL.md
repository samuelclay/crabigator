---
name: crabigator-release
description: Cut and verify a Crabigator release. Use when the user asks to release Crabigator, bump its release version, publish its GitHub and npm artifacts, or deploy the Worker as part of a release.
---

# Crabigator Release

Read the canonical [Claude release command](../../../.claude/commands/release.md) completely, then follow it as the release workflow. Keep that file as the source of truth; do not copy its procedure into this skill.

Adapt Claude command syntax as follows:

- Treat a version or bump level supplied with this skill invocation as the command's `$ARGUMENTS`. Treat no supplied value as empty arguments.
- When the workflow refers to another `/command`, read the matching file under `.claude/commands/` completely and apply its instructions instead of trying to invoke a Claude slash command.
- Ignore Claude-only frontmatter such as `allowed-tools` and `argument-hint`; use the tools available in the current Codex session.
- Follow the repository's `AGENTS.md`, including its release, verification, and logical-commit requirements.

Do not begin a release from outside the Crabigator repository. Preserve the workflow's stop-and-ask gates for a non-`main` branch, diverged history, existing tags, empty releases, authentication failures, or other unsafe release state.
