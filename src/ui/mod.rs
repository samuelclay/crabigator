mod layout;
pub mod widgets;

use ratatui::Frame;

use crate::app::App;
use layout::create_layout;
use widgets::{claude_pane, claude_stats, debug_hud, diff_summary, git_status};

pub fn draw(f: &mut Frame, app: &App) {
    let (main_area, status_areas) = create_layout(f.area());

    // Draw Claude Code PTY in top 80%
    claude_pane::draw(f, main_area, app);

    // Draw status widgets in bottom 20%
    git_status::draw(f, status_areas[0], &app.git_state);
    diff_summary::draw(f, status_areas[1], &app.diff_summary);
    claude_stats::draw(f, status_areas[2], &app.claude_stats);
    debug_hud::draw(f, status_areas[3], app);

    // Set cursor position LAST so it's not overridden by other widgets
    claude_pane::set_cursor(f, main_area, app);
}
