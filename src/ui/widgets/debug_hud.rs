use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::app::App;

pub fn draw(f: &mut Frame, area: Rect, app: &App) {
    let screen = app.claude_pty.screen();
    let (cursor_row, cursor_col) = screen.cursor_position();
    let scrollback_size = screen.scrollback();
    let screen_rows = screen.size().0;
    let screen_cols = screen.size().1;

    let lines = vec![
        Line::from(vec![
            Span::styled("PTY: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}x{}", screen_cols, screen_rows)),
        ]),
        Line::from(vec![
            Span::styled("Cursor: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("({}, {})", cursor_row, cursor_col)),
        ]),
        Line::from(vec![
            Span::styled("Scrollback: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{} lines", scrollback_size)),
        ]),
        Line::from(vec![
            Span::styled("Scroll offset: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{}", app.claude_pty.scroll_offset)),
        ]),
        Line::from(vec![
            Span::styled("Ctrl+A: ", Style::default().fg(Color::Yellow)),
            Span::raw(if app.ctrl_a_pressed { "PRESSED" } else { "no" }),
        ]),
        Line::from(vec![
            Span::styled("Mouse: ", Style::default().fg(Color::Yellow)),
            Span::raw(format!("{:?}", app.last_mouse_event)),
        ]),
    ];

    let paragraph = Paragraph::new(lines)
        .block(Block::default().borders(Borders::ALL).title("Debug HUD"))
        .style(Style::default().fg(Color::White));

    f.render_widget(paragraph, area);
}
