#!/usr/bin/env python3
"""
Generate a Tavus video summary of a crabigator session.

Usage:
    # Summarize most recent session
    ./scripts/session-video.py

    # Summarize a specific session
    ./scripts/session-video.py /tmp/crabigator-abc123

    # Summarize with a specific replica
    ./scripts/session-video.py --replica r6ae5b6efc9d

Environment variables:
    TAVUS_API_KEY       - Required. Your Tavus API key.
    ANTHROPIC_API_KEY   - Required. For Claude-powered summarization.
"""
import argparse
import glob
import json
import os
import pathlib
import sys
import time

# Auto-load .env from project root
_env_file = pathlib.Path(__file__).resolve().parent.parent / ".env"
if _env_file.exists():
    for line in _env_file.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            key, _, val = line.partition("=")
            os.environ.setdefault(key.strip(), val.strip())

import anthropic
import httpx

TAVUS_BASE = "https://tavusapi.com/v2"
DEFAULT_REPLICA = "r6ae5b6efc9d"  # Anna (phoenix-3 stock replica)
MAX_SCRIPT_WORDS = 90  # ~30 seconds of speech at ~3 words/sec


def find_latest_session() -> str | None:
    """Find the most recently modified crabigator session directory."""
    dirs = glob.glob("/tmp/crabigator-*/scrollback.log")
    if not dirs:
        return None
    # Sort by modification time, newest first
    dirs.sort(key=os.path.getmtime, reverse=True)
    return os.path.dirname(dirs[0])


def read_scrollback(session_dir: str) -> str:
    """Read and return the scrollback log, stripping ANSI codes."""
    path = os.path.join(session_dir, "scrollback.log")
    if not os.path.exists(path):
        print(f"Error: No scrollback.log in {session_dir}", file=sys.stderr)
        sys.exit(1)
    with open(path) as f:
        text = f.read()
    # Strip ANSI escape sequences
    import re
    text = re.sub(r'\x1b\[[0-9;]*[a-zA-Z]', '', text)
    # Strip XML-like tags from Claude Code
    text = re.sub(r'<[^>]+>', '', text)
    return text.strip()


def read_stats(session_dir: str) -> dict | None:
    """Try to read session stats from the stats JSON file."""
    session_id = os.path.basename(session_dir).replace("crabigator-", "")
    stats_path = f"/tmp/crabigator-stats-{session_id}.json"
    if os.path.exists(stats_path):
        with open(stats_path) as f:
            return json.load(f)
    return None


def generate_script(scrollback: str, stats: dict | None) -> str:
    """Use Claude to generate a concise video narration script."""
    client = anthropic.Anthropic()

    stats_context = ""
    if stats:
        duration = stats.get("session_duration_secs", 0)
        mins = int(duration // 60)
        tools = stats.get("tool_use_count", 0)
        prompts = stats.get("prompt_count", 0)
        stats_context = f"\nSession stats: {mins} minutes, {prompts} prompts, {tools} tool uses."

    # Truncate scrollback if very long (keep first and last portions)
    if len(scrollback) > 8000:
        scrollback = scrollback[:4000] + "\n\n[...middle of session omitted...]\n\n" + scrollback[-4000:]

    response = client.messages.create(
        model="claude-sonnet-4-20250514",
        max_tokens=200,
        messages=[{
            "role": "user",
            "content": f"""You are writing a spoken narration script for a short video recap of a coding session. The video features a digital human speaking directly to the viewer.

Here is the session transcript:
---
{scrollback}
---
{stats_context}

Write a casual, concise spoken script (maximum {MAX_SCRIPT_WORDS} words) that:
1. Opens with a brief greeting like "Hey!" or "Quick recap:"
2. Summarizes what was accomplished in the session
3. Mentions any key decisions or interesting findings
4. Closes naturally

Rules:
- Write ONLY the spoken words, no stage directions or formatting
- Keep it conversational and natural, like telling a coworker what you just did
- Do NOT use markdown, bullet points, or any formatting
- Stay under {MAX_SCRIPT_WORDS} words total"""
        }]
    )
    return response.content[0].text.strip()


def create_video(script: str, replica_id: str, api_key: str) -> dict:
    """Create a Tavus video and return the response."""
    resp = httpx.post(
        f"{TAVUS_BASE}/videos",
        headers={
            "x-api-key": api_key,
            "Content-Type": "application/json",
        },
        json={
            "replica_id": replica_id,
            "script": script,
            "video_name": "Crabigator Session Recap",
        },
        timeout=30,
    )
    resp.raise_for_status()
    return resp.json()


def poll_video(video_id: str, api_key: str, timeout: int = 300) -> dict:
    """Poll until video is ready or timeout."""
    start = time.time()
    print("Generating video", end="", flush=True)
    while time.time() - start < timeout:
        resp = httpx.get(
            f"{TAVUS_BASE}/videos/{video_id}",
            headers={"x-api-key": api_key},
            timeout=30,
        )
        resp.raise_for_status()
        data = resp.json()
        status = data.get("status")
        if status == "ready":
            print(" done!")
            return data
        if status == "error":
            print(" failed!")
            print(f"Error: Video generation failed: {data}", file=sys.stderr)
            sys.exit(1)
        print(".", end="", flush=True)
        time.sleep(5)
    print(" timed out!")
    print(f"Timeout: Video not ready after {timeout}s. Check: https://platform.tavus.io", file=sys.stderr)
    sys.exit(1)


def main():
    parser = argparse.ArgumentParser(description="Generate a Tavus video recap of a crabigator session")
    parser.add_argument("session_dir", nargs="?", help="Path to session directory (default: most recent)")
    parser.add_argument("--replica", default=DEFAULT_REPLICA, help=f"Tavus replica ID (default: {DEFAULT_REPLICA})")
    parser.add_argument("--script-only", action="store_true", help="Only generate and print the script, don't create video")
    args = parser.parse_args()

    # Check API keys
    tavus_key = os.environ.get("TAVUS_API_KEY")
    if not tavus_key and not args.script_only:
        print("Error: TAVUS_API_KEY environment variable not set", file=sys.stderr)
        sys.exit(1)
    if not os.environ.get("ANTHROPIC_API_KEY"):
        print("Error: ANTHROPIC_API_KEY environment variable not set", file=sys.stderr)
        sys.exit(1)

    # Find session
    session_dir = args.session_dir or find_latest_session()
    if not session_dir:
        print("Error: No crabigator session found in /tmp/", file=sys.stderr)
        sys.exit(1)
    print(f"Session: {session_dir}")

    # Read session data
    scrollback = read_scrollback(session_dir)
    if not scrollback:
        print("Error: Scrollback log is empty", file=sys.stderr)
        sys.exit(1)
    stats = read_stats(session_dir)

    # Generate script
    print("Generating narration script...")
    script = generate_script(scrollback, stats)
    word_count = len(script.split())
    print(f"\nScript ({word_count} words):")
    print(f"---\n{script}\n---")

    if args.script_only:
        return

    # Create video
    print(f"\nCreating video with replica {args.replica}...")
    video = create_video(script, args.replica, tavus_key)
    video_id = video["video_id"]
    print(f"Video ID: {video_id}")

    # Poll until ready
    result = poll_video(video_id, tavus_key)

    # Print results
    hosted = result.get("hosted_url", "")
    download = result.get("download_url", "")
    print(f"\nVideo ready!")
    print(f"  Watch:    {hosted}")
    if download:
        print(f"  Download: {download}")


if __name__ == "__main__":
    main()
