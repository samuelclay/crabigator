# 🦀 Crabigator

A Rust TUI wrapper for [Claude Code](https://claude.ai/code) that adds real-time status widgets without interfering with Claude's interface.

![Rust](https://img.shields.io/badge/rust-1.70%2B-orange)
![License](https://img.shields.io/badge/license-MIT-blue)

## Quick Start

```bash
# Clone the repository
git clone https://github.com/samuelclay/crabigator.git
cd crabigator

# Build and run
cargo build --release
./target/release/crabigator
```

**Prerequisites:**

- Rust 1.70+
- Claude Code CLI installed and authenticated (`claude` command available)

## Key Features

- 🖥️ **Transparent PTY Passthrough** - Claude Code runs exactly as normal; all input/output passes through untouched
- 📊 **Git Status Widget** - See modified, added, and deleted files at a glance
- 📝 **Semantic Diff Summary** - View changes organized by functions, classes, and structs (supports Rust, TypeScript, Python)
- ⏱️ **Session Statistics** - Track idle time, work time, token usage, and message count
- 🎨 **Native Terminal Experience** - Preserves scrollback, text selection, and clipboard
- ⚡ **Zero Interference** - Status widgets render below Claude Code using terminal scroll regions

## How It Works

Crabigator spawns Claude Code in a pseudo-terminal (PTY) and constrains its output to the top portion of your terminal using ANSI scroll region escape sequences. The bottom 20% displays status widgets drawn with raw escape codes, giving you visibility into your session without disrupting Claude's UI.

```
┌─────────────────────────────────────┐
│                                     │
│         Claude Code (PTY)           │
│                                     │
├─────────────────────────────────────┤
│ Stats │ Git       │ Changes         │
└─────────────────────────────────────┘
```

## Keyboard Shortcuts

All keyboard input is passed directly to Claude Code. Option/Alt key combinations work as expected for word navigation and deletion.

When Claude Code exits (via `/exit` or Ctrl+C), Crabigator exits automatically.

## Future Features

We're considering these enhancements based on community interest:

- 📱 **Mobile Prompting** - Send prompts from your phone that run on your desktop Claude Code
- 📲 **Remote Session Monitoring** - Check on Claude's progress from anywhere via mobile web interface
- 🔔 **Notifications** - Desktop and mobile notifications when Claude finishes or needs your input
- 💬 **Mobile Response to Prompts** - Answer Claude's questions from your phone when you're away from your desk
- 💾 **Session Persistence** - Save and resume sessions with full context
- 🎛️ **Configurable Layouts** - Choose which widgets to display and their positions

Have a feature request? [Open an issue](https://github.com/samuelclay/crabigator/issues)!

## Why "Crabigator"?

🦀 (Rust's crab) + 🐊 (because we're wrapping Claude like an alligator[1])

And don't forget Clawd and Netscape Navigator.

## Contributing

Contributions welcome! Please feel free to submit a Pull Request.

## License

MIT License - see [LICENSE](LICENSE) for details.

[1] RIP my albino friend. We had membership to Cal Academy for the past two years and miss him terribly.
