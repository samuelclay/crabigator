//! V2 App - Scroll region approach
//!
//! Architecture:
//! - Set terminal scroll region to top N rows for Claude Code
//! - Claude Code renders within that region (thinks it's the full terminal)
//! - We render our status widgets below the scroll region using ratatui
//! - PTY output passes through untouched

use anyhow::Result;
use crossterm::{
    cursor::{MoveTo, SavePosition, RestorePosition},
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent},
    execute,
};
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    symbols,
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph, Gauge},
    Terminal,
};
use std::io::{Write, stdout, Stdout};
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
        let mut stdout = stdout();
        stdout.write_all(data)?;
        stdout.flush()?;

        // Update our internal parser too (for any stats/analysis)
        self.claude_pty.process_output(data);

        Ok(())
    }

    /// Draw status bar in the reserved bottom area using ratatui
    fn draw_status_bar(&mut self) -> Result<()> {
        let mut stdout = stdout();

        // Save cursor position (we'll restore it after drawing)
        execute!(stdout, SavePosition)?;

        // Move to the status area
        execute!(stdout, MoveTo(0, self.pty_rows))?;

        // Create a temporary backend and terminal for the status area
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::with_options(
            backend,
            ratatui::TerminalOptions {
                viewport: ratatui::Viewport::Fixed(Rect {
                    x: 0,
                    y: self.pty_rows,
                    width: self.total_cols,
                    height: self.status_rows,
                }),
            },
        )?;

        terminal.draw(|f| {
            let area = f.area();

            // Create layout: separator line + three widget columns
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(1),  // Separator
                    Constraint::Min(0),     // Widgets
                ])
                .split(area);

            // Draw separator line
            let separator = Block::default()
                .style(Style::default().bg(Color::DarkGray));
            f.render_widget(separator, chunks[0]);

            // Draw separator text centered
            let sep_text = Paragraph::new(Line::from(vec![
                Span::styled(" ─── ", Style::default().fg(Color::Gray)),
                Span::styled("CRABIGATOR", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
                Span::styled(" ─── ", Style::default().fg(Color::Gray)),
            ]))
            .style(Style::default().bg(Color::DarkGray))
            .alignment(ratatui::layout::Alignment::Center);
            f.render_widget(sep_text, chunks[0]);

            // Split widget area into 3 columns
            let widget_chunks = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(33),
                    Constraint::Percentage(34),
                    Constraint::Percentage(33),
                ])
                .split(chunks[1]);

            // Git Status Widget
            self.render_git_widget(f, widget_chunks[0]);

            // Diff Summary Widget
            self.render_diff_widget(f, widget_chunks[1]);

            // Claude Stats Widget
            self.render_stats_widget(f, widget_chunks[2]);
        })?;

        // Get stdout back and restore cursor
        let mut stdout = terminal.backend_mut().by_ref();
        execute!(stdout, RestorePosition)?;

        Ok(())
    }

    fn render_git_widget(&self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(Span::styled(" Git ", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD)));

        let inner = block.inner(area);

        let file_count = self.git_state.files.len();
        let status_text = if file_count == 0 {
            vec![Line::from(Span::styled("✓ Clean", Style::default().fg(Color::Green)))]
        } else {
            let mut lines = vec![
                Line::from(Span::styled(
                    format!("{} file(s)", file_count),
                    Style::default().fg(Color::Yellow),
                )),
            ];
            // Show first few files
            for file in self.git_state.files.iter().take(inner.height.saturating_sub(1) as usize) {
                let (icon, color) = match file.status.as_str() {
                    "M" => ("●", Color::Yellow),
                    "A" => ("+", Color::Green),
                    "D" => ("−", Color::Red),
                    "?" => ("?", Color::Gray),
                    _ => ("•", Color::White),
                };
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(color)),
                    Span::raw(truncate_path(&file.path, inner.width.saturating_sub(3) as usize)),
                ]));
            }
            lines
        };

        let paragraph = Paragraph::new(status_text).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_diff_widget(&self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(Span::styled(" Changes ", Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)));

        let inner = block.inner(area);

        let mut lines: Vec<Line> = Vec::new();

        if self.diff_summary.files.is_empty() {
            lines.push(Line::from(Span::styled("No changes", Style::default().fg(Color::DarkGray))));
        } else {
            for file in self.diff_summary.files.iter().take(inner.height as usize) {
                let change_count = file.changes.len();
                let icon = if change_count > 0 { "◆" } else { "◇" };
                lines.push(Line::from(vec![
                    Span::styled(format!("{} ", icon), Style::default().fg(Color::Cyan)),
                    Span::raw(truncate_path(&file.path, inner.width.saturating_sub(5) as usize)),
                    Span::styled(format!(" ({})", change_count), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
    }

    fn render_stats_widget(&self, f: &mut ratatui::Frame, area: Rect) {
        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::DarkGray))
            .border_type(ratatui::widgets::BorderType::Rounded)
            .title(Span::styled(" Stats ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)));

        let lines = vec![
            Line::from(vec![
                Span::styled("⏱ ", Style::default().fg(Color::Blue)),
                Span::raw("Idle: "),
                Span::styled(
                    format_duration(self.claude_stats.idle_seconds),
                    Style::default().fg(if self.claude_stats.idle_seconds > 60 { Color::Yellow } else { Color::Green }),
                ),
            ]),
            Line::from(vec![
                Span::styled("⚡", Style::default().fg(Color::Yellow)),
                Span::raw(" Work: "),
                Span::styled(
                    format_duration(self.claude_stats.work_seconds),
                    Style::default().fg(Color::Cyan),
                ),
            ]),
            Line::from(vec![
                Span::styled("◈ ", Style::default().fg(Color::Magenta)),
                Span::raw("Tokens: "),
                Span::styled(
                    format_number(self.claude_stats.tokens_used),
                    Style::default().fg(Color::Magenta),
                ),
            ]),
            Line::from(vec![
                Span::styled("✉ ", Style::default().fg(Color::Blue)),
                Span::raw("Msgs: "),
                Span::styled(
                    self.claude_stats.messages_count.to_string(),
                    Style::default().fg(Color::Blue),
                ),
            ]),
        ];

        let paragraph = Paragraph::new(lines).block(block);
        f.render_widget(paragraph, area);
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
        let bytes = match key.code {
            KeyCode::Char(c) => {
                if key.modifiers.contains(KeyModifiers::CONTROL) {
                    vec![(c as u8) & 0x1f]
                } else {
                    let mut buf = [0u8; 4];
                    let s = c.encode_utf8(&mut buf);
                    s.as_bytes().to_vec()
                }
            }
            KeyCode::Enter => vec![b'\r'],
            KeyCode::Backspace => vec![0x7f],
            KeyCode::Tab => vec![b'\t'],
            KeyCode::Esc => vec![0x1b],
            KeyCode::Up => vec![0x1b, b'[', b'A'],
            KeyCode::Down => vec![0x1b, b'[', b'B'],
            KeyCode::Right => vec![0x1b, b'[', b'C'],
            KeyCode::Left => vec![0x1b, b'[', b'D'],
            KeyCode::Home => vec![0x1b, b'[', b'H'],
            KeyCode::End => vec![0x1b, b'[', b'F'],
            KeyCode::PageUp => vec![0x1b, b'[', b'5', b'~'],
            KeyCode::PageDown => vec![0x1b, b'[', b'6', b'~'],
            KeyCode::Delete => vec![0x1b, b'[', b'3', b'~'],
            KeyCode::Insert => vec![0x1b, b'[', b'2', b'~'],
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
