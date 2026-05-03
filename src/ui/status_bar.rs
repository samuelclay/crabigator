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

const BANNER_RESERVED: u16 = 2;
/// Minimum content rows reserved for widget data below the separator.
pub const MIN_WIDGET_DATA_ROWS: u16 = 4;
/// Status rows include the separator plus widget data rows.
pub const MIN_STATUS_ROWS: u16 = MIN_WIDGET_DATA_ROWS + 1;

/// Split terminal height into assistant PTY rows and status widget rows.
///
/// Banner rows are fixed between the two regions. Status rows include the
/// widget separator, so MIN_STATUS_ROWS preserves four rows for widget content.
pub fn split_terminal_rows(total_rows: u16) -> (u16, u16) {
    let preferred_status_rows = ((total_rows as f32 * 0.2) as u16).max(MIN_STATUS_ROWS);
    let max_status_rows = total_rows.saturating_sub(BANNER_RESERVED + 1).max(1);
    let status_rows = preferred_status_rows.min(max_status_rows);
    let pty_rows = total_rows
        .saturating_sub(status_rows + BANNER_RESERVED)
        .max(1);

    (pty_rows, status_rows)
}

/// Draw the entire status bar area with all widgets
///
/// IMPORTANT: This function receives the cursor position from the caller
/// to restore after drawing. We can't use CURSOR_SAVE/RESTORE because
/// our vt100 parser doesn't track those, so DSR responses would be wrong.
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
    cursor_position: Option<(u16, u16)>,  // (row, col) from vt100 parser, 0-indexed
) -> Result<()> {
    // Begin synchronized update - terminal batches all our drawing
    // so cursor movements don't interfere with Claude's incremental updates
    write!(stdout, "{}", escape::SYNC_BEGIN)?;

    // Clear the entire status bar area first to prevent artifacts
    // This is critical: without clearing, resizes or partial redraws leave old content
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;
    write!(stdout, "{}", escape::CLEAR_TO_END)?;

    // Banner space is always reserved between PTY and status bar
    // Draw banners if active, otherwise leave the space empty
    // Update banner takes first row, pairing banner takes remaining space
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
    let footer_rows = pairing_footer_rows(layout.status_rows, pairing_state);
    let widget_status_rows = layout.status_rows.saturating_sub(footer_rows).max(1);
    let widget_data_rows = widget_status_rows.saturating_sub(1);

    // Calculate column widths based on available height
    // In compact mode (short terminal), stats gets more width for two-column layout
    let compact = widget_status_rows <= MIN_STATUS_ROWS;

    let stats_width = if compact {
        // Wider stats for two-column layout: ~35% of width, min 36 chars
        ((layout.total_cols as f32) * 0.35).max(36.0) as u16
    } else {
        // Normal: ~22% of width, min 24 chars
        ((layout.total_cols as f32) * 0.22).max(24.0) as u16
    };

    // Account for separators: 2 separators between 3 columns
    let num_separators = 2;
    let remaining = layout.total_cols.saturating_sub(stats_width + num_separators as u16);

    // Check if git needs multiple columns (files > available rows)
    let git_available_rows = widget_status_rows.saturating_sub(2) as usize; // -2 for separator + header
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
    let widget_pty_rows = layout.pty_rows + BANNER_RESERVED;

    for widget_row in 1..=widget_data_rows {
        // Stats column (leftmost, fixed width)
        draw_stats_widget(
            stdout,
            WidgetArea {
                pty_rows: widget_pty_rows,
                col: 0,
                row: widget_row,
                width: stats_width,
                height: widget_status_rows,
            },
            session_stats,
            cloud_status,
            is_paired,
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
                height: widget_status_rows,
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
                height: widget_status_rows,
            },
            diff_summary,
            terminal_title,
            ide,
            cwd,
        )?;
    }

    // Draw full-width pairing URL footer (spans all columns)
    if footer_rows > 0 {
        if let Some(code) = pairing_state.pairing_code.as_deref() {
            if footer_rows == 2 {
                // Separator line
                let sep_row = layout.pty_rows + BANNER_RESERVED + widget_status_rows + 1;
                write!(stdout, "{}", escape::cursor_to(sep_row, 1))?;
                let line = "─".repeat(layout.total_cols as usize);
                write!(stdout, "{}{}{}", escape::fg(color::DARK_GRAY), line, RESET)?;
            }

            // Pair URL row
            let url_row = layout.pty_rows + BANNER_RESERVED + layout.status_rows;
            write!(stdout, "{}", escape::cursor_to(url_row, 1))?;
            let url = format!("https://drinkcrabigator.com/dashboard?setup={}", code);
            let url_display = format!("drinkcrabigator.com/dashboard?setup={}", code);
            let label = format!("{}Pair: {}", escape::fg(color::DARK_GRAY), RESET);
            let display = format!("{}{}{}{}", label, escape::fg(color::DARK_GRAY), url_display, RESET);
            // OSC 8 hyperlink
            write!(stdout, "\x1b]8;;{}\x07{}\x1b]8;;\x07", url, display)?;
        }
    }

    // Restore cursor to position known by vt100 parser
    // We use absolute positioning instead of CURSOR_SAVE/RESTORE because
    // the vt100 parser doesn't track those, which would cause DSR responses
    // to be wrong and corrupt Claude's incremental screen updates.
    if let Some((row, col)) = cursor_position {
        // Convert from 0-indexed (vt100) to 1-indexed (terminal)
        write!(stdout, "{}", escape::cursor_to(row + 1, col + 1))?;
    }

    // End synchronized update - terminal renders all our drawing atomically
    write!(stdout, "{}", escape::SYNC_END)?;
    stdout.flush()?;

    Ok(())
}

fn pairing_footer_rows(status_rows: u16, pairing_state: &PairingState) -> u16 {
    if pairing_state.pairing_code.is_none() {
        return 0;
    }

    let widget_data_rows = status_rows.saturating_sub(1);
    widget_data_rows.saturating_sub(MIN_WIDGET_DATA_ROWS).min(2)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pairing_state() -> PairingState {
        PairingState {
            pairing_code: Some("ABC-DEF-GHI".to_string()),
            ..PairingState::default()
        }
    }

    #[test]
    fn split_terminal_rows_keeps_four_widget_rows_when_possible() {
        let (pty_rows, status_rows) = split_terminal_rows(20);

        assert_eq!(status_rows, MIN_STATUS_ROWS);
        assert_eq!(pty_rows + BANNER_RESERVED + status_rows, 20);
    }

    #[test]
    fn split_terminal_rows_uses_preferred_status_height_when_larger() {
        let (pty_rows, status_rows) = split_terminal_rows(44);

        assert_eq!(status_rows, 8);
        assert_eq!(pty_rows + BANNER_RESERVED + status_rows, 44);
    }

    #[test]
    fn split_terminal_rows_shrinks_status_area_only_when_terminal_is_too_short() {
        let (pty_rows, status_rows) = split_terminal_rows(7);

        assert_eq!(pty_rows, 1);
        assert_eq!(status_rows, 4);
    }

    #[test]
    fn pairing_footer_uses_only_surplus_widget_rows() {
        let pairing = pairing_state();

        assert_eq!(pairing_footer_rows(MIN_STATUS_ROWS, &pairing), 0);
        assert_eq!(pairing_footer_rows(MIN_STATUS_ROWS + 1, &pairing), 1);
        assert_eq!(pairing_footer_rows(MIN_STATUS_ROWS + 3, &pairing), 2);
    }
}
