//! Output capture for streaming-ready session recording.
//!
//! Captures assistant CLI PTY output to two files:
//! - `scrollback.log`: Lines that have scrolled off the visible screen
//! - `screen.txt`: Current screen snapshot from vt100 (overwritten periodically)
//!
//! Uses vt100 screen diffing to detect when lines scroll off.

use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for output capture.
pub struct CaptureConfig {
    /// Whether capture is enabled (default: true, disabled with --no-capture)
    pub enabled: bool,
    /// Session ID for unique directory naming
    pub session_id: String,
}

/// Manages output capture to scrollback.log and screen.txt files.
pub struct CaptureManager {
    config: CaptureConfig,
    /// Base directory: /tmp/crabigator-capture-{session_id}/
    capture_dir: PathBuf,
    /// Buffered writer for scrollback.log
    scrollback_writer: Option<BufWriter<File>>,
    /// Previous screen content (for diffing)
    last_screen_lines: Vec<String>,
    /// Last screen.txt update time (for throttling)
    last_screen_update: Instant,
    /// Screen update interval
    screen_update_interval: Duration,
}

impl CaptureManager {
    /// Create a new CaptureManager.
    pub fn new(config: CaptureConfig) -> std::io::Result<Self> {
        if !config.enabled {
            return Ok(Self {
                config,
                capture_dir: PathBuf::new(),
                scrollback_writer: None,
                last_screen_lines: Vec::new(),
                last_screen_update: Instant::now(),
                screen_update_interval: Duration::from_millis(100),
            });
        }

        let capture_dir = PathBuf::from(format!(
            "/tmp/crabigator-capture-{}",
            config.session_id
        ));

        // Create directory
        fs::create_dir_all(&capture_dir)?;

        // Open scrollback.log for append
        let scrollback_path = capture_dir.join("scrollback.log");
        let scrollback_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&scrollback_path)?;
        let scrollback_writer = Some(BufWriter::new(scrollback_file));

        Ok(Self {
            config,
            capture_dir,
            scrollback_writer,
            last_screen_lines: Vec::new(),
            last_screen_update: Instant::now() - Duration::from_secs(10),
            screen_update_interval: Duration::from_millis(100),
        })
    }

    /// Capture scrollback by diffing screen contents.
    ///
    /// Compares current screen to previous screen. When lines scroll off
    /// the top (shift up), they're written to scrollback.log.
    pub fn capture_scrollback(&mut self, screen: &vt100::Screen) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let current_content = screen.contents();
        let current_lines: Vec<String> = current_content.lines().map(|s| s.to_string()).collect();

        // If this is the first capture, just save the state
        if self.last_screen_lines.is_empty() {
            self.last_screen_lines = current_lines;
            return Ok(());
        }

        // Detect scroll: check if the old screen's lines have shifted up
        // by looking for where the old top line appears in the new screen
        let scrolled_lines = self.detect_scrolled_lines(&current_lines);

        if !scrolled_lines.is_empty() {
            if let Some(ref mut writer) = self.scrollback_writer {
                for line in &scrolled_lines {
                    // Only write non-empty lines
                    let trimmed = line.trim_end();
                    if !trimmed.is_empty() {
                        writeln!(writer, "{}", trimmed)?;
                    }
                }
                writer.flush()?;
            }
        }

        self.last_screen_lines = current_lines;
        Ok(())
    }

    /// Detect lines that have scrolled off the top.
    fn detect_scrolled_lines(&self, current_lines: &[String]) -> Vec<String> {
        if self.last_screen_lines.is_empty() || current_lines.is_empty() {
            return Vec::new();
        }

        // Find where the first line of the old screen appears in the new screen
        // If it moved down (or disappeared), lines scrolled off
        let old_first = &self.last_screen_lines[0];

        // Look for old_first in current screen
        for (i, line) in current_lines.iter().enumerate() {
            if line == old_first && i > 0 {
                // The old first line is now at position i
                // That means i lines scrolled off the top
                // But we need to be careful - the match might be coincidental
                // Verify by checking if subsequent lines also match
                let mut matches = 0;
                for j in 0..self.last_screen_lines.len().min(current_lines.len() - i) {
                    if self.last_screen_lines[j] == current_lines[i + j] {
                        matches += 1;
                    }
                }

                // If at least half the lines match, this is likely a scroll
                if matches >= self.last_screen_lines.len() / 2 {
                    // Return the lines that scrolled off
                    return self.last_screen_lines[..i.min(self.last_screen_lines.len())].to_vec();
                }
            }
        }

        // Also check: if current screen is completely different,
        // the entire old screen may have scrolled off
        // This handles cases like screen clears followed by new content
        let common_lines = current_lines
            .iter()
            .filter(|line| self.last_screen_lines.contains(line))
            .count();

        // If very few lines in common and screen is full, old content scrolled
        if common_lines < 3 && !current_lines.is_empty() {
            // Return all old non-empty lines as scrollback
            return self.last_screen_lines
                .iter()
                .filter(|line| !line.trim().is_empty())
                .cloned()
                .collect();
        }

        Vec::new()
    }

    /// Update screen.txt if the throttle interval has elapsed.
    pub fn maybe_update_screen(&mut self, screen: &vt100::Screen) -> std::io::Result<bool> {
        if !self.config.enabled {
            return Ok(false);
        }

        if self.last_screen_update.elapsed() < self.screen_update_interval {
            return Ok(false);
        }

        self.update_screen(screen)?;
        Ok(true)
    }

    /// Force immediate screen.txt update.
    pub fn update_screen(&mut self, screen: &vt100::Screen) -> std::io::Result<()> {
        if !self.config.enabled {
            return Ok(());
        }

        let screen_path = self.capture_dir.join("screen.txt");
        let tmp_path = self.capture_dir.join("screen.txt.tmp");

        // Get screen contents (plain text)
        let contents = screen.contents();

        // Atomic write: temp file + rename
        fs::write(&tmp_path, &contents)?;
        fs::rename(&tmp_path, &screen_path)?;

        self.last_screen_update = Instant::now();
        Ok(())
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
