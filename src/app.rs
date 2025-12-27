//! Crabigator App - Scroll region approach
//!
//! Architecture:
//! - Set terminal scroll region to top N rows for the assistant CLI
//! - The assistant CLI renders within that region (thinks it's the full terminal)
//! - We render our status widgets below the scroll region
//! - PTY output passes through untouched

use anyhow::Result;
use crossterm::event::{self, Event, MouseEvent};
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::capture::{CaptureConfig, CaptureManager};
use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::platforms::Platform;
use crate::mirror::MirrorPublisher;
use crate::parsers::DiffSummary;
use crate::terminal::{escape, forward_key_to_pty, DsrChunk, DsrHandler, OscScanner, PlatformPty};
use crate::ui::{draw_status_bar, Layout};

/// Result from background git refresh
struct GitRefreshResult {
    git_state: GitState,
    diff_summary: DiffSummary,
    /// Time taken for git status refresh (ms)
    git_time_ms: u64,
    /// Time taken for diff summary parsing (ms)
    diff_time_ms: u64,
}

pub struct App {
    pub running: bool,
    pub platform: Box<dyn Platform>,
    pub platform_pty: PlatformPty,
    pub git_state: GitState,
    pub diff_summary: DiffSummary,
    pub session_stats: SessionStats,
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
    /// Handles terminal DSR responses for CLIs that request cursor position
    dsr_handler: DsrHandler,
    /// Scans for OSC title sequences from the CLI
    osc_scanner: OscScanner,
    /// Terminal title extracted from OSC sequences (e.g., "Claude Code Ghostty Integration")
    terminal_title: Option<String>,
    /// Time taken for initial git refresh (set once on first load)
    initial_git_time_ms: Option<u64>,
    /// Time taken for initial diff parsing (set once on first load)
    initial_diff_time_ms: Option<u64>,
}

impl App {
    pub async fn new(
        cols: u16,
        rows: u16,
        platform: Box<dyn Platform>,
        platform_args: Vec<String>,
        capture_enabled: bool,
    ) -> Result<Self> {
        let (pty_tx, pty_rx) = mpsc::channel(256);

        // Reserve bottom 20% for our status widgets (minimum 2 rows: separator + header)
        // Also ensure pty_rows is at least 1 to avoid PTY errors
        // Guard: ensure max >= min for clamp (handles very short terminals)
        let status_rows = ((rows as f32 * 0.2) as u16).clamp(2, rows.saturating_sub(1).max(2));
        let pty_rows = rows.saturating_sub(status_rows).max(1);

        // Give the assistant CLI only the top portion
        let platform_pty = PlatformPty::new(
            pty_tx,
            cols,
            pty_rows,
            platform.command(),
            platform_args,
        )
        .await?;
        let git_state = GitState::new();
        let diff_summary = DiffSummary::new();
        let session_stats = SessionStats::new();

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
            platform,
            platform_pty,
            git_state,
            diff_summary,
            session_stats,
            last_mouse_event: None,
            total_rows: rows,
            total_cols: cols,
            pty_rows,
            status_rows,
            cwd,
            pty_rx,
            mirror_publisher,
            capture_manager,
            dsr_handler: DsrHandler::new(),
            osc_scanner: OscScanner::new(),
            terminal_title: None,
            initial_git_time_ms: None,
            initial_diff_time_ms: None,
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
        // Move cursor to bottom of scroll region so the CLI starts there
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

        // Set up scroll region to constrain the CLI to the top area
        // Pass true to scroll existing content up and make room for status bar
        self.setup_scroll_region(true)?;

        // Initial status bar draw (shows "loading" state for git widgets)
        self.draw_status_bar()?;

        // Channel for receiving background git refresh results
        let (git_tx, mut git_rx) = mpsc::channel::<GitRefreshResult>(1);
        let mut git_refresh_pending = true; // Start with refresh pending

        // Spawn initial git refresh in background (non-blocking)
        // This allows the PTY to be visible immediately while git loads
        {
            let tx = git_tx.clone();
            tokio::spawn(async move {
                let git_state_tmp = GitState::new();
                let diff_summary_tmp = DiffSummary::new();

                // Time each refresh separately
                let git_start = Instant::now();
                let git_result = git_state_tmp.refresh().await;
                let git_time_ms = git_start.elapsed().as_millis() as u64;

                let diff_start = Instant::now();
                let diff_result = diff_summary_tmp.refresh().await;
                let diff_time_ms = diff_start.elapsed().as_millis() as u64;

                let git_state = git_result.unwrap_or_default();
                let diff_summary = diff_result.unwrap_or_default();
                let _ = tx.send(GitRefreshResult {
                    git_state,
                    diff_summary,
                    git_time_ms,
                    diff_time_ms,
                }).await;
            });
        }

        // Initial screen capture (write immediately so file isn't blank on startup)
        let _ = self
            .capture_manager
            .update_screen(self.platform_pty.screen());

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

                // Capture initial timing (only set once, on first load)
                if self.initial_git_time_ms.is_none() {
                    self.initial_git_time_ms = Some(result.git_time_ms);
                    self.initial_diff_time_ms = Some(result.diff_time_ms);
                }

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
                    // Timing not tracked for periodic refreshes (only initial)
                    let _ = tx.send(GitRefreshResult {
                        git_state,
                        diff_summary,
                        git_time_ms: 0,
                        diff_time_ms: 0,
                    }).await;
                });
            }

            // Refresh platform stats more frequently and redraw if state changed
            if last_hook_refresh.elapsed() >= hook_refresh_interval {
                let old_state = self.session_stats.platform_stats.state;
                self.session_stats
                    .refresh_platform_stats(self.platform.as_ref(), &self.cwd);
                let new_state = self.session_stats.platform_stats.state;
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

            // Update captures (throttled internally)
            if got_output {
                let _ = self.capture_manager.maybe_update_screen(self.platform_pty.screen());
                let _ = self.capture_manager.maybe_update_scrollback();
            }

            // Check if the platform CLI has exited
            if !self.platform_pty.is_running() {
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
                        self.platform_pty.write(text.as_bytes())?;
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

        // Clean up stats file before exit
        self.platform.cleanup_stats(&self.cwd);

        // Reset scroll region before exit
        self.reset_scroll_region()?;

        Ok(())
    }

    /// Write PTY output directly to stdout - transparent passthrough
    fn write_pty_output(&mut self, data: &[u8]) -> Result<()> {
        let mut stdout = stdout();
        let mut wrote_output = false;

        let chunks = self.dsr_handler.scan(data);
        for chunk in chunks {
            match chunk {
                DsrChunk::Output(bytes) => {
                    if bytes.is_empty() {
                        continue;
                    }

                    // Scan for OSC title sequences
                    let (passthrough, title) = self.osc_scanner.scan(&bytes);
                    if let Some(t) = title {
                        self.terminal_title = Some(t);
                    }

                    if passthrough.is_empty() {
                        continue;
                    }
                    wrote_output = true;
                    // Capture through our internal vt100 parser
                    if let Err(e) = self.capture_manager.capture_output(&passthrough) {
                        eprintln!("Capture error: {}", e);
                    }
                    self.platform_pty.process_output(&passthrough);
                    stdout.write_all(&passthrough)?;
                }
                DsrChunk::Request => {
                    let (row, col) = self.platform_pty.screen().cursor_position();
                    let response = escape::cursor_position_report(
                        row.saturating_add(1),
                        col.saturating_add(1),
                    );
                    self.platform_pty.write(response.as_bytes())?;
                }
            }
        }

        if wrote_output {
            stdout.flush()?;
        }

        Ok(())
    }

    /// Draw status bar using the widget system
    fn draw_status_bar(&mut self) -> Result<()> {
        // Update stats each draw
        self.session_stats.tick();

        let layout = Layout {
            pty_rows: self.pty_rows,
            total_cols: self.total_cols,
            status_rows: self.status_rows,
        };

        let mut stdout = stdout();
        draw_status_bar(
            &mut stdout,
            &layout,
            &self.session_stats,
            &self.git_state,
            &self.diff_summary,
            self.terminal_title.as_deref(),
        )?;

        // Publish mirror state (throttled, only when --profile)
        let _ = self.mirror_publisher.maybe_publish(
            &self.session_stats,
            &self.git_state,
            &self.diff_summary,
            self.terminal_title.as_deref(),
            self.initial_git_time_ms,
            self.initial_diff_time_ms,
        );

        Ok(())
    }

    async fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        if key.kind != crossterm::event::KeyEventKind::Press {
            return Ok(());
        }
        forward_key_to_pty(key, &mut self.platform_pty)?;
        Ok(())
    }

    fn handle_resize(&mut self, width: u16, height: u16) -> Result<()> {
        self.total_cols = width;
        self.total_rows = height;

        // Recalculate layout with same guards as App::new
        // Guard: ensure max >= min for clamp (handles very short terminals)
        self.status_rows = ((height as f32 * 0.2) as u16).clamp(2, height.saturating_sub(1).max(2));
        self.pty_rows = height.saturating_sub(self.status_rows).max(1);

        // Re-setup scroll region for new size (not initial, don't scroll content)
        self.setup_scroll_region(false)?;

        // Resize PTY to new dimensions (only the top portion)
        self.platform_pty.resize(width, self.pty_rows)?;

        // Redraw status bar in new position
        self.draw_status_bar()?;

        Ok(())
    }

}
