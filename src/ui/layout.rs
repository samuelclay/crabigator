use ratatui::layout::{Constraint, Direction, Layout, Rect};

pub fn create_layout(area: Rect) -> (Rect, Vec<Rect>) {
    // Main vertical split: 80% PTY, 20% status bar
    let main_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Percentage(80), Constraint::Percentage(20)])
        .split(area);

    let main_area = main_chunks[0];
    let status_area = main_chunks[1];

    // Status bar horizontal split: 4 widgets (git, diff, stats, debug)
    let status_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(20),
            Constraint::Percentage(25),
            Constraint::Percentage(20),
            Constraint::Percentage(35),
        ])
        .split(status_area);

    (main_area, status_chunks.to_vec())
}
