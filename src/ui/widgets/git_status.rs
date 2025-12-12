use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::git::GitState;

pub fn draw(f: &mut Frame, area: Rect, git_state: &GitState) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Git Status ");

    let inner_area = block.inner(area);

    let lines: Vec<Line> = git_state
        .files
        .iter()
        .take(inner_area.height as usize)
        .map(|file| {
            let (indicator, color) = match file.status.as_str() {
                "M" => ("M", Color::Yellow),
                "A" => ("A", Color::Green),
                "D" => ("D", Color::Red),
                "R" => ("R", Color::Cyan),
                "C" => ("C", Color::Cyan),
                "U" => ("U", Color::Magenta),
                "?" => ("?", Color::Gray),
                _ => (" ", Color::White),
            };

            Line::from(vec![
                Span::styled(
                    format!("{} ", indicator),
                    Style::default().fg(color).add_modifier(Modifier::BOLD),
                ),
                Span::raw(&file.path),
            ])
        })
        .collect();

    let paragraph = Paragraph::new(lines).block(block);
    f.render_widget(paragraph, area);
}
