use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Paragraph},
    Frame,
};

use crate::parsers::{ChangeType, DiffSummary, NodeKind};

pub fn draw(f: &mut Frame, area: Rect, diff_summary: &DiffSummary) {
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title(" Changes ");

    let inner_area = block.inner(area);

    let mut lines: Vec<Line> = Vec::new();

    for file in &diff_summary.files {
        // File header
        lines.push(Line::from(Span::styled(
            &file.path,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )));

        // Changed nodes
        for (i, node) in file.changes.iter().enumerate() {
            let is_last = i == file.changes.len() - 1;
            let prefix = if is_last { "  └─ " } else { "  ├─ " };

            let kind_str = match node.kind {
                NodeKind::Class => "class",
                NodeKind::Function => "fn",
                NodeKind::Method => "method",
                NodeKind::Struct => "struct",
                NodeKind::Enum => "enum",
                NodeKind::Trait => "trait",
                NodeKind::Impl => "impl",
                NodeKind::Module => "mod",
                NodeKind::Const => "const",
                NodeKind::Other => "",
            };

            let change_color = match node.change_type {
                ChangeType::Added => Color::Green,
                ChangeType::Modified => Color::Yellow,
                ChangeType::Deleted => Color::Red,
            };

            let change_str = match node.change_type {
                ChangeType::Added => "(added)",
                ChangeType::Modified => "(modified)",
                ChangeType::Deleted => "(deleted)",
            };

            lines.push(Line::from(vec![
                Span::raw(prefix),
                Span::styled(
                    format!("{} ", kind_str),
                    Style::default().fg(Color::Blue),
                ),
                Span::raw(&node.name),
                Span::raw(" "),
                Span::styled(change_str, Style::default().fg(change_color)),
            ]));

            // Render children (methods inside impl blocks, etc.)
            for (j, child) in node.children.iter().enumerate() {
                let child_prefix = if is_last { "       " } else { "  │    " };
                let child_connector = if j == node.children.len() - 1 {
                    "└─ "
                } else {
                    "├─ "
                };

                let child_kind = match child.kind {
                    NodeKind::Method => "method",
                    NodeKind::Function => "fn",
                    _ => "",
                };

                let child_change_color = match child.change_type {
                    ChangeType::Added => Color::Green,
                    ChangeType::Modified => Color::Yellow,
                    ChangeType::Deleted => Color::Red,
                };

                let child_change_str = match child.change_type {
                    ChangeType::Added => "(+)",
                    ChangeType::Modified => "(~)",
                    ChangeType::Deleted => "(-)",
                };

                lines.push(Line::from(vec![
                    Span::raw(format!("{}{}", child_prefix, child_connector)),
                    Span::styled(
                        format!("{} ", child_kind),
                        Style::default().fg(Color::Blue),
                    ),
                    Span::raw(&child.name),
                    Span::raw(" "),
                    Span::styled(child_change_str, Style::default().fg(child_change_color)),
                ]));
            }
        }
    }

    // Truncate to fit
    let visible_lines: Vec<Line> = lines
        .into_iter()
        .take(inner_area.height as usize)
        .collect();

    let paragraph = Paragraph::new(visible_lines).block(block);
    f.render_widget(paragraph, area);
}
