#!/usr/bin/env python3
"""Check focus recovery in a disposable tmux Crabigator/Codex session.

Start a test session with target/debug/crabigator codex, wait for its prompt,
then run: python3 scripts/e2e-focus-repaint.py TMUX_TARGET

This deliberately scrolls the host screen to reproduce duplicated widgets.
It tests recovery, not the original cause of the terminal corruption. It
does not resize the pane or submit a prompt. Captures go into a temp folder.
"""

import argparse
import os
from pathlib import Path
import subprocess
import tempfile
import time


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("target", help="disposable tmux pane running Crabigator")
    args = parser.parse_args()
    captures = Path(tempfile.mkdtemp(prefix="crabigator-focus-"))

    def tmux(*command):
        return subprocess.check_output(["tmux", *command], text=True)

    def screen():
        return tmux("capture-pane", "-pt", args.target)

    def history():
        return tmux("capture-pane", "-pt", args.target, "-S", "-", "-E", "-1")

    def wait_for(predicate):
        deadline = time.monotonic() + 15
        while time.monotonic() < deadline:
            value = screen()
            if predicate(value):
                return value
            time.sleep(0.1)
        raise AssertionError(f"Timed out; captures: {captures}")

    clean = screen()
    assert clean.count("Compactions") == 1, "Start with one visible Crabigator footer"
    (captures / "clean.txt").write_text(clean)
    tty, height = tmux(
        "display-message", "-pt", args.target, "#{pane_tty} #{pane_height}"
    ).split()
    height = int(height)
    assert height >= 20, "Use a test pane at least 20 rows tall"

    # Bypass Crabigator to move the real footer into the child's area. Its
    # internal screen stays intact, as it does with a host rendering fault.
    fd = os.open(tty, os.O_WRONLY)
    try:
        os.write(fd, f"\x1b[r\x1b[{height};1H".encode() + b"\r\n" * 8)
    finally:
        os.close(fd)
    damaged = wait_for(lambda value: value.count("Compactions") == 2)
    (captures / "damaged.txt").write_text(damaged)
    history_before = history()
    (captures / "history-before.txt").write_text(history_before)

    tmux("send-keys", "-t", args.target, "Escape", "[", "I")
    repaired = wait_for(lambda value: value.count("Compactions") == 1)
    (captures / "repaired.txt").write_text(repaired)
    first_content = next(line for line in clean.splitlines() if line.strip())
    assert first_content in repaired, f"Top content was not restored; captures: {captures}"
    history_after = history()
    (captures / "history-after.txt").write_text(history_after)
    assert history_before == history_after, f"Focus added scrollback; captures: {captures}"
    assert tmux("display-message", "-pt", args.target, "#{pane_height}").strip() == str(height)
    print(f"PASS: focus removed duplicated widgets without resizing or adding history. {captures}")


if __name__ == "__main__":
    main()
