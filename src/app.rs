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

use crate::capture::{CaptureConfig, CaptureManager, ScrollbackUpdate};
use crate::cloud::{CloudClient, SessionEventBuilder};
use crate::config::Config;
use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::ide::{self, IdeKind};
use crate::platforms::{Platform, SessionState};
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
    cwd: std::path::PathBuf,
    /// Detected IDE for clickable hyperlinks
    ide: IdeKind,
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
    /// Cloud client for streaming to drinkcrabigator.com (optional)
    cloud_client: Option<CloudClient>,
    /// Last state sent to cloud (to avoid duplicate events)
    last_cloud_state: Option<SessionState>,
    /// Last scrollback line count sent to cloud (for diffs)
    last_cloud_scrollback_lines: usize,
    /// Last title sent to cloud (to avoid duplicate events)
    last_cloud_title: Option<String>,
    /// Whether we've sent an initial stats payload to cloud
    cloud_stats_sent: bool,
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
        let cwd = std::env::current_dir().unwrap_or_default();
        let cwd_str = cwd.to_string_lossy().to_string();

        // Detect IDE from config or environment
        let ide = Config::load()
            .ok()
            .and_then(|c| c.ide)
            .and_then(|s| IdeKind::from_config(&s))
            .unwrap_or_else(ide::detect_ide);

        // Create mirror publisher (always enabled for inspection by other instances)
        let session_id = std::env::var("CRABIGATOR_SESSION_ID").unwrap_or_default();
        let mirror_publisher = MirrorPublisher::new(true, session_id.clone(), cwd_str.clone(), capture_enabled);

        // Create capture manager for output streaming
        let capture_config = CaptureConfig {
            enabled: capture_enabled,
            session_id: session_id.clone(),
        };
        let capture_manager = CaptureManager::new(capture_config)?;

        // Initialize cloud client (optional - don't fail if cloud is unreachable)
        let cloud_client = Self::init_cloud_client(&session_id, &cwd_str, platform.as_ref()).await;

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
            ide,
            pty_rx,
            mirror_publisher,
            capture_manager,
            dsr_handler: DsrHandler::new(),
            osc_scanner: OscScanner::new(),
            terminal_title: None,
            initial_git_time_ms: None,
            initial_diff_time_ms: None,
            cloud_client,
            last_cloud_state: None,
            last_cloud_scrollback_lines: 0,
            last_cloud_title: None,
            cloud_stats_sent: false,
        })
    }

    /// Initialize cloud client - returns None if cloud is unreachable
    async fn init_cloud_client(
        session_id: &str,
        cwd: &str,
        platform: &dyn Platform,
    ) -> Option<CloudClient> {
        // Try to create cloud client
        let mut client = match CloudClient::new() {
            Ok(c) => c,
            Err(e) => {
                // Style: dim gray label, red X, dim error
                eprintln!(
                    "\x1b[38;5;245m     Cloud\x1b[0m  \x1b[38;5;203m✗\x1b[0m \x1b[2m{}\x1b[0m",
                    e
                );
                return None;
            }
        };

        // Try to register session with cloud
        match client.register_session(session_id, cwd, platform.kind().as_str()).await {
            Ok(cloud_session_id) => {
                // Style: dim gray label, green checkmark, dim session ID
                eprintln!(
                    "\x1b[38;5;245m     Cloud\x1b[0m  \x1b[38;5;114m✓\x1b[0m \x1b[2m{}\x1b[0m",
                    cloud_session_id
                );
                Some(client)
            }
            Err(e) => {
                // Style: dim gray label, red X, dim error
                eprintln!(
                    "\x1b[38;5;245m     Cloud\x1b[0m  \x1b[38;5;203m✗\x1b[0m \x1b[2m{}\x1b[0m",
                    e
                );
                None
            }
        }
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
        let mut last_throbber_draw = Instant::now();
        let git_refresh_interval = Duration::from_secs(3);
        let hook_refresh_interval = Duration::from_millis(500);
        let status_debounce = Duration::from_millis(100);
        let throbber_interval = Duration::from_millis(100);

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

        // Track whether we've sent an initial screen capture (after PTY has rendered)
        let mut sent_initial_screen = false;
        let session_start = std::time::Instant::now();
        let mut last_initial_screen_attempt = session_start;

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

                // Stream git + changes snapshot to cloud
                self.send_cloud_git_changes_events();

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
                let old_last_updated = self.session_stats.platform_stats.last_updated;
                self.session_stats
                    .refresh_platform_stats(self.platform.as_ref(), &self.cwd.to_string_lossy());
                let new_state = self.session_stats.platform_stats.state;
                let new_last_updated = self.session_stats.platform_stats.last_updated;

                // Redraw immediately if state changed (e.g., Thinking -> Complete)
                if old_state != new_state {
                    self.draw_status_bar()?;
                    last_status_draw = Instant::now();
                }

                // Send initial state once, then on changes
                if self.last_cloud_state.is_none() || old_state != new_state {
                    self.send_cloud_state_event(new_state);
                }

                // Stream stats when platform stats update (or first send)
                if new_last_updated != old_last_updated || !self.cloud_stats_sent {
                    self.cloud_stats_sent = true;
                    self.session_stats.tick();
                    self.send_cloud_stats_event();
                    self.send_cloud_stats_update();
                }

                last_hook_refresh = Instant::now();
            }

            // Check for answers from cloud (mobile → desktop)
            self.check_cloud_answers()?;

            // Redraw status bar after PTY output settles (debounced)
            if got_output && last_status_draw.elapsed() >= status_debounce {
                self.draw_status_bar()?;
                last_status_draw = Instant::now();
                last_throbber_draw = Instant::now();
            }

            // Animate throbber independently when in active states (Thinking/Permission)
            let needs_throbber = matches!(
                self.session_stats.platform_stats.state,
                SessionState::Thinking | SessionState::Permission
            );
            if needs_throbber && last_throbber_draw.elapsed() >= throbber_interval {
                self.draw_status_bar()?;
                last_throbber_draw = Instant::now();
            }

            // Update captures (throttled internally)
            if got_output {
                if let Ok(Some(screen)) =
                    self.capture_manager
                        .maybe_update_screen(self.platform_pty.screen())
                {
                    self.send_cloud_screen_event(screen);
                    sent_initial_screen = true;
                }
                if let Ok(Some(update)) = self.capture_manager.maybe_update_scrollback() {
                    self.send_cloud_scrollback_event(update);
                }
            }

            // Send initial screen after terminal has had time to render
            // Try every 500ms until we get meaningful content (>50 bytes)
            // or give up after 5 seconds and send whatever we have
            if !sent_initial_screen && last_initial_screen_attempt.elapsed() > Duration::from_millis(500) {
                last_initial_screen_attempt = Instant::now();
                let elapsed = session_start.elapsed();
                if let Ok(contents) = self.capture_manager.update_screen(self.platform_pty.screen()) {
                    // Send if we have meaningful content or we've waited long enough
                    if contents.len() > 50 || elapsed > Duration::from_secs(5) {
                        self.send_cloud_screen_event(contents);
                        sent_initial_screen = true;
                    }
                }
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

        // Flush final stats + mark session ended in cloud
        if self.cloud_client.is_some() {
            self.session_stats.tick();
            self.send_cloud_stats_event();
            let tool_calls = self.session_stats.platform_stats.total_tool_calls();
            if let Some(ref client) = self.cloud_client {
                let _ = client
                    .end_session(
                        self.session_stats.platform_stats.prompts,
                        self.session_stats.platform_stats.completions,
                        tool_calls,
                        self.session_stats.thinking_seconds(),
                    )
                    .await;
            }
        }

        // Clean up capture directory before exit
        self.capture_manager.cleanup();

        // Clean up mirror file before exit
        self.mirror_publisher.cleanup();

        // Clean up stats file before exit
        self.platform.cleanup_stats(&self.cwd.to_string_lossy());

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
                        self.terminal_title = Some(t.clone());
                        self.send_cloud_title_event(t);
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

        // Get cloud status if connected
        let cloud_status = self.cloud_client.as_ref().map(|c| c.status());

        let mut stdout = stdout();
        draw_status_bar(
            &mut stdout,
            &layout,
            &self.session_stats,
            &self.git_state,
            &self.diff_summary,
            self.terminal_title.as_deref(),
            self.ide,
            &self.cwd,
            cloud_status.as_ref(),
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

    /// Send state change event to cloud
    fn send_cloud_state_event(&mut self, state: SessionState) {
        // Skip if state hasn't changed
        if self.last_cloud_state == Some(state) {
            return;
        }
        self.last_cloud_state = Some(state);

        if let Some(ref mut client) = self.cloud_client {
            let event = SessionEventBuilder::state(state);
            client.send_event(event);
            client.spawn_update_state(session_state_label(state));

            // Also send current screen so dashboard shows latest content
            // This is important when state changes without new PTY output (e.g., Stop event)
            if let Ok(screen_content) = self.capture_manager.update_screen(self.platform_pty.screen()) {
                let screen_event = SessionEventBuilder::screen(screen_content);
                client.send_event(screen_event);
            }
        }
    }

    /// Send scrollback diff event to cloud
    fn send_cloud_scrollback_event(&mut self, update: ScrollbackUpdate) {
        if update.total_lines <= self.last_cloud_scrollback_lines {
            return;
        }
        self.last_cloud_scrollback_lines = update.total_lines;

        if let Some(ref mut client) = self.cloud_client {
            let event = SessionEventBuilder::scrollback(update.diff, update.total_lines);
            client.send_event(event);
        }
    }

    /// Send screen snapshot event to cloud
    fn send_cloud_screen_event(&mut self, content: String) {
        if let Some(ref mut client) = self.cloud_client {
            let event = SessionEventBuilder::screen(content);
            client.send_event(event);
        }
    }

    /// Send title event to cloud
    fn send_cloud_title_event(&mut self, title: String) {
        // Skip if title hasn't changed
        if self.last_cloud_title.as_ref() == Some(&title) {
            return;
        }
        self.last_cloud_title = Some(title.clone());

        if let Some(ref mut client) = self.cloud_client {
            let event = SessionEventBuilder::title(title);
            client.send_event(event);
        }
    }

    /// Send stats event to cloud
    fn send_cloud_stats_event(&mut self) {
        if let Some(ref mut client) = self.cloud_client {
            let event = SessionEventBuilder::stats(
                &self.session_stats.platform_stats,
                self.session_stats.work_seconds,
                self.session_stats.thinking_seconds(),
            );
            client.send_event(event);
        }
    }

    /// Update session stats in cloud DB
    fn send_cloud_stats_update(&mut self) {
        if let Some(ref client) = self.cloud_client {
            let tool_calls = self.session_stats.platform_stats.total_tool_calls();
            client.spawn_update_stats(
                self.session_stats.platform_stats.prompts,
                self.session_stats.platform_stats.completions,
                tool_calls,
                self.session_stats.thinking_seconds(),
            );
        }
    }

    /// Send git + changes snapshot to cloud
    fn send_cloud_git_changes_events(&mut self) {
        if let Some(ref mut client) = self.cloud_client {
            client.send_event(SessionEventBuilder::git(&self.git_state));
            client.send_event(SessionEventBuilder::changes(&self.diff_summary));
        }
    }

    /// Check for answers from cloud and inject into PTY
    fn check_cloud_answers(&mut self) -> Result<()> {
        if let Some(ref mut client) = self.cloud_client {
            while let Some(answer) = client.try_recv_answer() {
                let text = answer.trim_end();
                // Write text as a single block
                self.platform_pty.write(text.as_bytes())?;
                // Small delay to ensure text is processed before Enter
                std::thread::sleep(std::time::Duration::from_millis(10));
                // Send Enter key (CR = 0x0D)
                self.platform_pty.write(&[0x0D])?;
            }
        }
        Ok(())
    }
}

fn session_state_label(state: SessionState) -> &'static str {
    match state {
        SessionState::Ready => "ready",
        SessionState::Thinking => "thinking",
        SessionState::Permission => "permission",
        SessionState::Question => "question",
        SessionState::Complete => "complete",
    }
}
