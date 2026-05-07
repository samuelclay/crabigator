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
use crate::recap::{RecapState, RecapStatus};
use crate::terminal::escape::{self, color, RESET};
use crate::update::UpdateState;

use super::{
    draw_changes_widget, draw_git_widget, draw_pairing_banner, draw_recap_handoff,
    draw_stats_widget, draw_update_banner, PairingState, WidgetArea, HANDOFF_RESERVED_ROWS,
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
    let preferred_status_rows = ((total_rows as f32 * 0.2) as u16).max(MIN_STATUS_ROWS);
    let max_status_rows = total_rows.saturating_sub(HANDOFF_RESERVED_ROWS + 1).max(1);
    let status_rows = preferred_status_rows.min(max_status_rows);
    let pty_rows = total_rows
        .saturating_sub(status_rows + HANDOFF_RESERVED_ROWS)
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
    let footer_rows = pairing_footer_rows(
        layout.status_rows,
        pairing_state,
        recap_state,
        recap_toast_visible,
    );
    let widget_status_rows = layout.status_rows.saturating_sub(footer_rows).max(1);
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

    // Draw full-width footer row: Pair URL on the left (when unpaired) and
    // an optional recap message (hint or startup toast) on the right.
    if footer_rows > 0 {
        if footer_rows == 2 {
            // Separator line above the footer.
            let sep_row = layout.pty_rows + HANDOFF_RESERVED_ROWS + widget_status_rows + 1;
            write!(stdout, "{}", escape::cursor_to(sep_row, 1))?;
            let line = "─".repeat(layout.total_cols as usize);
            write!(stdout, "{}{}{}", escape::fg(color::DARK_GRAY), line, RESET)?;
        }

        let footer_row = layout.pty_rows + HANDOFF_RESERVED_ROWS + layout.status_rows;
        // Start at column 2 so "Pair: " has the same 1-col edge margin as
        // the OSC'd terminal title gets on the changes-widget header.
        write!(stdout, "{}", escape::cursor_to(footer_row, 2))?;

        // Left side: Pair URL hyperlink if we still need to pair.
        // Account for the 1-col left margin (cursor starts at column 2).
        let mut left_visible_width = 1usize;
        if let Some(code) = pairing_state.pairing_code.as_deref() {
            let url = format!("https://drinkcrabigator.com/dashboard?setup={}", code);
            let url_display = format!("drinkcrabigator.com/dashboard?setup={}", code);
            left_visible_width += "Pair: ".chars().count() + url_display.chars().count();
            let label = format!("{}Pair: {}", escape::fg(color::DARK_GRAY), RESET);
            let display = format!(
                "{}{}{}{}",
                label,
                escape::fg(color::DARK_GRAY),
                url_display,
                RESET
            );
            write!(stdout, "\x1b]8;;{}\x07{}\x1b]8;;\x07", url, display)?;
        }

        // Reserve a 1-col right margin so the recap message doesn't sit
        // flush against the terminal edge.
        let usable_cols = (layout.total_cols as usize).saturating_sub(1);

        // Right side: pick the appropriate recap message.
        // - MissingKey  → persistent "Recaps off" hint with both commands.
        // - Toast       → transient "Recaps enabled" confirmation.
        let recap_message = if matches!(recap_state.status, RecapStatus::MissingKey) {
            let available = usable_cols.saturating_sub(left_visible_width);
            build_recap_hint(available)
        } else if recap_toast_visible {
            let available = usable_cols.saturating_sub(left_visible_width);
            build_recap_toast(available)
        } else {
            None
        };
        if let Some((padding, formatted)) = recap_message {
            // Indent slightly when the line would otherwise start with the
            // recap message — keeps it from hugging the left edge.
            let leading = if left_visible_width == 1 { 2 } else { padding };
            write!(stdout, "{}{}", " ".repeat(leading), formatted)?;
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

/// Build the "recaps are off — here's how" hint shown to the right of the
/// Pair URL when recaps are enabled but no Anthropic API key is configured.
///
/// Three tiers degrade gracefully as the terminal narrows:
/// - **Verbose** (≥78 cols): explains what recaps are and how to opt in/out.
/// - **Full**   (≥64 cols): drops the "Per-turn AI" preface but keeps both commands.
/// - **Compact** (≥47 cols): just the two commands, no state label.
///
/// Returns `(left_padding_cols, formatted_text)` for the chosen tier, or
/// `None` if the available width can't fit even the compact tier.
fn build_recap_hint(available_cols: usize) -> Option<(usize, String)> {
    // Visible-column counts for each rendered tier (no ANSI escapes counted).
    const VERBOSE_VISIBLE: usize = 74; // "✦ Per-turn AI recaps off — crabigator key <key> · crabigator recap disable"
    const FULL_VISIBLE: usize = 62; // "✦ Recaps off — crabigator key <key> · crabigator recap disable"
    const COMPACT_VISIBLE: usize = 45; // "✦ crabigator key  ·  crabigator recap disable"
    const VERBOSE_GAP: usize = 4;
    const FULL_GAP: usize = 2;
    const COMPACT_GAP: usize = 2;

    let sparkle = format!("{}✦{}", escape::fg(color::YELLOW), RESET);
    let dash = format!("{}—{}", escape::fg(color::DARK_GRAY), RESET);
    let dot = format!("{}·{}", escape::fg(color::DARK_GRAY), RESET);
    let key_cmd = format!("{}crabigator key{}", escape::fg(color::CYAN), RESET);
    let placeholder = format!("{}<key>{}", escape::fg(color::GRAY), RESET);
    let disable_cmd = format!(
        "{}crabigator recap disable{}",
        escape::fg(color::DARK_GRAY),
        RESET
    );

    if available_cols >= VERBOSE_VISIBLE + VERBOSE_GAP {
        let label = format!("{}Per-turn AI recaps off{}", escape::fg(color::GRAY), RESET);
        let formatted = format!(
            "{} {} {} {} {} {} {}",
            sparkle, label, dash, key_cmd, placeholder, dot, disable_cmd
        );
        Some((available_cols - VERBOSE_VISIBLE, formatted))
    } else if available_cols >= FULL_VISIBLE + FULL_GAP {
        let label = format!("{}Recaps off{}", escape::fg(color::GRAY), RESET);
        let formatted = format!(
            "{} {} {} {} {} {} {}",
            sparkle, label, dash, key_cmd, placeholder, dot, disable_cmd
        );
        Some((available_cols - FULL_VISIBLE, formatted))
    } else if available_cols >= COMPACT_VISIBLE + COMPACT_GAP {
        let formatted = format!("{} {}  {}  {}", sparkle, key_cmd, dot, disable_cmd);
        Some((available_cols - COMPACT_VISIBLE, formatted))
    } else {
        None
    }
}

/// Build the transient "Recaps enabled" toast shown for the first ten seconds
/// of a session when recaps are armed with a usable Anthropic API key.
///
/// Two tiers:
/// - **Full**    (≥30 cols): "✓ Per-turn AI recaps enabled".
/// - **Compact** (≥18 cols): "✓ Recaps enabled".
fn build_recap_toast(available_cols: usize) -> Option<(usize, String)> {
    const FULL_VISIBLE: usize = 28;
    const COMPACT_VISIBLE: usize = 16;
    const FULL_GAP: usize = 2;
    const COMPACT_GAP: usize = 2;

    let check = format!("{}✓{}", escape::fg(color::GREEN), RESET);

    if available_cols >= FULL_VISIBLE + FULL_GAP {
        let label = format!(
            "{}Per-turn AI recaps enabled{}",
            escape::fg(color::GRAY),
            RESET
        );
        let formatted = format!("{} {}", check, label);
        Some((available_cols - FULL_VISIBLE, formatted))
    } else if available_cols >= COMPACT_VISIBLE + COMPACT_GAP {
        let label = format!("{}Recaps enabled{}", escape::fg(color::GRAY), RESET);
        let formatted = format!("{} {}", check, label);
        Some((available_cols - COMPACT_VISIBLE, formatted))
    } else {
        None
    }
}

fn pairing_footer_rows(
    status_rows: u16,
    pairing_state: &PairingState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
) -> u16 {
    if recap_state.prefers_handoff() {
        return 0;
    }

    let needs_pair = pairing_state.pairing_code.is_some();
    let needs_recap_message = matches!(recap_state.status, RecapStatus::MissingKey)
        || (recap_toast_visible
            && matches!(
                recap_state.status,
                RecapStatus::Waiting | RecapStatus::Updating | RecapStatus::Ready
            ));

    if !needs_pair && !needs_recap_message {
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
    fn recap_hint_hides_when_too_narrow() {
        // Compact tier needs 45 visible cols + 2 gap = 47. Anything below hides.
        assert!(build_recap_hint(46).is_none());
        assert!(build_recap_hint(0).is_none());
    }

    #[test]
    fn recap_hint_tiers_grow_with_available_width() {
        // Compact tier (45 + 2 gap)
        let (padding, formatted) = build_recap_hint(47).expect("compact tier should render");
        assert_eq!(padding, 2);
        assert_eq!(visible_width(&formatted), 45);

        // Full tier (62 + 2 gap)
        let (padding, formatted) = build_recap_hint(64).expect("full tier should render");
        assert_eq!(padding, 2);
        assert_eq!(visible_width(&formatted), 62);

        // Verbose tier (74 + 4 gap)
        let (padding, formatted) = build_recap_hint(78).expect("verbose tier should render");
        assert_eq!(padding, 4);
        assert_eq!(visible_width(&formatted), 74);

        // Wider terminal grows the left padding (right-aligned hint).
        let (padding, formatted) = build_recap_hint(120).expect("verbose tier should render");
        assert_eq!(padding, 46);
        assert_eq!(visible_width(&formatted), 74);
    }

    #[test]
    fn recap_toast_tiers_grow_with_available_width() {
        // Below the compact threshold the toast is hidden.
        assert!(build_recap_toast(17).is_none());

        // Compact tier (16 + 2 gap)
        let (padding, formatted) = build_recap_toast(18).expect("compact tier should render");
        assert_eq!(padding, 2);
        assert_eq!(visible_width(&formatted), 16);

        // Full tier (28 + 2 gap)
        let (padding, formatted) = build_recap_toast(30).expect("full tier should render");
        assert_eq!(padding, 2);
        assert_eq!(visible_width(&formatted), 28);
    }

    #[test]
    fn pairing_footer_uses_only_surplus_widget_rows() {
        let pairing = pairing_state();
        let recap = RecapState::default();

        assert_eq!(
            pairing_footer_rows(MIN_STATUS_ROWS, &pairing, &recap, false),
            0
        );
        assert_eq!(
            pairing_footer_rows(MIN_STATUS_ROWS + 1, &pairing, &recap, false),
            1
        );
        assert_eq!(
            pairing_footer_rows(MIN_STATUS_ROWS + 3, &pairing, &recap, false),
            2
        );
    }

    #[test]
    fn footer_row_allocated_for_recap_message_without_pairing() {
        // Already paired (no pairing code) but recap toast wants to show.
        let pairing = PairingState::default();
        let recap = RecapState {
            enabled: true,
            status: RecapStatus::Waiting,
            ..RecapState::default()
        };
        assert_eq!(
            pairing_footer_rows(MIN_STATUS_ROWS + 1, &pairing, &recap, true),
            1
        );

        // Same state but the toast already faded → no footer needed.
        assert_eq!(
            pairing_footer_rows(MIN_STATUS_ROWS + 1, &pairing, &recap, false),
            0
        );

        // MissingKey hint also justifies the footer on its own.
        let recap_missing = RecapState {
            enabled: true,
            status: RecapStatus::MissingKey,
            ..RecapState::default()
        };
        assert_eq!(
            pairing_footer_rows(MIN_STATUS_ROWS + 1, &pairing, &recap_missing, false),
            1
        );
    }
}
