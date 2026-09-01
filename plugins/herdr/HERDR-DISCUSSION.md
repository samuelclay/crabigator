# Discussion draft for herdrdev/herdr

Title: Add Crabigator as a supported agent

Herdr can already wrap Claude Code, Codex, OpenCode, and Grok. I run those through Crabigator, a TUI that owns the pane process and paints status widgets at the bottom of the screen.

Today Herdr sees `crabigator` as a normal shell. Screen detection also looks at the bottom of the pane, which is Crabigator's widget bar, not the inner agent's prompt. So `HERDR_AGENT=claude crabigator` misclassifies.

What I want:

1. Identify the `crabigator` binary as an agent.
2. Show `crabigator · claude` (or codex / opencode / grok) in the agent row.
3. Use Crabigator's own idle / working / blocked reports when it runs inside a Herdr pane (`HERDR_ENV=1`).
4. Resume with `crabigator claude --resume <id>` (and the matching flags for Codex, OpenCode, and Grok).

I have a fork with that support and a Crabigator build that reports lifecycle state. Happy to walk through a live pane.

Fork: local worktree `feat/crabigator-agent` (this machine: `/Users/sclay/projects/herdr-worktrees/crabigator-agent`).
