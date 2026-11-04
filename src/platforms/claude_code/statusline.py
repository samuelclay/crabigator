#!/usr/bin/env python3
"""
Crabigator's statusline for Claude Code.

Crabigator starts Claude Code with this script as its statusline. Claude Code
gives a statusline the one thing its hooks and transcript never say: the size
of the context window. The script saves how full that window is for
crabigator, then runs the statusline the user set up (if any) with the same
input, so the statusline looks exactly as it always does.

The user's statusline is looked up on every run, in Claude Code's order
(.claude/settings.local.json, .claude/settings.json, then the user's
settings.json), so a statusline changed during the session still shows.
"""

import json
import os
import subprocess
import sys
from pathlib import Path

# A statusline command with both of these is this script, never the user's.
OWN_MARKERS = ("crabigator", "statusline.py")


def context_usage(data: dict) -> dict | None:
    """Tokens in the context window and its size, as Claude Code counts them
    for its own percent (input, cache writes and cache reads)."""
    window = data.get("context_window")
    if not isinstance(window, dict):
        return None
    limit = window.get("context_window_size")
    if not isinstance(limit, (int, float)) or limit <= 0:
        return None
    used = window.get("total_input_tokens")
    if not isinstance(used, (int, float)):
        usage = window.get("current_usage")
        if isinstance(usage, dict):
            used = sum(
                usage.get(key) or 0
                for key in (
                    "input_tokens",
                    "cache_creation_input_tokens",
                    "cache_read_input_tokens",
                )
            )
        else:
            used = 0
    return {"used": int(used), "limit": int(limit)}


def save_context(session_id: str, data: dict):
    """Write the session's context.json, only when it changes."""
    usage = context_usage(data)
    session_dir = Path(f"/tmp/crabigator-{session_id}")
    if not session_id or usage is None or not session_dir.is_dir():
        return
    path = session_dir / "context.json"
    payload = json.dumps(usage)
    try:
        if path.read_text() == payload:
            return
    except OSError:
        pass
    tmp = session_dir / f"context.json.{os.getpid()}.tmp"
    tmp.write_text(payload)
    os.replace(tmp, path)


def is_own(command: str) -> bool:
    return all(marker in command for marker in OWN_MARKERS)


def user_statusline(data: dict) -> str | None:
    """The command of the statusline the user set up, or None."""
    workspace = data.get("workspace") if isinstance(data.get("workspace"), dict) else {}
    project = Path(workspace.get("project_dir") or data.get("cwd") or os.getcwd())
    config_dir = Path(os.environ.get("CLAUDE_CONFIG_DIR") or Path.home() / ".claude")
    for path in (
        project / ".claude" / "settings.local.json",
        project / ".claude" / "settings.json",
        config_dir / "settings.json",
    ):
        try:
            settings = json.loads(path.read_text())
        except (OSError, ValueError):
            continue
        line = settings.get("statusLine") if isinstance(settings, dict) else None
        if not isinstance(line, dict):
            continue
        command = line.get("command")
        if line.get("type", "command") != "command" or not isinstance(command, str):
            return None
        if not command.strip() or is_own(command):
            return None
        return command
    return None


def main() -> int:
    raw = sys.stdin.buffer.read()
    try:
        data = json.loads(raw or b"{}")
    except ValueError:
        data = {}
    if not isinstance(data, dict):
        data = {}
    try:
        save_context(os.environ.get("CRABIGATOR_SESSION_ID", ""), data)
    except Exception:
        pass  # The user's statusline still shows.
    command = user_statusline(data)
    if command is None:
        return 0
    sys.stdout.flush()
    try:
        return subprocess.run(command, shell=True, input=raw).returncode
    except OSError:
        return 0


if __name__ == "__main__":
    sys.exit(main())
