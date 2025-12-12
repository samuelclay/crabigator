//! V2 App - Scroll region approach
//!
//! Architecture:
//! - Set terminal scroll region to top N rows for Claude Code
//! - Claude Code renders within that region (thinks it's the full terminal)
//! - We render our status widgets below the scroll region using raw ANSI escape sequences
//! - PTY output passes through untouched

use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent};
use std::io::{Write, stdout};
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::git::GitState;
use crate::hooks::ClaudeStats;
use crate::parsers::DiffSummary;
use crate::pty::ClaudePty;

pub struct AppV2 {
    pub running: bool,
    pub claude_pty: ClaudePty,
    pub git_state: GitState,
    pub diff_summary: DiffSummary,
    pub claude_stats: ClaudeStats,
    pub ctrl_a_pressed: bool,
    pub last_mouse_event: Option<MouseEvent>,

    // Layout
    pub total_rows: u16,
    pub total_cols: u16,
    pub pty_rows: u16,      // Rows given to PTY (total - status)
    pub status_rows: u16,   // Rows reserved for our status widgets

    pty_rx: mpsc::Receiver<Vec<u8>>,
}

impl AppV2 {
    pub async fn new(cols: u16, rows: u16) -> Result<Self> {
        let (pty_tx, pty_rx) = mpsc::channel(256);

        // Reserve bottom 20% for our status widgets (minimum 3 rows)
        let status_rows = ((rows as f32 * 0.2) as u16).max(3);
        let pty_rows = rows.saturating_sub(status_rows);

        // Give Claude Code only the top portion
        let claude_pty = ClaudePty::new(pty_tx, cols, pty_rows).await?;
        let git_state = GitState::new();
        let diff_summary = DiffSummary::new();
        let claude_stats = ClaudeStats::new();

        Ok(Self {
            running: true,
            claude_pty,
            git_state,
            diff_summary,
            claude_stats,
            ctrl_a_pressed: false,
            last_mouse_event: None,
            total_rows: rows,
            total_cols: cols,
            pty_rows,
            status_rows,
            pty_rx,
        })
    }

    /// Set scroll region to constrain PTY output to top area
    fn setup_scroll_region(&self) -> Result<()> {
        let mut stdout = stdout();
        // DECSTBM: Set Top and Bottom Margins (1-indexed)
        // This constrains scrolling to rows 1 through pty_rows
        write!(stdout, "\x1b[1;{}r", self.pty_rows)?;
        // Move cursor to top of scroll region
        write!(stdout, "\x1b[H")?;
        stdout.flush()?;
        Ok(())
    }

    /// Reset scroll region to full screen
    fn reset_scroll_region(&self) -> Result<()> {
        let mut stdout = stdout();
        write!(stdout, "\x1b[r")?;
        stdout.flush()?;
        Ok(())
    }

    pub async fn run(&mut self) -> Result<()> {
        let mut last_git_refresh = Instant::now();
        let mut last_status_draw = Instant::now();
        let git_refresh_interval = Duration::from_secs(3);
        let status_debounce = Duration::from_millis(100);

        // Set up scroll region to constrain Claude Code to top area
        self.setup_scroll_region()?;

        // Initial status bar draw
        self.draw_status_bar()?;

        while self.running {
            // Receive PTY output and write directly to stdout
            let mut got_output = false;
            while let Ok(data) = self.pty_rx.try_recv() {
                self.write_pty_output(&data)?;
                got_output = true;
            }

            // Refresh git status periodically
            if last_git_refresh.elapsed() >= git_refresh_interval {
                self.refresh_git_state().await;
                last_git_refresh = Instant::now();
            }

            // Redraw status bar after PTY output settles (debounced)
            if got_output && last_status_draw.elapsed() >= status_debounce {
                self.draw_status_bar()?;
                last_status_draw = Instant::now();
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

        // Reset scroll region before exit
        self.reset_scroll_region()?;

        Ok(())
    }

    /// Write PTY output directly to stdout - transparent passthrough
    fn write_pty_output(&mut self, data: &[u8]) -> Result<()> {
        // Update our internal parser (for stats/analysis)
        self.claude_pty.process_output(data);

        let mut stdout = stdout();
        stdout.write_all(data)?;
        stdout.flush()?;
        Ok(())
    }

    /// Draw status bar in the reserved bottom area using raw ANSI escape sequences
    fn draw_status_bar(&mut self) -> Result<()> {
        let mut stdout = stdout();

        // Save cursor position (cursor is already hidden globally)
        write!(stdout, "\x1b[s")?;

        // Move to status area (below the scroll region)
        write!(stdout, "\x1b[{};1H", self.pty_rows + 1)?;

        // Draw separator line with background
        write!(stdout, "\x1b[48;5;237m")?; // Dark gray background
        let title = "─── CRABIGATOR ───";
        let padding = (self.total_cols as usize).saturating_sub(title.len()) / 2;
        write!(stdout, "\x1b[90m{:padding$}\x1b[96;1m{}\x1b[0;90m\x1b[48;5;237m{:padding$}\x1b[0m",
            "", title, "", padding = padding)?;

        // Clear to end of line
        write!(stdout, "\x1b[K")?;

        // Draw the three widget columns
        let col_width = self.total_cols / 3;

        for row in 1..self.status_rows {
            write!(stdout, "\x1b[{};1H", self.pty_rows + 1 + row)?;

            // Git column
            self.draw_status_cell(&mut stdout, 0, row, col_width, "Git")?;

            // Changes column
            self.draw_status_cell(&mut stdout, col_width, row, col_width, "Changes")?;

            // Stats column
            self.draw_status_cell(&mut stdout, col_width * 2, row, self.total_cols - col_width * 2, "Stats")?;
        }

        // Restore cursor position (keep cursor hidden)
        write!(stdout, "\x1b[u")?;
        stdout.flush()?;

        Ok(())
    }

    fn draw_status_cell(&self, stdout: &mut std::io::Stdout, col: u16, row: u16, width: u16, section: &str) -> Result<()> {
        write!(stdout, "\x1b[{};{}H", self.pty_rows + 1 + row, col + 1)?;

        if row == 1 {
            // Header row
            let (color, title) = match section {
                "Git" => ("32", " Git "),      // Green
                "Changes" => ("33", " Changes "), // Yellow
                "Stats" => ("35", " Stats "),   // Magenta
                _ => ("37", section),
            };
            write!(stdout, "\x1b[90m╭─\x1b[{};1m{}\x1b[0;90m─", color, title)?;
            let remaining = width.saturating_sub(title.len() as u16 + 4);
            for _ in 0..remaining {
                write!(stdout, "─")?;
            }
            write!(stdout, "╮\x1b[0m")?;
        } else if row == self.status_rows - 1 {
            // Bottom border
            write!(stdout, "\x1b[90m╰")?;
            for _ in 0..(width.saturating_sub(2)) {
                write!(stdout, "─")?;
            }
            write!(stdout, "╯\x1b[0m")?;
        } else {
            // Content rows
            write!(stdout, "\x1b[90m│\x1b[0m")?;
            let content = self.get_status_content(section, row - 2);
            write!(stdout, "{}", content)?;
            // Pad to width
            let content_len = strip_ansi_len(&content);
            let pad = (width as usize).saturating_sub(content_len + 2);
            write!(stdout, "{:pad$}\x1b[90m│\x1b[0m", "", pad = pad)?;
        }
        Ok(())
    }

    fn get_status_content(&self, section: &str, line: u16) -> String {
        match section {
            "Git" => {
                let files = &self.git_state.files;
                if files.is_empty() {
                    if line == 0 { "\x1b[32m✓ Clean\x1b[0m".to_string() } else { String::new() }
                } else if line == 0 {
                    format!("\x1b[33m{} file(s)\x1b[0m", files.len())
                } else if let Some(file) = files.get((line - 1) as usize) {
                    let (icon, color) = match file.status.as_str() {
                        "M" => ("●", "33"),
                        "A" => ("+", "32"),
                        "D" => ("−", "31"),
                        "?" => ("?", "90"),
                        _ => ("•", "37"),
                    };
                    format!("\x1b[{}m{}\x1b[0m {}", color, icon, truncate_path(&file.path, 20))
                } else {
                    String::new()
                }
            }
            "Changes" => {
                if self.diff_summary.files.is_empty() {
                    if line == 0 { "\x1b[90mNo changes\x1b[0m".to_string() } else { String::new() }
                } else if let Some(file) = self.diff_summary.files.get(line as usize) {
                    format!("\x1b[36m◆\x1b[0m {} \x1b[90m({})\x1b[0m",
                        truncate_path(&file.path, 15), file.changes.len())
                } else {
                    String::new()
                }
            }
            "Stats" => {
                match line {
                    0 => format!("\x1b[34m⏱\x1b[0m Idle: \x1b[{}m{}\x1b[0m",
                        if self.claude_stats.idle_seconds > 60 { "33" } else { "32" },
                        format_duration(self.claude_stats.idle_seconds)),
                    1 => format!("\x1b[33m⚡\x1b[0m Work: \x1b[36m{}\x1b[0m",
                        format_duration(self.claude_stats.work_seconds)),
                    2 => format!("\x1b[35m◈\x1b[0m Tokens: \x1b[35m{}\x1b[0m",
                        format_number(self.claude_stats.tokens_used)),
                    3 => format!("\x1b[34m✉\x1b[0m Msgs: \x1b[34m{}\x1b[0m",
                        self.claude_stats.messages_count),
                    _ => String::new(),
                }
            }
            _ => String::new(),
        }
    }

    async fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        // Check for Ctrl+A prefix for app commands
        if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a') {
            self.ctrl_a_pressed = true;
            return Ok(());
        }

        if self.ctrl_a_pressed {
            self.ctrl_a_pressed = false;
            match key.code {
                KeyCode::Char('q') | KeyCode::Char('Q') => {
                    self.running = false;
                }
                KeyCode::Char('a') => {
                    self.claude_pty.write(&[0x01])?;
                }
                _ => {}
            }
            return Ok(());
        }

        // Forward all other keys to PTY
        self.forward_key_to_pty(key)?;
        Ok(())
    }

    fn forward_key_to_pty(&mut self, key: KeyEvent) -> Result<()> {
        let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
        let has_alt = key.modifiers.contains(KeyModifiers::ALT);
        let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

        // Calculate xterm modifier code: 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0)
        // Only used when we have modifiers on special keys
        let modifier_code = 1
            + (if has_shift { 1 } else { 0 })
            + (if has_alt { 2 } else { 0 })
            + (if has_ctrl { 4 } else { 0 });
        let has_modifiers = modifier_code > 1;

        let bytes = match key.code {
            KeyCode::Char(c) => {
                if has_ctrl && !has_alt && !has_shift {
                    // Ctrl+char: send control character
                    vec![(c.to_ascii_lowercase() as u8) & 0x1f]
                } else if has_alt && !has_ctrl {
                    // Alt/Option+char: send ESC prefix (meta key encoding)
                    let mut buf = [0u8; 4];
                    let actual_char = if has_shift { c } else { c };
                    let s = actual_char.encode_utf8(&mut buf);
                    let mut bytes = vec![0x1b]; // ESC prefix
                    bytes.extend_from_slice(s.as_bytes());
                    bytes
                } else if has_ctrl && has_alt {
                    // Ctrl+Alt+char: ESC prefix + control character
                    let mut bytes = vec![0x1b];
                    bytes.push((c.to_ascii_lowercase() as u8) & 0x1f);
                    bytes
                } else {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    s.as_bytes().to_vec()
                }
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => {
                if has_alt {
                    // Option+Backspace: delete word backwards (ESC + DEL)
                    vec![0x1b, 0x7f]
                } else if has_ctrl {
                    // Ctrl+Backspace: often used for delete word
                    vec![0x1b, 0x7f]
                } else {
                    vec![0x7f]
                }
            }
            KeyCode::Tab => {
                if has_shift {
                    // Shift+Tab: back tab (CSI Z)
                    vec![0x1b, b'[', b'Z']
                } else if has_ctrl {
                    // Ctrl+Tab: some terminals send this as CSI 9 ; modifier ~
                    format!("\x1b[9;{}~", modifier_code).into_bytes()
                } else {
                    vec![b'\t']
                }
            }
            KeyCode::BackTab => {
                // BackTab is already Shift+Tab on some platforms
                vec![0x1b, b'[', b'Z']
            }
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => {
                if has_modifiers {
                    format!("\x1b[1;{}A", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'A']
                }
            }
            KeyCode::Down => {
                if has_modifiers {
                    format!("\x1b[1;{}B", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'B']
                }
            }
            KeyCode::Right => {
                if has_modifiers {
                    format!("\x1b[1;{}C", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'C']
                }
            }
            KeyCode::Left => {
                if has_modifiers {
                    format!("\x1b[1;{}D", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'D']
                }
            }
            KeyCode::Home => {
                if has_modifiers {
                    format!("\x1b[1;{}H", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'H']
                }
            }
            KeyCode::End => {
                if has_modifiers {
                    format!("\x1b[1;{}F", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'F']
                }
            }
            KeyCode::PageUp => {
                if has_modifiers {
                    format!("\x1b[5;{}~", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'5', b'~']
                }
            }
            KeyCode::PageDown => {
                if has_modifiers {
                    format!("\x1b[6;{}~", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'6', b'~']
                }
            }
            KeyCode::Delete => {
                if has_alt && !has_ctrl && !has_shift {
                    // Option+Delete: delete word forward (ESC + d)
                    vec![0x1b, b'd']
                } else if has_modifiers {
                    format!("\x1b[3;{}~", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'3', b'~']
                }
            }
            KeyCode::Insert => {
                if has_modifiers {
                    format!("\x1b[2;{}~", modifier_code).into_bytes()
                } else {
                    vec![0x1b, b'[', b'2', b'~']
                }
            }
            // Function keys F1-F12
            KeyCode::F(n) => {
                // Function key encoding varies, using xterm-style
                let base_code = match n {
                    1 => "P",
                    2 => "Q",
                    3 => "R",
                    4 => "S",
                    5 => "15~",
                    6 => "17~",
                    7 => "18~",
                    8 => "19~",
                    9 => "20~",
                    10 => "21~",
                    11 => "23~",
                    12 => "24~",
                    _ => return Ok(()),
                };
                if has_modifiers && n >= 5 {
                    // F5-F12 use tilde format with modifiers
                    let num = match n {
                        5 => 15,
                        6 => 17,
                        7 => 18,
                        8 => 19,
                        9 => 20,
                        10 => 21,
                        11 => 23,
                        12 => 24,
                        _ => return Ok(()),
                    };
                    format!("\x1b[{};{}~", num, modifier_code).into_bytes()
                } else if has_modifiers && n <= 4 {
                    // F1-F4 use SS3 format, with modifiers use CSI 1 ; mod P/Q/R/S
                    format!("\x1b[1;{}{}", modifier_code, base_code).into_bytes()
                } else if n <= 4 {
                    // F1-F4 without modifiers: SS3 P/Q/R/S
                    format!("\x1bO{}", base_code).into_bytes()
                } else {
                    // F5-F12 without modifiers
                    format!("\x1b[{}", base_code).into_bytes()
                }
            }
            // Null character (Ctrl+Space or Ctrl+@)
            KeyCode::Null => vec![0x00],
            _ => return Ok(()),
        };

        self.claude_pty.write(&bytes)?;
        Ok(())
    }

    fn handle_resize(&mut self, width: u16, height: u16) -> Result<()> {
        self.total_cols = width;
        self.total_rows = height;

        // Recalculate layout
        self.status_rows = ((height as f32 * 0.2) as u16).max(3);
        self.pty_rows = height.saturating_sub(self.status_rows);

        // Re-setup scroll region for new size
        self.setup_scroll_region()?;

        // Resize PTY to new dimensions (only the top portion)
        self.claude_pty.resize(width, self.pty_rows)?;

        // Redraw status bar in new position
        self.draw_status_bar()?;

        Ok(())
    }

    async fn refresh_git_state(&mut self) {
        if let Ok(status) = self.git_state.refresh().await {
            self.git_state = status;
        }
        if let Ok(diff) = self.diff_summary.refresh().await {
            self.diff_summary = diff;
        }
    }
}

// Helper functions
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        // Show end of path (more useful)
        format!("...{}", &path[path.len() - (max_len - 3)..])
    }
}

fn format_duration(seconds: u64) -> String {
    let mins = seconds / 60;
    let secs = seconds % 60;
    if mins > 0 {
        format!("{}m {}s", mins, secs)
    } else {
        format!("{}s", secs)
    }
}

fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Calculate string length excluding ANSI escape sequences
fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += 1;
        }
    }
    len
}

