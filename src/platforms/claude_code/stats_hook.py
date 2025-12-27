#!/usr/bin/env python3
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

# Maximum number of events to keep in history
MAX_EVENT_HISTORY = 100

def debug_log(session_id: str, message: str):
    """Write debug message to hook log file."""
    if not session_id:
        return
    try:
        log_path = Path(f"/tmp/crabigator-{session_id}/hooks.log")
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with open(log_path, 'a') as f:
            f.write(f"{time.time():.3f} {message}\n")
    except Exception:
        pass  # Silently ignore logging errors

def get_stats_file(cwd: str) -> Path:
    """Get stats file path based on session ID (from env) or working directory hash."""
    session_id = os.environ.get("CRABIGATOR_SESSION_ID")
    if session_id:
        return Path(f"/tmp/crabigator-stats-{session_id}.json")
    # Fallback to cwd hash if no session ID
    cwd_hash = hashlib.md5(cwd.encode()).hexdigest()[:12]
    return Path(f"/tmp/crabigator-stats-{cwd_hash}.json")

def add_event(stats: dict, event: str, details: dict = None):
    """Add an event to the history log with timestamp."""
    if "event_history" not in stats:
        stats["event_history"] = []

    entry = {
        "ts": time.time(),
        "event": event,
        "state_before": stats.get("state", "ready"),
    }
    if details:
        entry["details"] = details

    stats["event_history"].append(entry)

    # Keep only the last N events
    if len(stats["event_history"]) > MAX_EVENT_HISTORY:
        stats["event_history"] = stats["event_history"][-MAX_EVENT_HISTORY:]

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
    session_id = os.environ.get("CRABIGATOR_SESSION_ID", "")
    try:
        data = json.load(sys.stdin)
    except json.JSONDecodeError as e:
        debug_log(session_id, f"JSON decode error: {e}")
        sys.exit(0)

    cwd = data.get("cwd", os.getcwd())
    event = data.get("hook_event_name", "")

    debug_log(session_id, f"EVENT: {event} cwd={cwd}")

    stats_file = get_stats_file(cwd)
    stats = load_stats(stats_file)

    debug_log(session_id, f"  state_before={stats.get('state', 'ready')} file={stats_file}")

    if event == "PermissionRequest":
        # Permission dialog is being shown to user
        tool_name = data.get("tool_name", "unknown")
        add_event(stats, event, {"tool": tool_name})
        stats["state"] = "permission"

    elif event == "PostToolUse":
        tool_name = data.get("tool_name", "unknown")
        add_event(stats, event, {"tool": tool_name})
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
        add_event(stats, event, {"pending_question": stats.get("pending_question", False)})
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
        add_event(stats, event)
        stats["subagent_messages"] += 1

    elif event == "PreCompact":
        add_event(stats, event)
        stats["compressions"] += 1

    elif event == "UserPromptSubmit":
        # User submitted input, Claude starts thinking
        add_event(stats, event)
        stats["prompts"] = stats.get("prompts", 0) + 1
        stats["state"] = "thinking"
        stats["pending_question"] = False
        stats["idle_since"] = None

    else:
        # Log unhandled events for debugging
        add_event(stats, event, {"unhandled": True})

    debug_log(session_id, f"  state_after={stats.get('state', 'ready')}")
    save_stats(stats_file, stats)
    debug_log(session_id, f"  saved to {stats_file}")
    sys.exit(0)

if __name__ == "__main__":
    main()
