# AGENTS.md

Guidance for coding agents (Claude Code, Codex CLI) working in this repository. `CLAUDE.md` is a symlink to this file, so both assistants read the same instructions.

## What This Project Is

Crabigator is a Rust TUI wrapper around the Claude Code, Codex, opencode, and Grok CLIs. It spawns the assistant CLI in a PTY (pseudo-terminal) and adds status widgets below the interface showing git status, file changes, session statistics, per-turn recaps, and tracked PRs. Sessions can stream to the official dashboard or a compatible self-hosted Cloudflare Worker in `workers/crabigator-api/`.

Platform selection:

```bash
crabigator                  # Uses default platform (config/env/claude)
crabigator codex            # Use Codex CLI
crabigator claude           # Use Claude Code
crabigator opencode         # Use opencode
crabigator grok             # Use Grok Build (aliases: grok-build, xai)
crabigator --platform grok  # Explicit flag
```

Other subcommands: `inspect` (view running instances), `prs` (live cross-session PR board; `--once` prints one frame — only sessions with a live mirror under /tmp appear, the durable history lives on the web dashboard's PR board; `/` search also greps each live session's transcript and shows the matched excerpt inline, Tab toggles surrounding context; `↑↓` select a session and Enter toggles a quick look pane that mirrors its live screen in the bottom half — while open, `↑↓` scroll that session's transcript and `←→` switch sessions; the default PR view shows one block per primary PR — its GitHub status inline and every touching session as a sub-row beneath it, sorted by the freshest of session activity and PR events, GitHub's updatedAt included; `p` flips to session view, the transpose: one block per session — so blocks map one-to-one to your sessions — with every PR the session touches as a sub-row beneath it; `w` watches any PR by URL or owner/repo#number — watched PRs live in the cloud like dispositions, show on every board (the open board runs `gh` for them and relays the stats), and can also be added by typing "track PR <url>" in a session; dispositions (promote/demote/dismiss) are scoped: the ✕ on a session's own strip or on a session-view sub-row applies only to that session — or to its worktree directory when the session runs in a linked worktree, so future sessions there inherit it — while the ✕ on a PR-view block or a watched PR still removes the PR group-wide; `r` shows or hides recaps, `a` cycles activity ages, and `s` toggles live/all sessions), `pair` (dashboard auth code), `recap` (enable/disable/status for turn recaps), `key` (save an Anthropic API key for recaps), `install-launcher` (macOS crabigator:// URL handler), `resume`/`continue`.

Preferences live in `~/.crabigator/config.toml` (default platform, `ide` for clickable file links, terminal emulator override, recap settings, and `[pr_board]` view preferences saved by the PR board).

## Ask Questions Liberally

**Use the AskUserQuestion tool frequently throughout development - not just during planning.**

Asking questions is encouraged and appreciated because it:
- Helps both of us think through problems more clearly
- Surfaces edge cases and requirements that might be missed
- Leads to better solutions through collaborative dialogue
- Catches misunderstandings early before code is written

Ask about:
- Clarifying requirements and desired behavior
- UI/UX preferences and design decisions
- Trade-offs between different approaches
- Edge cases and error handling
- Whether a proposed solution matches expectations
- Anything you're uncertain about

Don't assume - ask. Multiple rounds of questions are better than one large batch. Even mid-implementation, if something feels unclear or you're choosing between options, ask. The interactive back-and-forth is valuable.

## Plain Language: ISO 24495-1

All prose you write for humans must follow the ISO 24495-1 plain language principles. This applies to everything, with no exceptions: summaries of your work, PR descriptions, **git commit messages**, and **everything in a release** — release notes, highlights, and the GitHub release description:

1. **Relevant**: Include what the reader needs and nothing else. A commit message explains what changed and why it matters; it does not narrate the debugging journey or list every file touched.
2. **Findable**: Put the most important information first. The commit subject line carries the change; the body adds the "why". Summaries lead with the outcome, then supporting detail.
3. **Understandable**: Use familiar words, active voice, and short sentences. One idea per sentence. Prefer "Show the session ID in the status bar" over "Implement session identifier visibility enhancements".
4. **Usable**: Write so the reader can act on it. A summary should let the user verify the work; a commit message should let a future reader decide whether this commit is the one they're hunting for.

For commit messages specifically:
- Subject line: imperative mood, plain words, describes the user-visible or behavioral change (matches existing history, e.g. "Keep previous recap visible between turns").
- Body (when needed): explain why the change was made and any non-obvious consequences, in complete sentences.
- No jargon, abbreviations, or codenames the reader can't be expected to know.

## Build Commands

```bash
cargo build           # Build the project
cargo build --release # Release build
cargo check           # Quick type checking
cargo test            # Run tests
cargo clippy          # Lint (also: make lint)
```

## Running

```bash
make run             # Run with provider from .crabigator-provider (default: claude)
make claude          # Set provider to Claude Code and run
make codex           # Set provider to Codex CLI and run
make claude-yolo     # Claude with --dangerously-skip-permissions
make codex-yolo      # Codex with --dangerously-bypass-approvals-and-sandbox
make resume          # Resume last session
make continue        # Continue last conversation
make prs             # Build and run the cross-session PR board (alias: make pr)
```

## Testing

Fixture-based snapshots live under `tests/fixtures/` and are driven by `src/fixtures_tests.rs`.

```bash
make test            # Run all tests
make test-update     # Update fixture snapshots (CRABIGATOR_UPDATE_FIXTURES=1)
```

Fixture layout:
- `tests/fixtures/<name>/base` - baseline repo state
- `tests/fixtures/<name>/worktree` - working tree changes
- `tests/fixtures/<name>/fixture.json` - staged paths and stats
- `tests/fixtures/<name>/expected.json` - expected mirror JSON

## Architecture

The application uses a **scroll region approach** to layer UI:
- Sets terminal scroll region (DECSTBM escape sequence) to confine assistant CLI output to the top ~80% of the terminal
- The assistant CLI runs in a PTY and its output passes through untouched within that scroll region
- Status widgets are rendered below the scroll region using raw ANSI escape sequences
- No intermediate rendering library - all drawing is done with direct escape codes

### Key Modules

- **app.rs**: Main application loop and layout management. Handles scroll region setup, event polling, status bar drawing, and PTY passthrough.
- **cli.rs**: Argument parsing, subcommand dispatch, platform resolution.
- **config.rs**: `~/.crabigator/config.toml` loading/saving (platform, IDE, terminal preferences).
- **terminal/**: Terminal handling - `pty.rs` manages the PTY via `portable-pty`, `input.rs` forwards keyboard input, `escape.rs` centralizes all ANSI escape sequences (add new sequences here rather than inline), `osc.rs`/`dsr.rs` handle terminal queries, `redraw.rs` manages repaints.
- **git/**: Git state tracking via `git status --porcelain` and `git diff`.
- **parsers/**: Language-specific diff parsers (Rust, TypeScript, Python, Swift, Objective-C, generic) that extract semantic information (functions, classes, nested scopes) from git diffs. `scope_walker.rs` attributes changes to nested scopes; `summary.rs` builds the changes summary; `permission_prompt.rs` and `suggestion.rs` parse assistant screen content.
- **platforms/**: Platform abstraction layer:
  - `claude_code/`: Claude Code hooks (`stats_hook.py`, `hook_script.rs`) and transcript parsing (writes to `~/.claude/crabigator/`)
  - `codex_cli/`: Codex CLI session log and transcript parsing (reads `~/.codex/sessions`)
  - `opencode/`: opencode integration. Spawns the CLI with `--port` and follows the server's SSE `/event` stream for state, permissions, and the model; writes a normalized transcript log to `/tmp/crabigator-opencode-{session_id}.jsonl` for scrollback, recaps, and PR tracking. opencode's full-screen TUI runs on the alternate screen, which crabigator strips (see `ScrollRegionFilter`) so it paints inside the scroll region on the primary buffer.
  - `grok/`: Grok Build TUI integration. Spawns `grok` (found in `~/.grok/bin` if needed). Full-screen TUI is stripped onto the primary buffer like opencode. State comes from `~/.grok/sessions/<encoded-cwd>/<id>/events.jsonl` and the transcript from `updates.jsonl`; no hooks are installed.
- **hooks/**: `SessionStats` for session time tracking and platform stats integration.
- **ui/**: Status bar rendering - `status_bar.rs` orchestrates layout; `git.rs`, `changes.rs`, `stats.rs` are the individual widgets; `handoff.rs` is the strip above the widgets (setup prompts, update notices, latest recap, tracked PRs); `pr_cells.rs` is the PR cell rendering shared by the handoff strip and the PR board; `pairing.rs` renders full-width pairing/update banners; `sparkline.rs` renders Unicode sparklines.
- **cloud/**: Streaming to the configured cloud origin - endpoint selection, host-scoped device identity, session registration, event queue, and WebSocket connection with auto-reconnect.
- **capture.rs**: Output capture. Writes the session transcript to `scrollback.log` (from platform JSONL) and periodic screen snapshots to `screen.txt`.
- **mirror.rs**: Widget state mirroring. Publishes throttled JSON snapshots of all widget state to `inspect.json`.
- **inspect.rs**: `crabigator inspect` implementation for viewing other running instances.
- **recap.rs**: Automatic per-turn recaps, generated on the desktop from local transcripts; only the finished recap is sent to the cloud.
- **pr.rs / pr_rank.rs**: GitHub PR tracking for the recap - scrapes the turn transcript for PR mentions, enriches via `gh pr view`, and classifies PRs as primary or secondary.
- **prs_board.rs**: The `crabigator prs` cross-session PR board - reads every live session's `inspect.json`, groups PRs by repository, and renders the interactive board (search, detail levels, live/cloud toggle).
- **slack.rs**: Captures Slack permalinks mentioned in session transcripts so PRs and recaps can link back to their Slack threads.
- **mode.rs**: Detects Claude Code's operating mode (Normal, Auto-Accept, Plan) from screen content.
- **title.rs**: Automatic Claude/Codex title handling and official session-title selection from the primary PR.
- **update.rs**: Auto-update checks via the GitHub Releases API (npm, cargo, homebrew install methods).
- **ide.rs / launcher.rs / terminal_spawner.rs**: IDE hyperlink URLs (OSC 8), macOS crabigator:// URL handler, and new-terminal-window spawning.
- **pair.rs**: Pairing code generation for dashboard auto-login.
- **banner.rs**: Styled session start/end banners.

### Module Organization

This codebase uses `folder.rs` files instead of `folder/mod.rs` for module roots (Rust 2018+ style). For example:
- `src/ui.rs` is the module root for `src/ui/` (not `src/ui/mod.rs`)
- `src/terminal.rs` is the module root for `src/terminal/`

This keeps module declarations visible at the top level rather than buried in subdirectories.

### Input Handling

- All keyboard input forwards directly to the PTY
- Option/Alt key combinations are properly encoded for word navigation (Option+Left/Right) and word deletion (Option+Backspace/Delete)
- When the assistant CLI exits, Crabigator exits automatically

### Terminal Considerations

- Uses primary screen buffer (not alternate screen) to preserve native scrollback
- Mouse capture is disabled to allow native text selection
- Bracketed paste is enabled for efficient paste handling
- Panic handler restores terminal state to prevent corruption

### Session Directory

Each crabigator session creates `/tmp/crabigator-{session_id}/` containing:
- **scrollback.log**: Session transcript (append-only, built from the platform's JSONL log)
- **screen.txt**: Current screen snapshot from the vt100 parser (updated ~100ms)
- **inspect.json**: Widget state for external inspection (updated ~1s when changed)
- **hooks.log**: Debug log of hook invocations (Claude Code)

The session directory path is shown in the startup banner in debug builds (`cargo build`), but hidden in release builds.

Use `--no-capture` to disable output capture (scrollback.log and screen.txt).

**Claude Code Session UUID Symlink:**

On the first hook event, crabigator creates a symlink from the Claude Code conversation UUID to the session directory:
```
/tmp/crabigator-{claude_uuid} -> /tmp/crabigator-{crabigator_id}
```

This allows accessing the session directory using either ID. The Claude session UUID is also stored in the stats file as `claude_session_id`.

### Session IDs: Finding a Session From Any ID

A session has three identifiers, and **each one is a directory under `/tmp`** — the crabigator ID (canonical), the assistant conversation UUID (symlinked by the hook), and the cloud/streaming ID (symlinked once cloud registration succeeds).

**When the user gives you a bare ID with no label** — like `ab9b47bb` or `e881821b` — it is almost always the **streaming ID** shown in the status bar as `Streaming ab9b47bb`, which is only the **first 8 characters** of the full cloud session ID. Resolve it with a prefix glob, then read `transcript_path` out of `inspect.json`:

```bash
ID=ab9b47bb
ls -d /tmp/crabigator-$ID*                          # → the session directory (a symlink)
jq -r '.transcript_path, .session_id, .cwd' /tmp/crabigator-$ID*/inspect.json
```

`inspect.json` carries `session_id`, `cloud_session_id`, and `transcript_path`, so any one ID leads to the others and to the assistant's JSONL conversation log (`~/.claude/projects/…` or `~/.codex/sessions/…`).

If the glob finds nothing, the session predates the cloud symlink or never registered. Fall back to searching by content:

```bash
grep -l "$ID" /tmp/crabigator-*/inspect.json                             # ID recorded in a mirror
grep -rl "distinctive phrase from the session" ~/.claude/projects ~/.codex/sessions
```

`crabigator inspect` reports each session once, under its canonical directory, even though all three aliases match the glob.

### Self-Inspection (for Claude Code)

When running inside crabigator, Claude Code can inspect its own session using the conversation UUID (visible in the Claude Code UI or provided by the user):

```bash
cat /tmp/crabigator-{uuid}/screen.txt      # Current screen
cat /tmp/crabigator-{uuid}/scrollback.log  # Conversation transcript
cat /tmp/crabigator-{uuid}/inspect.json    # Widget state (stats, git status, etc.)
cat /tmp/crabigator-{uuid}/hooks.log       # Hook event log
```

### Instance Inspection

Use `crabigator inspect` to view other running instances:
- `crabigator inspect` - list all instances
- `crabigator inspect /path` - filter by working directory
- `crabigator inspect --watch` - continuous monitoring
- `crabigator inspect --raw` - output raw JSON
- `crabigator inspect --history` - show hook event history for debugging

### Claude Code Hooks

Crabigator installs Python hooks into Claude Code's `~/.claude/settings.json` to track session state (thinking, permission, complete, etc.) and statistics.

**Hook files:**
- `~/.claude/crabigator/stats-hook.py` - The Python hook script
- `~/.claude/crabigator/hooks-meta.json` - Version metadata for change detection
- `/tmp/crabigator-stats-{session_id}.json` - Per-session stats written by hooks
- `/tmp/crabigator-{session_id}/hooks.log` - Debug log of hook invocations

**Hook versioning:**
- Hooks are versioned by both `HOOK_VERSION` (from Cargo.toml) and an MD5 hash of the script content
- On startup, crabigator checks if installed hooks match the current version/hash
- If mismatched or missing, hooks are automatically reinstalled

**Updating hooks:**
1. Edit `src/platforms/claude_code/stats_hook.py` (the Python script)
2. Run `make reinstall-hooks` to clear the version metadata (REQUIRED after any edit!)
3. Start a new crabigator session - hooks will be reinstalled automatically

**IMPORTANT:** Always run `make reinstall-hooks` after editing the hook script!

**Debugging hooks:**
```bash
crabigator inspect --history ~/projects  # View event history and hooks.log
cat /tmp/crabigator-{session}/hooks.log  # Raw hook invocation log
```

**Hook events handled:**
- `UserPromptSubmit` → state = thinking
- `PermissionRequest` → state = permission (or question if AskUserQuestion, plan if ExitPlanMode)
- `PostToolUse` → state = thinking (tracks tool counts)
- `Stop` → state = complete (or question if AskUserQuestion was used)
- `SubagentStop`, `PreCompact` → increment counters

## Cloud Infrastructure

Cloudflare Workers project for real-time session streaming to the configured dashboard origin.

### Structure

```
workers/crabigator-api/
├── src/
│   ├── index.ts                # Worker entry point
│   ├── router.ts               # Route dispatch
│   ├── durable-objects/        # SessionDO, SessionListDO, UsageDO
│   ├── handlers/               # sessions, devices, pairing, pr-board, pr-overrides, analytics, payments, telemetry, …
│   ├── auth/                   # Device auth, HMAC signing, middleware
│   ├── dashboard.ts + dashboard/   # Dashboard HTML, CSS, JS, icons
│   ├── landing.ts + landing/       # Landing page (incl. WebGL)
│   ├── staff-dashboard.ts + staff-dashboard/
│   ├── assets/                 # OG images
│   └── types/                  # Shared TypeScript types
├── wrangler.example.jsonc      # Tracked, annotated Worker config template
├── wrangler.production.jsonc   # Tracked config for the official drinkcrabigator.com deployment
└── wrangler.jsonc              # Ignored local config for self-hosted deployments
```

### Commands

```bash
make deploy      # Deploy to Cloudflare
make typecheck   # TypeScript type checking
make dev         # Local dev server
```

Project slash commands exist for common flows: `/deploy`, `/commit-push-deploy`, and `/release` (see `.claude/commands/`).

### Key Notes

- **Dashboard**: HTML assembled in `dashboard.ts` with `ansiToHtml()` for terminal rendering
- **256-color**: Uses xterm formula `value = idx === 0 ? 0 : idx * 40 + 55`
- **Deploys break WebSockets**: Desktop auto-reconnects with exponential backoff (1s-30s)
- **Session state**: Managed by Durable Objects (`SessionDO` per session, `SessionListDO` for the roster, `UsageDO` for usage tracking)
- **Auth**: Desktop device_id + HMAC-SHA256 signatures; optional staff tools use a shared-key login and KV sessions
- **SVG Icons**: NEVER inline SVG icons in TypeScript template files. Keep all SVG icons in dedicated `icons.ts` files (`src/landing/icons.ts`, `src/dashboard/icons.ts`), export them as named string constants, and import where needed. For favicons, use URL-encoded versions (with `%23` for `#` in colors).

### Usage Analytics

```bash
make cf-usage    # Show Cloudflare usage stats and scaling capacity
```

Queries the Cloudflare GraphQL API for worker requests, Durable Objects, and D1 usage. Script at `scripts/cf-usage.sh` reads the wrangler OAuth token automatically. Related: `make reset-usage` (clear today's usage rows) and `make sync-usage GROUP=<group_id>`.

### Querying the D1 Database

The D1 binding is `DB`. The official deployment's config is the tracked
`workers/crabigator-api/wrangler.production.jsonc`; the tracked
`workers/crabigator-api/wrangler.example.jsonc` is the public template, and a
self-hosted copy lives in the ignored `workers/crabigator-api/wrangler.jsonc`.

The official account is not the default wrangler profile, so pass its profile
with `--profile` (the `/deploy` command carries the name). Use `WRANGLER_CONFIG`
and `WRANGLER_PROFILE` with the `make` targets for the same reason.

```bash
# Query production database
wrangler d1 execute DB --remote --config workers/crabigator-api/wrangler.production.jsonc --profile <official-profile> --command "SELECT * FROM page_views LIMIT 5"

# Example: Traffic sources by referrer domain
wrangler d1 execute DB --remote --config workers/crabigator-api/wrangler.production.jsonc --profile <official-profile> --command "
SELECT referrer_domain, COUNT(DISTINCT visitor_id) as visitors
FROM page_views
WHERE created_at > strftime('%s', 'now', '-30 days')
GROUP BY referrer_domain
ORDER BY visitors DESC"
```

Key tables: `page_views`, `analytics_events`, `funnel_events`, `email_signups`, `npm_downloads`, `daily_usage`, `devices`.

## Browser Testing

Use the Chrome browser automation tools (Claude-in-Chrome MCP) to test the dashboard and landing page visually.

### Dashboard Auto-Login

Automated browser sessions have no cookies. To authenticate:

1. **Generate a pairing code:**
   ```bash
   crabigator pair          # or: cargo run --release -- pair
   # Output: ABC-DEF-GHI
   ```
2. **Navigate to the dashboard with the setup parameter:**
   ```
   https://drinkcrabigator.com/dashboard?setup=ABC-DEF-GHI
   ```

The dashboard auto-claims the code and authenticates. Codes expire in 5 minutes and can only be claimed once; the `pair` command validates cached codes and generates new ones when needed. A page snapshot after navigating should show the session list — "Setup Failed" means the code was stale, so run `crabigator pair` again.

**Single-session view** - filter to one session for focused testing (hides Style/Account buttons, forces single-column layout):
```
https://drinkcrabigator.com/dashboard?session=SESSION_ID
```

## E2E Testing with tmux

Agents can spawn and control Crabigator instances for end-to-end testing using tmux. This generates real assistant sessions whose output can be verified locally or on the dashboard. A scripted Codex flow exists as `make e2e-codex-tmux` (`scripts/e2e-codex-tmux.sh`).

### Basic Workflow

```bash
# 1. Start crabigator in a detached tmux session
tmux new-session -d -s crab "cd /path/to/test/project && crabigator"

# 2. Wait for startup
sleep 3

# 3. Send a prompt
tmux send-keys -t crab "explain this codebase" Enter

# 4. Wait for processing, then inspect
sleep 5
cat /tmp/crabigator-*/screen.txt
cat /tmp/crabigator-*/scrollback.log

# 5. Clean up
tmux kill-session -t crab
```

### Codex tmux Workflow (No Browser)

When the user says "run end-to-end test with tmux", default to this Codex flow:

1. Start Codex in tmux: `tmux new-session -d -s crab-e2e "cd /path/to/project && crabigator codex"`.
2. Confirm the session directory from startup output (debug builds show `/tmp/crabigator-<id>/`) or by reading the newest `inspect.json`.
3. Trigger a permission prompt by asking Codex to run a command that requires escalation; verify `inspect.json` state becomes `permission` and the pane shows the approval menu.
4. Answer the permission prompt from tmux (approve or deny) and verify state transitions back out of `permission`.
5. Trigger `request_user_input`; if it is available, verify `question` behavior and answer via tmux; if unavailable in Default mode, verify the attempted tool call and the "unavailable" tool output in the Codex JSONL log.
6. Validate everything via local files only (`inspect.json`, `screen.txt`, `scrollback.log`, `~/.codex/sessions/...jsonl`) without opening a web browser.

### Special Keys

```bash
tmux send-keys -t crab Escape "[" "Z"   # Shift+Tab (mode cycling)
tmux send-keys -t crab Escape           # Escape
tmux send-keys -t crab Tab              # Tab
tmux send-keys -t crab Up               # Arrow keys (Up/Down/Left/Right)
```

### Inspecting Output

**Local files** (in `/tmp/crabigator-*/`): `screen.txt`, `scrollback.log`, `inspect.json`.

**Dashboard** (via browser tools): navigate to `https://drinkcrabigator.com/dashboard` (see Browser Testing above for auth), then take a snapshot or screenshot to verify session state, screen preview, and widgets.

## Code Quality

After completing code changes, use the code-simplifier agent to clean up the code:

```
Use the Task tool with subagent_type="code-simplifier:code-simplifier" to review and simplify recent changes
```

The code simplifier will:
- Remove unused CSS classes, variables, and functions
- Clean up dead code and redundant logic
- Simplify overly complex patterns
- Preserve all functionality

## Commit Practices

Commit messages follow the ISO 24495-1 plain language rules above: plain-words imperative subject, most important information first, body explains why.

Split changes into separate logical commits rather than one large commit. Each commit should represent a single coherent change:

- **One feature/fix per commit**: If you added a debug display feature AND fixed a bug, those should be separate commits
- **Separate by layer**: Rust changes vs Worker/TypeScript changes should generally be separate commits
- **Group related files**: Files that work together for a single feature go in the same commit

Example of good commit splitting:
```
ad3b290 Show session ID next to Streaming status in debug builds
6a4613b Add auto-retry for failed cloud registration
a3e4927 Verify desktop connection before marking sessions as zombie
deb5b0e Use D1 as source of truth for active sessions in dashboard
```

Each commit should:
- Be buildable/deployable on its own (no broken intermediate states)
- Be easy to revert independently if needed

## Releasing a New Version

Use the `/release` command (`.claude/commands/release.md`) - it covers version selection, bumps, tagging, release notes, npm verification, and Worker deployment. Release notes and the GitHub release description follow the ISO 24495-1 plain language rules above: highlights lead with user-visible outcomes in plain words, and every section is written for a reader deciding whether to upgrade. Key invariants if releasing manually:

1. **Version sync is CI-enforced**: `Cargo.toml` and `npm/package.json` must carry the same version. Commit the bump ("Bump version to X.Y.Z"), push, then tag `vX.Y.Z` and push the tag.
2. **Watch the workflow**: `gh run list --limit 1` then `gh run watch <run-id>`.
3. **Verify assets**: `gh release view vX.Y.Z` should show 6 assets (darwin/linux tar.gz and win32 zip, each for arm64 and x64).
4. **If the release fails** (usually version mismatch): delete the tag locally and remotely (`git tag -d vX.Y.Z && git push origin --delete vX.Y.Z`), fix the mismatch, commit, re-tag, and push again. Never overwrite a published tag.
5. **Clean up drafts**: failed workflows may leave draft releases. `gh release list` to check; `gh release edit vX.Y.Z --draft=false` to publish or `gh release delete vX.Y.Z --yes` to remove.
