# <img src="assets/crab.svg" width="32" height="32" alt="Crabigator"> Crabigator

Control Claude Code from anywhere. Answer prompts from your phone while Claude runs on your desktop.

[![npm version](https://img.shields.io/npm/v/crabigator)](https://www.npmjs.com/package/crabigator)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

<p align="center">
  <img src="assets/hero-screenshot.png" alt="Crabigator running Claude Code with status widgets" width="800">
</p>

## What is Crabigator?

Crabigator is a terminal wrapper that runs Claude Code (or Codex CLI) with real-time status widgets and **remote control from your phone**. Claude runs natively on your machine exactly as intended, while Crabigator streams the session to a web dashboard where you can:

- <img src="assets/shield-check.svg" width="16" height="16"> **Answer permission requests** — Approve file writes, command execution, and tool use from anywhere
- <img src="assets/chat-dots.svg" width="16" height="16"> **Respond to questions** — When Claude asks for clarification, reply from your phone
- <img src="assets/eye.svg" width="16" height="16"> **Monitor progress** — Watch Claude think, see what files it's reading, track token usage
- <img src="assets/bell.svg" width="16" height="16"> **Stay in the loop** — Get notified when Claude needs your input

## Installation

### npm (recommended)

```bash
npm install -g crabigator
```

### Cargo (from source)

```bash
git clone https://github.com/samuelclay/crabigator.git
cd crabigator
cargo install --path .
```

### Prerequisites

- **Claude Code** or **Codex CLI** installed and authenticated
- Node.js 18+ (for npm install) or Rust 1.70+ (for cargo install)
- macOS, Linux, or Windows (WSL)

## Quick Start

```bash
# Run with Claude Code (default)
crabigator

# Or explicitly specify the platform
crabigator claude
crabigator codex
```

The first time you run Crabigator, it will prompt you to pair with your phone:

1. A pairing code appears in your terminal (e.g., `ABC-DEF-GHI`)
2. Open [drinkcrabigator.com/dashboard](https://drinkcrabigator.com/dashboard) on your phone
3. Enter the pairing code to connect

Once paired, your sessions automatically stream to the dashboard.

## Features

### <img src="assets/device-mobile.svg" width="20" height="20"> Remote Control

Answer Claude's prompts from your phone when you're away from your desk. Permission requests, questions, and plan approvals all work remotely.

### <img src="assets/chart-line-up.svg" width="20" height="20"> Status Widgets

Real-time widgets below Claude's interface show:

- <img src="assets/clock.svg" width="14" height="14"> **Session stats** — Time elapsed, prompts sent, tokens used
- <img src="assets/folder.svg" width="14" height="14"> **Git status** — Modified, added, and deleted files
- <img src="assets/dna.svg" width="14" height="14"> **Semantic diff** — Changes organized by functions and classes

### <img src="assets/terminal.svg" width="20" height="20"> Native Terminal Experience

- <img src="assets/check-circle.svg" width="14" height="14"> Claude Code runs in a PTY exactly as normal
- <img src="assets/check-circle.svg" width="14" height="14"> Full scrollback history preserved
- <img src="assets/check-circle.svg" width="14" height="14"> Native text selection and clipboard
- <img src="assets/check-circle.svg" width="14" height="14"> All keyboard shortcuts work (Option+Arrow, etc.)

### <img src="assets/screens.svg" width="20" height="20"> Multi-Platform

Supports both [Claude Code](https://claude.ai/code) (Anthropic) and [Codex CLI](https://github.com/openai/codex) (OpenAI).

## How It Works

```
┌─────────────────────────────────────┐
│                                     │
│         Claude Code (PTY)           │  ← Runs exactly as normal
│                                     │
├─────────────────────────────────────┤
│ Stats │ Git Status │ File Changes   │  ← Status widgets
└─────────────────────────────────────┘
                │
                ▼
        ┌───────────────┐
        │  Cloud Relay  │  ← Streams to drinkcrabigator.com
        └───────────────┘
                │
                ▼
        ┌───────────────┐
        │ Your Phone    │  ← Answer prompts remotely
        └───────────────┘
```

Crabigator spawns Claude Code in a pseudo-terminal and uses ANSI scroll region escape sequences to confine its output to the top portion of your terminal. Status widgets render below using raw escape codes. Session state streams to Cloudflare Workers via WebSocket for the mobile dashboard.

## Architecture

The codebase is organized into focused modules:

### <img src="assets/cube.svg" width="18" height="18"> Core

| Module | Description |
|--------|-------------|
| [`src/app.rs`](src/app.rs) | Main application loop, scroll region management, event handling |
| [`src/main.rs`](src/main.rs) | CLI entry point, argument parsing, session initialization |
| [`src/config.rs`](src/config.rs) | Configuration loading/saving (`~/.crabigator/config.toml`) |

### <img src="assets/terminal.svg" width="18" height="18"> Terminal

| Module | Description |
|--------|-------------|
| [`src/terminal/pty.rs`](src/terminal/pty.rs) | PTY management via `portable-pty`, spawns Claude/Codex |
| [`src/terminal/input.rs`](src/terminal/input.rs) | Keyboard input forwarding with Option/Alt key encoding |
| [`src/terminal/escape.rs`](src/terminal/escape.rs) | ANSI escape sequence definitions (colors, cursor, scroll regions) |

### <img src="assets/browser.svg" width="18" height="18"> User Interface

| Module | Description |
|--------|-------------|
| [`src/ui/status_bar.rs`](src/ui/status_bar.rs) | Main status bar layout and rendering |
| [`src/ui/stats.rs`](src/ui/stats.rs) | Session statistics widget (time, tokens, prompts) |
| [`src/ui/git.rs`](src/ui/git.rs) | Git status widget |
| [`src/ui/changes.rs`](src/ui/changes.rs) | File changes widget with semantic diff |
| [`src/ui/pairing.rs`](src/ui/pairing.rs) | Pairing code display for mobile setup |

### <img src="assets/plugs-connected.svg" width="18" height="18"> Platform Integrations

| Module | Description |
|--------|-------------|
| [`src/platforms/claude_code.rs`](src/platforms/claude_code.rs) | Claude Code hooks and session stats |
| [`src/platforms/codex_cli.rs`](src/platforms/codex_cli.rs) | Codex CLI log parsing |

### <img src="assets/code.svg" width="18" height="18"> Language Parsers

| Module | Description |
|--------|-------------|
| [`src/parsers/rust.rs`](src/parsers/rust.rs) | Rust semantic diff (functions, structs, impls) |
| [`src/parsers/typescript.rs`](src/parsers/typescript.rs) | TypeScript/JavaScript parsing |
| [`src/parsers/python.rs`](src/parsers/python.rs) | Python parsing |
| [`src/parsers/generic.rs`](src/parsers/generic.rs) | Fallback for other languages |

### <img src="assets/cloud.svg" width="18" height="18"> Cloud Integration

| Module | Description |
|--------|-------------|
| [`src/cloud/`](src/cloud/) | WebSocket client, authentication, session streaming |
| [`src/mirror.rs`](src/mirror.rs) | Widget state serialization for external inspection |
| [`src/capture.rs`](src/capture.rs) | Terminal output capture for streaming |

### <img src="assets/globe.svg" width="18" height="18"> Cloud Backend

| Module | Description |
|--------|-------------|
| [`workers/crabigator-api/`](workers/crabigator-api/) | Cloudflare Workers backend |
| [`workers/crabigator-api/src/index.ts`](workers/crabigator-api/src/index.ts) | API routes and WebSocket handling |
| [`workers/crabigator-api/src/session-do.ts`](workers/crabigator-api/src/session-do.ts) | Durable Object for session state |
| [`workers/crabigator-api/src/dashboard.ts`](workers/crabigator-api/src/dashboard.ts) | Mobile dashboard HTML/CSS/JS |

## Commands

```bash
crabigator              # Start with default platform (Claude Code)
crabigator claude       # Use Claude Code explicitly
crabigator codex        # Use Codex CLI
crabigator pair         # Generate a new pairing code
crabigator inspect      # View other running instances
crabigator --help       # Show all options
```

## Session Files

Each session creates `/tmp/crabigator-{session_id}/` containing:

- <img src="assets/scroll.svg" width="14" height="14"> `scrollback.log` — Clean text transcript (ANSI stripped)
- <img src="assets/desktop.svg" width="14" height="14"> `screen.txt` — Current screen snapshot
- <img src="assets/file-code.svg" width="14" height="14"> `mirror.json` — Widget state for external tools

## Configuration

Crabigator stores preferences in `~/.crabigator/config.toml`:

```toml
platform = "claude"  # or "codex"
```

Claude Code hooks are installed to `~/.claude/crabigator/` for tracking session state and statistics.

## Why "Crabigator"?

<img src="assets/crab.svg" width="18" height="18"> It's a quadruple wordplay:

- **Claude** — The AI we're wrapping
- **Navigator** — It navigates and controls your Claude Code sessions
- **Crab** — Rust's mascot is Ferris the crab
- **Alligator** — Named after the late Claude, the beloved albino alligator at the California Academy of Sciences

## Contributing

Contributions welcome! Please feel free to submit a Pull Request.

```bash
# Development
cargo build              # Debug build
cargo build --release    # Release build
cargo test               # Run tests
cargo clippy             # Lint

# Cloud development
cd workers/crabigator-api
npm run dev              # Local dev server
```

## License

MIT License - see [LICENSE](LICENSE) for details.

---

<p align="center">
  <a href="https://drinkcrabigator.com">drinkcrabigator.com</a> ·
  <a href="https://github.com/samuelclay/crabigator/issues">Report a Bug</a> ·
  <a href="https://github.com/samuelclay/crabigator/issues">Request a Feature</a>
</p>
