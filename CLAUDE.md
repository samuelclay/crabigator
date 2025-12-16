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

- **app_v2.rs**: Main application loop and layout management. Handles scroll region setup, event polling, status bar drawing, and PTY passthrough.
- **pty/claude.rs**: PTY management using `portable-pty`. Spawns the `claude` CLI, handles input/output channels, and terminal resizing.
- **git/**: Git state tracking via `git status --porcelain` and `git diff`.
- **parsers/**: Language-specific diff parsers (Rust, TypeScript, Python, generic) that extract semantic information (functions, classes, etc.) from git diffs.
- **hooks/**: Session statistics tracking (idle time, work time, tokens, message count).

### Input Handling

- All keyboard input forwards directly to the PTY
- Option/Alt key combinations are properly encoded for word navigation (Option+Left/Right) and word deletion (Option+Backspace/Delete)
- When Claude Code exits, Crabigator exits automatically

### Terminal Considerations

- Uses primary screen buffer (not alternate screen) to preserve native scrollback
- Mouse capture is disabled to allow native text selection
- Bracketed paste is enabled for efficient paste handling
- Panic handler restores terminal state to prevent corruption
