# Crabigator

Control Claude Code from anywhere. Answer prompts from your phone while Claude runs on your desktop.

[![npm version](https://img.shields.io/npm/v/crabigator)](https://www.npmjs.com/package/crabigator)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)](https://www.rust-lang.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](LICENSE)

<p align="center">
  <img src="https://drinkcrabigator.com/hero-screenshot.png" alt="Crabigator running Claude Code with status widgets" width="800">
</p>

## What is Crabigator?

Crabigator is a terminal wrapper that runs Claude Code (or Codex CLI) with real-time status widgets and **remote control from your phone**. Claude runs natively on your machine exactly as intended, while Crabigator streams the session to a web dashboard where you can:

- **Answer permission requests** - Approve file writes, command execution, and tool use from anywhere
- **Respond to questions** - When Claude asks for clarification, reply from your phone
- **Monitor progress** - Watch Claude think, see what files it's reading, track token usage
- **Stay in the loop** - Get notified when Claude needs your input

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

### Remote Control

Answer Claude's prompts from your phone when you're away from your desk. Permission requests, questions, and plan approvals all work remotely.

### Status Widgets

Real-time widgets below Claude's interface show:

- **Session stats** - Time elapsed, prompts sent, tokens used
- **Git status** - Modified, added, and deleted files
- **Semantic diff** - Changes organized by functions and classes

### Native Terminal Experience

- Claude Code runs in a PTY exactly as normal
- Full scrollback history preserved
- Native text selection and clipboard
- All keyboard shortcuts work (Option+Arrow, etc.)

### Multi-Platform

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

### Core

| Module | Description |
|--------|-------------|
| [`src/app.rs`](src/app.rs) | Main application loop, scroll region management, event handling |
| [`src/main.rs`](src/main.rs) | CLI entry point, argument parsing, session initialization |
| [`src/config.rs`](src/config.rs) | Configuration loading/saving (`~/.crabigator/config.toml`) |

### Terminal

| Module | Description |
|--------|-------------|
| [`src/terminal/pty.rs`](src/terminal/pty.rs) | PTY management via `portable-pty`, spawns Claude/Codex |
| [`src/terminal/input.rs`](src/terminal/input.rs) | Keyboard input forwarding with Option/Alt key encoding |
| [`src/terminal/escape.rs`](src/terminal/escape.rs) | ANSI escape sequence definitions (colors, cursor, scroll regions) |

### User Interface

| Module | Description |
|--------|-------------|
| [`src/ui/status_bar.rs`](src/ui/status_bar.rs) | Main status bar layout and rendering |
| [`src/ui/stats.rs`](src/ui/stats.rs) | Session statistics widget (time, tokens, prompts) |
| [`src/ui/git.rs`](src/ui/git.rs) | Git status widget |
| [`src/ui/changes.rs`](src/ui/changes.rs) | File changes widget with semantic diff |
| [`src/ui/pairing.rs`](src/ui/pairing.rs) | Pairing code display for mobile setup |

### Platform Integrations

| Module | Description |
|--------|-------------|
| [`src/platforms/claude_code.rs`](src/platforms/claude_code.rs) | Claude Code hooks and session stats |
| [`src/platforms/codex_cli.rs`](src/platforms/codex_cli.rs) | Codex CLI log parsing |

### Language Parsers

| Module | Description |
|--------|-------------|
| [`src/parsers/rust.rs`](src/parsers/rust.rs) | Rust semantic diff (functions, structs, impls) |
| [`src/parsers/typescript.rs`](src/parsers/typescript.rs) | TypeScript/JavaScript parsing |
| [`src/parsers/python.rs`](src/parsers/python.rs) | Python parsing |
| [`src/parsers/generic.rs`](src/parsers/generic.rs) | Fallback for other languages |

### Cloud Integration

| Module | Description |
|--------|-------------|
| [`src/cloud/`](src/cloud/) | WebSocket client, authentication, session streaming |
| [`src/mirror.rs`](src/mirror.rs) | Widget state serialization for external inspection |
| [`src/capture.rs`](src/capture.rs) | Terminal output capture for streaming |

### Cloud Backend

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

- `scrollback.log` - Clean text transcript (ANSI stripped)
- `screen.txt` - Current screen snapshot
- `mirror.json` - Widget state for external tools

## Configuration

Crabigator stores preferences in `~/.crabigator/config.toml`:

```toml
platform = "claude"  # or "codex"
```

Claude Code hooks are installed to `~/.claude/crabigator/` for tracking session state and statistics.

## Why "Crabigator"?

Rust's mascot is a crab. We're wrapping Claude like an alligator wraps its prey. Plus: Clawd + Navigator.

RIP to Cal Academy's albino alligator Claude. We had membership for two years and miss him terribly.

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
