//! Python hook script for Claude Code stats tracking
//!
//! The hook script handles Claude Code events and writes session stats
//! to a JSON file that crabigator reads for its stats widget.

/// Current hook version - should match Cargo.toml version
pub const HOOK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Python hook script content
///
/// Handles these Claude Code events:
/// - PermissionRequest: Permission dialog shown
/// - PostToolUse: Tool execution completed
/// - Stop: Claude finished responding
/// - SubagentStop: Subagent task completed
/// - PreCompact: Context compression triggered
/// - UserPromptSubmit: User submitted input
pub const HOOK_SCRIPT: &str = r#"#!/usr/bin/env python3
"""
Crabigator stats hook for Claude Code
Handles: PermissionRequest, PostToolUse, Stop, SubagentStop, PreCompact, UserPromptSubmit

State machine:
  - ready: Initial state (nothing happened yet)
  - thinking: Claude is actively processing
  - permission: Claude is waiting for permission approval
  - question: Claude asked a question (AskUserQuestion tool)
  - complete: Claude finished responding
"""
# crabigator-hook-version: {VERSION}

import json
import hashlib
import os
import sys
import time
from pathlib import Path

def get_stats_file(cwd: str) -> Path:
    """Get stats file path based on session ID (from env) or working directory hash."""
    session_id = os.environ.get("CRABIGATOR_SESSION_ID")
    if session_id:
        return Path(f"/tmp/crabigator-stats-{session_id}.json")
    # Fallback to cwd hash if no session ID
    cwd_hash = hashlib.md5(cwd.encode()).hexdigest()[:12]
    return Path(f"/tmp/crabigator-stats-{cwd_hash}.json")

def load_stats(stats_file: Path) -> dict:
    """Load existing stats or return defaults."""
    if stats_file.exists():
        try:
            with open(stats_file) as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            pass
    return {
        "prompts": 0,
        "completions": 0,
        "subagent_messages": 0,
        "compressions": 0,
        "tools": {},
        "tool_timestamps": [],
        "state": "ready",
        "pending_question": False,
        "idle_since": None,
        "last_updated": None,
    }

def save_stats(stats_file: Path, stats: dict):
    """Atomically save stats to file."""
    stats["last_updated"] = time.time()

    # Write to temp file then rename for atomicity
    # Use unique temp file name to avoid race conditions between concurrent hooks
    temp_file = stats_file.with_suffix(f'.{os.getpid()}.tmp')
    try:
        with open(temp_file, 'w') as f:
            json.dump(stats, f)
        temp_file.rename(stats_file)
    except OSError:
        # If rename fails, try to clean up temp file
        try:
            temp_file.unlink(missing_ok=True)
        except Exception:
            pass

def main():
    try:
        data = json.load(sys.stdin)
    except json.JSONDecodeError:
        sys.exit(0)

    cwd = data.get("cwd", os.getcwd())
    event = data.get("hook_event_name", "")

    stats_file = get_stats_file(cwd)
    stats = load_stats(stats_file)

    if event == "PermissionRequest":
        # Permission dialog is being shown to user
        stats["state"] = "permission"

    elif event == "PostToolUse":
        tool_name = data.get("tool_name", "unknown")
        stats["tools"][tool_name] = stats["tools"].get(tool_name, 0) + 1
        if "tool_timestamps" not in stats:
            stats["tool_timestamps"] = []
        stats["tool_timestamps"].append(time.time())
        # Mark if this was a question tool
        if tool_name == "AskUserQuestion":
            stats["pending_question"] = True
        # Tool completed - back to thinking (more tools may follow)
        stats["state"] = "thinking"

    elif event == "Stop":
        stats["completions"] = stats.get("completions", 0) + 1
        # Transition to question or complete based on pending flag
        if stats.get("pending_question"):
            stats["state"] = "question"
            stats["pending_question"] = False
        else:
            stats["state"] = "complete"
        # Start idle timer
        stats["idle_since"] = time.time()

    elif event == "SubagentStop":
        stats["subagent_messages"] += 1

    elif event == "PreCompact":
        stats["compressions"] += 1

    elif event == "UserPromptSubmit":
        # User submitted input, Claude starts thinking
        stats["prompts"] = stats.get("prompts", 0) + 1
        stats["state"] = "thinking"
        stats["pending_question"] = False
        stats["idle_since"] = None

    save_stats(stats_file, stats)
    sys.exit(0)

if __name__ == "__main__":
    main()
"#;

/// Get the hook script content with version embedded
pub fn script_with_version() -> String {
    HOOK_SCRIPT.replace("{VERSION}", HOOK_VERSION)
}
