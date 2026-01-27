# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build          # Build the project
cargo build --release # Release build
cargo run            # Run the application
cargo check          # Quick type checking
cargo test           # Run tests
cargo clippy         # Lint
```

## Running

```bash
make run             # Run with provider from .crabigator-provider (default: claude)
make claude          # Set provider to Claude Code and run
make codex           # Set provider to Codex CLI and run
make resume          # Resume last session
make continue        # Continue last conversation
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

## What This Project Is

Crabigator is a Rust TUI wrapper around the Claude Code and Codex CLIs. It spawns the assistant CLI in a PTY (pseudo-terminal) and adds status widgets below the interface showing git status, file changes, and session statistics.

### Platform Selection

Crabigator supports multiple assistant CLIs:
- **Claude Code** (Anthropic)
- **Codex CLI** (OpenAI)

Platform selection:
```bash
crabigator                 # Uses default platform (config/env/claude)
crabigator codex           # Use Codex CLI
crabigator claude          # Use Claude Code
crabigator --platform codex # Explicit flag
```

Platform preference is saved in `~/.crabigator/config.toml`.

## Architecture

The application uses a **scroll region approach** to layer UI:
- Sets terminal scroll region (DECSTBM escape sequence) to confine assistant CLI output to the top ~80% of the terminal
- The assistant CLI runs in a PTY and its output passes through untouched within that scroll region
- Status widgets are rendered below the scroll region using raw ANSI escape sequences
- No intermediate rendering library (ratatui was removed) - all drawing is done with direct escape codes

### Key Modules

- **app.rs**: Main application loop and layout management. Handles scroll region setup, event polling, status bar drawing, and PTY passthrough.
- **config.rs**: Configuration loading/saving for `~/.crabigator/config.toml` (platform preferences).
- **terminal/**: Terminal handling - `pty.rs` manages PTY via `portable-pty` (spawns the platform CLI, handles I/O), `input.rs` handles keyboard input forwarding, `escape.rs` centralizes all ANSI escape sequences (colors, styles, cursor control, screen clearing) - add new sequences here rather than inline.
- **git/**: Git state tracking via `git status --porcelain` and `git diff`.
- **parsers/**: Language-specific diff parsers (Rust, TypeScript, Python, generic) that extract semantic information (functions, classes, etc.) from git diffs.
- **hooks/**: `SessionStats` for session time tracking and platform stats integration.
- **platforms/**: Platform abstraction layer with `Platform` implementations:
  - `claude_code.rs`: Claude Code hooks and stats (writes to `~/.claude/crabigator/`)
  - `codex_cli.rs`: Codex CLI session log parsing (reads `~/.codex/sessions`)
- **ui/**: Status bar rendering - `status_bar.rs` orchestrates layout, with `git.rs`, `changes.rs`, `stats.rs` for individual widgets.
- **mirror.rs**: Widget state mirroring for external inspection. Publishes throttled JSON snapshots of all widget state.
- **inspect.rs**: Inspect command implementation for viewing other running crabigator instances.
- **capture.rs**: Output capture for streaming. Writes raw PTY bytes to scrollback.log and periodic screen snapshots to screen.txt.

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
- **scrollback.log**: Clean text transcript (ANSI stripped, complete lines only)
- **screen.txt**: Current screen snapshot from vt100 parser (updated ~100ms)
- **mirror.json**: Widget state for external inspection (updated ~1s when changed)

The session directory path is shown in the startup banner in debug builds (`cargo build`), but hidden in release builds (`cargo build --release`).

Use `--no-capture` to disable output capture (scrollback.log and screen.txt).

**Claude Code Session UUID Symlink:**

On the first hook event, crabigator creates a symlink from the Claude Code conversation UUID to the session directory:
```
/tmp/crabigator-{claude_uuid} -> /tmp/crabigator-{crabigator_id}
```

This allows accessing the session directory using either ID. The Claude session UUID is also stored in the stats file as `claude_session_id`.

### Self-Inspection (for Claude Code)

When running inside crabigator, Claude Code can inspect its own session using the conversation UUID. The UUID is visible in the Claude Code UI or can be provided by the user.

**To inspect your own session:**
```bash
# Using Claude Code conversation UUID (e.g., f5cd7167-fc18-4ab2-8686-274fdfb098e1)
cat /tmp/crabigator-{uuid}/screen.txt      # Current screen
cat /tmp/crabigator-{uuid}/scrollback.log  # Conversation transcript
cat /tmp/crabigator-{uuid}/mirror.json     # Widget state (stats, git status, etc.)
cat /tmp/crabigator-{uuid}/hooks.log       # Hook event log
```

**Example workflow:**
1. User provides their Claude Code session UUID (from URL or UI)
2. Use the UUID to read session files and inspect current state
3. The `mirror.json` contains widget data including git status, session stats, and claude_session_id

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
- To force reinstall after modifying the hook script: `make reinstall-hooks`

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
- `PermissionRequest` → state = permission (or question if AskUserQuestion)
- `PostToolUse` → state = thinking (tracks tool counts)
- `Stop` → state = complete (or question if AskUserQuestion was used)
- `SubagentStop`, `PreCompact` → increment counters

## Cloud Infrastructure

Cloudflare Workers project for real-time session streaming to drinkcrabigator.com.

### Structure

```
workers/crabigator-api/
├── src/
│   ├── index.ts            # Main worker entry, routes
│   ├── dashboard.ts        # Dashboard HTML (inline)
│   ├── session-do.ts       # Durable Object for session state
│   └── auth/tokens.ts      # Device auth, HMAC signing
└── wrangler.toml           # Worker config
```

### Commands

```bash
make deploy                                   # Deploy to Cloudflare
make typecheck                                # TypeScript type checking
cd workers/crabigator-api && npm run dev      # Local dev
```

### Key Notes

- **Dashboard**: Inline HTML in `dashboard.ts` with `ansiToHtml()` for terminal rendering
- **256-color**: Uses xterm formula `value = idx === 0 ? 0 : idx * 40 + 55`
- **Deploys break WebSockets**: Desktop auto-reconnects with exponential backoff (1s-30s)
- **Session state**: Managed by Durable Objects (`SessionDO`)
- **Auth**: Desktop device_id + HMAC-SHA256 signatures, no user accounts
- **SVG Icons**: Keep raw SVG icons in dedicated `icons.ts` files rather than inline in HTML templates:
  - `src/landing/icons.ts` - Icons for landing page
  - `src/dashboard/icons.ts` - Icons for dashboard (favicon, etc.)
  - Export icons as named string constants and import where needed

### Usage Analytics

```bash
make cf-usage    # Show Cloudflare usage stats and scaling capacity
```

Queries Cloudflare GraphQL API for worker requests, Durable Objects, and D1 usage. Shows free tier consumption and estimates scaling headroom. Script at `scripts/cf-usage.sh` reads wrangler OAuth token automatically.

## Browser Testing with PlayWriter MCP

The PlayWriter MCP allows Claude Code to control Chrome for testing the dashboard and other web functionality.

### Opening Chrome

```bash
open -a "Google Chrome" --new --args --new-window "https://drinkcrabigator.com/dashboard"
```

### Connecting PlayWriter

After Chrome opens, the user must click the **PlayWriter extension icon** in Chrome's toolbar to enable control. Without this, you'll get "Extension not connected" errors.

### Viewing the Dashboard

```javascript
// Navigate and get accessibility snapshot
await page.goto('https://drinkcrabigator.com/dashboard');
await page.waitForLoadState('load');
console.log(await accessibilitySnapshot({ page }));
```

The accessibility snapshot shows the dashboard structure including:
- Session list with state (thinking/permission/complete), path, and session ID
- Screen preview showing the terminal output
- Stats widget (session time, prompts, completions, tools)
- Changes widget

### Full Circle Test

You can view your own running session on the dashboard - the screen preview will show the conversation you're currently having, creating a recursive view of yourself.

### Chrome MCP Auto-Login

The Chrome MCP controls an isolated Chrome instance without cookies. To authenticate:

1. **Generate a pairing code:**
   ```bash
   cargo run --release -- pair
   # Or if crabigator is in PATH:
   crabigator pair
   ```
   This outputs a code like `ABC-DEF-GHI`

2. **Navigate to the dashboard with the setup parameter:**
   ```
   https://drinkcrabigator.com/dashboard?setup=ABC-DEF-GHI
   ```

The dashboard auto-claims the code and authenticates. Codes expire in 5 minutes and can only be claimed once. The `pair` command automatically validates cached codes and generates new ones when needed.

**Complete Claude Code workflow:**
```bash
# 1. Get pairing code (automatically validates cache)
cargo run --release -- pair
# Output: VQ6-NBP-EPM

# 2. Use Chrome MCP to navigate (replace with actual code)
mcp__chrome-devtools__navigate_page(url: "https://drinkcrabigator.com/dashboard?setup=VQ6-NBP-EPM", type: "url")

# 3. Take snapshot to verify authentication worked
mcp__chrome-devtools__take_snapshot()
# Should show session list, not "Setup Failed"
```

### Troubleshooting

- **"Invalid or expired pairing token"**: Run `crabigator pair` again to get a fresh code
- **"Extension not connected"**: User needs to click the Chrome DevTools MCP extension icon in Chrome's toolbar
- **Connection errors**: The MCP server may need to be restarted
- **No pages**: Ask user to restart Chrome (known Chrome bug)

## E2E Testing with tmux

Claude Code can spawn and control Crabigator instances for end-to-end testing using tmux. This allows testing dashboard visual output by generating real Claude Code sessions.

### Basic Workflow

```bash
# 1. Start crabigator in a detached tmux session
tmux new-session -d -s crab "cd /path/to/test/project && crabigator"

# 2. Wait for startup
sleep 3

# 3. Send a prompt
tmux send-keys -t crab "explain this codebase" Enter

# 4. Wait for processing, then inspect via Chrome MCP or local files
sleep 5
cat /tmp/crabigator-*/screen.txt
cat /tmp/crabigator-*/scrollback.log

# 5. Clean up
tmux kill-session -t crab
```

### Special Keys

```bash
# Shift+Tab (mode cycling)
tmux send-keys -t crab Escape "[" "Z"

# Escape
tmux send-keys -t crab Escape

# Tab
tmux send-keys -t crab Tab

# Arrow keys
tmux send-keys -t crab Up
tmux send-keys -t crab Down
```

### Inspecting Output

**Local files** (in `/tmp/crabigator-*/`):
- `screen.txt` - Current terminal snapshot with ANSI codes
- `scrollback.log` - Conversation transcript
- `inspect.json` - Widget state

**Dashboard** (via Chrome MCP):
1. Navigate to `https://drinkcrabigator.com/dashboard`
2. Use `take_snapshot` to get accessibility tree
3. Use `take_screenshot` for visual verification

**Single-session view** - Filter to one session for focused testing:
```
https://drinkcrabigator.com/dashboard?session=SESSION_ID
```
This hides Style/Account buttons and forces single-column layout.

### Example: Testing Plan Mode Display

```bash
# Start session
tmux new-session -d -s test "cd ~/test-project && crabigator"
sleep 3

# Get session ID for single-session dashboard view
SESSION_ID=$(cat /tmp/crabigator-*/inspect.json 2>/dev/null | grep -o '"session_id":"[^"]*"' | tail -1 | cut -d'"' -f4)
echo "Session: $SESSION_ID"

# Enter plan mode
tmux send-keys -t test "make a plan for adding authentication" Enter

# Wait for plan
sleep 10

# Check dashboard via Chrome MCP (single-session view)
# mcp__chrome-devtools__navigate_page(url: "https://drinkcrabigator.com/dashboard?session=$SESSION_ID")
# mcp__chrome-devtools__take_screenshot()

# Check local files
cat /tmp/crabigator-*/screen.txt
cat /tmp/crabigator-*/scrollback.log

# Cleanup
tmux kill-session -t test
```

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
