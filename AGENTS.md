# AGENTS.md

This file provides guidance to AI coding assistants when working with code in this repository.

## Build Commands

```bash
cargo build          # Build the project
cargo build --release # Release build
cargo check          # Quick type checking
cargo test           # Run tests
cargo clippy         # Lint
```

## Running

```bash
make                 # Run with last-used platform (default: claude)
make claude          # Run with Claude Code (saves preference)
make codex           # Run with Codex (saves preference)
make resume          # Resume last session
make continue        # Continue last conversation
make test-update     # Update fixture snapshots
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

Crabigator is a Rust TUI wrapper around AI coding assistant CLIs (Claude Code and Codex). It spawns the CLI in a PTY (pseudo-terminal) and adds status widgets below the interface showing git status, file changes, and session statistics.

### Multi-Platform Support

Crabigator supports multiple AI CLI platforms:
- **Claude Code** (Anthropic) - default
- **Codex** (OpenAI)

Platform selection:
```bash
crabigator              # Uses last-used platform (default: claude)
crabigator codex        # Use Codex
crabigator claude       # Use Claude Code
crabigator --platform codex  # Explicit flag
```

Platform preference is saved in `~/.crabigator/config.toml`.

## Architecture

The application uses a **scroll region approach** to layer UI:
- Sets terminal scroll region (DECSTBM escape sequence) to confine CLI output to the top ~80% of the terminal
- The AI CLI runs in a PTY and its output passes through untouched within that scroll region
- Status widgets are rendered below the scroll region using raw ANSI escape sequences
- No intermediate rendering library (ratatui was removed) - all drawing is done with direct escape codes

### Key Modules

- **app.rs**: Main application loop and layout management. Handles scroll region setup, event polling, status bar drawing, and PTY passthrough.
- **config.rs**: Configuration loading/saving for `~/.crabigator/config.toml` (platform preferences).
- **terminal/**: Terminal handling - `pty.rs` manages PTY via `portable-pty` (spawns CLI, handles I/O), `input.rs` handles keyboard input forwarding, `escape.rs` provides ANSI escape sequence utilities.
- **git/**: Git state tracking via `git status --porcelain` and `git diff`.
- **parsers/**: Language-specific diff parsers (Rust, TypeScript, Python, generic) that extract semantic information (functions, classes, etc.) from git diffs.
- **hooks/**: `SessionStats` for session time tracking and platform stats integration.
- **platforms/**: Platform abstraction layer with `Platform` trait. Implementations:
  - `claude_code.rs`: Claude Code hooks and stats (writes to `~/.claude/crabigator/`)
  - `codex.rs`: Codex notify hooks and stats (writes to `~/.codex/crabigator/`)
- **ui/**: Status bar rendering - `status_bar.rs` orchestrates layout, with `git.rs`, `changes.rs`, `stats.rs` for individual widgets.
- **mirror.rs**: Widget state mirroring for external inspection. Publishes throttled JSON snapshots of all widget state.
- **inspect.rs**: Inspect command implementation for viewing other running crabigator instances.
- **capture.rs**: Output capture for streaming. Writes raw PTY bytes to stream.log and periodic screen snapshots to screen.txt.

### Input Handling

- All keyboard input forwards directly to the PTY
- Option/Alt key combinations are properly encoded for word navigation (Option+Left/Right) and word deletion (Option+Backspace/Delete)
- When the CLI exits, Crabigator exits automatically

### Terminal Considerations

- Uses primary screen buffer (not alternate screen) to preserve native scrollback
- Mouse capture is disabled to allow native text selection
- Bracketed paste is enabled for efficient paste handling
- Panic handler restores terminal state to prevent corruption

### Output Capture

Crabigator captures CLI output for streaming and inspection. At startup, a banner shows file paths.

Files created in `/tmp/crabigator-capture-{session_id}/`:
- **scrollback.log**: Clean text transcript (append-only). Only complete lines are written - animations/spinners using carriage return are filtered out. ANSI escape sequences are stripped.
- **screen.txt**: Current screen snapshot from vt100 parser. Updated every ~100ms. Atomic writes prevent partial reads.

Use `--no-capture` to disable capture.

### Instance Inspection

Every running crabigator instance writes widget state to `/tmp/crabigator-mirror-{session_id}.json`:
- Throttled to max once per second
- Only writes when widget content has changed (hash-based detection)
- Contains both raw data and pre-rendered text for each widget
- File is cleaned up on exit

Use `crabigator inspect` to view other running instances:
- `crabigator inspect` - list all instances
- `crabigator inspect /path` - filter by working directory
- `crabigator inspect --watch` - continuous monitoring
- `crabigator inspect --raw` - output raw JSON
