//! Output capture for streaming-ready session recording.
//!
//! Captures assistant CLI PTY output to files:
//! - `scrollback.log`: Clean text transcript without ANSI codes (append-only)
//! - `screen.txt`: Current screen snapshot with ANSI codes (rendered by vt100)
//!
//! Uses a separate vt100 parser with a huge virtual screen to capture
//! all output without losing anything to scrollback.

use std::fs::{self, OpenOptions};
#[cfg(debug_assertions)]
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::time::{Duration, Instant};

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
    /// Base directory: /tmp/crabigator-{session_id}/
    capture_dir: PathBuf,
    /// Our own vt100 parser with huge screen to capture all output
    capture_parser: vt100::Parser,
    /// Last scrollback.log update time (for throttling)
    last_scrollback_update: Instant,
    /// Scrollback update interval (scales with buffer size)
    scrollback_update_interval: Duration,
    /// Last screen.txt update time (for throttling)
    last_screen_update: Instant,
    /// Screen update interval
    screen_update_interval: Duration,
    /// Last cursor row written to scrollback (for incremental updates)
    last_scrollback_row: u16,
    /// Raw PTY output log file (debug builds only)
    #[cfg(debug_assertions)]
    raw_log: Option<File>,
    /// Current raw log size for rotation checks
    #[cfg(debug_assertions)]
    raw_log_size: u64,
}

impl CaptureManager {
    /// Create a new CaptureManager.
    pub fn new(config: CaptureConfig) -> std::io::Result<Self> {
        // Use a very tall virtual screen (10000 rows) so content never scrolls off
        // Width of 300 should handle most terminal widths
        let capture_parser = vt100::Parser::new(10000, 300, 0);

        if !config.enabled {
            return Ok(Self {
                config,
                capture_dir: PathBuf::new(),
                capture_parser,
                last_scrollback_update: Instant::now(),
                scrollback_update_interval: Duration::from_millis(100),
                last_screen_update: Instant::now(),
                screen_update_interval: Duration::from_millis(100),
                last_scrollback_row: 0,
                #[cfg(debug_assertions)]
                raw_log: None,
                #[cfg(debug_assertions)]
                raw_log_size: 0,
            });
        }

        let capture_dir = PathBuf::from(format!(
            "/tmp/crabigator-{}",
            config.session_id
        ));

        // Create directory
        fs::create_dir_all(&capture_dir)?;

        // Open raw log file for appending (debug builds only)
        #[cfg(debug_assertions)]
        let (raw_log, raw_log_size) = {
            let path = capture_dir.join("pty_raw.log");
            let size = path.metadata().map(|m| m.len()).unwrap_or(0);
            let file = OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)?;
            (Some(file), size)
        };

        Ok(Self {
            config,
            capture_dir,
            capture_parser,
            last_scrollback_update: Instant::now() - Duration::from_secs(10),
            scrollback_update_interval: Duration::from_millis(100),
            last_screen_update: Instant::now() - Duration::from_secs(10),
            screen_update_interval: Duration::from_millis(100),
            last_scrollback_row: 0,
            #[cfg(debug_assertions)]
            raw_log,
            #[cfg(debug_assertions)]
            raw_log_size,
        })
    }

    /// Process PTY output bytes through our capture parser.
    ///
    /// This feeds the bytes to our internal vt100 parser which has a huge
    /// virtual screen, so all content accumulates without scrolling off.
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

        // Process through our capture parser
        self.capture_parser.process(data);

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

    /// Append new rows to scrollback.log (incremental update).
    ///
    /// Only appends rows that haven't been written yet, making this O(new rows)
    /// instead of O(total rows). Critical for performance in long sessions.
    pub fn update_scrollback(&mut self) -> std::io::Result<Option<ScrollbackUpdate>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let screen = self.capture_parser.screen();
        let (_, cols) = screen.size();
        let (cursor_row, _) = screen.cursor_position();

        // Skip if no new rows to write
        if cursor_row <= self.last_scrollback_row && self.last_scrollback_row > 0 {
            self.last_scrollback_update = Instant::now();
            return Ok(None);
        }

        let start_row = self.last_scrollback_row as usize;
        let end_row = cursor_row as usize + 1;

        // Build only the new content (plain text, no ANSI - much faster)
        let mut content: Vec<u8> = Vec::new();
        for row_str in screen.rows(0, cols).skip(start_row).take(end_row - start_row) {
            let trimmed = row_str.trim_end();
            content.extend_from_slice(trimmed.as_bytes());
            content.push(b'\n');
        }

        if content.is_empty() {
            self.last_scrollback_update = Instant::now();
            return Ok(None);
        }

        // Append to scrollback file
        let scrollback_path = self.capture_dir.join("scrollback.log");
        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&scrollback_path)?;
        file.write_all(&content)?;

        self.last_scrollback_row = cursor_row;
        self.last_scrollback_update = Instant::now();
        Ok(Some(ScrollbackUpdate {
            diff: String::from_utf8_lossy(&content).to_string(),
            total_lines: end_row,
        }))
    }

    /// Update screen.txt if the throttle interval has elapsed.
    pub fn maybe_update_screen(&mut self, screen: &vt100::Screen) -> std::io::Result<Option<String>> {
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
    /// Uses the capture_parser's screen (which is fed PTY data) not the passed-in screen.
    pub fn update_screen(&mut self, _screen: &vt100::Screen) -> std::io::Result<String> {
        if !self.config.enabled {
            return Ok(String::new());
        }

        // Use our capture_parser's screen (which has the actual content)
        // The platform_pty.screen() passed in is unused because its parser isn't fed data
        let screen = self.capture_parser.screen();
        let (_, cols) = screen.size();
        let (cursor_row, _) = screen.cursor_position();

        // Collect all rows with their formatted content
        let formatted_rows: Vec<Vec<u8>> = screen.rows_formatted(0, cols).collect();

        // Find the last row we need to include (max of cursor position and last non-empty row)
        let mut last_needed_row = cursor_row as usize;
        for (row_idx, row) in formatted_rows.iter().enumerate() {
            let row_str = String::from_utf8_lossy(row);
            if !row_str.trim().is_empty() && row_idx > last_needed_row {
                last_needed_row = row_idx;
            }
        }

        // Build content row-by-row with explicit newlines
        let mut content = Vec::new();
        for (row_idx, row) in formatted_rows.iter().enumerate() {
            if row_idx > last_needed_row {
                break;
            }
            content.extend_from_slice(row);
            // Add newline after each row
            content.push(b'\n');
        }

        let screen_path = self.capture_dir.join("screen.txt");
        let tmp_path = self.capture_dir.join("screen.txt.tmp");
        fs::write(&tmp_path, &content)?;
        fs::rename(&tmp_path, &screen_path)?;

        self.last_screen_update = Instant::now();
        Ok(String::from_utf8_lossy(&content).to_string())
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
