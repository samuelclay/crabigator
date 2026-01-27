//! Crabigator App - Scroll region approach
//!
//! Architecture:
//! - Set terminal scroll region to top N rows for the assistant CLI
//! - The assistant CLI renders within that region (thinks it's the full terminal)
//! - We render our status widgets below the scroll region
//! - PTY output passes through untouched

use anyhow::Result;
use crossterm::event::{Event, EventStream, MouseEvent};
use futures_util::StreamExt;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::interval;

use crate::capture::{CaptureConfig, CaptureManager, ScrollbackUpdate};
use crate::cloud::{CloudClient, PairingStatusResponse, SessionEventBuilder};
use crate::config::Config;
use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::ide::{self, IdeKind};
use crate::platforms::{Platform, SessionState};
use crate::mirror::MirrorPublisher;
use crate::parsers::DiffSummary;
use crate::terminal::{escape, forward_key_to_pty, DsrChunk, DsrHandler, OscScanner, PlatformPty};
use crate::ui::{draw_status_bar, Layout, PairingState, throbber_frame_index};
use crate::update::UpdateState;

/// Time PTY must be quiet before drawing status bar (prevents mid-burst draws)
const PTY_SETTLE_TIME: Duration = Duration::from_millis(30);

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
    /// Number of options in last sent exit plan prompt (for re-sending when options appear)
    last_exit_plan_option_count: usize,
    /// Pairing state for mobile device linking
    pairing_state: PairingState,
    /// Update state for version banner
    update_state: UpdateState,
    /// Last time we polled pairing status
    last_pairing_poll: Instant,
    /// Pending pairing poll result
    pending_pairing_poll: Option<std::sync::mpsc::Receiver<anyhow::Result<PairingStatusResponse>>>,
    /// Whether there are active viewers watching via dashboard/phone
    /// When false, we reduce screen send frequency to save DO costs
    cloud_viewers_active: bool,
    /// Last time we sent a screen event when no viewers are active
    /// Used to throttle to 1s intervals instead of 100ms
    last_reduced_screen_send: Instant,
    /// Hash of last git state sent to cloud (for deduplication)
    last_cloud_git_hash: Option<u64>,
    /// Hash of last changes sent to cloud (for deduplication)
    last_cloud_changes_hash: Option<u64>,
    /// Hash of last screen sent to cloud (for deduplication)
    last_cloud_screen_hash: Option<u64>,
    /// Hash of last status bar state (to avoid flickering redraws)
    last_status_bar_hash: Option<u64>,
}

impl App {
    pub async fn new(
        cols: u16,
        rows: u16,
        platform: Box<dyn Platform>,
        platform_args: Vec<String>,
        capture_enabled: bool,
        update_state: UpdateState,
    ) -> Result<Self> {
        let (pty_tx, pty_rx) = mpsc::channel(256);

        // Reserve bottom 20% for our status widgets (minimum 2 rows: separator + header)
        // Also ensure pty_rows is at least 1 to avoid PTY errors
        // Guard: ensure max >= min for clamp (handles very short terminals)
        let status_rows = ((rows as f32 * 0.2) as u16).clamp(2, rows.saturating_sub(1).max(2));
        // Reserve 2 extra rows for the pairing banner (top border + content)
        // These rows sit between the PTY and the status bar
        let banner_reserved = 2u16;
        let pty_rows = rows.saturating_sub(status_rows + banner_reserved).max(1);

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

        // Note: We don't pre-initialize transcript_path on startup because we can't
        // reliably determine which session is being resumed. The hooks will provide
        // the correct transcript_path once Claude Code starts.

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
            title_history: Vec::new(),
            initial_git_time_ms: None,
            initial_diff_time_ms: None,
            cloud_client,
            last_cloud_state: None,
            last_cloud_scrollback_lines: 0,
            last_cloud_title: None,
            cloud_stats_sent: false,
            last_cloud_prompt_sent: false,
            last_exit_plan_option_count: 0,
            pairing_state: PairingState::default(),
            update_state,
            last_pairing_poll: Instant::now(),
            pending_pairing_poll: None,
            cloud_viewers_active: false,
            last_reduced_screen_send: Instant::now(),
            last_cloud_git_hash: None,
            last_cloud_changes_hash: None,
            last_cloud_screen_hash: None,
            last_status_bar_hash: None,
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

        // On initial setup, scroll existing content up into scrollback
        // so Claude can start in fresh space without losing history
        if initial {
            // Scroll the entire visible area up by printing newlines
            // This pushes existing content into scrollback
            for _ in 0..self.total_rows {
                write!(stdout, "\n")?;
            }
            // Move cursor back to top of screen
            write!(stdout, "{}", escape::CURSOR_HOME)?;
            // Clear status bar area
            write!(stdout, "{}", escape::cursor_to(self.pty_rows + 1, 1))?;
            write!(stdout, "{}", escape::CLEAR_TO_END)?;
            // Return to top
            write!(stdout, "{}", escape::CURSOR_HOME)?;
        }

        // DECSTBM: Set Top and Bottom Margins (1-indexed)
        // This constrains scrolling to rows 1 through pty_rows
        write!(stdout, "{}", escape::scroll_region(1, self.pty_rows))?;

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

    /// Clear the status bar area (below scroll region) to remove artifacts
    fn clear_status_area(&self) -> Result<()> {
        let mut stdout = stdout();
        // Save cursor so we don't disrupt PTY cursor position
        write!(stdout, "{}", escape::CURSOR_SAVE)?;
        // Move to first status bar row and clear from there to end of screen
        write!(stdout, "{}", escape::cursor_to(self.pty_rows + 1, 1))?;
        write!(stdout, "{}", escape::CLEAR_TO_END)?;
        // Re-establish scroll region after operating outside it to prevent
        // some terminals from resetting cursor origin or scroll context
        write!(stdout, "{}", escape::scroll_region(1, self.pty_rows))?;
        // Restore cursor to PTY position
        write!(stdout, "{}", escape::CURSOR_RESTORE)?;
        stdout.flush()?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut last_status_draw = Instant::now();
        let mut last_throbber_draw = Instant::now();
        let mut last_pty_output = Instant::now();
        let status_debounce = Duration::from_millis(100);

        // Event-driven timers using tokio intervals (no polling!)
        let mut git_interval = interval(Duration::from_secs(1));
        let mut hook_interval = interval(Duration::from_millis(500));
        let mut throbber_interval_timer = interval(Duration::from_millis(100));
        let mut full_refresh_interval = interval(Duration::from_secs(5));
        let mut cloud_reconnect_interval = interval(Duration::from_secs(2));
        let mut status_draw_interval = interval(Duration::from_millis(50));

        // Async terminal event stream (replaces polling)
        let mut event_stream = EventStream::new();

        // Set up scroll region to constrain the CLI to the top area
        // Pass true to scroll existing content up and make room for status bar
        self.setup_scroll_region(true)?;

        // Initial status bar draw (shows "loading" state for git widgets)
        self.draw_status_bar()?;

        // Initialize pairing state (check for linked devices, generate token if needed)
        self.init_pairing().await;

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

        // Initial screen send interval
        let mut initial_screen_interval = interval(Duration::from_millis(500));

        // Event-driven main loop using tokio::select!
        // This replaces the polling loop - we only wake up when something actually happens
        loop {
            // Check if throbber animation is needed (for conditional timer)
            let needs_throbber = matches!(
                self.session_stats.effective_state(),
                SessionState::Thinking | SessionState::Permission
            );

            tokio::select! {
                // PTY output - highest priority, handle immediately
                Some(data) = self.pty_rx.recv() => {
                    self.write_pty_output(&data)?;
                    last_pty_output = Instant::now();

                    // Don't draw status bar here - let status_draw_interval handle it
                    // after PTY output settles. This prevents drawing mid-burst.

                    // Update captures and send to cloud
                    self.handle_pty_output_capture(&mut sent_initial_screen)?;
                }

                // Terminal events (keyboard, resize, paste) - async stream, no polling!
                Some(event_result) = event_stream.next() => {
                    match event_result {
                        Ok(Event::Key(key)) => {
                            self.handle_key_event(key).await?;
                        }
                        Ok(Event::Resize(width, height)) => {
                            self.handle_resize(width, height)?;
                        }
                        Ok(Event::Paste(text)) => {
                            self.platform_pty.write(text.as_bytes())?;
                        }
                        Ok(Event::Mouse(mouse)) => {
                            self.last_mouse_event = Some(mouse);
                        }
                        Ok(_) => {}
                        Err(e) => {
                            // Log error but continue - terminal events shouldn't crash the app
                            eprintln!("Terminal event error: {}", e);
                        }
                    }
                }

                // Git refresh results
                Some(result) = git_rx.recv() => {
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

                    // Redraw with new data (if PTY quiet, otherwise status_draw_interval will handle)
                    if last_pty_output.elapsed() >= PTY_SETTLE_TIME {
                        self.draw_status_bar()?;
                        last_status_draw = Instant::now();
                    }
                }

                // Git refresh timer - only fires when not already pending
                _ = git_interval.tick(), if !git_refresh_pending => {
                    git_refresh_pending = true;
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
                        let _ = tx.send(GitRefreshResult {
                            git_state,
                            diff_summary,
                            git_time_ms: 0,
                            diff_time_ms: 0,
                        }).await;
                    });
                }

                // Hook/stats refresh timer
                _ = hook_interval.tick() => {
                    self.handle_hook_refresh(&mut last_status_draw, last_pty_output)?;
                }

                // Throbber animation timer - only active when in Thinking/Permission state
                _ = throbber_interval_timer.tick(), if needs_throbber => {
                    // Only animate if PTY has been quiet (don't animate mid-burst)
                    if last_pty_output.elapsed() >= PTY_SETTLE_TIME {
                        if last_throbber_draw.elapsed() >= Duration::from_millis(100) {
                            self.draw_status_bar()?;
                            last_throbber_draw = Instant::now();
                        }
                    }
                }

                // Status bar draw timer - draws when PTY output has settled
                _ = status_draw_interval.tick() => {
                    let quiet_for = last_pty_output.elapsed();
                    let since_draw = last_status_draw.elapsed();

                    // Draw if quiet long enough AND debounce passed
                    if quiet_for >= PTY_SETTLE_TIME && since_draw >= status_debounce {
                        self.draw_status_bar()?;
                        last_status_draw = Instant::now();
                        last_throbber_draw = Instant::now();
                    }
                }

                // Full refresh timer - prevent visual artifact buildup
                // Note: No clear_status_area() call here - draw_status_bar() handles
                // clearing internally within its sync block to prevent flickering
                _ = full_refresh_interval.tick() => {
                    // Force redraw by clearing the hash
                    self.last_status_bar_hash = None;
                    self.draw_status_bar()?;
                    last_status_draw = Instant::now();
                }

                // Cloud reconnect timer
                _ = cloud_reconnect_interval.tick() => {
                    // Check for commands from cloud
                    self.check_cloud_commands()?;

                    // Keep cloud WebSocket connected
                    if let Some(ref mut client) = self.cloud_client {
                        if !client.is_connected() {
                            client.try_reconnect();
                        }
                    }

                    // Poll viewer status
                    if let Some(ref mut client) = self.cloud_client {
                        let was_active = self.cloud_viewers_active;
                        self.cloud_viewers_active = client.poll_viewer_status();

                        if !was_active && self.cloud_viewers_active {
                            if let Ok(contents) = self.capture_manager.update_screen(self.platform_pty.screen()) {
                                self.send_cloud_screen_event(contents);
                            }
                        }
                    }

                    // Poll pairing status
                    self.maybe_poll_pairing();

                    // Send initial scrollback if needed
                    self.maybe_send_initial_scrollback();
                }

                // Initial screen send timer - only until first screen is sent
                _ = initial_screen_interval.tick(), if !sent_initial_screen => {
                    let elapsed = session_start.elapsed();
                    if let Ok(contents) = self.capture_manager.update_screen(self.platform_pty.screen()) {
                        if contents.len() > 50 || elapsed > Duration::from_secs(5) {
                            self.send_cloud_screen_event(contents);
                            sent_initial_screen = true;
                        }
                    }
                }
            }

            // Check if the platform CLI has exited
            if !self.running || !self.platform_pty.is_running() {
                break;
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

        // Reset scroll region before exit (don't clear status area - it will
        // naturally scroll away, and clearing leaves ugly blank space)
        self.reset_scroll_region()?;

        // Move cursor below the status bar so the next shell prompt
        // appears after our content (pty_rows + banner_reserved + status_rows + 1)
        let mut stdout = stdout();
        let final_row = self.pty_rows + 2 + self.status_rows + 1;
        write!(stdout, "{}", escape::cursor_to(final_row, 1))?;
        stdout.flush()?;

        Ok(())
    }

    /// Write PTY output directly to stdout - transparent passthrough
    /// V2: DSR handling kept (Claude needs accurate cursor positions in PTY space)
    ///     Scroll region injection removed (was causing visual glitches)
    fn write_pty_output(&mut self, data: &[u8]) -> Result<()> {
        let mut stdout = stdout();
        let mut wrote_output = false;

        // Scan for DSR (cursor position) requests - Claude needs responses in PTY coordinates
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
                        // Strip leading progress spinner characters for history
                        let clean_title = t.trim_start_matches(|c: char| {
                            matches!(c, '*' | '✱' | '✲' | '✳' | '✴' | '✵' | '✶' | '✷' | '✸' | '✹' | '✺' | '✻' | '✼' | '✽' | '❇' | '❈' | '⟳' | '◐' | '◑' | '◒' | '◓' | ' ')
                            || ('\u{2800}'..='\u{28FF}').contains(&c)
                        }).to_string();

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
                    // Respond with cursor position from our vt100 parser
                    // This gives Claude accurate coordinates in PTY space
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
    /// Returns true if status bar was actually redrawn (content changed)
    fn draw_status_bar(&mut self) -> Result<()> {
        // Update stats each draw
        self.session_stats.tick();

        // Compute hash of status bar inputs to detect changes
        let current_hash = {
            use std::hash::{Hash, Hasher};
            let mut hasher = std::collections::hash_map::DefaultHasher::new();

            // Layout
            self.pty_rows.hash(&mut hasher);
            self.total_cols.hash(&mut hasher);
            self.status_rows.hash(&mut hasher);

            // Session stats (key fields that affect display)
            // Use discriminant for enum since SessionState doesn't impl Hash
            std::mem::discriminant(&self.session_stats.effective_state()).hash(&mut hasher);
            self.session_stats.work_seconds.hash(&mut hasher);
            self.session_stats.thinking_seconds().hash(&mut hasher);
            self.session_stats.platform_stats.prompts.hash(&mut hasher);
            self.session_stats.platform_stats.completions.hash(&mut hasher);
            self.session_stats.platform_stats.compressions.hash(&mut hasher);
            self.session_stats.platform_stats.tools.len().hash(&mut hasher);

            // Git state
            self.git_state.branch.hash(&mut hasher);
            self.git_state.files.len().hash(&mut hasher);
            self.git_state.loading.hash(&mut hasher);

            // Diff summary
            self.diff_summary.files.len().hash(&mut hasher);
            self.diff_summary.loading.hash(&mut hasher);

            // Terminal title
            self.terminal_title.hash(&mut hasher);

            // Cloud status
            if let Some(client) = &self.cloud_client {
                let status = client.status();
                status.connected.hash(&mut hasher);
            }

            // Pairing state
            self.pairing_state.has_linked_devices.hash(&mut hasher);
            self.pairing_state.pairing_token.hash(&mut hasher);

            // Update state
            self.update_state.banner_rows().hash(&mut hasher);

            // Include throbber frame when animating to trigger redraws on frame change
            let needs_animation = matches!(
                self.session_stats.effective_state(),
                crate::platforms::SessionState::Thinking | crate::platforms::SessionState::Permission
            );
            if needs_animation {
                throbber_frame_index().hash(&mut hasher);
            }

            hasher.finish()
        };

        // Skip redraw if nothing changed
        if self.last_status_bar_hash == Some(current_hash) {
            return Ok(());
        }
        self.last_status_bar_hash = Some(current_hash);

        // Banner space is reserved at startup (2 rows between PTY and status bar)
        // The layout uses the actual pty_rows which already accounts for this
        let layout = Layout {
            pty_rows: self.pty_rows,
            total_cols: self.total_cols,
            status_rows: self.status_rows,
        };

        let mut stdout = stdout();

        // Get cursor position from vt100 parser to restore after drawing
        // This is critical: we must restore cursor to the position the parser knows,
        // so that DSR responses match what Claude expects
        let cursor_position = Some(self.platform_pty.screen().cursor_position());

        // Get cloud status if connected
        let cloud_status = self.cloud_client.as_ref().map(|c| c.status());
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
            &self.pairing_state,
            &self.update_state,
            cursor_position,
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

    /// Handle PTY output capture and cloud streaming
    fn handle_pty_output_capture(&mut self, sent_initial_screen: &mut bool) -> Result<()> {
        let screen_to_send = if self.cloud_viewers_active {
            // Bypass throttle - get screen immediately for real-time streaming
            self.capture_manager
                .update_screen(self.platform_pty.screen())
                .ok()
        } else if self.last_reduced_screen_send.elapsed() >= Duration::from_secs(1) {
            // No viewers: only capture/send once per second
            self.capture_manager
                .maybe_update_screen(self.platform_pty.screen())
                .ok()
                .flatten()
        } else {
            // No viewers and within 1s throttle: still update local file at 100ms
            let _ = self
                .capture_manager
                .maybe_update_screen(self.platform_pty.screen());
            None // Don't send to cloud
        };

        if let Some(screen) = screen_to_send {
            // Detect mode from screen content
            let new_mode = crate::mode::detect_mode(&screen);
            if new_mode != self.session_stats.platform_stats.mode {
                self.session_stats.platform_stats.mode = new_mode;
                self.send_cloud_stats_event();
            }

            // Detect interrupted state from screen
            if crate::parsers::is_interrupted(&screen) {
                let current_state = self.session_stats.effective_state();
                if current_state != SessionState::Interrupted {
                    self.session_stats.set_interrupted();
                    self.send_cloud_state_event(SessionState::Interrupted);
                    self.draw_status_bar().ok();
                }
            }

            // Deduplicate: only send if content changed
            let hash = {
                use std::hash::{Hash, Hasher};
                let mut hasher = std::collections::hash_map::DefaultHasher::new();
                screen.hash(&mut hasher);
                hasher.finish()
            };

            if self.last_cloud_screen_hash != Some(hash) {
                self.last_cloud_screen_hash = Some(hash);
                self.send_cloud_screen_event(screen);
                if !self.cloud_viewers_active {
                    self.last_reduced_screen_send = Instant::now();
                }
            }
            *sent_initial_screen = true;
        }

        if let Ok(Some(update)) = self.capture_manager.maybe_update_scrollback() {
            self.send_cloud_scrollback_event(update);
        }

        Ok(())
    }

    /// Handle hook/stats refresh
    fn handle_hook_refresh(&mut self, last_status_draw: &mut Instant, last_pty_output: Instant) -> Result<()> {
        let old_effective_state = self.session_stats.effective_state();
        let old_last_updated = self.session_stats.platform_stats.last_updated;
        self.session_stats
            .refresh_platform_stats(self.platform.as_ref(), &self.cwd.to_string_lossy());
        let new_effective_state = self.session_stats.effective_state();
        let new_last_updated = self.session_stats.platform_stats.last_updated;

        // Update transcript path for scrollback capture
        if let Some(ref path) = self.session_stats.platform_stats.transcript_path {
            self.capture_manager
                .set_transcript_path(Some(path.clone()));
        }

        // Redraw if effective state changed (and PTY is quiet)
        if old_effective_state != new_effective_state {
            if last_pty_output.elapsed() >= PTY_SETTLE_TIME {
                self.draw_status_bar()?;
                *last_status_draw = Instant::now();
            }
            // If PTY is busy, status_draw_interval will catch this soon
        }

        // Send initial state once, then on changes
        if self.last_cloud_state.is_none() || old_effective_state != new_effective_state {
            self.send_cloud_state_event(new_effective_state);

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

        // For ExitPlan prompts, keep checking for options
        if matches!(
            self.session_stats.platform_stats.active_prompt.as_ref(),
            Some(crate::platforms::ActivePrompt::ExitPlan)
        ) && self.last_exit_plan_option_count < 3 {
            self.send_cloud_prompt_event();
        }

        // Stream stats when platform stats update
        if new_last_updated != old_last_updated || !self.cloud_stats_sent {
            self.cloud_stats_sent = true;
            self.session_stats.tick();
            self.send_cloud_stats_event();
            self.send_cloud_stats_update();
        }

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
        // Clear old status bar area BEFORE changing dimensions
        // This is critical: if terminal grows, old status bar would be above new position
        self.clear_status_area()?;

        self.total_cols = width;
        self.total_rows = height;

        // Recalculate layout with same guards as App::new
        // Guard: ensure max >= min for clamp (handles very short terminals)
        self.status_rows = ((height as f32 * 0.2) as u16).clamp(2, height.saturating_sub(1).max(2));
        // Reserve 2 rows for banner space (same as in App::new)
        let banner_reserved = 2u16;
        self.pty_rows = height.saturating_sub(self.status_rows + banner_reserved).max(1);

        // Re-setup scroll region for new size (not initial, don't scroll content)
        self.setup_scroll_region(false)?;

        // Resize PTY to new dimensions (only the top portion)
        self.platform_pty.resize(width, self.pty_rows)?;

        // Keep capture manager in sync with PTY dimensions
        self.capture_manager.resize(width, self.pty_rows);

        // Force redraw with new dimensions (draw_status_bar clears internally
        // within its sync block to prevent flickering)
        self.last_status_bar_hash = None;
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

    /// Send git + changes snapshot to cloud (with deduplication)
    fn send_cloud_git_changes_events(&mut self) {
        if self.cloud_client.is_none() {
            return;
        }

        // Compute hashes before borrowing cloud_client
        let git_hash = self.compute_git_hash();
        let changes_hash = self.compute_changes_hash();

        // Check if git state changed
        let send_git = self.last_cloud_git_hash != Some(git_hash);
        let send_changes = self.last_cloud_changes_hash != Some(changes_hash);

        if send_git {
            self.last_cloud_git_hash = Some(git_hash);
        }
        if send_changes {
            self.last_cloud_changes_hash = Some(changes_hash);
        }

        // Now borrow client and send events
        if let Some(ref mut client) = self.cloud_client {
            if send_git {
                client.send_event(SessionEventBuilder::git(&self.git_state));
            }
            if send_changes {
                client.send_event(SessionEventBuilder::changes(&self.diff_summary));
            }
        }
    }

    /// Compute hash of git state for deduplication
    fn compute_git_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        self.git_state.branch.hash(&mut hasher);
        self.git_state.files.len().hash(&mut hasher);
        for f in &self.git_state.files {
            f.path.hash(&mut hasher);
            f.status.hash(&mut hasher);
            f.additions.hash(&mut hasher);
            f.deletions.hash(&mut hasher);
        }
        hasher.finish()
    }

    /// Compute hash of changes for deduplication
    ///
    /// Uses by_language() which provides deterministic ordering (sorted by language,
    /// then by name/file_path). This ensures consistent hashes even though the
    /// underlying diff parsers use HashMap with non-deterministic iteration order.
    fn compute_changes_hash(&self) -> u64 {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();

        // Use by_language() for deterministic ordering
        let by_lang = self.diff_summary.by_language();
        by_lang.len().hash(&mut hasher);
        for lang in &by_lang {
            lang.language.hash(&mut hasher);
            lang.changes.len().hash(&mut hasher);
            for c in &lang.changes {
                c.name.hash(&mut hasher);
                c.additions.hash(&mut hasher);
                c.deletions.hash(&mut hasher);
                // Note: intentionally not hashing line_number as it can vary
                // between parses without affecting the semantic content
            }
        }
        hasher.finish()
    }

    /// Send prompt event to cloud (for interactive dashboard)
    fn send_cloud_prompt_event(&mut self) {
        if self.cloud_client.is_none() {
            return;
        }

        let active_prompt = self.session_stats.platform_stats.active_prompt.as_ref();

        // Parse permission prompt from screen for permission and exit_plan prompts
        let permission_prompt = match active_prompt {
            Some(crate::platforms::ActivePrompt::Permission { .. })
            | Some(crate::platforms::ActivePrompt::ExitPlan) => {
                // Get current screen content for parsing
                if let Ok(screen_content) =
                    self.capture_manager.update_screen(self.platform_pty.screen())
                {
                    let parsed = crate::parsers::PermissionPrompt::parse(&screen_content);

                    // Debug logging for exit plan mode
                    #[cfg(debug_assertions)]
                    if matches!(active_prompt, Some(crate::platforms::ActivePrompt::ExitPlan)) {
                        let debug_path = self.capture_manager.capture_dir().join("parse_debug.txt");
                        let valid = parsed.as_ref().map(|p| p.is_valid()).unwrap_or(false);
                        let debug_info = format!(
                            "=== Parse Debug (ExitPlan) ===\n\
                             Timestamp: {:?}\n\
                             Screen content length: {} bytes\n\n\
                             --- Screen content (ANSI stripped) ---\n{}\n\n\
                             --- Parse result ---\n{:?}\n\n\
                             --- is_valid() ---\n{}\n",
                            std::time::SystemTime::now(),
                            screen_content.len(),
                            crate::parsers::strip_ansi_for_debug(&screen_content),
                            parsed,
                            valid
                        );
                        let _ = std::fs::write(&debug_path, debug_info);
                    }

                    parsed.filter(|p| p.is_valid())
                } else {
                    None
                }
            }
            _ => None,
        };

        // For exit plan prompts, track option count and only send if changed
        // This handles the timing issue where screen isn't ready when hook fires
        if matches!(active_prompt, Some(crate::platforms::ActivePrompt::ExitPlan)) {
            let new_option_count = permission_prompt.as_ref().map(|p| p.options.len()).unwrap_or(0);

            // Skip if option count hasn't changed (avoid duplicate sends)
            // But always send if we have 0 options (fallback) and now have real options
            if new_option_count == self.last_exit_plan_option_count && new_option_count > 0 {
                return;
            }

            self.last_exit_plan_option_count = new_option_count;
        }

        // Build and send the event
        let event = SessionEventBuilder::prompt(active_prompt, permission_prompt.as_ref());
        if let Some(ref mut client) = self.cloud_client {
            client.send_event(event);
        }

        // Track whether we sent a prompt (for clearing later)
        self.last_cloud_prompt_sent = active_prompt.is_some();

        // Reset exit plan option count when leaving exit plan state
        if !matches!(active_prompt, Some(crate::platforms::ActivePrompt::ExitPlan)) {
            self.last_exit_plan_option_count = 0;
        }
    }

    /// Send full scrollback and screen to cloud after (re)connection
    fn maybe_send_initial_scrollback(&mut self) {
        if let Some(ref mut client) = self.cloud_client {
            if client.take_just_connected() {
                // Reset git/changes hashes to force resend on next refresh
                self.last_cloud_git_hash = None;
                self.last_cloud_changes_hash = None;

                // Reset viewer state so first heartbeat triggers a screen send
                // (in case viewers were already active before disconnect)
                self.cloud_viewers_active = false;

                // Send full scrollback history for initial sync
                if let Some(content) = self.capture_manager.get_full_scrollback() {
                    let event = SessionEventBuilder::scrollback_history(content);
                    client.send_event(event);
                }

                // Send current screen immediately (don't wait for viewer heartbeat)
                if let Ok(contents) = self.capture_manager.update_screen(self.platform_pty.screen()) {
                    self.send_cloud_screen_event(contents);
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

            // Handle incoming key sequences (for Tab instructions)
            while let Some(steps) = client.try_recv_key_sequence() {
                for step in steps {
                    match step {
                        crate::cloud::KeyStep::Key { key } => {
                            let bytes: &[u8] = match key.as_str() {
                                "up" => &[0x1b, b'[', b'A'],      // CSI A - cursor up
                                "down" => &[0x1b, b'[', b'B'],    // CSI B - cursor down
                                "tab" => &[0x09],                  // Tab
                                "enter" => &[0x0D],                // Carriage return
                                "shift_tab" => &[0x1b, b'[', b'Z'], // CSI Z - shift+tab
                                _ => continue,
                            };
                            self.platform_pty.write(bytes)?;
                        }
                        crate::cloud::KeyStep::Text { text } => {
                            self.platform_pty.write(text.as_bytes())?;
                        }
                        crate::cloud::KeyStep::Delay { ms } => {
                            std::thread::sleep(std::time::Duration::from_millis(ms as u64));
                        }
                    }
                    // Small delay between steps to ensure terminal processes them
                    std::thread::sleep(std::time::Duration::from_millis(20));
                }
            }
        }
        Ok(())
    }

    // ========================================
    // Pairing Methods
    // ========================================

    /// Initialize pairing state on startup
    /// Checks for existing linked devices and always generates pairing token
    async fn init_pairing(&mut self) {
        let Some(ref client) = self.cloud_client else {
            return;
        };

        // Check for existing linked devices (for stats display)
        if let Ok(response) = client.get_linked_devices().await {
            self.pairing_state.has_linked_devices = !response.devices.is_empty();
        }

        // ALWAYS generate a pairing token (allows adding more devices)
        match client.generate_pairing_token().await {
            Ok(response) => {
                self.pairing_state.pairing_token = Some(response.token);
                self.pairing_state.pairing_code = Some(response.code);
            }
            Err(_) => {
                // Silently fail - pairing widget won't show
            }
        }
    }

    /// Poll pairing status periodically (non-blocking)
    fn maybe_poll_pairing(&mut self) {
        // Only poll if we have a pairing token and no linked devices
        if self.pairing_state.has_linked_devices || self.pairing_state.pairing_token.is_none() {
            return;
        }

        // Clear expired toast
        self.pairing_state.maybe_clear_toast();

        // Check for pending poll result
        if let Some(ref rx) = self.pending_pairing_poll {
            match rx.try_recv() {
                Ok(Ok(response)) => {
                    if response.paired {
                        // Pairing complete!
                        let device_name = response.mobile_name.unwrap_or_else(|| "device".to_string());
                        self.pairing_state.set_just_paired(device_name);
                    } else if response.expired {
                        // Token expired - need to regenerate
                        // This will be handled on next poll cycle
                        self.pairing_state.pairing_token = None;
                        self.pairing_state.pairing_code = None;
                    }
                    self.pending_pairing_poll = None;
                }
                Ok(Err(_)) => {
                    // Poll failed - ignore and retry
                    self.pending_pairing_poll = None;
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    // Still polling
                    return;
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.pending_pairing_poll = None;
                }
            }
        }

        // Poll every 2 seconds
        if self.last_pairing_poll.elapsed() < Duration::from_secs(2) {
            return;
        }
        self.last_pairing_poll = Instant::now();

        // Start async poll
        let Some(ref token) = self.pairing_state.pairing_token else {
            return;
        };
        let Some(ref client) = self.cloud_client else {
            return;
        };

        // Clone what we need for the async task
        let token = token.clone();
        let http = client.http_client().clone();
        let api_url = client.api_url().to_string();
        let device = client.device().clone();

        let (tx, rx) = std::sync::mpsc::channel();
        self.pending_pairing_poll = Some(rx);

        std::thread::spawn(move || {
            let rt = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap();

            let result = rt.block_on(async {
                let url = format!("{}/pairing/{}/status", api_url, token);
                let headers = device.auth_headers("GET", &format!("/api/pairing/{}/status", token))?;

                let mut req = http.get(&url);
                for (key, value) in headers {
                    req = req.header(&key, &value);
                }

                let response = req.send().await?;
                if !response.status().is_success() {
                    anyhow::bail!("Poll failed");
                }

                let data: PairingStatusResponse = response.json().await?;
                Ok(data)
            });

            let _ = tx.send(result);
        });
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
