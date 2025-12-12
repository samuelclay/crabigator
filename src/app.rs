//! Crabigator App - Scroll region approach
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

pub struct App {
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

impl App {
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
        // Update stats each draw
        self.claude_stats.tick();

        let mut stdout = stdout();

        // Save cursor position
        write!(stdout, "\x1b[s")?;

        // Move to status area (below the scroll region)
        write!(stdout, "\x1b[{};1H", self.pty_rows + 1)?;

        // Draw thin separator line
        write!(stdout, "\x1b[48;5;236m\x1b[38;5;240m")?;
        for _ in 0..self.total_cols {
            write!(stdout, "─")?;
        }
        write!(stdout, "\x1b[0m")?;

        // Calculate column widths: Stats has min width, Git and Changes share the rest
        let stats_width = 22u16; // Fixed width for stats
        let remaining = self.total_cols.saturating_sub(stats_width + 2); // 2 for separators
        let git_width = remaining / 2;
        let changes_width = remaining - git_width;

        // Draw content rows
        for row in 1..self.status_rows {
            write!(stdout, "\x1b[{};1H", self.pty_rows + 1 + row)?;

            // Stats column (leftmost, fixed width)
            self.draw_stats_widget(&mut stdout, 0, row, stats_width)?;

            // Separator
            write!(stdout, "\x1b[38;5;240m│\x1b[0m")?;

            // Git column
            self.draw_git_widget(&mut stdout, stats_width + 1, row, git_width)?;

            // Separator
            write!(stdout, "\x1b[38;5;240m│\x1b[0m")?;

            // Changes column (rightmost)
            self.draw_changes_widget(&mut stdout, stats_width + git_width + 2, row, changes_width)?;
        }

        // Restore cursor position
        write!(stdout, "\x1b[u")?;
        stdout.flush()?;

        Ok(())
    }

    fn draw_stats_widget(&self, stdout: &mut std::io::Stdout, col: u16, row: u16, width: u16) -> Result<()> {
        write!(stdout, "\x1b[{};{}H", self.pty_rows + 1 + row, col + 1)?;

        let content = match row {
            1 => {
                // Header
                format!("\x1b[38;5;141m Stats\x1b[0m")
            }
            2 => {
                // Idle time with color based on duration
                let idle_color = if self.claude_stats.idle_seconds < 5 {
                    "38;5;83" // Bright green
                } else if self.claude_stats.idle_seconds < 60 {
                    "38;5;228" // Yellow
                } else {
                    "38;5;203" // Red
                };
                format!("\x1b[38;5;245m⏱ Idle\x1b[0m \x1b[{}m{}\x1b[0m", idle_color, self.claude_stats.format_idle())
            }
            3 => {
                // Session/work time
                format!("\x1b[38;5;245m⚡ Session\x1b[0m \x1b[38;5;39m{}\x1b[0m", self.claude_stats.format_work())
            }
            4 => {
                // Tokens
                format!("\x1b[38;5;245m◈ Tokens\x1b[0m \x1b[38;5;213m{}\x1b[0m", format_number(self.claude_stats.tokens_used))
            }
            5 => {
                // Messages
                format!("\x1b[38;5;245m✉ Msgs\x1b[0m \x1b[38;5;75m{}\x1b[0m", self.claude_stats.messages_count)
            }
            _ => String::new(),
        };

        write!(stdout, "{}", content)?;
        let content_len = strip_ansi_len(&content);
        let pad = (width as usize).saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;

        Ok(())
    }

    fn draw_git_widget(&self, stdout: &mut std::io::Stdout, col: u16, row: u16, width: u16) -> Result<()> {
        write!(stdout, "\x1b[{};{}H", self.pty_rows + 1 + row, col + 1)?;

        let files = &self.git_state.files;

        if row == 1 {
            // Header with branch name
            let branch = if self.git_state.branch.is_empty() {
                "Git"
            } else {
                &self.git_state.branch
            };
            let header = format!("\x1b[38;5;114m {}\x1b[0m", truncate_path(branch, 15));
            write!(stdout, "{}", header)?;
            let content_len = strip_ansi_len(&header);
            let pad = (width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
            return Ok(());
        }

        if files.is_empty() {
            if row == 2 {
                let content = "\x1b[38;5;83m✓ Clean\x1b[0m";
                write!(stdout, "{}", content)?;
                let pad = (width as usize).saturating_sub(strip_ansi_len(content));
                write!(stdout, "{:pad$}", "", pad = pad)?;
            } else {
                write!(stdout, "{:width$}", "", width = width as usize)?;
            }
            return Ok(());
        }

        // Calculate max changes for scaling the bar graph
        let max_changes = files.iter().map(|f| f.total_changes()).max().unwrap_or(1).max(1);

        // Get file for this row (row 2 = index 0, etc.)
        let file_idx = (row - 2) as usize;
        if let Some(file) = files.get(file_idx) {
            // File name (truncated)
            let name = get_filename(&file.path);
            let name_width = (width as usize).saturating_sub(12); // Leave room for bar
            let truncated_name = truncate_path(name, name_width);

            // Status icon
            let (icon, icon_color) = match file.status.as_str() {
                "M" => ("●", "38;5;220"), // Yellow
                "A" => ("+", "38;5;83"),  // Green
                "D" => ("−", "38;5;203"), // Red
                "?" => ("?", "38;5;245"), // Gray
                _ => ("•", "38;5;250"),
            };

            // Create scaled bar (max 8 chars)
            let bar = create_diff_bar(file.additions, file.deletions, max_changes, 8);

            let content = format!(
                "\x1b[{}m{}\x1b[0m {} {}",
                icon_color, icon, truncated_name, bar
            );

            write!(stdout, "{}", content)?;
            let content_len = strip_ansi_len(&content);
            let pad = (width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            write!(stdout, "{:width$}", "", width = width as usize)?;
        }

        Ok(())
    }

    fn draw_changes_widget(&self, stdout: &mut std::io::Stdout, col: u16, row: u16, width: u16) -> Result<()> {
        write!(stdout, "\x1b[{};{}H", self.pty_rows + 1 + row, col + 1)?;

        if row == 1 {
            // Header
            let header = format!("\x1b[38;5;179m Changes\x1b[0m");
            write!(stdout, "{}", header)?;
            let pad = (width as usize).saturating_sub(strip_ansi_len(&header));
            write!(stdout, "{:pad$}", "", pad = pad)?;
            return Ok(());
        }

        // Collect all semantic changes across files
        let all_changes: Vec<_> = self.diff_summary.files.iter()
            .flat_map(|f| f.changes.iter().map(move |c| (f, c)))
            .collect();

        if all_changes.is_empty() {
            if row == 2 {
                let content = "\x1b[38;5;245mNo semantic changes\x1b[0m";
                write!(stdout, "{}", content)?;
                let pad = (width as usize).saturating_sub(strip_ansi_len(content));
                write!(stdout, "{:pad$}", "", pad = pad)?;
            } else {
                write!(stdout, "{:width$}", "", width = width as usize)?;
            }
            return Ok(());
        }

        // Multi-column layout: calculate how many items per column
        let item_width = 20usize; // Each item takes ~20 chars
        let num_cols = (width as usize / item_width).max(1);
        let items_per_row = num_cols;

        // Row 2 onwards shows changes
        let row_idx = (row - 2) as usize;
        let start_idx = row_idx * items_per_row;

        let mut output = String::new();
        for col_idx in 0..items_per_row {
            let idx = start_idx + col_idx;
            if idx >= all_changes.len() {
                break;
            }

            let (_file, change) = &all_changes[idx];
            let (icon, color) = get_change_icon_color(&change.kind);
            let name = truncate_path(&change.name, item_width - 3);

            output.push_str(&format!("\x1b[{}m{}\x1b[0m{} ", color, icon, name));
        }

        write!(stdout, "{}", output)?;
        let content_len = strip_ansi_len(&output);
        let pad = (width as usize).saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;

        Ok(())
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

use crate::parsers::NodeKind;

// Helper functions
fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        // Show end of path (more useful)
        format!("…{}", &path[path.len() - (max_len - 1)..])
    }
}

/// Extract just the filename from a path
fn get_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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

/// Create a scaled diff bar showing additions (green) and deletions (red)
/// Max width is `max_width` characters, scaled proportionally to `max_changes`
fn create_diff_bar(additions: usize, deletions: usize, max_changes: usize, max_width: usize) -> String {
    let total = additions + deletions;
    if total == 0 {
        return format!("\x1b[38;5;240m{}\x1b[0m", "·".repeat(max_width.min(2)));
    }

    // Scale to max_width based on max_changes
    let scaled_total = ((total as f64 / max_changes as f64) * max_width as f64).ceil() as usize;
    let bar_width = scaled_total.min(max_width).max(1);

    // Distribute bar width between additions and deletions
    let add_chars = if total > 0 {
        ((additions as f64 / total as f64) * bar_width as f64).round() as usize
    } else {
        0
    };
    let del_chars = bar_width.saturating_sub(add_chars);

    let mut bar = String::new();
    if add_chars > 0 {
        bar.push_str(&format!("\x1b[38;5;83m{}\x1b[0m", "+".repeat(add_chars)));
    }
    if del_chars > 0 {
        bar.push_str(&format!("\x1b[38;5;203m{}\x1b[0m", "-".repeat(del_chars)));
    }

    bar
}

/// Get icon and color for a semantic change type
fn get_change_icon_color(kind: &NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::Class => ("◆", "38;5;141"),    // Purple - class
        NodeKind::Function => ("ƒ", "38;5;39"),  // Blue - function
        NodeKind::Method => ("·", "38;5;75"),    // Light blue - method
        NodeKind::Struct => ("▣", "38;5;179"),   // Orange - struct
        NodeKind::Enum => ("◇", "38;5;220"),     // Yellow - enum
        NodeKind::Trait => ("◈", "38;5;213"),    // Pink - trait
        NodeKind::Impl => ("▸", "38;5;114"),     // Green - impl
        NodeKind::Module => ("▢", "38;5;245"),   // Gray - module
        NodeKind::Const => ("●", "38;5;208"),    // Orange - const
        NodeKind::Other => ("•", "38;5;245"),    // Gray - other
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

