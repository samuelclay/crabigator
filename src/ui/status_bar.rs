//! Status bar rendering
//!
//! Coordinates the layout and rendering of all status bar widgets.

use std::io::{Stdout, Write};
use std::path::Path;

use anyhow::Result;

use crate::cloud::CloudStatus;
use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::ide::IdeKind;
use crate::parsers::DiffSummary;
use crate::terminal::escape::{self, color, RESET};
use crate::update::UpdateState;

use super::{draw_changes_widget, draw_git_widget, draw_pairing_banner, draw_update_banner, draw_stats_widget, PairingState, WidgetArea};

/// Layout information needed for rendering widgets
pub struct Layout {
    pub pty_rows: u16,
    pub total_cols: u16,
    pub status_rows: u16,
}

/// Draw the entire status bar area with all widgets
#[allow(clippy::too_many_arguments)]
pub fn draw_status_bar(
    stdout: &mut Stdout,
    layout: &Layout,
    session_stats: &SessionStats,
    git_state: &GitState,
    diff_summary: &DiffSummary,
    terminal_title: Option<&str>,
    ide: IdeKind,
    cwd: &Path,
    cloud_status: Option<&CloudStatus>,
    pairing_state: &PairingState,
    update_state: &UpdateState,
) -> Result<()> {
    // Save cursor position
    write!(stdout, "{}", escape::CURSOR_SAVE)?;

    // Clear the entire status bar area first to prevent artifacts
    // This is critical: without clearing, resizes or partial redraws leave old content
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;
    write!(stdout, "{}", escape::CLEAR_TO_END)?;

    // Banner space is always reserved (2 rows between PTY and status bar)
    // Draw banners if active, otherwise leave the space empty
    // Update banner takes first row, pairing banner takes remaining space
    const BANNER_RESERVED: u16 = 2;
    let update_banner_rows = update_state.banner_rows();
    let pairing_compact = update_banner_rows > 0;
    let pairing_banner_rows = if pairing_compact {
        pairing_state.banner_rows_compact()
    } else {
        pairing_state.banner_rows()
    };

    // Move to banner area (below PTY scroll region)
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;

    // Track current row for stacking banners
    let mut current_banner_row = layout.pty_rows + 1;

    // Draw update banner first (if needed) - single row
    if update_banner_rows > 0 {
        draw_update_banner(
            stdout,
            current_banner_row,
            layout.total_cols,
            update_state,
        )?;
        current_banner_row += update_banner_rows;
    }

    // Draw pairing banner if needed (and there's room)
    if pairing_banner_rows > 0 {
        let banner_limit = layout.pty_rows + BANNER_RESERVED;
        let end_row = current_banner_row
            .saturating_add(pairing_banner_rows)
            .saturating_sub(1);
        if end_row <= banner_limit {
            draw_pairing_banner(
                stdout,
                current_banner_row,
                layout.total_cols,
                pairing_state,
                pairing_compact,
            )?;
        }
    }

    // Draw thick separator line (always after the reserved banner space)
    let separator_row = layout.pty_rows + 1 + BANNER_RESERVED;
    write!(stdout, "{}", escape::cursor_to(separator_row, 1))?;
    write!(stdout, "{}{}", escape::bg(color::BG_DARK), escape::fg(color::DARK_GRAY))?;
    for _ in 0..layout.total_cols {
        write!(stdout, "━")?;
    }
    write!(stdout, "{}", RESET)?;
    let is_paired = pairing_state.has_linked_devices;

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

    // Account for separators: 2 separators between 3 columns
    let num_separators = 2;
    let remaining = layout.total_cols.saturating_sub(stats_width + num_separators as u16);

    // Check if git needs multiple columns (files > available rows)
    let widget_rows = layout.status_rows;
    let git_available_rows = widget_rows.saturating_sub(2) as usize; // -2 for separator + header
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

    // Draw content rows (after reserved banner space + separator)
    let first_widget_row = 1 + BANNER_RESERVED;
    let widget_pty_rows = layout.pty_rows + BANNER_RESERVED;
    for row in first_widget_row..layout.status_rows {
        write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1 + row, 1))?;

        // Stats column (leftmost, fixed width)
        // Pass pairing token for "Pair another device" link when already paired
        let pairing_token = if is_paired {
            pairing_state.pairing_token.as_deref()
        } else {
            None
        };
        // Adjust widget row to be relative (starting from 1)
        let widget_row = row - BANNER_RESERVED;
        draw_stats_widget(
            stdout,
            WidgetArea {
                pty_rows: widget_pty_rows,
                col: 0,
                row: widget_row,
                width: stats_width,
                height: layout.status_rows,
            },
            session_stats,
            cloud_status,
            is_paired,
            pairing_token,
        )?;

        // Separator
        write!(stdout, "{}│{}", escape::fg(color::DARK_GRAY), RESET)?;

        // Track current column position
        let mut current_col = stats_width + 1;

        // Git column
        draw_git_widget(
            stdout,
            WidgetArea {
                pty_rows: widget_pty_rows,
                col: current_col,
                row: widget_row,
                width: git_width,
                height: layout.status_rows,
            },
            git_state,
            ide,
            cwd,
        )?;

        // Separator
        write!(stdout, "{}│{}", escape::fg(color::DARK_GRAY), RESET)?;
        current_col += git_width + 1;

        // Changes column (rightmost)
        draw_changes_widget(
            stdout,
            WidgetArea {
                pty_rows: widget_pty_rows,
                col: current_col,
                row: widget_row,
                width: changes_width,
                height: layout.status_rows,
            },
            diff_summary,
            terminal_title,
            ide,
            cwd,
        )?;
    }

    // Re-establish scroll region after drawing outside it
    // This prevents some terminals from resetting scroll context
    write!(stdout, "{}", escape::scroll_region(1, layout.pty_rows))?;

    // Restore cursor position
    write!(stdout, "{}", escape::CURSOR_RESTORE)?;
    stdout.flush()?;

    Ok(())
}
