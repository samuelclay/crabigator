# Crabigator terminal history

Based on vt100 0.16.2 (MIT), from https://github.com/doy/vt100-rust.

Local changes:
- Report terminal history erasure (CSI 3 J and full reset), so Codex redraws
  replace the saved rows instead of duplicating them.
- Drain rows that have left the primary screen without resetting parser state.
- Serialize history as independent rows with text and SGR styles, including
  blank cells and each row's original width; omit cursor and erase commands.
- Keep history for scroll regions starting at row zero, including a shorter
  region above a fixed footer. Interior scroll regions do not create history.

Capture drains after each small input chunk, so its in-memory history stays
bounded. Regression tests live in src/capture.rs in the parent project.
