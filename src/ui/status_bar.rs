//! Status bar rendering
//!
//! Coordinates the layout and rendering of all status bar widgets.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::parsers::DiffSummary;
use crate::terminal::escape::{self, color, RESET};

use super::{draw_changes_widget, draw_git_widget, draw_stats_widget};

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
    session_stats: &SessionStats,
    git_state: &GitState,
    diff_summary: &DiffSummary,
) -> Result<()> {
    // Save cursor position
    write!(stdout, "{}", escape::CURSOR_SAVE)?;

    // Move to status area (below the scroll region)
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;

    // Draw thick separator line (matching banner style)
    write!(stdout, "{}{}", escape::bg(color::BG_DARK), escape::fg(color::DARK_GRAY))?;
    for _ in 0..layout.total_cols {
        write!(stdout, "━")?;
    }
    write!(stdout, "{}", RESET)?;

    // Calculate column widths based on available height
    // In compact mode (short terminal), stats gets more width for two-column layout
    let compact = layout.status_rows <= 5;
    let stats_width = if compact {
        // Wider stats for two-column layout: ~30% of width, min 36 chars
        ((layout.total_cols as f32) * 0.30).max(36.0) as u16
    } else {
        // Normal: ~17% of width, min 24 chars
        ((layout.total_cols as f32) * 0.17).max(24.0) as u16
    };
    let remaining = layout.total_cols.saturating_sub(stats_width + 2); // 2 for separators

    // Check if git needs multiple columns (files > available rows)
    let git_available_rows = layout.status_rows.saturating_sub(2) as usize; // -2 for separator + header
    let git_needs_multi_column = git_state.files.len() > git_available_rows;

    // Flex ratio: git gets 4/8 if multi-column, 3/8 if single-column
    let (git_width, changes_width) = if git_needs_multi_column {
        // 4:4 split (50/50)
        let git_w = remaining / 2;
        (git_w, remaining - git_w)
    } else {
        // 3:5 split - git gets less, changes gets more
        let git_w = (remaining * 3) / 8;
        (git_w, remaining - git_w)
    };

    // Draw content rows
    for row in 1..layout.status_rows {
        write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1 + row, 1))?;

        // Stats column (leftmost, fixed width)
        draw_stats_widget(stdout, layout.pty_rows, 0, row, stats_width, layout.status_rows, session_stats)?;

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
