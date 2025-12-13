//! Widget modules for status bar rendering
//!
//! Each widget is responsible for rendering its own section of the status bar.
//! Widgets use raw ANSI escape sequences for terminal output.

mod changes;
mod git;
mod stats;

pub use changes::draw_changes_widget;
pub use git::draw_git_widget;
pub use stats::draw_stats_widget;

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::escape::{self, color, RESET};
use crate::git::GitState;
use crate::hooks::ClaudeStats;
use crate::parsers::DiffSummary;

/// Layout information needed for rendering widgets
pub struct Layout {
    pub pty_rows: u16,
    pub total_cols: u16,
    pub status_rows: u16,
}

/// Draw the entire status bar area with all widgets
pub fn draw_status_bar(
    stdout: &mut Stdout,
    layout: &Layout,
    claude_stats: &ClaudeStats,
    git_state: &GitState,
    diff_summary: &DiffSummary,
) -> Result<()> {
    // Save cursor position
    write!(stdout, "{}", escape::CURSOR_SAVE)?;

    // Move to status area (below the scroll region)
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;

    // Draw thin separator line
    write!(stdout, "{}{}", escape::bg(color::BG_DARK), escape::fg(color::DARK_GRAY))?;
    for _ in 0..layout.total_cols {
        write!(stdout, "─")?;
    }
    write!(stdout, "{}", RESET)?;

    // Calculate column widths: Stats has min width, Git and Changes share the rest
    let stats_width = 22u16; // Fixed width for stats
    let remaining = layout.total_cols.saturating_sub(stats_width + 2); // 2 for separators
    let git_width = remaining / 2;
    let changes_width = remaining - git_width;

    // Draw content rows
    for row in 1..layout.status_rows {
        write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1 + row, 1))?;

        // Stats column (leftmost, fixed width)
        draw_stats_widget(stdout, layout.pty_rows, 0, row, stats_width, claude_stats)?;

        // Separator
        write!(stdout, "{}│{}", escape::fg(color::DARK_GRAY), RESET)?;

        // Git column
        draw_git_widget(
            stdout,
            layout.pty_rows,
            stats_width + 1,
            row,
            git_width,
            layout.status_rows,
            git_state,
        )?;

        // Separator
        write!(stdout, "{}│{}", escape::fg(color::DARK_GRAY), RESET)?;

        // Changes column (rightmost)
        draw_changes_widget(
            stdout,
            layout.pty_rows,
            stats_width + git_width + 2,
            row,
            changes_width,
            layout.status_rows,
            diff_summary,
        )?;
    }

    // Restore cursor position
    write!(stdout, "{}", escape::CURSOR_RESTORE)?;
    stdout.flush()?;

    Ok(())
}
