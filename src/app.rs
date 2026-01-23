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
use crate::terminal::{escape, forward_key_to_pty, DsrChunk, DsrHandler, OscScanner, PlatformPty, RedrawScanner};
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
    /// Scans for full screen redraw sequences from the CLI
    redraw_scanner: RedrawScanner,
    /// Last time we triggered a HUD redraw from detected redraw sequences
    last_redraw_trigger: Instant,
    /// Terminal title extracted from OSC sequences (e.g., "Claude Code Ghostty Integration")
    terminal_title: Option<String>,
    /// History of all terminal titles during this session
    title_history: Vec<String>,
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
    /// Whether we've sent a prompt event (to track clearing)
    last_cloud_prompt_sent: bool,
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
        // Must match PTY dimensions for escape sequences to work correctly
        let capture_config = CaptureConfig {
            enabled: capture_enabled,
            session_id: session_id.clone(),
        };
        let capture_manager = CaptureManager::new(capture_config, cols, pty_rows)?;

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
            redraw_scanner: RedrawScanner::new(),
            last_redraw_trigger: Instant::now(),
            terminal_title: None,
            title_history: Vec::new(),
            initial_git_time_ms: None,
            initial_diff_time_ms: None,
            cloud_client,
            last_cloud_state: None,
            last_cloud_scrollback_lines: 0,
            last_cloud_title: None,
            cloud_stats_sent: false,
            last_cloud_prompt_sent: false,
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

        // Only move cursor on initial setup - during resize/redraw we preserve
        // the CLI's cursor position to avoid disrupting its rendering
        if initial {
            // Move cursor to bottom of scroll region so the CLI starts there
            // and naturally scrolls up as it produces output (like a normal shell)
            write!(stdout, "{}", escape::cursor_to(self.pty_rows, 1))?;
        }

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

        // Debounce interval for redraw detection (prevents excessive redraws during rapid updates)
        let redraw_debounce = Duration::from_millis(50);

        while self.running {
            // Receive PTY output and write directly to stdout
            let mut got_output = false;
            let mut needs_hud_redraw = false;
            while let Ok(data) = self.pty_rx.try_recv() {
                if self.write_pty_output(&data)? {
                    needs_hud_redraw = true;
                }
                got_output = true;
            }

            // Handle CLI-initiated full screen redraw (e.g., after resize via SIGWINCH)
            // Re-establish scroll region and refresh HUD with debouncing
            if needs_hud_redraw && self.last_redraw_trigger.elapsed() >= redraw_debounce {
                self.last_redraw_trigger = Instant::now();
                self.setup_scroll_region(false)?;
                self.draw_status_bar()?;
                last_status_draw = Instant::now();
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
                let old_effective_state = self.session_stats.effective_state();
                let old_last_updated = self.session_stats.platform_stats.last_updated;
                self.session_stats
                    .refresh_platform_stats(self.platform.as_ref(), &self.cwd.to_string_lossy());
                let new_effective_state = self.session_stats.effective_state();
                let new_last_updated = self.session_stats.platform_stats.last_updated;

                // Redraw immediately if effective state changed (e.g., Thinking -> Complete, or Interrupted -> Thinking)
                if old_effective_state != new_effective_state {
                    self.draw_status_bar()?;
                    last_status_draw = Instant::now();
                }

                // Send initial state once, then on changes
                if self.last_cloud_state.is_none() || old_effective_state != new_effective_state {
                    self.send_cloud_state_event(new_effective_state);

                    // Send prompt event when entering/leaving interactive states
                    let is_interactive = matches!(
                        new_effective_state,
                        SessionState::Question | SessionState::Permission
                    );
                    let was_interactive = matches!(
                        old_effective_state,
                        SessionState::Question | SessionState::Permission
                    );

                    if is_interactive || (was_interactive && self.last_cloud_prompt_sent) {
                        self.send_cloud_prompt_event();
                    }
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

            // Check for commands from cloud (answers + key sequences)
            self.check_cloud_commands()?;

            // Send full scrollback after cloud (re)connects
            self.maybe_send_initial_scrollback();

            // Redraw status bar after PTY output settles (debounced)
            if got_output && last_status_draw.elapsed() >= status_debounce {
                self.draw_status_bar()?;
                last_status_draw = Instant::now();
                last_throbber_draw = Instant::now();
            }

            // Animate throbber independently when in active states (Thinking/Permission)
            let needs_throbber = matches!(
                self.session_stats.effective_state(),
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
                    // Detect mode from screen content
                    let new_mode = crate::mode::detect_mode(&screen);
                    if new_mode != self.session_stats.platform_stats.mode {
                        self.session_stats.platform_stats.mode = new_mode;
                        // Send updated stats to cloud when mode changes
                        self.send_cloud_stats_event();
                    }

                    // Detect interrupted state from screen (user hit Escape on permission)
                    // This catches cases where the hook doesn't fire
                    if crate::parsers::is_interrupted(&screen) {
                        let current_state = self.session_stats.effective_state();
                        if current_state != SessionState::Interrupted {
                            self.session_stats.set_interrupted();
                            self.send_cloud_state_event(SessionState::Interrupted);
                            self.draw_status_bar().ok();
                        }
                    }

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
    /// Returns true if a full screen redraw was detected (requiring HUD refresh)
    fn write_pty_output(&mut self, data: &[u8]) -> Result<bool> {
        let mut stdout = stdout();
        let mut wrote_output = false;
        let mut needs_redraw = false;

        let chunks = self.dsr_handler.scan(data);
        for chunk in chunks {
            match chunk {
                DsrChunk::Output(bytes) => {
                    if bytes.is_empty() {
                        continue;
                    }

                    // Scan for full screen redraw sequences
                    let redraw_result = self.redraw_scanner.scan(&bytes);
                    if redraw_result.needs_redraw {
                        needs_redraw = true;
                    }

                    // Scan for OSC title sequences
                    let (passthrough, title) = self.osc_scanner.scan(&redraw_result.output);
                    if let Some(t) = title {
                        // Strip leading progress spinner characters for history
                        // Includes: ASCII asterisk, dingbat asterisks, sparkles, rotation arrows,
                        // circle quarters, and all braille patterns (U+2800-U+28FF)
                        let clean_title = t.trim_start_matches(|c: char| {
                            matches!(c, '*' | '✱' | '✲' | '✳' | '✴' | '✵' | '✶' | '✷' | '✸' | '✹' | '✺' | '✻' | '✼' | '✽' | '❇' | '❈' | '⟳' | '◐' | '◑' | '◒' | '◓' | ' ')
                            || ('\u{2800}'..='\u{28FF}').contains(&c)  // All braille patterns
                        }).to_string();

                        // Add to history if not already present (no duplicates)
                        // Skip generic default titles like "Claude Code" - they can be the
                        // current title but shouldn't clutter the history
                        let is_default_title = clean_title == "Claude Code" || clean_title == "Codex CLI";
                        if !clean_title.is_empty() && !is_default_title && !self.title_history.contains(&clean_title) {
                            self.title_history.push(clean_title.clone());
                        }
                        self.terminal_title = Some(clean_title.clone());
                        self.send_cloud_title_event(clean_title);
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

        Ok(needs_redraw)
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
            &self.title_history,
            self.initial_git_time_ms,
            self.initial_diff_time_ms,
        );

        Ok(())
    }

    async fn handle_key_event(&mut self, key: crossterm::event::KeyEvent) -> Result<()> {
        use crossterm::event::{KeyCode, KeyModifiers};

        if key.kind != crossterm::event::KeyEventKind::Press {
            return Ok(());
        }

        // Detect interrupt keys (ESC or Ctrl+C) while thinking
        let is_interrupt = key.code == KeyCode::Esc
            || (key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL));

        if is_interrupt && self.session_stats.effective_state() == SessionState::Thinking {
            self.session_stats.set_interrupted();
            // Immediately send state update to cloud
            self.send_cloud_state_event(SessionState::Interrupted);
            // Redraw to show interrupted state
            self.draw_status_bar()?;
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

        // Keep capture manager in sync with PTY dimensions
        self.capture_manager.resize(width, self.pty_rows);

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

            // Also send the full title history
            if !self.title_history.is_empty() {
                let history_event =
                    SessionEventBuilder::title_history(self.title_history.clone());
                client.send_event(history_event);
            }
        }
    }

    /// Send stats event to cloud
    fn send_cloud_stats_event(&mut self) {
        if let Some(ref mut client) = self.cloud_client {
            // Parse permission prompt from screen when in permission state
            let permission_prompt = if self.session_stats.effective_state() == SessionState::Permission {
                // Get current screen content
                if let Ok(screen_content) = self.capture_manager.update_screen(self.platform_pty.screen()) {
                    // Parse permission prompt from screen
                    crate::parsers::PermissionPrompt::parse(&screen_content)
                        .filter(|p| p.is_valid())
                } else {
                    None
                }
            } else {
                None
            };

            let event = SessionEventBuilder::stats(
                &self.session_stats,
                permission_prompt.as_ref(),
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

    /// Send prompt event to cloud (for interactive dashboard)
    fn send_cloud_prompt_event(&mut self) {
        if self.cloud_client.is_none() {
            return;
        }

        let active_prompt = self.session_stats.platform_stats.active_prompt.as_ref();

        // Parse options from screen for permission and exit_plan prompts
        let permission_options = match active_prompt {
            Some(crate::platforms::ActivePrompt::Permission { .. })
            | Some(crate::platforms::ActivePrompt::ExitPlan) => {
                // Get current screen content for parsing
                if let Ok(screen_content) =
                    self.capture_manager.update_screen(self.platform_pty.screen())
                {
                    let options = crate::screen_parser::parse_permission_options(&screen_content);
                    if options.is_empty() {
                        None // Use fallback options in builder
                    } else {
                        Some(options)
                    }
                } else {
                    None
                }
            }
            _ => None,
        };

        // Build and send the event
        let event = SessionEventBuilder::prompt(active_prompt, permission_options);
        if let Some(ref mut client) = self.cloud_client {
            client.send_event(event);
        }

        // Track whether we sent a prompt (for clearing later)
        self.last_cloud_prompt_sent = active_prompt.is_some();
    }

    /// Send full scrollback to cloud after (re)connection
    fn maybe_send_initial_scrollback(&mut self) {
        if let Some(ref mut client) = self.cloud_client {
            if client.take_just_connected() {
                // Send full scrollback history for initial sync
                if let Some(content) = self.capture_manager.get_full_scrollback() {
                    let event = SessionEventBuilder::scrollback_history(content);
                    client.send_event(event);
                }
            }
        }
    }

    /// Check for answers and key commands from cloud and inject into PTY
    fn check_cloud_commands(&mut self) -> Result<()> {
        if let Some(ref mut client) = self.cloud_client {
            // Handle incoming text answers
            while let Some(answer) = client.try_recv_answer() {
                let text = answer.trim_end();
                // Write text as a single block
                self.platform_pty.write(text.as_bytes())?;
                // Small delay to ensure text is processed before Enter
                std::thread::sleep(std::time::Duration::from_millis(10));
                // Send Enter key (CR = 0x0D)
                self.platform_pty.write(&[0x0D])?;
            }

            // Handle incoming key commands
            while let Some(key) = client.try_recv_key() {
                match key.as_str() {
                    "shift_tab" => {
                        // Shift+Tab: CSI Z (ESC [ Z) - cycles Claude Code modes
                        self.platform_pty.write(&[0x1b, b'[', b'Z'])?;
                    }
                    _ => {
                        // Unknown key command - ignore
                    }
                }
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
        SessionState::Interrupted => "interrupted",
    }
}
