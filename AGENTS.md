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
cd workers/crabigator-api && npm run dev      # Local dev
cd workers/crabigator-api && npm run typecheck
```

### Key Notes

- **Dashboard**: Inline HTML in `dashboard.ts` with `ansiToHtml()` for terminal rendering
- **256-color**: Uses xterm formula `value = idx === 0 ? 0 : idx * 40 + 55`
- **Deploys break WebSockets**: Desktop auto-reconnects with exponential backoff (1s-30s)
- **Session state**: Managed by Durable Objects (`SessionDO`)
- **Auth**: Desktop device_id + HMAC-SHA256 signatures, no user accounts
