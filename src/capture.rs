//! Output capture for streaming-ready session recording.
//!
//! Captures assistant CLI output to files:
//! - `scrollback.log`: Session transcript from platform JSONL (append-only)
//! - `screen.txt`: Current screen snapshot with ANSI codes (rendered by vt100)
//! - `terminal_history.log`: Styled rows that actually scrolled off the terminal
//!
//! Remote viewers use terminal history. The transcript remains available locally
//! for conversation search; it is never substituted for terminal output.

use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::platforms::claude_code::transcript as claude_transcript;
use crate::platforms::codex_cli::transcript as codex_transcript;
use crate::platforms::grok::transcript as grok_transcript;
use crate::platforms::opencode::transcript as opencode_transcript;
use crate::platforms::PlatformKind;

/// Maximum size for raw PTY log before rotation (50MB)
#[cfg(debug_assertions)]
const RAW_LOG_MAX_SIZE: u64 = 50 * 1024 * 1024;

/// Configuration for output capture.
pub struct CaptureConfig {
    /// Whether capture is enabled (default: true, disabled with --no-capture)
    pub enabled: bool,
    /// Session ID for unique directory naming
    pub session_id: String,
}

/// Incremental scrollback update for streaming
pub struct ScrollbackUpdate {
    /// Replace history after the child clears saved terminal rows (CSI 3 J).
    pub reset: bool,
    /// Newly appended terminal rows (SGR styles, newline-delimited)
    pub diff: String,
    /// Total line count after applying this diff
    pub total_lines: usize,
}

/// Manages output capture to scrollback and screen files.
pub struct CaptureManager {
    config: CaptureConfig,
    platform: PlatformKind,
    /// Base directory: /tmp/crabigator-{session_id}/
    capture_dir: PathBuf,
    /// Terminal parser that retains rows leaving the assistant viewport.
    capture_parser: vt100::Parser,
    /// Rows that left the terminal since the last history flush.
    pending_history: String,
    history_reset_pending: bool,
    total_history_lines: usize,
    /// Last scrollback.log update time (for throttling)
    last_scrollback_update: Instant,
    /// Scrollback update interval
    scrollback_update_interval: Duration,
    /// Last screen.txt update time (for throttling)
    last_screen_update: Instant,
    /// Screen update interval
    screen_update_interval: Duration,
    /// Path to the assistant platform transcript JSONL file
    transcript_path: Option<PathBuf>,
    /// Byte offset into transcript file (for incremental reads)
    transcript_offset: u64,
    /// Pending tool calls awaiting results (persisted across incremental reads)
    claude_pending_tools: HashMap<String, claude_transcript::PendingToolUse>,
    /// Pending Codex tool calls awaiting results (persisted across reads)
    codex_pending_tools: HashMap<String, codex_transcript::PendingToolUse>,
    /// Pending Grok tool calls awaiting results (persisted across reads)
    grok_pending_tools: HashMap<String, grok_transcript::PendingToolUse>,
    /// Raw PTY output log file (debug builds only)
    #[cfg(debug_assertions)]
    raw_log: Option<File>,
    /// Current raw log size for rotation checks
    #[cfg(debug_assertions)]
    raw_log_size: u64,
}

/// Render a vt100 screen into the same string format used for screen.txt.
///
/// This is intentionally independent from CaptureManager so callers can inspect
/// the live PTY screen even when file capture is disabled.
pub fn screen_to_string(screen: &vt100::Screen) -> String {
    let (rows, cols) = screen.size();

    let formatted_rows: Vec<Vec<u8>> =
        crate::catch_quiet(|| screen.rows_formatted(0, cols).take(rows as usize).collect())
            .unwrap_or_default();

    let last_nonempty = formatted_rows
        .iter()
        .enumerate()
        .filter(|(_, row)| row.iter().any(|&b| !b.is_ascii_whitespace()))
        .map(|(idx, _)| idx)
        .next_back()
        .unwrap_or(0);

    let mut content = Vec::new();
    for row_bytes in formatted_rows.into_iter().take(last_nonempty + 1) {
        content.extend_from_slice(&row_bytes);
        content.extend_from_slice(b"\x1b[0m");
        content.push(b'\n');
    }

    String::from_utf8_lossy(&content).to_string()
}

/// `row col` when the child is showing a cursor, or empty when it is hidden.
///
/// The attach view reads this beside `screen.txt`. Row and column are the
/// vt100 screen's, starting at 0.
pub fn cursor_record(screen: &vt100::Screen) -> String {
    if screen.hide_cursor() {
        return String::new();
    }
    let (row, col) = screen.cursor_position();
    format!("{row} {col}\n")
}

// Drained every 256 input bytes. A scrolling command removes at most u16::MAX
// rows. This is a limit, not an allocation, and also covers later resizes.
const HISTORY_CAPACITY: usize = u16::MAX as usize * 128 + 256;

impl CaptureManager {
    /// Create a new CaptureManager.
    ///
    /// Terminal history uses actual PTY dimensions. The selected platform
    /// only determines the format of the separate searchable transcript.
    pub fn new(
        config: CaptureConfig,
        platform: PlatformKind,
        cols: u16,
        rows: u16,
    ) -> std::io::Result<Self> {
        // Use actual terminal dimensions for screen capture
        let capture_parser = vt100::Parser::new(rows, cols, HISTORY_CAPACITY);

        if !config.enabled {
            return Ok(Self {
                config,
                platform,
                capture_dir: PathBuf::new(),
                capture_parser,
                pending_history: String::new(),
                history_reset_pending: false,
                total_history_lines: 0,
                last_scrollback_update: Instant::now(),
                scrollback_update_interval: Duration::from_millis(500),
                last_screen_update: Instant::now(),
                screen_update_interval: Duration::from_millis(100),
                transcript_path: None,
                transcript_offset: 0,
                claude_pending_tools: HashMap::new(),
                codex_pending_tools: HashMap::new(),
                grok_pending_tools: HashMap::new(),
                #[cfg(debug_assertions)]
                raw_log: None,
                #[cfg(debug_assertions)]
                raw_log_size: 0,
            });
        }

        let capture_dir = PathBuf::from(format!("/tmp/crabigator-{}", config.session_id));

        // Create directory
        fs::create_dir_all(&capture_dir)?;

        // Open raw log file for appending (debug builds only)
        #[cfg(debug_assertions)]
        let (raw_log, raw_log_size) = {
            let path = capture_dir.join("pty_raw.log");
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let file = OpenOptions::new().create(true).append(true).open(&path)?;
            (Some(file), size)
        };

        Ok(Self {
            config,
            platform,
            capture_dir,
            capture_parser,
            pending_history: String::new(),
            history_reset_pending: false,
            total_history_lines: 0,
            last_scrollback_update: Instant::now() - Duration::from_secs(10),
            scrollback_update_interval: Duration::from_millis(500),
            last_screen_update: Instant::now() - Duration::from_secs(10),
            screen_update_interval: Duration::from_millis(100),
            transcript_path: None,
            transcript_offset: 0,
            claude_pending_tools: HashMap::new(),
            codex_pending_tools: HashMap::new(),
            grok_pending_tools: HashMap::new(),
            #[cfg(debug_assertions)]
            raw_log,
            #[cfg(debug_assertions)]
            raw_log_size,
        })
    }

    /// Resize the capture parser to match PTY dimensions.
    pub fn resize(&mut self, cols: u16, rows: u16) {
        crate::guarded_resize(&mut self.capture_parser, cols, rows);
    }

    /// Set the transcript file path for scrollback reading.
    /// Called when we receive transcript_path from platform stats or on startup.
    pub fn set_transcript_path(&mut self, path: Option<String>) {
        if let Some(p) = path {
            let path_buf = PathBuf::from(&p);
            // Only update if path changed
            if self.transcript_path.as_ref() != Some(&path_buf) {
                self.transcript_path = Some(path_buf);
                // Reset state when switching to a different file
                self.transcript_offset = 0;
                self.claude_pending_tools.clear();
                self.codex_pending_tools.clear();
                self.grok_pending_tools.clear();
                // Clear scrollback.log to avoid mixing content from different sessions
                if self.config.enabled {
                    let scrollback_path = self.capture_dir.join("scrollback.log");
                    let _ = fs::write(&scrollback_path, "");
                }
            }
        }
    }

    /// Process PTY output bytes through our capture parser.
    ///
    /// Use exactly the virtualized bytes sent to the host terminal. Drain only
    /// rows that scroll offscreen, never snapshots of transient repaints.
    pub fn capture_output(&mut self, data: &[u8]) -> std::io::Result<()> {
        if !self.config.enabled || data.is_empty() {
            return Ok(());
        }

        // Write raw bytes to log for debugging escape sequences (debug builds only)
        #[cfg(debug_assertions)]
        {
            // Rotate if exceeding max size
            if self.raw_log_size > RAW_LOG_MAX_SIZE {
                self.rotate_raw_log()?;
            }
            if let Some(ref mut log) = self.raw_log {
                if log.write_all(data).is_ok() {
                    self.raw_log_size += data.len() as u64;
                }
            }
        }

        // A CSI scroll can move at most one screen per command. This chunk
        // bound leaves room for every row even in a burst of scroll commands.
        for chunk in data.chunks(256) {
            crate::guarded_process(&mut self.capture_parser, chunk, HISTORY_CAPACITY);
            if self.capture_parser.screen_mut().take_history_cleared() {
                self.pending_history.clear();
                self.history_reset_pending = true;
            }
            let history = self
                .capture_parser
                .screen_mut()
                .drain_scrollback_formatted();
            self.pending_history
                .push_str(&String::from_utf8_lossy(&history));
        }

        Ok(())
    }

    /// Rotate the raw PTY log by truncating it (debug builds only).
    #[cfg(debug_assertions)]
    fn rotate_raw_log(&mut self) -> std::io::Result<()> {
        let path = self.capture_dir.join("pty_raw.log");
        // Just truncate - we don't need to keep old data for debugging
        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(&path)?;
        self.raw_log = Some(file);
        self.raw_log_size = 0;
        Ok(())
    }

    /// Update scrollback.log if the throttle interval has elapsed.
    pub fn maybe_update_scrollback(&mut self) -> std::io::Result<Option<ScrollbackUpdate>> {
        if !self.config.enabled {
            return Ok(None);
        }

        if self.last_scrollback_update.elapsed() < self.scrollback_update_interval {
            return Ok(None);
        }

        self.update_scrollback()
    }

    /// Flush terminal history for remote viewers and refresh the searchable transcript.
    pub fn update_scrollback(&mut self) -> std::io::Result<Option<ScrollbackUpdate>> {
        // A missing/unreadable transcript must never stop terminal capture.
        let _ = self.update_transcript();
        self.last_scrollback_update = Instant::now();
        self.flush_terminal_history()
    }

    fn flush_terminal_history(&mut self) -> std::io::Result<Option<ScrollbackUpdate>> {
        if !self.config.enabled || (self.pending_history.is_empty() && !self.history_reset_pending)
        {
            return Ok(None);
        }
        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(!self.history_reset_pending)
            .truncate(self.history_reset_pending)
            .open(self.capture_dir.join("terminal_history.log"))?;
        file.write_all(self.pending_history.as_bytes())?;
        let diff = std::mem::take(&mut self.pending_history);
        let reset = std::mem::take(&mut self.history_reset_pending);
        if reset {
            self.total_history_lines = 0;
        }
        self.total_history_lines += diff.lines().count();
        Ok(Some(ScrollbackUpdate {
            reset,
            diff,
            total_lines: self.total_history_lines,
        }))
    }

    /// Append new content to scrollback.log from the platform transcript.
    ///
    /// Reads incrementally from the JSONL transcript file, which contains
    /// clean conversation history without status bar noise.
    fn update_transcript(&mut self) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let Some(ref transcript_path) = self.transcript_path else {
            // No transcript path yet - nothing to read
            self.last_scrollback_update = Instant::now();
            return Ok(());
        };

        if !transcript_path.exists() {
            self.last_scrollback_update = Instant::now();
            return Ok(());
        }

        // Read new content from the platform-specific JSONL schema. Pending tool
        // calls persist across reads so delayed results can still be summarized.
        let (new_content, new_offset) = match self.platform {
            PlatformKind::Claude => claude_transcript::read_transcript(
                transcript_path,
                self.transcript_offset,
                &mut self.claude_pending_tools,
            )?,
            PlatformKind::Codex => codex_transcript::read_transcript(
                transcript_path,
                self.transcript_offset,
                &mut self.codex_pending_tools,
            )?,
            PlatformKind::Opencode => {
                opencode_transcript::read_transcript(transcript_path, self.transcript_offset)?
            }
            PlatformKind::Grok => grok_transcript::read_transcript(
                transcript_path,
                self.transcript_offset,
                &mut self.grok_pending_tools,
            )?,
        };

        if new_content.is_empty() {
            self.transcript_offset = new_offset;
            self.last_scrollback_update = Instant::now();
            return Ok(());
        }

        // Append to scrollback file
        let scrollback_path = self.capture_dir.join("scrollback.log");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&scrollback_path)?;
        file.write_all(new_content.as_bytes())?;

        self.transcript_offset = new_offset;
        self.last_scrollback_update = Instant::now();

        Ok(())
    }

    /// Get the full scrollback content accumulated so far.
    /// Used for initial sync when connecting to cloud.
    ///
    /// Flush pending rows first so a reconnect cannot resend them as a diff.
    pub fn get_full_scrollback(&mut self) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        self.flush_terminal_history().ok()?;
        let scrollback_path = self.capture_dir.join("terminal_history.log");
        if !scrollback_path.exists() {
            return Some(String::new());
        }

        fs::read_to_string(&scrollback_path).ok()
    }

    /// Update screen.txt if the throttle interval has elapsed.
    pub fn maybe_update_screen(
        &mut self,
        screen: &vt100::Screen,
    ) -> std::io::Result<Option<String>> {
        if !self.config.enabled {
            return Ok(None);
        }

        if self.last_screen_update.elapsed() < self.screen_update_interval {
            return Ok(None);
        }

        let contents = self.update_screen(screen)?;
        Ok(Some(contents))
    }

    /// Force immediate screen.txt update.
    /// Uses the platform_pty's screen (actual terminal size with scroll region).
    pub fn update_screen(&mut self, screen: &vt100::Screen) -> std::io::Result<String> {
        if !self.config.enabled {
            return Ok(String::new());
        }

        let contents = screen_to_string(screen);
        let content = contents.as_bytes();

        let screen_path = self.capture_dir.join("screen.txt");
        let tmp_path = self.capture_dir.join("screen.txt.tmp");
        fs::write(&tmp_path, content)?;
        fs::rename(&tmp_path, &screen_path)?;
        let cursor_path = self.capture_dir.join("cursor.txt");
        let cursor_tmp = self.capture_dir.join("cursor.txt.tmp");
        fs::write(&cursor_tmp, cursor_record(screen))?;
        fs::rename(&cursor_tmp, &cursor_path)?;

        self.last_screen_update = Instant::now();
        Ok(contents)
    }

    /// Session directory when capture is on. The attach socket lives here.
    pub fn mirror_dir(&self) -> Option<&std::path::Path> {
        self.config
            .enabled
            .then_some(self.capture_dir.as_path())
            .filter(|dir| !dir.as_os_str().is_empty())
    }

    /// Capture directory for debug dumps. Only called from debug builds.
    #[cfg(debug_assertions)]
    pub fn capture_dir(&self) -> &PathBuf {
        &self.capture_dir
    }

    /// Whether capture is on. Only called from debug builds.
    #[cfg(debug_assertions)]
    pub fn is_enabled(&self) -> bool {
        self.config.enabled
    }

    /// Cleanup - remove capture directory on exit.
    pub fn cleanup(&self) {
        if self.config.enabled && self.capture_dir.exists() {
            let _ = fs::remove_dir_all(&self.capture_dir);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::cursor_record;

    #[test]
    fn hyperlinks_survive_capture_history_and_screen_repaints() {
        let mut capture = capture(40, 2);
        let open = "\x1b]8;;https://example.com/pr;a=1\x1b\\";
        let close = "\x1b]8;;\x1b\\";
        // OSC can arrive in pieces; SGR reset must not end a hyperlink.
        for byte in format!("{open}\x1b[34;4mRead\x1b[0m PR{close} plain").bytes() {
            capture.capture_output(&[byte]).unwrap();
        }
        let screen = super::screen_to_string(capture.capture_parser.screen());
        assert!(screen.contains(open));
        assert!(screen.contains(&format!("PR{close}")));
        capture.capture_output(b"\r\nsecond\r\nthird").unwrap();
        let history = capture.update_scrollback().unwrap().unwrap().diff;
        assert!(history.contains(open));
        assert!(history.contains(&format!("PR{close}")));
        assert!(!super::screen_to_string(capture.capture_parser.screen()).contains(open));
        capture.capture_output(format!("\r{open}old{close}\r\x1b[2Knew").as_bytes()).unwrap();
        assert!(!super::screen_to_string(capture.capture_parser.screen()).contains(open));
        capture.cleanup();
    }

    #[test]
    fn hyperlink_snapshots_and_diffs_round_trip_wrapped_rows() {
        let mut original = vt100::Parser::new(4, 10, 10);
        original.process(b"\x1b]8;;https://example.com/one\x07abcdefghijklmnop\x1b[0mqrst\x1b]8;;\x07 plain");
        let mut restored = vt100::Parser::new(4, 10, 10);
        restored.process(&original.screen().contents_formatted());
        assert_eq!(super::screen_to_string(restored.screen()), super::screen_to_string(original.screen()));
        let previous = original.screen().clone();
        // Changing only the destination must still update cells in a screen diff.
        original.process(b"\x1b[H\x1b]8;;https://example.com/two\x07abcd\x1b]8;;\x07\x1b[2;1H\x1b[K");
        restored.process(&original.screen().contents_diff(&previous));
        assert_eq!(super::screen_to_string(restored.screen()), super::screen_to_string(original.screen()));
        // A full redraw must clear a destination left open by earlier output.
        let mut plain = vt100::Parser::new(4, 10, 0);
        plain.process(b"plain");
        restored.process(b"\x1b]8;;https://old.example\x07");
        restored.process(&plain.screen().contents_formatted());
        assert_eq!(super::screen_to_string(restored.screen()), super::screen_to_string(plain.screen()));
        restored.process(b"\x1b]8;;https://old.example\x07");
        restored.process(&plain.screen().attributes_formatted());
        restored.process(b" text");
        assert!(!super::screen_to_string(restored.screen()).contains("old.example"));
    }

    #[test]
    fn cursor_record_follows_the_visible_cell() {
        let mut parser = vt100::Parser::new(5, 20, 0);
        parser.process(b"hi");
        assert_eq!(cursor_record(parser.screen()), "0 2\n");

        parser.process(b"\x1b[?25l");
        assert_eq!(cursor_record(parser.screen()), "");

        parser.process(b"\x1b[?25h");
        assert_eq!(cursor_record(parser.screen()), "0 2\n");
    }
    fn capture(cols: u16, rows: u16) -> super::CaptureManager {
        super::CaptureManager::new(
            super::CaptureConfig {
                enabled: true,
                session_id: format!("history-test-{}", uuid::Uuid::new_v4()),
            },
            crate::platforms::PlatformKind::Codex,
            cols,
            rows,
        )
        .unwrap()
    }

    fn plain(content: &str) -> String {
        regex::Regex::new(r"\x1b\[[0-9;]*m")
            .unwrap()
            .replace_all(content, "")
            .into_owned()
    }

    #[test]
    fn terminal_history_keeps_scrolled_rows_not_repaints_or_transcripts() {
        let mut capture = capture(20, 3);
        capture
            .capture_output(b"\x1b[31mold\r\x1b[2Kfinal\x1b[0m\r\nsecond\r\nthird")
            .unwrap();
        assert!(capture.update_scrollback().unwrap().is_none());
        capture.capture_output(b"\r\nfourth").unwrap();
        let update = capture.update_scrollback().unwrap().unwrap();
        assert_eq!(plain(&update.diff).trim_end(), "final");
        assert!(update.diff.contains("\x1b[31m"));
        assert_eq!(update.total_lines, 1);
        // Late transcript discovery must not replace terminal history.
        capture.set_transcript_path(Some("/nonexistent/transcript.jsonl".into()));
        assert_eq!(capture.get_full_scrollback().unwrap(), update.diff);
        assert!(capture.update_scrollback().unwrap().is_none());
        capture.cleanup();
    }

    #[test]
    fn codex_top_scroll_region_preserves_history_and_excludes_fixed_footer() {
        let mut capture = capture(20, 5);
        capture
            .capture_output(b"\x1b[5;1Hfooter\x1b[1;3r\x1b[Hone\r\ntwo\r\nthree\r\nfour")
            .unwrap();
        assert_eq!(plain(&capture.get_full_scrollback().unwrap()), "one\n");
        capture.capture_output(b"\x1b[2;3r\x1b[3;1H\n\n").unwrap();
        assert!(
            capture.update_scrollback().unwrap().is_none(),
            "interior scrolling is not history"
        );
        capture.cleanup();
    }

    #[test]
    fn history_survives_large_bursts_split_escapes_and_reconnects_without_duplicates() {
        let mut capture = capture(40, 3);
        let stream = (0..1200)
            .map(|n| format!("\x1b[32mrow {n}\x1b[0m\r\n"))
            .collect::<String>();
        for chunk in stream.as_bytes().chunks(7) {
            capture.capture_output(chunk).unwrap();
        }
        let history = capture.get_full_scrollback().unwrap();
        let expected = (0..1198).map(|n| format!("row {n}\n")).collect::<String>();
        assert_eq!(plain(&history), expected);
        assert!(capture.update_scrollback().unwrap().is_none());
        capture.capture_output(b"next\r\n").unwrap();
        let update = capture.update_scrollback().unwrap().unwrap();
        assert_eq!(plain(&update.diff), "row 1198\n");
        assert_eq!(update.total_lines, 1199);
        assert_eq!(
            capture.get_full_scrollback().unwrap(),
            history + &update.diff
        );
        capture.cleanup();
    }

    #[test]
    fn history_retains_spaces_blank_rows_unicode_and_original_width_after_resize() {
        let mut capture = capture(10, 2);
        capture
            .capture_output("\x1b[3G界e\u{301}\r\n\r\n0123456789X\r\n".as_bytes())
            .unwrap();
        capture.resize(5, 2);
        assert_eq!(
            plain(&capture.get_full_scrollback().unwrap()),
            "  界e\u{301}\n\n0123456789\n"
        );
        capture.cleanup();
    }
    #[test]
    fn codex_history_erase_replaces_already_sent_rows_even_across_split_sequences() {
        let mut capture = capture(20, 3);
        capture.capture_output(b"old1\r\nold2\r\nold3\r\n").unwrap();
        assert!(!capture.update_scrollback().unwrap().unwrap().reset);
        capture.capture_output(b"\x1b[2J\x1b[H\x1b[3").unwrap();
        capture
            .capture_output(b"Jnew1\r\nnew2\r\nnew3\r\n")
            .unwrap();
        let update = capture.update_scrollback().unwrap().unwrap();
        assert!(update.reset);
        assert_eq!(plain(&update.diff), "new1\n");
        assert_eq!(update.total_lines, 1);
        assert_eq!(capture.get_full_scrollback().unwrap(), update.diff);
        capture.capture_output(b"\x1b[3J").unwrap();
        let update = capture.update_scrollback().unwrap().unwrap();
        assert!(update.reset);
        assert!(update.diff.is_empty());
        assert_eq!(capture.get_full_scrollback().unwrap(), "");
        capture.cleanup();
    }
}
