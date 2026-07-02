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
use crate::recap::RecapState;
use crate::terminal::escape::{self, color, RESET};
use crate::update::UpdateState;

use super::{
    changes_natural_rows, draw_changes_widget, draw_git_widget, draw_pairing_banner,
    draw_recap_handoff, draw_stats_widget, draw_update_banner, git_natural_rows,
    stats_natural_rows, PairingState, WidgetArea, HANDOFF_RESERVED_ROWS,
};

/// Layout information needed for rendering widgets
pub struct Layout {
    pub pty_rows: u16,
    pub total_cols: u16,
    pub status_rows: u16,
}

/// Minimum content rows reserved for widget data below the separator.
pub const MIN_WIDGET_DATA_ROWS: u16 = 4;
/// Status rows include the separator plus widget data rows.
pub const MIN_STATUS_ROWS: u16 = MIN_WIDGET_DATA_ROWS + 1;

/// Split terminal height into assistant PTY rows and status widget rows.
///
/// Handoff rows are fixed between the two regions. Status rows include the
/// widget separator, so MIN_STATUS_ROWS preserves four rows for widget content.
pub fn split_terminal_rows(total_rows: u16) -> (u16, u16) {
    let status_rows = preferred_status_rows_max(total_rows);
    let pty_rows = total_rows
        .saturating_sub(status_rows + HANDOFF_RESERVED_ROWS)
        .max(1);

    (pty_rows, status_rows)
}

/// Upper bound on status rows for the current terminal height — the historical
/// "20% of screen, never less than MIN_STATUS_ROWS, never more than fits."
pub fn preferred_status_rows_max(total_rows: u16) -> u16 {
    let preferred_status_rows = ((total_rows as f32 * 0.2) as u16).max(MIN_STATUS_ROWS);
    let max_status_rows = total_rows.saturating_sub(HANDOFF_RESERVED_ROWS + 1).max(1);
    preferred_status_rows.min(max_status_rows)
}

/// No bottom-edge footer is rendered any more — the pair code lives in the
/// stats header and the recap toast/hint live in the handoff strip — so no
/// row is ever claimed below the widgets.
fn desired_footer_rows(
    _pairing_state: &PairingState,
    _recap_state: &RecapState,
    _recap_toast_visible: bool,
) -> u16 {
    0
}

/// Estimate the column widths the renderer will assign to git and changes,
/// using the same proportions as `draw_status_bar`. Returned as
/// `(git_width, changes_width)`. The estimate ignores the multi-column flex
/// path because that decision depends on how many rows we ultimately give git
/// — a rough split is enough for the natural-row heuristics.
fn estimate_column_widths(total_cols: u16) -> (u16, u16) {
    let stats_width = ((total_cols as f32) * 0.22).max(24.0) as u16;
    let separators: u16 = 2;
    let remaining = total_cols.saturating_sub(stats_width + separators);
    let git_w = (remaining * 3) / 8;
    let changes_w = remaining.saturating_sub(git_w);
    (git_w, changes_w)
}

/// Compute the status row count needed to show every widget's natural content
/// (plus the separator and any active footer), clamped between MIN_STATUS_ROWS
/// and the historical 20% ceiling. The result lets the widget area shrink when
/// content is shorter than the cap — including when git or changes use packed
/// layouts that consume far fewer rows than there are items.
#[allow(clippy::too_many_arguments)]
pub fn compute_dynamic_status_rows(
    total_rows: u16,
    total_cols: u16,
    session_stats: &SessionStats,
    git_state: &GitState,
    diff_summary: &DiffSummary,
    terminal_title: Option<&str>,
    pairing_state: &PairingState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
) -> u16 {
    let (git_w, changes_w) = estimate_column_widths(total_cols);
    let has_title = terminal_title.is_some_and(|t| !t.is_empty());
    let natural = stats_natural_rows(session_stats)
        .max(git_natural_rows(git_state, git_w))
        .max(changes_natural_rows(diff_summary, changes_w, has_title));
    let footer = desired_footer_rows(pairing_state, recap_state, recap_toast_visible);
    let desired = natural.saturating_add(1).saturating_add(footer); // +1 separator
    let preferred_max = preferred_status_rows_max(total_rows);
    desired.clamp(MIN_STATUS_ROWS, preferred_max)
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
    recap_state: &RecapState,
    recap_toast_visible: bool,
    cursor_position: Option<(u16, u16)>, // (row, col) from vt100 parser, 0-indexed
) -> Result<()> {
    // Begin synchronized update - terminal batches all our drawing
    // so cursor movements don't interfere with Claude's incremental updates
    write!(stdout, "{}", escape::SYNC_BEGIN)?;

    // Clear the entire status bar area first to prevent artifacts
    // This is critical: without clearing, resizes or partial redraws leave old content
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;
    write!(stdout, "{}", escape::CLEAR_TO_END)?;

    // Handoff space is always reserved between PTY and status bar.
    // Draw update/recap/pairing content if active, otherwise leave it empty.
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
        draw_update_banner(stdout, current_banner_row, layout.total_cols, update_state)?;
        current_banner_row += update_banner_rows;
    }

    let handoff_limit = layout.pty_rows + HANDOFF_RESERVED_ROWS;
    let remaining_rows = handoff_limit
        .saturating_sub(current_banner_row)
        .saturating_add(1);
    let recap_rows = draw_recap_handoff(
        stdout,
        current_banner_row,
        layout.total_cols,
        recap_state,
        remaining_rows,
        recap_toast_visible,
    )?;
    current_banner_row += recap_rows;

    // Draw pairing banner if needed, there's room, and recap is not occupying the handoff.
    if recap_rows == 0 && pairing_banner_rows > 0 {
        let end_row = current_banner_row
            .saturating_add(pairing_banner_rows)
            .saturating_sub(1);
        if end_row <= handoff_limit {
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
    let separator_row = layout.pty_rows + 1 + HANDOFF_RESERVED_ROWS;
    write!(stdout, "{}", escape::cursor_to(separator_row, 1))?;
    write!(
        stdout,
        "{}{}",
        escape::bg(color::BG_DARK),
        escape::fg(color::DARK_GRAY)
    )?;
    for _ in 0..layout.total_cols {
        write!(stdout, "━")?;
    }
    write!(stdout, "{}", RESET)?;
    let is_paired = pairing_state.has_linked_devices;
    // No bottom footer any more — the entire status_rows budget goes to the
    // widget separator and widget content.
    let widget_status_rows = layout.status_rows.max(1);
    let widget_data_rows = widget_status_rows.saturating_sub(1);

    // Calculate column widths based on available height.
    // In compact mode (short terminal), stats gets more width for two-column layout.
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
    let remaining = layout
        .total_cols
        .saturating_sub(stats_width + num_separators as u16);

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

    // Draw content rows (after reserved handoff space + separator).
    let widget_pty_rows = layout.pty_rows + HANDOFF_RESERVED_ROWS;
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
            pairing_state.pairing_code.as_deref(),
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

    // The Pair code lives in the stats header and the recap toast/hint live
    // in the handoff strip above the PTY, so there's no longer a bottom
    // footer row to render here.

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

// Pair URL and recap messages now render in the stats header and handoff
// strip respectively, so no functions are needed for footer-row sizing —
// `desired_footer_rows` always returns 0.

#[cfg(test)]
mod tests {
    use super::*;
    use crate::git::GitState;
    use crate::parsers::DiffSummary;
    use crate::recap::RecapStatus;

    fn pairing_state() -> PairingState {
        PairingState {
            pairing_code: Some("ABC-DEF-GHI".to_string()),
            ..PairingState::default()
        }
    }

    #[test]
    fn dynamic_status_rows_shrinks_when_widgets_have_minimal_content() {
        // Stats always wants 7 rows (header + 6 body rows). Git/changes are
        // empty defaults, so 7 + 1 separator = 8 status rows — comfortably
        // under the 20% ceiling for a 60-row terminal.
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();
        let pairing = PairingState::default();
        let recap = RecapState::default();

        let rows = compute_dynamic_status_rows(
            60, 200, &stats, &git, &diff, None, &pairing, &recap, false,
        );
        assert_eq!(rows, 8);
    }

    #[test]
    fn dynamic_status_rows_caps_at_preferred_max() {
        // 200 git files want many rows. Even with a multi-column estimate we
        // exceed the 20% ceiling on a 60-row terminal. Must not go past it.
        let stats = SessionStats::default();
        let mut git = GitState::default();
        git.files = (0..200)
            .map(|i| crate::git::FileStatus {
                status: "M".to_string(),
                path: format!("file_{i}.rs"),
                additions: 1,
                deletions: 0,
                is_folder: false,
                file_count: 0,
            })
            .collect();
        let diff = DiffSummary::default();
        let pairing = PairingState::default();
        let recap = RecapState::default();

        let rows = compute_dynamic_status_rows(
            60, 120, &stats, &git, &diff, None, &pairing, &recap, false,
        );
        assert_eq!(rows, preferred_status_rows_max(60));
    }

    #[test]
    fn dynamic_status_rows_reserves_room_for_pair_footer() {
        // Pair code now lives in the stats header, not the footer — so an
        // active pairing code on its own no longer claims any footer rows.
        // Stats natural 7 + 1 separator = 8 status rows.
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();
        let recap = RecapState::default();

        let rows = compute_dynamic_status_rows(
            80,
            200,
            &stats,
            &git,
            &diff,
            None,
            &pairing_state(),
            &recap,
            false,
        );
        assert_eq!(rows, 8);
    }

    #[test]
    fn dynamic_status_rows_floors_at_min_status_rows() {
        // If the terminal is so short that the preferred max equals
        // MIN_STATUS_ROWS, the dynamic value can't drop below it.
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();
        let pairing = PairingState::default();
        let recap = RecapState::default();

        let rows = compute_dynamic_status_rows(
            15, 100, &stats, &git, &diff, None, &pairing, &recap, false,
        );
        assert!(rows >= MIN_STATUS_ROWS);
        assert!(rows <= preferred_status_rows_max(15));
    }

    #[test]
    fn dynamic_status_rows_collapses_when_changes_pack_into_few_rows() {
        // 18 Rust changes pack into ~6 rows on a wide terminal — far less than
        // the 20-row total a one-per-row layout would need. The widget area
        // should size to the packed estimate, leaving more room for the CLI.
        use crate::parsers::{ChangeNode, ChangeType, FileChanges, NodeKind};

        let stats = SessionStats::default();
        let git = GitState::default();

        let changes: Vec<ChangeNode> = (0..18)
            .map(|i| ChangeNode {
                kind: NodeKind::Function,
                name: format!("symbol_{i}"),
                additions: 5,
                deletions: 1,
                change_type: ChangeType::Added,
                file_path: Some(format!("src/file_{i}.rs")),
                line_number: None,
                children: Vec::new(),
            })
            .collect();
        let diff = DiffSummary {
            files: vec![FileChanges {
                path: "src/lib.rs".to_string(),
                language: "Rust".to_string(),
                changes,
            }],
            loading: false,
        };
        let pairing = PairingState::default();
        let recap = RecapState::default();

        // Wide terminal (250 cols): preferred ceiling is 20, but the natural
        // packed layout for stats(7) and changes(~7) plus separator should
        // come in well under that — typical of the user's bottom-screenshot
        // resize complaint.
        let rows = compute_dynamic_status_rows(
            100, 250, &stats, &git, &diff, None, &pairing, &recap, false,
        );
        assert!(
            rows < 12,
            "expected packed layout to keep status_rows under 12, got {}",
            rows
        );
        assert!(rows >= MIN_STATUS_ROWS);
    }

    #[test]
    fn split_terminal_rows_keeps_four_widget_rows_when_possible() {
        let (pty_rows, status_rows) = split_terminal_rows(20);

        assert_eq!(status_rows, MIN_STATUS_ROWS);
        assert_eq!(pty_rows + HANDOFF_RESERVED_ROWS + status_rows, 20);
    }

    #[test]
    fn split_terminal_rows_uses_preferred_status_height_when_larger() {
        let (pty_rows, status_rows) = split_terminal_rows(44);

        assert_eq!(status_rows, 8);
        assert_eq!(pty_rows + HANDOFF_RESERVED_ROWS + status_rows, 44);
    }

    #[test]
    fn split_terminal_rows_shrinks_status_area_only_when_terminal_is_too_short() {
        let (pty_rows, status_rows) = split_terminal_rows(8);

        assert_eq!(pty_rows, 1);
        assert_eq!(status_rows, 4);
    }

    fn visible_width(s: &str) -> usize {
        crate::ui::utils::strip_ansi_len(s)
    }

    #[test]
    fn footer_helper_always_returns_zero() {
        // The bottom-edge footer is gone — pair code lives in the stats
        // header and recap messages live in the handoff strip — so the
        // footer-row helper should never claim a row regardless of inputs.
        let recap_missing = RecapState {
            enabled: true,
            status: RecapStatus::MissingKey,
            ..RecapState::default()
        };
        let recap_waiting = RecapState {
            enabled: true,
            status: RecapStatus::Waiting,
            ..RecapState::default()
        };
        assert_eq!(
            desired_footer_rows(&pairing_state(), &recap_missing, false),
            0
        );
        assert_eq!(
            desired_footer_rows(&PairingState::default(), &recap_waiting, true),
            0
        );
        assert_eq!(
            desired_footer_rows(&PairingState::default(), &RecapState::default(), false),
            0
        );
    }
}
