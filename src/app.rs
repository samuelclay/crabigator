//! Crabigator App - Scroll region approach
//!
//! Architecture:
//! - Set terminal scroll region to top N rows for Claude Code
//! - Claude Code renders within that region (thinks it's the full terminal)
//! - We render our status widgets below the scroll region
//! - PTY output passes through untouched

use anyhow::Result;
use crossterm::event::{self, Event, MouseEvent};
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::capture::{CaptureConfig, CaptureManager};
use crate::git::GitState;
use crate::hooks::ClaudeStats;
use crate::mirror::MirrorPublisher;
use crate::parsers::DiffSummary;
use crate::terminal::{escape, forward_key_to_pty, ClaudePty};
use crate::ui::{draw_status_bar, Layout};

/// Result from background git refresh
struct GitRefreshResult {
    git_state: GitState,
    diff_summary: DiffSummary,
}

pub struct App {
    pub running: bool,
    pub claude_pty: ClaudePty,
    pub git_state: GitState,
    pub diff_summary: DiffSummary,
    pub claude_stats: ClaudeStats,
    pub last_mouse_event: Option<MouseEvent>,

    // Layout
    pub total_rows: u16,
    pub total_cols: u16,
    pub pty_rows: u16,
    pub status_rows: u16,

    /// Current working directory for platform stats
    cwd: String,
    pty_rx: mpsc::Receiver<Vec<u8>>,
    /// Mirror publisher for external inspection
    mirror_publisher: MirrorPublisher,
    /// Output capture manager for streaming
    capture_manager: CaptureManager,
}

impl App {
    pub async fn new(cols: u16, rows: u16, claude_args: Vec<String>, capture_enabled: bool) -> Result<Self> {
        let (pty_tx, pty_rx) = mpsc::channel(256);

        // Reserve bottom 20% for our status widgets (minimum 3 rows)
        let status_rows = ((rows as f32 * 0.2) as u16).max(3);
        let pty_rows = rows.saturating_sub(status_rows);

        // Give Claude Code only the top portion
        let claude_pty = ClaudePty::new(pty_tx, cols, pty_rows, claude_args).await?;
        let git_state = GitState::new();
        let diff_summary = DiffSummary::new();
        let claude_stats = ClaudeStats::new();

        // Get current working directory for platform stats
        let cwd = std::env::current_dir()
            .map(|p| p.to_string_lossy().to_string())
            .unwrap_or_default();

        // Create mirror publisher (always enabled for inspection by other instances)
        let session_id = std::env::var("CRABIGATOR_SESSION_ID").unwrap_or_default();
        let mirror_publisher = MirrorPublisher::new(true, session_id.clone(), cwd.clone(), capture_enabled);

        // Create capture manager for output streaming
        let capture_config = CaptureConfig {
            enabled: capture_enabled,
            session_id,
        };
        let capture_manager = CaptureManager::new(capture_config)?;

        Ok(Self {
            running: true,
            claude_pty,
            git_state,
            diff_summary,
            claude_stats,
            last_mouse_event: None,
            total_rows: rows,
            total_cols: cols,
            pty_rows,
            status_rows,
            cwd,
            pty_rx,
            mirror_publisher,
            capture_manager,
        })
    }

    /// Set scroll region to constrain PTY output to top area
    fn setup_scroll_region(&self, initial: bool) -> Result<()> {
        let mut stdout = stdout();

        // On initial setup, scroll existing terminal content up to make room
        // for our status bar. This preserves the user's last commands.
        if initial {
            // Move to bottom of terminal and emit newlines to push content up
            write!(stdout, "{}", escape::cursor_to(self.total_rows, 1))?;
            write!(stdout, "{}", escape::scroll_up(self.status_rows))?;
        }

        // DECSTBM: Set Top and Bottom Margins (1-indexed)
        // This constrains scrolling to rows 1 through pty_rows
        write!(stdout, "{}", escape::scroll_region(1, self.pty_rows))?;
        // Move cursor to bottom of scroll region so Claude Code starts there
        // and naturally scrolls up as it produces output (like a normal shell)
        write!(stdout, "{}", escape::cursor_to(self.pty_rows, 1))?;
        stdout.flush()?;
        Ok(())
    }

    /// Reset scroll region to full screen
    fn reset_scroll_region(&self) -> Result<()> {
        let mut stdout = stdout();
        write!(stdout, "{}", escape::SCROLL_REGION_RESET)?;
        stdout.flush()?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut last_git_refresh = Instant::now();
        let mut last_hook_refresh = Instant::now();
        let mut last_status_draw = Instant::now();
        let git_refresh_interval = Duration::from_secs(3);
        let hook_refresh_interval = Duration::from_millis(500);
        let status_debounce = Duration::from_millis(100);

        // Set up scroll region to constrain Claude Code to top area
        // Pass true to scroll existing content up and make room for status bar
        self.setup_scroll_region(true)?;

        // Initial status bar draw
        self.draw_status_bar()?;

        // Trigger initial git refresh immediately
        self.refresh_git_state().await;
        self.draw_status_bar()?;

        // Channel for receiving background git refresh results
        let (git_tx, mut git_rx) = mpsc::channel::<GitRefreshResult>(1);
        let mut git_refresh_pending = false;

        while self.running {
            // Receive PTY output and write directly to stdout
            let mut got_output = false;
            while let Ok(data) = self.pty_rx.try_recv() {
                self.write_pty_output(&data)?;
                got_output = true;
            }

            // Check for completed background git refresh (non-blocking)
            if let Ok(result) = git_rx.try_recv() {
                self.git_state = result.git_state;
                self.diff_summary = result.diff_summary;
                git_refresh_pending = false;
                // Redraw with new data
                self.draw_status_bar()?;
                last_status_draw = Instant::now();
            }

            // Spawn background git refresh periodically (if not already pending)
            if !git_refresh_pending && last_git_refresh.elapsed() >= git_refresh_interval {
                git_refresh_pending = true;
                last_git_refresh = Instant::now();
                let tx = git_tx.clone();
                tokio::spawn(async move {
                    let git_state_tmp = GitState::new();
                    let diff_summary_tmp = DiffSummary::new();
                    let (git_result, diff_result) = tokio::join!(
                        git_state_tmp.refresh(),
                        diff_summary_tmp.refresh()
                    );
                    let git_state = git_result.unwrap_or_default();
                    let diff_summary = diff_result.unwrap_or_default();
                    let _ = tx.send(GitRefreshResult { git_state, diff_summary }).await;
                });
            }

            // Refresh platform stats more frequently and redraw if state changed
            if last_hook_refresh.elapsed() >= hook_refresh_interval {
                let old_state = self.claude_stats.platform_stats.state;
                self.claude_stats.refresh_platform_stats(&self.cwd);
                let new_state = self.claude_stats.platform_stats.state;
                // Redraw immediately if state changed (e.g., Thinking -> Complete)
                if old_state != new_state {
                    self.draw_status_bar()?;
                    last_status_draw = Instant::now();
                }
                last_hook_refresh = Instant::now();
            }

            // Redraw status bar after PTY output settles (debounced)
            if got_output && last_status_draw.elapsed() >= status_debounce {
                self.draw_status_bar()?;
                last_status_draw = Instant::now();
            }

            // Update screen capture (throttled internally)
            if got_output {
                let _ = self.capture_manager.maybe_update_screen(self.claude_pty.screen());
            }

            // Check if Claude Code has exited
            if !self.claude_pty.is_running() {
                self.running = false;
                break;
            }

            // Poll for terminal events
            if event::poll(Duration::from_millis(50))? {
                match event::read()? {
                    Event::Key(key) => {
                        self.handle_key_event(key).await?;
                    }
                    Event::Resize(width, height) => {
                        self.handle_resize(width, height)?;
                    }
                    Event::Paste(text) => {
                        self.claude_pty.write(text.as_bytes())?;
                    }
                    Event::Mouse(mouse) => {
                        self.last_mouse_event = Some(mouse);
                    }
                    _ => {}
                }
            }
        }

        // Clean up capture directory before exit
        self.capture_manager.cleanup();

        // Clean up mirror file before exit
        self.mirror_publisher.cleanup();

        // Reset scroll region before exit
        self.reset_scroll_region()?;

        Ok(())
    }

    /// Write PTY output directly to stdout - transparent passthrough
    fn write_pty_output(&mut self, data: &[u8]) -> Result<()> {
        // Update our internal parser first (so screen state is current)
        self.claude_pty.process_output(data);

        // Capture scrollback by diffing screen state
        if let Err(e) = self.capture_manager.capture_scrollback(self.claude_pty.screen()) {
            // Log error but don't fail - capture is non-critical
            eprintln!("Capture error: {}", e);
        }

        let mut stdout = stdout();
        stdout.write_all(data)?;
        stdout.flush()?;
        Ok(())
    }

    /// Draw status bar using the widget system
    fn draw_status_bar(&mut self) -> Result<()> {
        // Update stats each draw
        self.claude_stats.tick();

        let layout = Layout {
            pty_rows: self.pty_rows,
            total_cols: self.total_cols,
            status_rows: self.status_rows,
        };

        let mut stdout = stdout();
        draw_status_bar(
            &mut stdout,
            &layout,
            &self.claude_stats,
            &self.git_state,
            &self.diff_summary,
        )?;

        // Publish mirror state (throttled, only when --profile)
        let _ = self.mirror_publisher.maybe_publish(
            &self.claude_stats,
            &self.git_state,
            &self.diff_summary,
        );

        Ok(())
    }

    async fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        forward_key_to_pty(key, &mut self.claude_pty)?;
        Ok(())
    }

    fn handle_resize(&mut self, width: u16, height: u16) -> Result<()> {
        self.total_cols = width;
        self.total_rows = height;

        // Recalculate layout
        self.status_rows = ((height as f32 * 0.2) as u16).max(3);
        self.pty_rows = height.saturating_sub(self.status_rows);

        // Re-setup scroll region for new size (not initial, don't scroll content)
        self.setup_scroll_region(false)?;

        // Resize PTY to new dimensions (only the top portion)
        self.claude_pty.resize(width, self.pty_rows)?;

        // Redraw status bar in new position
        self.draw_status_bar()?;

        Ok(())
    }

    async fn refresh_git_state(&mut self) {
        // Run git status and diff parsing in parallel
        let (git_result, diff_result) = tokio::join!(
            self.git_state.refresh(),
            self.diff_summary.refresh()
        );

        if let Ok(status) = git_result {
            self.git_state = status;
        }
        if let Ok(diff) = diff_result {
            self.diff_summary = diff;
        }
    }
}
