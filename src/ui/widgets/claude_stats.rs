use ratatui::{
    layout::Rect,
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::hooks::ClaudeStats;

pub fn draw(f: &mut Frame, area: Rect, stats: &ClaudeStats) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Claude Stats ");

    let idle_color = if stats.idle_seconds > 60 {
        Color::Yellow
    } else {
        Color::Green
    };

    let lines = vec![
        Line::from(vec![
            Span::raw("Idle: "),
            Span::styled(format_duration(stats.idle_seconds), Style::default().fg(idle_color)),
        ]),
        Line::from(vec![
            Span::raw("Work: "),
            Span::styled(
                format_duration(stats.work_seconds),
                Style::default().fg(Color::Cyan),
            ),
        ]),
        Line::from(vec![
            Span::raw("Tokens: "),
            Span::styled(
                format_number(stats.tokens_used),
                Style::default().fg(Color::Magenta),
            ),
        ]),
        Line::from(vec![
            Span::raw("Msgs: "),
            Span::styled(
                stats.messages_count.to_string(),
                Style::default().fg(Color::Blue),
            ),
        ]),
    ];

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
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
