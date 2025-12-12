use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers, MouseEvent, MouseEventKind};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::io::Stdout;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;

use crate::git::GitState;
use crate::hooks::ClaudeStats;
use crate::parsers::DiffSummary;
use crate::pty::ClaudePty;
use crate::ui;

pub struct App {
    pub running: bool,
    pub claude_pty: ClaudePty,
    pub git_state: GitState,
    pub diff_summary: DiffSummary,
    pub claude_stats: ClaudeStats,
    pub ctrl_a_pressed: bool,
    pub last_mouse_event: Option<MouseEvent>,
    pub pty_area_height: u16,
    pty_rx: mpsc::Receiver<Vec<u8>>,
}

impl App {
    pub async fn new(cols: u16, rows: u16) -> Result<Self> {
        let (pty_tx, pty_rx) = mpsc::channel(256);
        // PTY gets 80% of terminal height
        let pty_rows = (rows as f32 * 0.8) as u16;
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
            pty_area_height: pty_rows,
            pty_rx,
        })
    }

    pub async fn run(&mut self, terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
        let mut last_git_refresh = Instant::now();
        let git_refresh_interval = Duration::from_secs(3);

        // Initial resize to match terminal size
        let size = terminal.size()?;
        let pty_height = (size.height as f32 * 0.8) as u16;
        self.claude_pty.resize(size.width, pty_height)?;

        while self.running {
            // Draw UI
            terminal.draw(|f| ui::draw(f, self))?;

            // Receive PTY output (non-blocking)
            while let Ok(data) = self.pty_rx.try_recv() {
                self.claude_pty.process_output(&data);
            }

            // Refresh git status periodically
            if last_git_refresh.elapsed() >= git_refresh_interval {
                self.refresh_git_state().await;
                last_git_refresh = Instant::now();
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
                        // Write pasted text directly to PTY as a single chunk
                        self.claude_pty.write(text.as_bytes())?;
                    }
                    Event::Mouse(mouse) => {
                        self.handle_mouse_event(mouse)?;
                    }
                    _ => {}
                }
            }
        }

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
                    // Ctrl+A, A sends Ctrl+A to PTY
                    self.claude_pty.write(&[0x01])?;
                }
                // Scrollback navigation
                KeyCode::PageUp => {
                    self.claude_pty.scroll_up(10);
                }
                KeyCode::PageDown => {
                    self.claude_pty.scroll_down(10);
                }
                KeyCode::Home => {
                    // Scroll to top of scrollback
                    let max = self.claude_pty.screen().scrollback();
                    self.claude_pty.scroll_offset = max;
                }
                KeyCode::End => {
                    // Scroll to bottom (current view)
                    self.claude_pty.reset_scroll();
                }
                KeyCode::Char('k') | KeyCode::Up => {
                    self.claude_pty.scroll_up(1);
                }
                KeyCode::Char('j') | KeyCode::Down => {
                    self.claude_pty.scroll_down(1);
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
                    // Convert to control character
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
        // Calculate PTY area (top 80%)
        let pty_height = (height as f32 * 0.8) as u16;
        self.pty_area_height = pty_height;
        self.claude_pty.resize(width, pty_height)?;
        Ok(())
    }

    fn handle_mouse_event(&mut self, mouse: MouseEvent) -> Result<()> {
        self.last_mouse_event = Some(mouse);

        // Only handle scroll events in the PTY area (top 80%)
        if mouse.row < self.pty_area_height {
            match mouse.kind {
                MouseEventKind::ScrollUp => {
                    self.claude_pty.scroll_up(3);
                }
                MouseEventKind::ScrollDown => {
                    self.claude_pty.scroll_down(3);
                }
                _ => {}
            }
        }
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
