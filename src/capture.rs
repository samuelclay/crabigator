//! Output capture for streaming-ready session recording.
//!
//! Captures assistant CLI output to files:
//! - `scrollback.log`: Session transcript from platform JSONL (append-only)
//! - `screen.txt`: Current screen snapshot with ANSI codes (rendered by vt100)
//!
//! Scrollback is read from Claude Code or Codex transcript files, which provide
//! clean conversation history without status bar noise.

use std::collections::HashMap;
#[cfg(debug_assertions)]
use std::fs::File;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::platforms::claude_code::transcript as claude_transcript;
use crate::platforms::codex_cli::transcript as codex_transcript;
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
    /// Newly appended lines (plain text, newline-delimited)
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
    /// vt100 parser for screen capture (uses actual terminal dimensions)
    capture_parser: vt100::Parser,
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
    /// Total line count in scrollback.log
    total_scrollback_lines: usize,
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

impl CaptureManager {
    /// Create a new CaptureManager.
    ///
    /// Screen capture uses actual terminal dimensions.
    /// Scrollback is read from the selected platform's transcript JSONL file.
    pub fn new(
        config: CaptureConfig,
        platform: PlatformKind,
        cols: u16,
        rows: u16,
    ) -> std::io::Result<Self> {
        // Use actual terminal dimensions for screen capture
        let capture_parser = vt100::Parser::new(rows, cols, 0);

        if !config.enabled {
            return Ok(Self {
                config,
                platform,
                capture_dir: PathBuf::new(),
                capture_parser,
                last_scrollback_update: Instant::now(),
                scrollback_update_interval: Duration::from_millis(500),
                last_screen_update: Instant::now(),
                screen_update_interval: Duration::from_millis(100),
                transcript_path: None,
                transcript_offset: 0,
                claude_pending_tools: HashMap::new(),
                codex_pending_tools: HashMap::new(),
                total_scrollback_lines: 0,
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
            last_scrollback_update: Instant::now() - Duration::from_secs(10),
            scrollback_update_interval: Duration::from_millis(500),
            last_screen_update: Instant::now() - Duration::from_secs(10),
            screen_update_interval: Duration::from_millis(100),
            transcript_path: None,
            transcript_offset: 0,
            claude_pending_tools: HashMap::new(),
            codex_pending_tools: HashMap::new(),
            total_scrollback_lines: 0,
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
                self.total_scrollback_lines = 0;
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
    /// Feeds bytes to our internal vt100 parser with large virtual screen.
    /// All content accumulates in the 10000-row buffer.
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

        crate::guarded_process(&mut self.capture_parser, data, 0);

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

    /// Append new content to scrollback.log from the platform transcript.
    ///
    /// Reads incrementally from the JSONL transcript file, which contains
    /// clean conversation history without status bar noise.
    pub fn update_scrollback(&mut self) -> std::io::Result<Option<ScrollbackUpdate>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let Some(ref transcript_path) = self.transcript_path else {
            // No transcript path yet - nothing to read
            self.last_scrollback_update = Instant::now();
            return Ok(None);
        };

        if !transcript_path.exists() {
            self.last_scrollback_update = Instant::now();
            return Ok(None);
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
        };

        if new_content.is_empty() {
            self.last_scrollback_update = Instant::now();
            return Ok(None);
        }

        // Count lines in new content
        let lines_added = new_content.lines().count();

        // Append to scrollback file
        let scrollback_path = self.capture_dir.join("scrollback.log");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&scrollback_path)?;
        file.write_all(new_content.as_bytes())?;

        self.transcript_offset = new_offset;
        self.total_scrollback_lines += lines_added;
        self.last_scrollback_update = Instant::now();

        Ok(Some(ScrollbackUpdate {
            diff: new_content,
            total_lines: self.total_scrollback_lines,
        }))
    }

    /// Get the full scrollback content accumulated so far.
    /// Used for initial sync when connecting to cloud.
    ///
    /// Reads from the scrollback.log file which contains formatted
    /// conversation history from the assistant platform's transcript.
    pub fn get_full_scrollback(&self) -> Option<String> {
        if !self.config.enabled {
            return None;
        }

        let scrollback_path = self.capture_dir.join("scrollback.log");
        if !scrollback_path.exists() {
            return None;
        }

        match fs::read_to_string(&scrollback_path) {
            Ok(content) if !content.is_empty() => Some(content),
            _ => None,
        }
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

        self.last_screen_update = Instant::now();
        Ok(contents)
    }

    /// Get the capture directory path.
    #[allow(dead_code)]
    pub fn capture_dir(&self) -> &PathBuf {
        &self.capture_dir
    }

    /// Check if capture is enabled.
    #[allow(dead_code)]
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
