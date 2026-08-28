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
use crate::pr::SessionPr;
use crate::recap::RecapState;
use crate::slack::SlackThread;
use crate::terminal::escape::{self, color, RESET};
use crate::title::session_title_hierarchy;
use crate::update::UpdateState;

use super::cooldown::{self, Cooldowns};
use super::{
    changes_natural_rows, draw_changes_widget, draw_git_widget, draw_pairing_banner,
    draw_pr_handoff, draw_pr_separator, draw_recap_handoff, draw_stats_widget, draw_update_banner,
    git_natural_rows, pr_handoff_rows, pr_separator_rows, recap_handoff_rows, stats_render_rows,
    stats_use_compact_layout, total_handoff_rows, PairingState, WidgetArea, MAX_RECAP_ROWS,
};

/// Layout information needed for rendering widgets
pub struct Layout {
    pub pty_rows: u16,
    pub total_cols: u16,
    pub status_rows: u16,
    pub handoff_rows: u16,
}

/// Minimum content rows reserved for widget data below the separator.
pub const MIN_WIDGET_DATA_ROWS: u16 = 4;
/// Status rows include the separator plus widget data rows.
pub const MIN_STATUS_ROWS: u16 = MIN_WIDGET_DATA_ROWS + 1;

fn stats_widget_width(total_cols: u16, compact: bool) -> u16 {
    if compact {
        ((total_cols as f32) * 0.35).max(36.0) as u16
    } else {
        ((total_cols as f32) * 0.22).max(24.0) as u16
    }
}

/// Split terminal height into assistant PTY rows and status widget rows.
///
/// `handoff_rows` is the (possibly PR-expanded) handoff height reserved between
/// the two regions. Status rows include the widget separator, so MIN_STATUS_ROWS
/// preserves four rows for widget content.
pub fn split_terminal_rows(total_rows: u16, handoff_rows: u16) -> (u16, u16) {
    let status_rows = preferred_status_rows_max(total_rows, handoff_rows);
    let pty_rows = total_rows.saturating_sub(status_rows + handoff_rows).max(1);

    (pty_rows, status_rows)
}

/// Upper bound on status rows for the current terminal height — the historical
/// "20% of screen, never less than MIN_STATUS_ROWS, never more than fits."
pub fn preferred_status_rows_max(total_rows: u16, handoff_rows: u16) -> u16 {
    let preferred_status_rows = ((total_rows as f32 * 0.2) as u16).max(MIN_STATUS_ROWS);
    let max_status_rows = total_rows.saturating_sub(handoff_rows + 1).max(1);
    preferred_status_rows.min(max_status_rows)
}

/// Exact height needed by update, recap, and pairing content above the PR list.
fn banner_handoff_rows(
    width: u16,
    pairing_state: &PairingState,
    update_state: &UpdateState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
) -> u16 {
    let update_rows = update_state.banner_rows();
    let remaining = MAX_RECAP_ROWS.saturating_sub(update_rows);
    let recap_rows = recap_handoff_rows(width, recap_state, remaining, recap_toast_visible);
    let pairing_rows = if recap_rows == 0 {
        if update_rows > 0 {
            pairing_state.banner_rows_compact()
        } else {
            pairing_state.banner_rows()
        }
        .min(remaining)
    } else {
        0
    };

    update_rows + recap_rows.max(pairing_rows)
}

/// Exact height reserved between the assistant PTY and widget separator.
pub fn handoff_rows(
    width: u16,
    pairing_state: &PairingState,
    update_state: &UpdateState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
    prs: &[SessionPr],
) -> u16 {
    total_handoff_rows(
        banner_handoff_rows(
            width,
            pairing_state,
            update_state,
            recap_state,
            recap_toast_visible,
        ),
        prs,
    )
}

/// Estimate the column widths the renderer will assign to git and changes,
/// using the same proportions as `draw_status_bar`. Returned as
/// `(git_width, changes_width)`. The estimate ignores the multi-column flex
/// path because that decision depends on how many rows we ultimately give git
/// — a rough split is enough for the natural-row heuristics.
fn estimate_column_widths(total_cols: u16, compact_stats: bool) -> (u16, u16) {
    let stats_width = stats_widget_width(total_cols, compact_stats);
    let separators: u16 = 2;
    let remaining = total_cols.saturating_sub(stats_width + separators);
    let git_w = (remaining * 3) / 8;
    let changes_w = remaining.saturating_sub(git_w);
    (git_w, changes_w)
}

/// Compute the status row count needed to show every widget's natural content
/// (plus the separator), clamped between MIN_STATUS_ROWS and the historical
/// 20% ceiling. The result lets the widget area shrink when content is shorter
/// than the cap — including when git or changes use packed layouts that consume
/// far fewer rows than there are items.
#[allow(clippy::too_many_arguments)]
pub fn compute_dynamic_status_rows(
    total_rows: u16,
    total_cols: u16,
    session_stats: &SessionStats,
    git_state: &GitState,
    diff_summary: &DiffSummary,
    terminal_title: Option<&str>,
    prs: &[SessionPr],
    slack_threads: &[SlackThread],
    handoff_rows: u16,
) -> u16 {
    let preferred_max = preferred_status_rows_max(total_rows, handoff_rows);
    let available_rows = preferred_max.saturating_sub(1);
    let compact_stats = stats_use_compact_layout(available_rows);
    let (git_w, changes_w) = estimate_column_widths(total_cols, compact_stats);
    let titles = session_title_hierarchy(prs, terminal_title);
    let natural = stats_render_rows(available_rows, session_stats)
        .max(git_natural_rows(git_state, git_w))
        .max(changes_natural_rows(
            diff_summary,
            changes_w,
            titles.row_count(),
            slack_threads.len(),
        ));
    // A short terminal can leave fewer rows than MIN_STATUS_ROWS; keep the
    // lower bound at or below the upper bound so `clamp` never panics.
    let desired = natural.saturating_add(1); // +1 separator
    desired.clamp(MIN_STATUS_ROWS.min(preferred_max), preferred_max)
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
    slack_threads: &[SlackThread],
    ide: IdeKind,
    cwd: &Path,
    cloud_status: Option<&CloudStatus>,
    pairing_state: &PairingState,
    update_state: &UpdateState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
    prs: &[SessionPr],
    pr_scope: &str,
    cursor_position: Option<(u16, u16)>, // (row, col) from vt100 parser, 0-indexed
    cooldowns: &Cooldowns,
    now_ms: u64,
) -> Result<()> {
    // Begin synchronized update - terminal batches all our drawing
    // so cursor movements don't interfere with Claude's incremental updates
    write!(stdout, "{}", escape::SYNC_BEGIN)?;

    // Clear the entire status bar area first to prevent artifacts
    // This is critical: without clearing, resizes or partial redraws leave old content
    write!(stdout, "{}", escape::cursor_to(layout.pty_rows + 1, 1))?;
    write!(stdout, "{}", escape::CLEAR_TO_END)?;

    // The handoff reserves only the rows its visible content actually needs.
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

    let banner_rows_reserved = layout
        .handoff_rows
        .saturating_sub(pr_separator_rows(prs))
        .saturating_sub(pr_handoff_rows(prs));
    let handoff_limit = layout.pty_rows + banner_rows_reserved;
    let remaining_rows = banner_rows_reserved.saturating_sub(update_banner_rows);
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

    // PR list occupies the handoff rows that grew below the recap region. The
    // layout already reserved these rows (pty_rows accounts for total_handoff),
    // so they sit directly above the widget separator.
    let pr_rows = pr_handoff_rows(prs);
    if pr_rows > 0 {
        let divider_row = layout.pty_rows + 1 + banner_rows_reserved;
        draw_pr_separator(stdout, divider_row, layout.total_cols)?;
        let pr_start = divider_row + pr_separator_rows(prs);
        draw_pr_handoff(
            stdout,
            pr_start,
            layout.total_cols,
            prs,
            pr_scope,
            pr_rows,
            cooldowns,
            now_ms,
        )?;
    }

    // Draw thick separator line (always after the reserved handoff space)
    let separator_row = layout.pty_rows + 1 + layout.handoff_rows;
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

    // Widen stats as soon as its normal list would be clipped. The compact
    // renderer uses the extra width to fit two complete metrics on each row.
    let compact = stats_use_compact_layout(widget_data_rows);
    let stats_width = stats_widget_width(layout.total_cols, compact);

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
    let titles = session_title_hierarchy(prs, terminal_title);
    let state_tint = cooldowns.tint(cooldown::SESSION_STATE_KEY, now_ms);

    // Draw content rows (after reserved handoff space + separator).
    let widget_pty_rows = layout.pty_rows + layout.handoff_rows;
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
            state_tint,
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
            titles,
            slack_threads,
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

    fn pairing_state() -> PairingState {
        PairingState {
            pairing_code: Some("ABC-DEF-GHI".to_string()),
            ..PairingState::default()
        }
    }

    #[test]
    fn compact_stats_get_more_width() {
        assert_eq!(stats_widget_width(200, false), 44);
        assert_eq!(stats_widget_width(200, true), 70);
    }

    #[test]
    fn short_status_area_uses_compact_stats_height() {
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();

        let rows = compute_dynamic_status_rows(35, 180, &stats, &git, &diff, None, &[], &[], 0);

        assert_eq!(rows, MIN_STATUS_ROWS);
    }

    #[test]
    fn dynamic_status_rows_shrinks_when_widgets_have_minimal_content() {
        // Stats always wants 7 rows (header + 6 body rows). Git/changes are
        // empty defaults, so 7 + 1 separator = 8 status rows — comfortably
        // under the 20% ceiling for a 60-row terminal.
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();
        let rows = compute_dynamic_status_rows(60, 200, &stats, &git, &diff, None, &[], &[], 0);
        assert_eq!(rows, 8);
    }

    #[test]
    fn dynamic_status_rows_survives_short_terminal() {
        // An 8-row terminal with a 3-row handoff strip leaves only 4 rows for
        // status, below MIN_STATUS_ROWS. This used to panic inside `clamp`.
        let rows = compute_dynamic_status_rows(
            8,
            120,
            &SessionStats::default(),
            &GitState::default(),
            &DiffSummary::default(),
            None,
            &[],
            &[],
            3,
        );
        assert_eq!(rows, preferred_status_rows_max(8, 3));
        assert!(rows < MIN_STATUS_ROWS);
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
        let rows = compute_dynamic_status_rows(
            60,
            120,
            &stats,
            &git,
            &diff,
            None,
            &[],
            &[],
            MAX_RECAP_ROWS,
        );
        assert_eq!(rows, preferred_status_rows_max(60, MAX_RECAP_ROWS));
    }

    #[test]
    fn dynamic_status_rows_reserves_room_for_pair_footer() {
        // Pair code now lives in the stats header, not the footer — so an
        // active pairing code on its own no longer claims any footer rows.
        // Stats natural 7 + 1 separator = 8 status rows.
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();
        let pairing = pairing_state();
        let handoff = handoff_rows(
            200,
            &pairing,
            &UpdateState::default(),
            &RecapState::default(),
            false,
            &[],
        );
        let rows =
            compute_dynamic_status_rows(80, 200, &stats, &git, &diff, None, &[], &[], handoff);
        assert_eq!(rows, 8);
    }

    #[test]
    fn dynamic_status_rows_floors_at_min_status_rows() {
        // If the terminal is so short that the preferred max equals
        // MIN_STATUS_ROWS, the dynamic value can't drop below it.
        let stats = SessionStats::default();
        let git = GitState::default();
        let diff = DiffSummary::default();
        let rows = compute_dynamic_status_rows(
            15,
            100,
            &stats,
            &git,
            &diff,
            None,
            &[],
            &[],
            MAX_RECAP_ROWS,
        );
        assert!(rows >= MIN_STATUS_ROWS);
        assert!(rows <= preferred_status_rows_max(15, MAX_RECAP_ROWS));
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
                scope: Vec::new(),
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
        // Wide terminal (250 cols): preferred ceiling is 20, but the natural
        // packed layout for stats(7) and changes(~7) plus separator should
        // come in well under that — typical of the user's bottom-screenshot
        // resize complaint.
        let rows = compute_dynamic_status_rows(100, 250, &stats, &git, &diff, None, &[], &[], 0);
        assert!(
            rows < 12,
            "expected packed layout to keep status_rows under 12, got {}",
            rows
        );
        assert!(rows >= MIN_STATUS_ROWS);
    }

    #[test]
    fn split_terminal_rows_keeps_four_widget_rows_when_possible() {
        let (pty_rows, status_rows) = split_terminal_rows(20, MAX_RECAP_ROWS);

        assert_eq!(status_rows, MIN_STATUS_ROWS);
        assert_eq!(pty_rows + MAX_RECAP_ROWS + status_rows, 20);
    }

    #[test]
    fn split_terminal_rows_uses_preferred_status_height_when_larger() {
        let (pty_rows, status_rows) = split_terminal_rows(44, MAX_RECAP_ROWS);

        assert_eq!(status_rows, 8);
        assert_eq!(pty_rows + MAX_RECAP_ROWS + status_rows, 44);
    }

    #[test]
    fn split_terminal_rows_reclaims_unused_recap_rows() {
        let (with_recap, _) = split_terminal_rows(44, MAX_RECAP_ROWS);
        let (without_recap, _) = split_terminal_rows(44, 0);

        assert_eq!(without_recap - with_recap, MAX_RECAP_ROWS);
    }

    #[test]
    fn split_terminal_rows_shrinks_status_area_only_when_terminal_is_too_short() {
        let (pty_rows, status_rows) = split_terminal_rows(8, MAX_RECAP_ROWS);

        assert_eq!(pty_rows, 1);
        assert_eq!(status_rows, 4);
    }

    #[test]
    fn handoff_rows_reserve_only_visible_content() {
        let update = UpdateState::default();
        let recap = RecapState::default();
        assert_eq!(
            handoff_rows(200, &PairingState::default(), &update, &recap, false, &[]),
            0
        );
        assert_eq!(
            handoff_rows(200, &pairing_state(), &update, &recap, false, &[]),
            2
        );
    }
}
