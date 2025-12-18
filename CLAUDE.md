# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Build Commands

```bash
cargo build          # Build the project
cargo build --release # Release build
cargo run            # Run the application
cargo check          # Quick type checking
cargo test           # Run tests
```

## What This Project Is

Crabigator is a Rust TUI wrapper around the Claude Code CLI. It spawns Claude Code in a PTY (pseudo-terminal) and adds status widgets below the Claude Code interface showing git status, file changes, and session statistics.

## Architecture

The application uses a **scroll region approach** to layer UI:
- Sets terminal scroll region (DECSTBM escape sequence) to confine Claude Code output to the top ~80% of the terminal
- Claude Code runs in a PTY and its output passes through untouched within that scroll region
- Status widgets are rendered below the scroll region using raw ANSI escape sequences
- No intermediate rendering library (ratatui was removed) - all drawing is done with direct escape codes

### Key Modules

- **app.rs**: Main application loop and layout management. Handles scroll region setup, event polling, status bar drawing, and PTY passthrough.
- **terminal/**: Terminal handling - `pty.rs` manages PTY via `portable-pty` (spawns `claude` CLI, handles I/O), `input.rs` handles keyboard input forwarding, `escape.rs` provides ANSI escape sequence utilities.
- **git/**: Git state tracking via `git status --porcelain` and `git diff`.
- **parsers/**: Language-specific diff parsers (Rust, TypeScript, Python, generic) that extract semantic information (functions, classes, etc.) from git diffs.
- **hooks/**: `ClaudeStats` for session time tracking and platform stats integration.
- **platforms/**: Platform-specific integrations (e.g., `claude_code.rs` for reading Claude Code's hook-generated stats files).
- **ui/**: Status bar rendering - `status_bar.rs` orchestrates layout, with `git.rs`, `changes.rs`, `stats.rs` for individual widgets.
- **mirror.rs**: Widget state mirroring for external inspection. Publishes throttled JSON snapshots of all widget state.
- **inspect.rs**: Inspect command implementation for viewing other running crabigator instances.
- **capture.rs**: Output capture for streaming. Writes raw PTY bytes to stream.log and periodic screen snapshots to screen.txt.

### Input Handling

- All keyboard input forwards directly to the PTY
- Option/Alt key combinations are properly encoded for word navigation (Option+Left/Right) and word deletion (Option+Backspace/Delete)
- When Claude Code exits, Crabigator exits automatically

### Terminal Considerations

- Uses primary screen buffer (not alternate screen) to preserve native scrollback
- Mouse capture is disabled to allow native text selection
- Bracketed paste is enabled for efficient paste handling
- Panic handler restores terminal state to prevent corruption

### Output Capture

Crabigator captures Claude Code output for streaming and inspection. At startup, a banner shows file paths.

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
