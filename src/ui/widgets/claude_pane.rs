use ratatui::{
    buffer::Buffer,
    layout::{Position, Rect},
    style::{Color, Modifier, Style},
    widgets::Widget,
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let widget = PtyWidget {
        screen: app.claude_pty.screen(),
        scroll_offset: app.claude_pty.scroll_offset,
    };
    f.render_widget(widget, area);
}

/// Set the terminal cursor position to match the PTY cursor. Call this LAST after all widgets are drawn.
pub fn set_cursor(f: &mut Frame, area: Rect, app: &App) {
    if app.claude_pty.scroll_offset == 0 {
        let (cursor_row, cursor_col) = app.claude_pty.screen().cursor_position();
        if cursor_row < area.height && cursor_col < area.width {
            f.set_cursor_position(Position::new(
                area.left() + cursor_col,
                area.top() + cursor_row,
            ));
        }
    }
}

struct PtyWidget<'a> {
    screen: &'a vt100::Screen,
    scroll_offset: usize,
}

impl<'a> Widget for PtyWidget<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let rows = area.height as usize;
        let cols = area.width as usize;

        // Get scrollback content if we're scrolled
        let scrollback = self.screen.scrollback();

        for row in 0..rows {
            // Calculate which line to render
            // If scroll_offset > 0, we're looking at scrollback buffer
            let line_idx = if self.scroll_offset > 0 {
                // Scrollback: offset from the end of scrollback buffer
                let scrollback_lines = scrollback;
                if self.scroll_offset > scrollback_lines {
                    continue;
                }
                // We want to show lines from scrollback
                // scroll_offset = how many lines up from current view
                scrollback_lines.saturating_sub(self.scroll_offset) + row
            } else {
                row
            };

            for col in 0..cols {
                let x = area.left() + col as u16;
                let y = area.top() + row as u16;

                if x >= area.right() || y >= area.bottom() {
                    continue;
                }

                // Get cell from screen (row, col)
                let cell = if self.scroll_offset > 0 && line_idx < scrollback {
                    // Reading from scrollback
                    self.screen.cell(line_idx as u16, col as u16)
                } else if self.scroll_offset == 0 {
                    self.screen.cell(row as u16, col as u16)
                } else {
                    continue;
                };

                if let Some(cell) = cell {
                    let ch = cell.contents();
                    let fg = convert_color(cell.fgcolor());
                    let bg = convert_color(cell.bgcolor());

                    let mut style = Style::default().fg(fg).bg(bg);

                    if cell.bold() {
                        style = style.add_modifier(Modifier::BOLD);
                    }
                    if cell.italic() {
                        style = style.add_modifier(Modifier::ITALIC);
                    }
                    if cell.underline() {
                        style = style.add_modifier(Modifier::UNDERLINED);
                    }
                    if cell.inverse() {
                        style = style.fg(bg).bg(fg);
                    }

                    let buf_cell = &mut buf[(x, y)];
                    buf_cell.set_symbol(if ch.is_empty() { " " } else { &ch });
                    buf_cell.set_style(style);
                }
            }
        }

        // Show cursor if not scrolled
        if self.scroll_offset == 0 {
            let (cursor_row, cursor_col) = self.screen.cursor_position();
            if cursor_row < area.height && cursor_col < area.width {
                let x = area.left() + cursor_col;
                let y = area.top() + cursor_row;
                buf[(x, y)].set_style(
                    Style::default()
                        .add_modifier(Modifier::REVERSED),
                );
            }
        } else {
            // Show scroll indicator when viewing scrollback
            let indicator = format!(" [{}↑] ", self.scroll_offset);
            let indicator_style = Style::default()
                .fg(Color::Black)
                .bg(Color::Yellow)
                .add_modifier(Modifier::BOLD);

            let start_x = area.right().saturating_sub(indicator.len() as u16 + 1);
            for (i, ch) in indicator.chars().enumerate() {
                let x = start_x + i as u16;
                if x < area.right() {
                    buf[(x, area.top())].set_symbol(&ch.to_string());
                    buf[(x, area.top())].set_style(indicator_style);
                }
            }
        }
    }
}

fn convert_color(color: vt100::Color) -> Color {
    match color {
        vt100::Color::Default => Color::Reset,
        vt100::Color::Idx(0) => Color::Black,
        vt100::Color::Idx(1) => Color::Red,
        vt100::Color::Idx(2) => Color::Green,
        vt100::Color::Idx(3) => Color::Yellow,
        vt100::Color::Idx(4) => Color::Blue,
        vt100::Color::Idx(5) => Color::Magenta,
        vt100::Color::Idx(6) => Color::Cyan,
        vt100::Color::Idx(7) => Color::Gray,
        vt100::Color::Idx(8) => Color::DarkGray,
        vt100::Color::Idx(9) => Color::LightRed,
        vt100::Color::Idx(10) => Color::LightGreen,
        vt100::Color::Idx(11) => Color::LightYellow,
        vt100::Color::Idx(12) => Color::LightBlue,
        vt100::Color::Idx(13) => Color::LightMagenta,
        vt100::Color::Idx(14) => Color::LightCyan,
        vt100::Color::Idx(15) => Color::White,
        vt100::Color::Idx(idx) => Color::Indexed(idx),
        vt100::Color::Rgb(r, g, b) => Color::Rgb(r, g, b),
    }
}
