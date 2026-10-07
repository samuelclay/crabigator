#!/usr/bin/env python3
"""
Crabigator stats hook for Claude Code
Registered for every hook event Claude Code offers, except WorktreeCreate and
WorktreeRemove (a hook there replaces git's own worktree handling) and
FileChanged (it only watches files a hook names).

Every event appends one line to the session's activity log
(/tmp/crabigator-{session}/activity.jsonl), which the flow column reads as it
happens. Events that move the session's state or counters also update the
stats file.

State machine:
  - ready: Initial state (nothing happened yet)
  - thinking: Claude is actively processing
  - permission: Claude is waiting for permission approval
  - question: Claude asked a question (AskUserQuestion tool)
  - complete: Claude finished responding

A subagent's events (they carry `agent_id`) never end or restart the main
turn: a background agent working after the turn leaves the session complete.
"""
# crabigator-hook-version: {VERSION}

import json
import hashlib
import os
import sys
import time
from contextlib import contextmanager
from pathlib import Path

try:
    import fcntl
except ImportError:  # Windows: no advisory locks
    fcntl = None

# Maximum number of events to keep in history
MAX_EVENT_HISTORY = 100

# Events that change the session's state or counters, so they update the
# stats file. Every other event only goes to the activity log: any stats write
# clears crabigator's "interrupted" state, so an event that can land after an
# interrupt (an idle notification, a tool cut short) must not write it.
STATS_EVENTS = {
    "SessionStart",
    "UserPromptSubmit",
    "PermissionRequest",
    "PostToolUse",
    "PostToolUseFailure",
    "Stop",
    "StopFailure",
    "SubagentStop",
    "PreCompact",
    "PostModelSwitch",
}

# Events that fire before every tool call or with every streamed batch of
# lines: hooks.log gets a one-line summary instead of the raw input.
FREQUENT_EVENTS = {"PreToolUse", "PostToolBatch", "MessageDisplay"}


def debug_log(session_id: str, message: str):
    """Write debug message to hook log file."""
    if not session_id:
        return
    try:
        log_path = Path(f"/tmp/crabigator-{session_id}/hooks.log")
        log_path.parent.mkdir(parents=True, exist_ok=True)
        with open(log_path, "a") as f:
            f.write(f"{time.time():.3f} {message}\n")
    except Exception:
        pass  # Silently ignore logging errors


def activity_log_path(session_id: str) -> Path:
    return Path(f"/tmp/crabigator-{session_id}/activity.jsonl")


def count_lines(text) -> int:
    """Lines in a tool's new text (flow's countLines)."""
    return text.count("\n") + 1 if isinstance(text, str) else 0


def lines_written(tool: str, tool_input) -> int | None:
    """How many lines a write-ish tool call changes, or None for any other tool (flow's linesWritten)."""
    i = tool_input if isinstance(tool_input, dict) else {}
    if tool == "Write":
        return count_lines(i.get("content"))
    if tool == "Edit":
        return count_lines(i.get("new_string"))
    if tool == "MultiEdit":
        edits = i.get("edits")
        if not isinstance(edits, list):
            return 1
        return sum(count_lines(e.get("new_string")) for e in edits if isinstance(e, dict))
    if tool == "NotebookEdit":
        return count_lines(i.get("new_source"))
    return None


def activity_entry(event: str, data: dict) -> dict:
    """One activity log line: the event and a few small fields about it, never
    a tool's input or the streamed text itself."""
    entry = {"ts": round(time.time(), 3), "ev": event}
    if data.get("agent_id"):
        entry["agent"] = data["agent_id"]
    effort = data.get("effort")
    if isinstance(effort, dict) and effort.get("level"):
        entry["effort"] = effort["level"]
    if data.get("tool_name"):
        entry["tool"] = data["tool_name"]
    if data.get("tool_use_id"):
        entry["id"] = data["tool_use_id"]

    if event == "PreToolUse":
        lines = lines_written(data.get("tool_name", ""), data.get("tool_input"))
        if lines is not None:
            entry["lines"] = lines
    elif event == "PostToolUseFailure":
        if data.get("is_interrupt"):
            entry["interrupt"] = True
    elif event == "MessageDisplay":
        delta = data.get("delta")
        entry["chars"] = len(delta) if isinstance(delta, str) else 0
    elif event == "PostToolBatch":
        calls = data.get("tool_calls")
        entry["calls"] = len(calls) if isinstance(calls, list) else 0
    else:
        for key in ("agent_type", "error", "trigger", "source", "reason", "notification_type"):
            if data.get(key) is not None:
                entry[key] = data[key]
    return entry


def append_activity(session_id: str, entry: dict):
    """Append one line to the activity log. A single small append is atomic,
    so hooks running at once never interleave their lines."""
    if not session_id:
        return
    try:
        path = activity_log_path(session_id)
        path.parent.mkdir(parents=True, exist_ok=True)
        with open(path, "a") as f:
            f.write(json.dumps(entry, separators=(",", ":")) + "\n")
    except Exception:
        pass


def create_claude_session_symlink(crabigator_session_id: str, claude_session_id: str):
    """Create a symlink from Claude Code's session UUID to crabigator's session directory.

    This allows accessing /tmp/crabigator-{claude_uuid} which points to the actual
    /tmp/crabigator-{crabigator_id} directory, enabling correlation between the two.
    """
    if not crabigator_session_id or not claude_session_id:
        return

    # Don't create symlink if they're the same
    if crabigator_session_id == claude_session_id:
        return

    try:
        target = Path(f"/tmp/crabigator-{crabigator_session_id}")
        symlink = Path(f"/tmp/crabigator-{claude_session_id}")

        # Only create if target exists and symlink doesn't
        if target.exists() and not symlink.exists():
            symlink.symlink_to(target)
            debug_log(crabigator_session_id, f"Created symlink: {symlink} -> {target}")
    except Exception as e:
        debug_log(crabigator_session_id, f"Failed to create symlink: {e}")


def get_stats_file(cwd: str) -> Path:
    """Get stats file path based on session ID (from env) or working directory hash."""
    session_id = os.environ.get("CRABIGATOR_SESSION_ID")
    if session_id:
        return Path(f"/tmp/crabigator-stats-{session_id}.json")
    # Fallback to cwd hash if no session ID
    cwd_hash = hashlib.md5(cwd.encode()).hexdigest()[:12]
    return Path(f"/tmp/crabigator-stats-{cwd_hash}.json")


@contextmanager
def stats_lock(stats_file: Path):
    """Hold the stats file's lock while reading, changing and saving it: hooks
    for parallel tool calls run at once and would otherwise lose updates."""
    if fcntl is None:
        yield
        return
    try:
        lock = open(stats_file.with_suffix(".lock"), "a")
    except OSError:
        yield
        return
    with lock:
        try:
            fcntl.flock(lock, fcntl.LOCK_EX)
        except OSError:
            pass
        yield


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
        "model": None,
    }

def extract_cwd_from_transcript(transcript_path: str) -> str | None:
    """Restore the most recent directory recorded by the resumed conversation."""
    latest = None
    try:
        with open(transcript_path) as transcript:
            for line in transcript:
                try:
                    entry = json.loads(line)
                except json.JSONDecodeError:
                    continue
                if not isinstance(entry, dict) or entry.get("isSidechain"):
                    continue
                candidate = entry.get("cwd")
                if isinstance(candidate, str) and Path(candidate).is_absolute():
                    latest = candidate
    except (OSError, UnicodeError):
        return None
    return latest if latest and Path(latest).is_dir() else None


def extract_model_from_transcript(transcript_path: str) -> str | None:
    """Extract model name from transcript file (reads last few lines for efficiency)."""
    if not transcript_path:
        return None
    try:
        path = Path(transcript_path)
        if not path.exists():
            return None
        # Read last 50KB to find recent model info
        with open(path, 'rb') as f:
            f.seek(0, 2)  # End of file
            size = f.tell()
            f.seek(max(0, size - 50000))
            content = f.read().decode('utf-8', errors='ignore')

        # Find the last model reference
        model = None
        for line in content.split('\n'):
            if '"model":' in line and 'claude' in line:
                try:
                    data = json.loads(line)
                    if 'message' in data and 'model' in data['message']:
                        model = data['message']['model']
                except (json.JSONDecodeError, KeyError):
                    pass
        return model
    except Exception:
        return None


def api_model_name(model) -> str | None:
    """A hook's model name as the transcript writes it: "claude-opus-5-5[1m]" → "claude-opus-5-5"."""
    if not isinstance(model, str) or not model:
        return None
    return model.split("[", 1)[0]


def save_stats(stats_file: Path, stats: dict):
    """Atomically save stats to file."""
    stats["last_updated"] = time.time()

    # Write to temp file then rename for atomicity
    # Use unique temp file name to avoid race conditions between concurrent hooks
    temp_file = stats_file.with_suffix(f".{os.getpid()}.tmp")
    try:
        with open(temp_file, "w") as f:
            json.dump(stats, f)
        temp_file.rename(stats_file)
    except OSError:
        # If rename fails, try to clean up temp file
        try:
            temp_file.unlink(missing_ok=True)
        except Exception:
            pass


def count_tool(stats: dict, tool_name: str):
    """Count a finished tool call (it succeeded or failed) for the stats widget."""
    stats["tools"][tool_name] = stats["tools"].get(tool_name, 0) + 1
    if "tool_timestamps" not in stats:
        stats["tool_timestamps"] = []
    stats["tool_timestamps"].append(time.time())
    # Cap timestamps to prevent unbounded growth over long sessions
    # 1000 entries is enough for sparkline visualization
    if len(stats["tool_timestamps"]) > 1000:
        stats["tool_timestamps"] = stats["tool_timestamps"][-1000:]


def clear_permission(stats: dict):
    stats.pop("permission", None)
    stats.pop("permission_agent", None)
    stats.pop("state_before_permission", None)


def turn_ended(stats: dict):
    """The main turn is over: nothing is being prompted any more."""
    stats["turn_active"] = False
    stats["active_prompt"] = None
    stats["idle_since"] = time.time()
    clear_permission(stats)


def handle_tool_finished(stats: dict, event: str, data: dict):
    """PostToolUse / PostToolUseFailure: a tool call finished, by the main loop or a subagent."""
    tool_name = data.get("tool_name", "unknown")
    agent_id = data.get("agent_id")
    details = {"tool": tool_name}
    if agent_id:
        details["agent"] = agent_id
    add_event(stats, event, details)
    count_tool(stats, tool_name)

    if agent_id:
        # A subagent's call ends only a permission prompt it raised itself;
        # the session goes back to what it was doing before the prompt.
        if stats.get("state") == "permission" and stats.get("permission_agent") == agent_id:
            stats["active_prompt"] = None
            if stats.get("turn_active"):
                stats["state"] = "thinking"
            else:
                stats["state"] = stats.get("state_before_permission") or "complete"
            clear_permission(stats)
        return

    # Mark if this was a question tool so Stop transitions to "question" state
    # (a failed question was dismissed: nothing is waiting on the answer).
    stats["pending_question"] = event == "PostToolUse" and tool_name in (
        "AskUserQuestion",
        "ExitPlanMode",
    )
    # Clear active_prompt since tool completed (user responded to permission/question)
    stats["active_prompt"] = None
    # Tool completed - back to thinking (more tools may follow)
    stats["state"] = "thinking"
    # Clear permission data since we're no longer waiting
    clear_permission(stats)


def main():
    crabigator_session_id = os.environ.get("CRABIGATOR_SESSION_ID", "")
    try:
        data = json.load(sys.stdin)
    except json.JSONDecodeError as e:
        debug_log(crabigator_session_id, f"JSON decode error: {e}")
        return

    cwd = data.get("cwd", os.getcwd())
    event = data.get("hook_event_name", "")
    activity = activity_entry(event, data)
    append_activity(crabigator_session_id, activity)

    if event in FREQUENT_EVENTS:
        debug_log(crabigator_session_id, f"EVENT: {event} {json.dumps(activity)}")
        return

    # Extract Claude Code's session UUID from hook data
    claude_session_id = data.get("session_id", "")

    debug_log(crabigator_session_id, f"EVENT: {event} cwd={cwd} claude_session={claude_session_id}")
    debug_log(crabigator_session_id, f"RAW_DATA: {json.dumps(data)}")

    # A tool cut short by Esc: the turn is over, and crabigator's screen
    # already shows it interrupted.
    interrupted = event == "PostToolUseFailure" and data.get("is_interrupt")
    if event not in STATS_EVENTS or interrupted:
        return

    # Create symlink from Claude session UUID to crabigator directory (first event only)
    if claude_session_id and crabigator_session_id:
        create_claude_session_symlink(crabigator_session_id, claude_session_id)

    stats_file = get_stats_file(cwd)
    with stats_lock(stats_file):
        stats = load_stats(stats_file)
        handle_event(stats, event, data, cwd, crabigator_session_id)
        save_stats(stats_file, stats)
    debug_log(crabigator_session_id, f"  state_after={stats.get('state', 'ready')} saved to {stats_file}")


def handle_event(stats: dict, event: str, data: dict, cwd: str, crabigator_session_id: str):
    """Apply one hook event to the stats."""
    claude_session_id = data.get("session_id", "")

    # Store Claude Code's session UUID for correlation
    if claude_session_id:
        stats["claude_session_id"] = claude_session_id

    # Store transcript path for scrollback reading
    transcript_path = data.get("transcript_path")
    if transcript_path:
        stats["transcript_path"] = transcript_path

    stats["working_directory"] = cwd
    if event == "SessionStart" and data.get("source") == "resume" and transcript_path:
        stats["working_directory"] = extract_cwd_from_transcript(transcript_path) or cwd

    # Extract model from transcript if not already known
    if transcript_path and not stats.get("model"):
        model = extract_model_from_transcript(transcript_path)
        if model:
            stats["model"] = model
            debug_log(crabigator_session_id, f"  extracted model={model}")

    debug_log(crabigator_session_id, f"  state_before={stats.get('state', 'ready')}")

    if event == "SessionStart":
        add_event(stats, event, {"source": data.get("source", "startup")})
        # Compaction starts a new context while the current turn continues.
        if data.get("source") != "compact":
            stats["state"] = "ready"
            stats["turn_active"] = False
            stats["active_prompt"] = None
            clear_permission(stats)
            stats["pending_question"] = False
            stats["model"] = api_model_name(data.get("model")) or stats.get("model")

    elif event == "PermissionRequest":
        # Permission dialog is being shown to user
        tool_name = data.get("tool_name", "unknown")
        tool_input = data.get("tool_input", {})
        permission_suggestions = data.get("permission_suggestions", [])
        agent_id = data.get("agent_id")

        details = {"tool": tool_name}
        if agent_id:
            details["agent"] = agent_id
        add_event(stats, event, details)

        # Build active_prompt based on tool type
        if tool_name == "AskUserQuestion":
            stats["active_prompt"] = {
                "type": "question",
                "questions": tool_input.get("questions", []),
            }
            stats["state"] = "question"
            stats["pending_question"] = True
        elif tool_name == "ExitPlanMode":
            stats["active_prompt"] = {"type": "exit_plan"}
            stats["state"] = "question"
            stats["pending_question"] = True
        else:
            stats["active_prompt"] = {
                "type": "permission",
                "tool_name": tool_name,
                "tool_input": tool_input,
            }
            # Who asked, and what the session was doing, so the answer can
            # put it back (a background agent may ask after the turn ended).
            if stats.get("state") not in ("thinking", "permission"):
                stats["state_before_permission"] = stats.get("state")
            stats["permission_agent"] = agent_id
            stats["state"] = "permission"
            # Store permission details for dashboard
            stats["permission"] = {
                "tool": tool_name,
                "input": tool_input,
                "suggestions": permission_suggestions,
            }

    elif event in ("PostToolUse", "PostToolUseFailure"):
        handle_tool_finished(stats, event, data)

    elif event == "Stop":
        add_event(stats, event, {"pending_question": stats.get("pending_question", False)})
        stats["completions"] = stats.get("completions", 0) + 1
        # Transition to question or complete based on pending flag
        if stats.get("pending_question"):
            stats["state"] = "question"
            stats["pending_question"] = False
        else:
            stats["state"] = "complete"
        # Always clear active_prompt - the turn is over, nothing is being prompted.
        # If the question was answered, PostToolUse already cleared it (no-op here).
        # If the question was rejected/escaped, this clears the stale prompt.
        turn_ended(stats)

    elif event == "StopFailure":
        # An API error ended the turn (rate limit, overload, ...): without
        # this the session would sit at "thinking" until the next prompt.
        add_event(stats, event, {"error": data.get("error")})
        stats["state"] = "complete"
        stats["pending_question"] = False
        turn_ended(stats)

    elif event == "SubagentStop":
        add_event(stats, event, {"agent_type": data.get("agent_type")})
        stats["subagent_messages"] += 1

    elif event == "PreCompact":
        add_event(stats, event, {"trigger": data.get("trigger")})
        stats["compressions"] += 1

    elif event == "PostModelSwitch":
        add_event(stats, event, {"to_model": data.get("to_model")})
        stats["model"] = api_model_name(data.get("to_model")) or stats.get("model")

    elif event == "UserPromptSubmit":
        # User submitted input, Claude starts thinking
        add_event(stats, event)
        stats["prompts"] = stats.get("prompts", 0) + 1
        stats["state"] = "thinking"
        stats["turn_active"] = True
        stats["pending_question"] = False
        stats["active_prompt"] = None  # Clear any pending prompt
        stats["idle_since"] = None
        clear_permission(stats)


if __name__ == "__main__":
    try:
        main()
    except Exception as e:
        session_id = os.environ.get("CRABIGATOR_SESSION_ID", "")
        debug_log(session_id, f"Unhandled hook error: {type(e).__name__}: {e}")
    sys.exit(0)
