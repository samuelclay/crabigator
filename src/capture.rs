//! Output capture for streaming-ready session recording.
//!
//! Captures assistant CLI PTY output to files:
//! - `scrollback.log`: Session transcript with ANSI codes (append-only)
//! - `screen.txt`: Current screen snapshot with ANSI codes (rendered by vt100)
//!
//! Uses a vt100 parser matching PTY dimensions with large scrollback buffer.
//! Content that scrolls off the visible area is captured to scrollback.log.

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
    /// vt100 parser matching PTY dimensions with large scrollback buffer
    capture_parser: vt100::Parser,
    /// Last scrollback.log update time (for throttling)
    last_scrollback_update: Instant,
    /// Scrollback update interval (scales with buffer size)
    scrollback_update_interval: Duration,
    /// Last screen.txt update time (for throttling)
    last_screen_update: Instant,
    /// Screen update interval
    screen_update_interval: Duration,
    /// Amount of scrollback content already written to file
    last_scrollback_extracted: usize,
    /// Total line count in scrollback.log
    total_scrollback_lines: usize,
    /// Raw PTY output log file (debug builds only)
    #[cfg(debug_assertions)]
    raw_log: Option<File>,
    /// Current raw log size for rotation checks
    #[cfg(debug_assertions)]
    raw_log_size: u64,
}

impl CaptureManager {
    /// Create a new CaptureManager.
    ///
    /// Uses a large virtual screen (10000 rows) so all content remains accessible.
    /// Claude Code's status bar content is filtered out during extraction.
    pub fn new(config: CaptureConfig, cols: u16, _rows: u16) -> std::io::Result<Self> {
        // Use a very tall virtual screen (10000 rows) so content never scrolls off
        // We filter out Claude Code's status bar during extraction
        let capture_parser = vt100::Parser::new(10000, cols, 0);

        if !config.enabled {
            return Ok(Self {
                config,
                capture_dir: PathBuf::new(),
                capture_parser,
                last_scrollback_update: Instant::now(),
                scrollback_update_interval: Duration::from_millis(100),
                last_screen_update: Instant::now(),
                screen_update_interval: Duration::from_millis(100),
                last_scrollback_extracted: 0,
                total_scrollback_lines: 0,
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
            last_scrollback_extracted: 0,
            total_scrollback_lines: 0,
            #[cfg(debug_assertions)]
            raw_log,
            #[cfg(debug_assertions)]
            raw_log_size,
        })
    }

    /// Resize the capture parser width to match PTY.
    /// Height stays at 10000 to keep all content accessible.
    pub fn resize(&mut self, cols: u16, _rows: u16) {
        self.capture_parser.set_size(10000, cols);
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
    /// Extracts rows from the capture parser, filtering out Claude Code's
    /// status bar content (context percentage, edit hints, path, token count).
    pub fn update_scrollback(&mut self) -> std::io::Result<Option<ScrollbackUpdate>> {
        if !self.config.enabled {
            return Ok(None);
        }

        let screen = self.capture_parser.screen();
        let (_, cols) = screen.size();
        let (cursor_row, _) = screen.cursor_position();

        // Skip if no new rows to write
        if cursor_row as usize <= self.last_scrollback_extracted && self.last_scrollback_extracted > 0 {
            self.last_scrollback_update = Instant::now();
            return Ok(None);
        }

        let start_row = self.last_scrollback_extracted;
        let end_row = cursor_row as usize + 1;

        // Build the new content with ANSI codes for color preservation
        // Filter out Claude Code's status bar content and collapse empty lines
        let mut content: Vec<u8> = Vec::new();
        let mut consecutive_empty = 0u8;
        let mut lines_extracted = 0usize;

        for row_bytes in screen.rows_formatted(0, cols).skip(start_row).take(end_row - start_row) {
            let row_str = String::from_utf8_lossy(&row_bytes);
            let trimmed = row_str.trim_end();

            // Skip Claude Code status bar lines
            if Self::is_status_bar_line(trimmed) {
                continue;
            }

            if trimmed.is_empty() {
                consecutive_empty += 1;
                // Allow at most one blank line (paragraph break)
                if consecutive_empty <= 1 {
                    content.push(b'\n');
                    lines_extracted += 1;
                }
            } else {
                consecutive_empty = 0;
                content.extend_from_slice(trimmed.as_bytes());
                content.extend_from_slice(b"\x1b[0m"); // Reset to prevent color leakage
                content.push(b'\n');
                lines_extracted += 1;
            }
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

        self.last_scrollback_extracted = cursor_row as usize;
        self.total_scrollback_lines += lines_extracted;
        self.last_scrollback_update = Instant::now();

        Ok(Some(ScrollbackUpdate {
            diff: String::from_utf8_lossy(&content).to_string(),
            total_lines: self.total_scrollback_lines,
        }))
    }

    /// Check if a line is part of Claude Code's status bar UI.
    ///
    /// These lines are redrawn frequently and should not be captured
    /// in the scrollback transcript.
    fn is_status_bar_line(line: &str) -> bool {
        // Context percentage indicator
        if line.contains("Context left until auto-compact:") {
            return true;
        }

        // Edit mode hint with file counts
        if line.contains("accept edits on") && line.contains("shift+tab") {
            return true;
        }

        // Plan mode hint
        if line.contains("plan mode") && line.contains("shift+tab") {
            return true;
        }

        // Token count (usually at end of status bar)
        // Match pattern like "140812 tokens" at the end
        if line.ends_with(" tokens") {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Some(second_last) = parts.get(parts.len() - 2) {
                    if second_last.chars().all(|c| c.is_ascii_digit()) {
                        return true;
                    }
                }
            }
        }

        // MCP server status
        if line.contains("MCP server") && (line.contains("needs auth") || line.contains("/mcp")) {
            return true;
        }

        // Path with git branch - match "~/path ‹branch›" or "~/path ‹branch*›"
        // These appear as part of Claude Code's prompt/status area
        if line.contains('‹') && line.contains('›') && line.starts_with("  ~/") {
            return true;
        }

        // Horizontal separator lines (all same character repeated)
        let chars: Vec<char> = line.chars().collect();
        if chars.len() > 50 {
            let first = chars[0];
            if first == '─' || first == '━' || first == '-' || first == '=' {
                if chars.iter().all(|&c| c == first) {
                    return true;
                }
            }
        }

        false
    }

    /// Get the full scrollback content accumulated so far.
    /// Used for initial sync when connecting to cloud.
    ///
    /// Reads from the scrollback.log file which contains all content
    /// that has scrolled off the visible area.
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
    /// Uses the platform_pty's screen (actual terminal size with scroll region).
    pub fn update_screen(&mut self, screen: &vt100::Screen) -> std::io::Result<String> {
        if !self.config.enabled {
            return Ok(String::new());
        }

        // Use the passed-in screen (platform_pty.screen()) which has proper terminal dimensions
        // This is much smaller than our 10,000 row capture_parser (typically 40-60 rows)
        let (rows, cols) = screen.size();

        // Collect all rows (small fixed size - typically 40-60 rows)
        let formatted_rows: Vec<Vec<u8>> = screen
            .rows_formatted(0, cols)
            .take(rows as usize)
            .collect();

        // Find last non-empty row
        let last_nonempty = formatted_rows
            .iter()
            .enumerate()
            .filter(|(_, row)| row.iter().any(|&b| !b.is_ascii_whitespace()))
            .map(|(idx, _)| idx)
            .next_back()
            .unwrap_or(0);

        // Build content up to last non-empty row
        let mut content = Vec::new();
        for row_bytes in formatted_rows.into_iter().take(last_nonempty + 1) {
            content.extend_from_slice(&row_bytes);
            // Reset ANSI attributes at end of row to prevent color leakage
            // The vt100 crate's rows_formatted() doesn't emit reset codes
            content.extend_from_slice(b"\x1b[0m");
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
