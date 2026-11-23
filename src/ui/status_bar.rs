//! Status bar rendering
//!
//! Coordinates the layout and rendering of all status bar widgets.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::io::Write;
use std::path::Path;

use anyhow::Result;

use crate::cloud::CloudStatus;
use crate::flow::Cells;
use crate::git::GitState;
use crate::hooks::SessionStats;
use crate::ide::IdeKind;
use crate::parsers::DiffSummary;
use crate::pr::SessionPr;
use crate::recap::RecapState;
use crate::session_mark::SessionMark;
use crate::slack::SlackThread;
use crate::terminal::escape::{self, color, RESET};
use crate::title::session_title_hierarchy;
use crate::update::UpdateState;

use super::cooldown::{self, Cooldowns};
use super::flow::{
    draw_flow_cells_diff, draw_flow_column_diff, draw_flow_label, format_flow_row_text,
    write_flow_cells_diff, FlowRect, FlowView,
};
use super::{
    changes_natural_rows, draw_pairing_banner, draw_pr_handoff, draw_pr_separator,
    draw_recap_handoff, draw_update_banner, git_natural_rows, pr_handoff_rows, pr_separator_rows,
    recap_handoff_rows, render_changes_rows, render_git_rows, render_stats_rows, stats_render_rows,
    stats_use_compact_layout, total_handoff_rows, PairingState, MAX_RECAP_ROWS,
};

/// Layout information needed for rendering widgets
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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
        // The identity chip sits left of "Session", so the column needs a
        // few more cells than the old bullet-only label.
        ((total_cols as f32) * 0.22).max(30.0) as u16
    }
}

/// The flow column takes this share of the terminal's width (percent)...
const FLOW_SHARE_PERCENT: u32 = 15;
/// ...from this many terminal columns. Narrower, git would get too few cells.
pub const FLOW_MIN_TOTAL_COLS: u16 = 100;

/// The flow column's usual width, or `None` when the terminal is too narrow for it.
fn usual_flow_width(total_cols: u16) -> Option<u16> {
    (total_cols >= FLOW_MIN_TOTAL_COLS)
        .then(|| (u32::from(total_cols) * FLOW_SHARE_PERCENT / 100) as u16)
}

/// The flow column's width with `status_rows` of widgets, or `None` when the
/// terminal is too narrow for it. A column 9 to 12 rows tall that its usual
/// width would keep horizontal narrows to just under twice its height, a
/// portrait column where flow's scenes turn vertical (see `is_tall`).
pub fn flow_column_width(total_cols: u16, status_rows: u16) -> Option<u16> {
    let usual = usual_flow_width(total_cols)?;
    let rows = status_rows.saturating_sub(1); // less the separator
    let tall = |width: u16| crate::flow::is_tall(usize::from(width), usize::from(rows));
    Some(if rows > 8 && !tall(usual) {
        usual.min(rows * 2 - 1)
    } else {
        usual
    })
}

/// Stats' width beside a flow column: its usual share, shrunk in proportion
/// to the room flow takes (22% × 0.85, or 35% × 0.85 compact), but never
/// below its usual minimum, since stats matters most.
fn stats_widget_width_with_flow(total_cols: u16, compact: bool) -> u16 {
    let t = u32::from(total_cols);
    if compact {
        (t * 2975 / 10_000).max(36) as u16
    } else {
        (t * 1870 / 10_000).max(30) as u16
    }
}

/// How wide each widget column is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColumnWidths {
    pub stats: u16,
    pub git: u16,
    pub changes: u16,
    /// The flow column, when it shows.
    pub flow: Option<u16>,
}

/// Split the terminal's width between the widgets: Stats │ Git │ Changes,
/// and │ Flow when `flow` is on and the terminal is wide enough. Without
/// flow the widths are exactly the three-column ones. Git and changes split
/// what stats (and flow) leave 3:5, or 4:4 when git needs several columns.
pub fn column_widths(
    total_cols: u16,
    status_rows: u16,
    compact_stats: bool,
    git_multi_column: bool,
    flow: bool,
) -> ColumnWidths {
    let flow_width = if flow {
        flow_column_width(total_cols, status_rows)
    } else {
        None
    };
    let (stats, separators) = match flow_width {
        Some(_) => (stats_widget_width_with_flow(total_cols, compact_stats), 3),
        None => (stats_widget_width(total_cols, compact_stats), 2),
    };
    let remaining = total_cols.saturating_sub(stats + separators + flow_width.unwrap_or(0));
    let git = if git_multi_column {
        remaining / 2
    } else {
        (remaining * 3) / 8
    };
    ColumnWidths {
        stats,
        git,
        changes: remaining - git,
        flow: flow_width,
    }
}

/// Where the flow column sits for this layout, or `None` when it has no room.
/// Its left edge is always `total_cols - width`: the other columns and the
/// separators fill exactly what flow leaves.
pub fn flow_column_rect(layout: &Layout) -> Option<FlowRect> {
    let rows = layout.status_rows.max(1).saturating_sub(1);
    let width = flow_column_width(layout.total_cols, layout.status_rows)?;
    (rows > 0).then_some(FlowRect {
        widget_pty_rows: layout.pty_rows + layout.handoff_rows,
        col: layout.total_cols - width,
        width,
        rows,
    })
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

/// Status rows that make the flow column tall enough for flow's vertical
/// scenes (see `is_tall`) within the 20% ceiling, or `None` when even a
/// narrowed column can't be. The shortest that is tall at the column's usual
/// width, or else the tallest that fits, where the column narrows to portrait.
fn tall_flow_status_rows(total_cols: u16, preferred_max: u16) -> Option<u16> {
    let width = usize::from(usual_flow_width(total_cols)?);
    // The column's rows are the status rows less the separator.
    let usual = (1..preferred_max).find(|&rows| crate::flow::is_tall(width, usize::from(rows)));
    match usual {
        Some(rows) => Some(rows + 1),
        None => (preferred_max.saturating_sub(1) > 8).then_some(preferred_max),
    }
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
fn estimate_column_widths(total_cols: u16, compact_stats: bool, flow: bool) -> (u16, u16) {
    // Flow at its usual width: the narrowest git and changes can be.
    let widths = column_widths(total_cols, 0, compact_stats, false, flow);
    (widths.git, widths.changes)
}

/// Compute the status row count needed to show every widget's natural content
/// (plus the separator), clamped between MIN_STATUS_ROWS and the historical
/// 20% ceiling. The result lets the widget area shrink when content is shorter
/// than the cap — including when git or changes use packed layouts that consume
/// far fewer rows than there are items. With the flow column showing, it grows
/// to give the column a tall grid, where flow's scenes turn vertical, as long
/// as that fits under the ceiling.
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
    flow: bool,
) -> u16 {
    let preferred_max = preferred_status_rows_max(total_rows, handoff_rows);
    let available_rows = preferred_max.saturating_sub(1);
    let compact_stats = stats_use_compact_layout(available_rows);
    let (git_w, changes_w) = estimate_column_widths(total_cols, compact_stats, flow);
    let titles = session_title_hierarchy(prs, terminal_title);
    let natural = stats_render_rows(available_rows, session_stats)
        .max(git_natural_rows(git_state, git_w))
        .max(changes_natural_rows(
            diff_summary,
            changes_w,
            titles.row_count(),
            slack_threads,
        ));
    // A short terminal can leave fewer rows than MIN_STATUS_ROWS; keep the
    // lower bound at or below the upper bound so `clamp` never panics.
    let mut desired = natural.saturating_add(1); // +1 separator
    if flow {
        if let Some(tall) = tall_flow_status_rows(total_cols, preferred_max) {
            desired = desired.max(tall);
        }
    }
    desired.clamp(MIN_STATUS_ROWS.min(preferred_max), preferred_max)
}

/// Render the handoff strip (update banner, recap/pairing, PR separator & PRs)
/// into `out`.
#[allow(clippy::too_many_arguments)]
fn render_handoff_into(
    out: &mut Vec<u8>,
    layout: &Layout,
    pairing_state: &PairingState,
    update_state: &UpdateState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
    prs: &[SessionPr],
    pr_scope: &str,
    cooldowns: &Cooldowns,
    now_ms: u64,
) -> Result<()> {
    if layout.handoff_rows == 0 {
        return Ok(());
    }

    let update_banner_rows = update_state.banner_rows();
    let pairing_compact = update_banner_rows > 0;
    let pairing_banner_rows = if pairing_compact {
        pairing_state.banner_rows_compact()
    } else {
        pairing_state.banner_rows()
    };

    let mut current_banner_row = layout.pty_rows + 1;
    if update_banner_rows > 0 {
        draw_update_banner(out, current_banner_row, layout.total_cols, update_state)?;
        current_banner_row += update_banner_rows;
    }

    let banner_rows_reserved = layout
        .handoff_rows
        .saturating_sub(pr_separator_rows(prs))
        .saturating_sub(pr_handoff_rows(prs));
    let handoff_limit = layout.pty_rows + banner_rows_reserved;
    let remaining_rows = banner_rows_reserved.saturating_sub(update_banner_rows);
    let recap_rows = draw_recap_handoff(
        out,
        current_banner_row,
        layout.total_cols,
        recap_state,
        remaining_rows,
        recap_toast_visible,
    )?;
    current_banner_row += recap_rows;

    if recap_rows == 0 && pairing_banner_rows > 0 {
        let end_row = current_banner_row
            .saturating_add(pairing_banner_rows)
            .saturating_sub(1);
        if end_row <= handoff_limit {
            draw_pairing_banner(
                out,
                current_banner_row,
                layout.total_cols,
                pairing_state,
                pairing_compact,
            )?;
        }
    }

    let pr_rows = pr_handoff_rows(prs);
    if pr_rows > 0 {
        let divider_row = layout.pty_rows + 1 + banner_rows_reserved;
        draw_pr_separator(out, divider_row, layout.total_cols)?;
        let pr_start = divider_row + pr_separator_rows(prs);
        draw_pr_handoff(
            out,
            pr_start,
            layout.total_cols,
            prs,
            pr_scope,
            pr_rows,
            cooldowns,
            now_ms,
        )?;
    }

    Ok(())
}

/// Render the thick separator bar and optional flow scene label into `out`.
fn render_separator_into(out: &mut Vec<u8>, layout: &Layout, flow: Option<FlowView>) -> Result<()> {
    let separator_row = layout.pty_rows + 1 + layout.handoff_rows;
    write!(
        out,
        "\x1b[{};1H{}{}{}{}",
        separator_row,
        escape::bg(color::BG_DARK),
        escape::fg(color::DARK_GRAY),
        "━".repeat(layout.total_cols as usize),
        RESET
    )?;
    if let (Some(view), Some(width)) = (
        flow,
        flow_column_width(layout.total_cols, layout.status_rows),
    ) {
        draw_flow_label(out, separator_row, layout.total_cols, width, view)?;
    }
    Ok(())
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct GitCacheKey {
    version: u64,
    width: u16,
    height: u16,
    ide: IdeKind,
    cwd: std::path::PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChangesCacheKey {
    version: u64,
    width: u16,
    height: u16,
    titles_hash: u64,
    slack_hash: u64,
    ide: IdeKind,
    cwd: std::path::PathBuf,
    session_mark: SessionMark,
}

/// Stateful status bar renderer that caches per-module rows and emits
/// differential updates (only changed rows/modules) in a single buffered write.
///
/// Full `CLEAR_TO_END` repaints happen only on initial draw, layout/width
/// changes, or after `invalidate()` (resize, scroll-region reset, focus regain).
pub struct StatusBarRenderer {
    buf: Vec<u8>,
    needs_full_clear: bool,
    last_layout: Option<Layout>,
    last_widths: Option<ColumnWidths>,
    last_handoff_key: Option<u64>,
    last_handoff_bytes: Vec<u8>,
    last_separator_bytes: Vec<u8>,
    last_stats_rows: Vec<String>,
    last_git_rows: Vec<String>,
    last_changes_rows: Vec<String>,
    last_flow_rows: Vec<String>,
    last_flow_cells: Option<(FlowRect, Cells)>,
    git_cache: Option<(GitCacheKey, Vec<String>)>,
    changes_cache: Option<(ChangesCacheKey, Vec<String>)>,
}

impl Default for StatusBarRenderer {
    fn default() -> Self {
        Self::new()
    }
}

impl StatusBarRenderer {
    pub fn new() -> Self {
        Self {
            buf: Vec::with_capacity(16 * 1024),
            needs_full_clear: true,
            last_layout: None,
            last_widths: None,
            last_handoff_key: None,
            last_handoff_bytes: Vec::new(),
            last_separator_bytes: Vec::new(),
            last_stats_rows: Vec::new(),
            last_git_rows: Vec::new(),
            last_changes_rows: Vec::new(),
            last_flow_rows: Vec::new(),
            last_flow_cells: None,
            git_cache: None,
            changes_cache: None,
        }
    }

    /// Force a full `CLEAR_TO_END` repaint on the next draw (resize, scroll
    /// region reset, layout height change, or focus regain).
    pub fn invalidate(&mut self) {
        self.needs_full_clear = true;
        self.last_flow_cells = None;
    }

    /// Repaint only changed cells of a scene's `Cells` grid directly into the
    /// terminal buffer (cell-run diffing with CUF + combined truecolor SGR).
    pub(crate) fn draw_flow_cells_only(
        &mut self,
        stdout: &mut dyn Write,
        rect: FlowRect,
        cells: &Cells,
        cursor_position: Option<(u16, u16)>,
        child_attrs: &[u8],
    ) -> Result<()> {
        draw_flow_cells_diff(
            stdout,
            &mut self.buf,
            rect,
            cells,
            &mut self.last_flow_cells,
            cursor_position,
            child_attrs,
        )
    }

    /// Repaint only changed rows of the flow column (used when picker is open).
    #[allow(clippy::too_many_arguments)]
    pub fn draw_flow_only(
        &mut self,
        stdout: &mut dyn Write,
        rect: FlowRect,
        rows: &[String],
        exact_width: bool,
        cursor_position: Option<(u16, u16)>,
        child_attrs: &[u8],
    ) -> Result<()> {
        self.last_flow_cells = None;
        draw_flow_column_diff(
            stdout,
            &mut self.buf,
            rect,
            rows,
            exact_width,
            &mut self.last_flow_rows,
            cursor_position,
            child_attrs,
        )
    }

    /// Draw the status bar, performing either a full clear+repaint (when layout
    /// or column widths changed, or after `invalidate()`) or an incremental
    /// patch emitting only the changed modules/rows.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        stdout: &mut dyn Write,
        layout: &Layout,
        session_stats: &SessionStats,
        git_state: &GitState,
        git_version: u64,
        diff_summary: &DiffSummary,
        diff_version: u64,
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
        cursor_position: Option<(u16, u16)>,
        child_attrs: &[u8],
        cooldowns: &Cooldowns,
        now_ms: u64,
        session_mark: SessionMark,
        secondary_attached: bool,
        flow: Option<FlowView>,
        flow_cells: Option<&Cells>,
        flow_exact_width: bool,
    ) -> Result<()> {
        let is_paired = pairing_state.has_linked_devices;
        let widget_status_rows = layout.status_rows.max(1);
        let widget_data_rows = widget_status_rows.saturating_sub(1);

        let compact = stats_use_compact_layout(widget_data_rows);
        let git_available_rows = widget_status_rows.saturating_sub(2) as usize;
        let git_needs_multi_column = git_state.files.len() > git_available_rows;
        let widths = column_widths(
            layout.total_cols,
            layout.status_rows,
            compact,
            git_needs_multi_column,
            matches!(flow, Some(FlowView::Scene { .. })),
        );

        let full_clear = self.needs_full_clear
            || self.last_layout != Some(*layout)
            || self.last_widths != Some(widths);

        // Handoff strip (memoized when inputs didn't change).
        let handoff_key = compute_handoff_key(
            layout,
            pairing_state,
            update_state,
            recap_state,
            recap_toast_visible,
            prs,
            pr_scope,
            cooldowns,
            now_ms,
        );
        let handoff_changed = full_clear || self.last_handoff_key != Some(handoff_key);
        if handoff_changed {
            self.last_handoff_bytes.clear();
            render_handoff_into(
                &mut self.last_handoff_bytes,
                layout,
                pairing_state,
                update_state,
                recap_state,
                recap_toast_visible,
                prs,
                pr_scope,
                cooldowns,
                now_ms,
            )?;
            self.last_handoff_key = Some(handoff_key);
        }

        // Separator bar + flow label.
        let mut new_separator = Vec::with_capacity(layout.total_cols as usize * 3 + 64);
        render_separator_into(&mut new_separator, layout, flow)?;
        let separator_changed = full_clear || new_separator != self.last_separator_bytes;

        // Stats rows (cheap; diffed per row so throbber ticks patch row 1 only).
        let state_tint = cooldowns.tint(cooldown::SESSION_STATE_KEY, now_ms);
        let new_stats = render_stats_rows(
            widths.stats,
            widget_status_rows,
            session_stats,
            cloud_status,
            is_paired,
            pairing_state.pairing_code.as_deref(),
            state_tint,
            session_mark,
            secondary_attached,
        );

        // Git rows (memoized by git_version + geometry + ide + cwd).
        let git_key = GitCacheKey {
            version: git_version,
            width: widths.git,
            height: widget_status_rows,
            ide,
            cwd: cwd.to_path_buf(),
        };
        let git_rebuilt = self
            .git_cache
            .as_ref()
            .is_none_or(|(key, _)| *key != git_key);
        if git_rebuilt {
            let rows = render_git_rows(widths.git, widget_status_rows, git_state, ide, cwd);
            self.git_cache = Some((git_key, rows));
        }

        // Changes rows (memoized by diff_version + geometry + titles + slack + ide + cwd + mark).
        let titles = session_title_hierarchy(prs, terminal_title);
        let changes_key = ChangesCacheKey {
            version: diff_version,
            width: widths.changes,
            height: widget_status_rows,
            titles_hash: hash_titles(titles),
            slack_hash: hash_slack(slack_threads),
            ide,
            cwd: cwd.to_path_buf(),
            session_mark,
        };
        let changes_rebuilt = self
            .changes_cache
            .as_ref()
            .is_none_or(|(key, _)| *key != changes_key);
        if changes_rebuilt {
            let rows = render_changes_rows(
                widths.changes,
                widget_status_rows,
                diff_summary,
                titles,
                slack_threads,
                ide,
                cwd,
                session_mark,
            );
            self.changes_cache = Some((changes_key, rows));
        }

        let new_git = &self.git_cache.as_ref().unwrap().1;
        let new_changes = &self.changes_cache.as_ref().unwrap().1;

        let widget_pty_rows = layout.pty_rows + layout.handoff_rows;
        let git_col = widths.stats + 1;
        let changes_col = git_col + widths.git + 1;
        let flow_col = changes_col + widths.changes + 1;
        let sep_str = format!("{}│{}", escape::fg(color::DARK_GRAY), RESET);

        self.buf.clear();

        if full_clear {
            self.buf.extend_from_slice(escape::SYNC_BEGIN.as_bytes());
            write!(
                self.buf,
                "\x1b[{};1H{}",
                layout.pty_rows + 1,
                escape::CLEAR_TO_END
            )?;
            self.buf.extend_from_slice(&self.last_handoff_bytes);
            self.buf.extend_from_slice(&new_separator);

            let flow_rows_slice = match flow {
                Some(FlowView::Scene { rows, .. }) => Some(rows),
                _ => None,
            };
            self.last_flow_rows.clear();
            self.last_flow_cells = None;
            let flow_rect = widths.flow.map(|w| FlowRect {
                widget_pty_rows,
                col: flow_col,
                width: w,
                rows: widget_data_rows,
            });
            let matching_cells = flow_rect.and_then(|r| {
                flow_cells
                    .filter(|c| c.columns == r.width as usize && c.rows == r.rows as usize)
                    .map(|c| (r, c))
            });
            if matching_cells.is_none() {
                if let Some(flow_width) = widths.flow {
                    self.last_flow_rows.reserve(widget_data_rows as usize);
                    for idx in 0..widget_data_rows as usize {
                        let text = format_flow_row_text(
                            flow_rows_slice.and_then(|r| r.get(idx)).map(String::as_str),
                            flow_width as usize,
                            flow_exact_width,
                        );
                        self.last_flow_rows.push(text);
                    }
                }
            }

            for idx in 0..widget_data_rows as usize {
                let term_row = widget_pty_rows + 2 + idx as u16;
                write!(self.buf, "\x1b[{};1H", term_row)?;
                self.buf.extend_from_slice(new_stats[idx].as_bytes());
                self.buf.extend_from_slice(sep_str.as_bytes());
                self.buf.extend_from_slice(new_git[idx].as_bytes());
                self.buf.extend_from_slice(sep_str.as_bytes());
                self.buf.extend_from_slice(new_changes[idx].as_bytes());
                if widths.flow.is_some() {
                    self.buf.extend_from_slice(sep_str.as_bytes());
                    if matching_cells.is_none() {
                        self.buf.extend_from_slice(self.last_flow_rows[idx].as_bytes());
                    }
                }
            }
            if let Some((r, cells)) = matching_cells {
                let mut started = true;
                write_flow_cells_diff(&mut self.buf, r, cells, None, &mut started);
                self.last_flow_cells = Some((r, cells.clone()));
            }

            if let Some((row, col)) = cursor_position {
                write!(self.buf, "\x1b[{};{}H", row + 1, col + 1)?;
            }
            self.buf.extend_from_slice(child_attrs);
            self.buf.extend_from_slice(escape::SYNC_END.as_bytes());

            stdout.write_all(&self.buf)?;
            stdout.flush()?;

            self.needs_full_clear = false;
            self.last_layout = Some(*layout);
            self.last_widths = Some(widths);
            self.last_separator_bytes = new_separator;
            self.last_stats_rows = new_stats;
            self.last_git_rows = new_git.clone();
            self.last_changes_rows = new_changes.clone();
            return Ok(());
        }

        // Incremental patch path: emit ONLY what actually changed!
        let mut any_changed = false;
        let begin_sync = |buf: &mut Vec<u8>, started: &mut bool| {
            if !*started {
                buf.extend_from_slice(escape::SYNC_BEGIN.as_bytes());
                *started = true;
            }
        };

        if handoff_changed && layout.handoff_rows > 0 {
            begin_sync(&mut self.buf, &mut any_changed);
            for offset in 0..layout.handoff_rows {
                write!(
                    self.buf,
                    "\x1b[{};1H{}",
                    layout.pty_rows + 1 + offset,
                    escape::CLEAR_LINE
                )?;
            }
            self.buf.extend_from_slice(&self.last_handoff_bytes);
        }

        if separator_changed {
            begin_sync(&mut self.buf, &mut any_changed);
            self.buf.extend_from_slice(&new_separator);
            self.last_separator_bytes = new_separator;
        }

        let flow_rect = widths.flow.map(|w| FlowRect {
            widget_pty_rows,
            col: flow_col,
            width: w,
            rows: widget_data_rows,
        });
        let matching_cells = flow_rect.and_then(|r| {
            flow_cells
                .filter(|c| c.columns == r.width as usize && c.rows == r.rows as usize)
                .map(|c| (r, c))
        });

        for idx in 0..widget_data_rows as usize {
            let term_row = widget_pty_rows + 2 + idx as u16;
            if self.last_stats_rows.get(idx).map(String::as_str) != Some(new_stats[idx].as_str()) {
                begin_sync(&mut self.buf, &mut any_changed);
                write!(self.buf, "\x1b[{};1H", term_row)?;
                self.buf.extend_from_slice(new_stats[idx].as_bytes());
            }
            if git_rebuilt
                && self.last_git_rows.get(idx).map(String::as_str) != Some(new_git[idx].as_str())
            {
                begin_sync(&mut self.buf, &mut any_changed);
                write!(self.buf, "\x1b[{};{}H", term_row, git_col + 1)?;
                self.buf.extend_from_slice(new_git[idx].as_bytes());
            }
            if changes_rebuilt
                && self.last_changes_rows.get(idx).map(String::as_str)
                    != Some(new_changes[idx].as_str())
            {
                begin_sync(&mut self.buf, &mut any_changed);
                write!(self.buf, "\x1b[{};{}H", term_row, changes_col + 1)?;
                self.buf.extend_from_slice(new_changes[idx].as_bytes());
            }
            if matching_cells.is_none() {
                if let (Some(flow_width), Some(FlowView::Scene { rows, .. })) = (widths.flow, flow) {
                    self.last_flow_cells = None;
                    let raw = rows.get(idx).map(String::as_str);
                    if flow_exact_width {
                        if let Some(text) = raw {
                            if self.last_flow_rows.get(idx).map(String::as_str) != Some(text) {
                                begin_sync(&mut self.buf, &mut any_changed);
                                write!(self.buf, "\x1b[{};{}H", term_row, flow_col + 1)?;
                                self.buf.extend_from_slice(text.as_bytes());
                                if idx < self.last_flow_rows.len() {
                                    self.last_flow_rows[idx].clear();
                                    self.last_flow_rows[idx].push_str(text);
                                }
                            }
                            continue;
                        }
                    }
                    let formatted = format_flow_row_text(raw, flow_width as usize, false);
                    if self.last_flow_rows.get(idx).map(String::as_str) != Some(formatted.as_str())
                    {
                        begin_sync(&mut self.buf, &mut any_changed);
                        write!(self.buf, "\x1b[{};{}H", term_row, flow_col + 1)?;
                        self.buf.extend_from_slice(formatted.as_bytes());
                        if idx < self.last_flow_rows.len() {
                            self.last_flow_rows[idx] = formatted;
                        }
                    }
                }
            }
        }

        if let Some((r, cells)) = matching_cells {
            let prev = self
                .last_flow_cells
                .as_ref()
                .filter(|(prev_r, _)| *prev_r == r)
                .map(|(_, c)| c);
            write_flow_cells_diff(&mut self.buf, r, cells, prev, &mut any_changed);
            match &mut self.last_flow_cells {
                Some((prev_r, prev_c))
                    if *prev_r == r && prev_c.words.len() == cells.words.len() =>
                {
                    prev_c.columns = cells.columns;
                    prev_c.rows = cells.rows;
                    prev_c.words.copy_from_slice(&cells.words);
                }
                slot => {
                    *slot = Some((r, cells.clone()));
                }
            }
        }

        self.last_stats_rows = new_stats;
        if git_rebuilt {
            self.last_git_rows = new_git.clone();
        }
        if changes_rebuilt {
            self.last_changes_rows = new_changes.clone();
        }

        if !any_changed {
            return Ok(());
        }

        if let Some((row, col)) = cursor_position {
            write!(self.buf, "\x1b[{};{}H", row + 1, col + 1)?;
        }
        self.buf.extend_from_slice(child_attrs);
        self.buf.extend_from_slice(escape::SYNC_END.as_bytes());
        stdout.write_all(&self.buf)?;
        stdout.flush()?;
        Ok(())
    }
}

#[allow(clippy::too_many_arguments)]
fn compute_handoff_key(
    layout: &Layout,
    pairing_state: &PairingState,
    update_state: &UpdateState,
    recap_state: &RecapState,
    recap_toast_visible: bool,
    prs: &[SessionPr],
    pr_scope: &str,
    cooldowns: &Cooldowns,
    now_ms: u64,
) -> u64 {
    let mut hasher = DefaultHasher::new();
    layout.pty_rows.hash(&mut hasher);
    layout.total_cols.hash(&mut hasher);
    layout.handoff_rows.hash(&mut hasher);
    update_state.should_show_banner().hash(&mut hasher);
    update_state.new_version.hash(&mut hasher);
    update_state.summary.hash(&mut hasher);
    pairing_state.has_linked_devices.hash(&mut hasher);
    pairing_state.pairing_code.hash(&mut hasher);
    pairing_state.just_paired.hash(&mut hasher);
    pairing_state.should_show_toast().hash(&mut hasher);
    std::mem::discriminant(&recap_state.status).hash(&mut hasher);
    recap_state.line_delta.hash(&mut hasher);
    recap_state.latest.hash(&mut hasher);
    if recap_state.latest.is_some() {
        // Recap age text changes at 1-minute resolution; bucket by 10s so it
        // stays fresh without invalidating during high-frequency throbber ticks.
        (now_ms / 10_000).hash(&mut hasher);
    }
    recap_toast_visible.hash(&mut hasher);
    pr_scope.hash(&mut hasher);
    prs.len().hash(&mut hasher);
    for pr in prs {
        pr.url.hash(&mut hasher);
        pr.title.hash(&mut hasher);
        pr.branch.hash(&mut hasher);
        pr.state.hash(&mut hasher);
        pr.is_draft.hash(&mut hasher);
        pr.additions.hash(&mut hasher);
        pr.deletions.hash(&mut hasher);
        pr.changed_files.hash(&mut hasher);
        pr.mergeable.hash(&mut hasher);
        pr.merge_state_status.hash(&mut hasher);
        pr.checks_passed.hash(&mut hasher);
        pr.checks_failed.hash(&mut hasher);
        pr.checks_pending.hash(&mut hasher);
        pr.unresolved_comments.hash(&mut hasher);
        pr.review_decision.hash(&mut hasher);
        pr.primary.hash(&mut hasher);
        pr.dismissed.hash(&mut hasher);
        pr.watched.hash(&mut hasher);
        pr.fetch_limited.hash(&mut hasher);
        pr.fetch_error.hash(&mut hasher);
    }
    if !prs.is_empty() && cooldowns.active(now_ms) {
        cooldown::shade_step(now_ms).hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_titles(titles: crate::title::SessionTitleHierarchy<'_>) -> u64 {
    let mut hasher = DefaultHasher::new();
    titles.generated_title.hash(&mut hasher);
    if let Some(pr) = titles.official_pr {
        pr.number.hash(&mut hasher);
        pr.title.hash(&mut hasher);
        pr.url.hash(&mut hasher);
    }
    hasher.finish()
}

fn hash_slack(threads: &[SlackThread]) -> u64 {
    let mut hasher = DefaultHasher::new();
    threads.hash(&mut hasher);
    hasher.finish()
}

/// Draw the entire status bar area with all widgets
///
/// IMPORTANT: This function receives the cursor position from the caller
/// to restore after drawing. We can't use CURSOR_SAVE/RESTORE because
/// our vt100 parser doesn't track those, so DSR responses would be wrong.
#[allow(clippy::too_many_arguments)]
pub fn draw_status_bar(
    stdout: &mut dyn Write,
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
    session_mark: SessionMark,
    secondary_attached: bool,
    flow: Option<FlowView>,
) -> Result<()> {
    let mut renderer = StatusBarRenderer::new();
    renderer.draw(
        stdout,
        layout,
        session_stats,
        git_state,
        0,
        diff_summary,
        0,
        terminal_title,
        slack_threads,
        ide,
        cwd,
        cloud_status,
        pairing_state,
        update_state,
        recap_state,
        recap_toast_visible,
        prs,
        pr_scope,
        cursor_position,
        b"",
        cooldowns,
        now_ms,
        session_mark,
        secondary_attached,
        flow,
        None,
        false,
    )
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

        let rows =
            compute_dynamic_status_rows(35, 180, &stats, &git, &diff, None, &[], &[], 0, false);

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
        let rows =
            compute_dynamic_status_rows(60, 200, &stats, &git, &diff, None, &[], &[], 0, false);
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
            false,
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
            false,
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
        let rows = compute_dynamic_status_rows(
            80,
            200,
            &stats,
            &git,
            &diff,
            None,
            &[],
            &[],
            handoff,
            false,
        );
        assert_eq!(rows, 8);
    }

    #[test]
    fn a_roomy_terminal_grows_the_widgets_for_a_vertical_flow_scene() {
        let rows = |total_rows, total_cols, flow| {
            let (stats, git, diff) = (
                SessionStats::default(),
                GitState::default(),
                DiffSummary::default(),
            );
            compute_dynamic_status_rows(
                total_rows,
                total_cols,
                &stats,
                &git,
                &diff,
                None,
                &[],
                &[],
                0,
                flow,
            )
        };
        // 200 columns give flow 30; 13 of its rows make a tall grid, plus the separator.
        assert_eq!(rows(80, 200, true), 14);
        assert!(crate::flow::is_tall(30, 13) && !crate::flow::is_tall(30, 12));
        // Without the flow column, the widgets keep to their content.
        assert_eq!(rows(80, 200, false), 8);
        assert_eq!(flow_column_width(200, 14), Some(30));
        // 14 rows would pass 20% of 55, so the module takes the 11 that fit, and
        // the column narrows from 32 to 19, a portrait column 10 rows tall.
        assert_eq!(rows(55, 219, true), 11);
        assert_eq!(flow_column_width(219, 11), Some(19));
        assert!(crate::flow::is_tall(19, 10));
        // A column already narrow enough keeps its width: 15 columns at 9 rows.
        assert_eq!(rows(60, 100, true), 10);
        assert_eq!(flow_column_width(100, 10), Some(15));
        // Under 50 rows there is no room for 9: the scene stays horizontal, at its usual width.
        assert_eq!(rows(45, 219, true), 8);
        assert_eq!(flow_column_width(219, 8), Some(32));
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
            false,
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
        let diff = DiffSummary::from_files(vec![FileChanges {
            path: "src/lib.rs".to_string(),
            language: "Rust".to_string(),
            changes,
        }]);
        // Wide terminal (250 cols): preferred ceiling is 20, but the natural
        // packed layout for stats(7) and changes(~7) plus separator should
        // come in well under that — typical of the user's bottom-screenshot
        // resize complaint.
        let rows =
            compute_dynamic_status_rows(100, 250, &stats, &git, &diff, None, &[], &[], 0, false);
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

    /// The three-column split before the flow column existed.
    fn legacy_widths(total: u16, compact: bool, multi: bool) -> (u16, u16, u16) {
        let stats = stats_widget_width(total, compact);
        let remaining = total.saturating_sub(stats + 2);
        let git = if multi {
            remaining / 2
        } else {
            (remaining * 3) / 8
        };
        (stats, git, remaining - git)
    }

    #[test]
    fn column_widths_without_flow_are_the_three_column_ones() {
        for total in 40..=400 {
            for compact in [false, true] {
                for multi in [false, true] {
                    let w = column_widths(total, 0, compact, multi, false);
                    assert_eq!(
                        (w.stats, w.git, w.changes),
                        legacy_widths(total, compact, multi)
                    );
                    assert_eq!(w.flow, None);
                }
            }
        }
    }

    #[test]
    fn flow_takes_fifteen_percent_and_the_others_shrink_in_proportion() {
        let cases = [
            (120, false, false, (30, 25, 44, 18)),
            (120, true, false, (36, 23, 40, 18)),
            (200, false, false, (37, 48, 82, 30)),
            (200, true, false, (59, 40, 68, 30)),
            (200, false, true, (37, 65, 65, 30)),
        ];
        for (total, compact, multi, (stats, git, changes, flow)) in cases {
            let w = column_widths(total, 0, compact, multi, true);
            assert_eq!(
                (w.stats, w.git, w.changes, w.flow),
                (stats, git, changes, Some(flow)),
                "{total} cols, compact {compact}, multi {multi}"
            );
        }
    }

    #[test]
    fn flow_columns_fill_the_width_exactly_and_stats_keeps_its_minimum() {
        for total in FLOW_MIN_TOTAL_COLS..=400 {
            for compact in [false, true] {
                let w = column_widths(total, 0, compact, false, true);
                let flow = w.flow.expect("flow shows from 100 columns");
                assert_eq!(w.stats + w.git + w.changes + flow + 3, total);
                assert!(w.stats >= if compact { 36 } else { 30 });
            }
        }
    }

    #[test]
    fn narrow_terminals_have_no_flow_column() {
        assert_eq!(flow_column_width(FLOW_MIN_TOTAL_COLS - 1, 0), None);
        assert_eq!(column_widths(99, 0, false, false, true).flow, None);
        assert_eq!(
            column_widths(99, 0, false, false, true),
            column_widths(99, 0, false, false, false)
        );
    }

    #[test]
    fn flow_column_rect_sits_at_the_right_edge_below_the_separator() {
        let layout = Layout {
            pty_rows: 30,
            total_cols: 120,
            status_rows: 8,
            handoff_rows: 2,
        };
        assert_eq!(
            flow_column_rect(&layout),
            Some(FlowRect {
                widget_pty_rows: 32,
                col: 102,
                width: 18,
                rows: 7
            })
        );
        let narrow = Layout {
            total_cols: 90,
            ..layout
        };
        assert_eq!(flow_column_rect(&narrow), None);
        let no_rows = Layout {
            status_rows: 1,
            ..layout
        };
        assert_eq!(flow_column_rect(&no_rows), None);
    }

    /// A full draw of an empty session at 120×40 with nine widget rows,
    /// replayed on a terminal.
    fn draw_with_flow(flow: FlowView) -> (vt100::Parser, Layout) {
        let (total_cols, total_rows) = (120u16, 40u16);
        let layout = Layout {
            pty_rows: 31,
            total_cols,
            status_rows: 9,
            handoff_rows: 0,
        };
        let mut out = Vec::new();
        draw_status_bar(
            &mut out,
            &layout,
            &SessionStats::default(),
            &GitState::default(),
            &DiffSummary::default(),
            None,
            &[],
            IdeKind::default(),
            Path::new("/tmp"),
            None,
            &PairingState::default(),
            &UpdateState::default(),
            &RecapState::default(),
            false,
            &[],
            "",
            Some((0, 0)),
            &Cooldowns::default(),
            0,
            SessionMark::from_seed("flow-draw"),
            false,
            Some(flow),
        )
        .unwrap();
        let mut parser = vt100::Parser::new(total_rows, total_cols, 0);
        parser.process(b"top row\r\n");
        parser.process(&out);
        (parser, layout)
    }

    #[test]
    fn a_full_draw_puts_flow_after_a_third_separator_at_the_right_edge() {
        let rows: Vec<String> = (0..8).map(|_| "\x1b[38;2;200;80;10m▓".repeat(30)).collect();
        let (parser, layout) = draw_with_flow(FlowView::Scene {
            rows: &rows,
            scene: "surf",
        });
        let total_cols = layout.total_cols;
        let screen = parser.screen();
        // Nothing scrolled: the first row is still there.
        assert!(screen.contents().starts_with("top row"));
        let flow = flow_column_rect(&layout).unwrap();
        // The separator over the column names the scene and the key.
        let separator: String = (flow.col..total_cols)
            .map(|c| screen.cell(31, c).unwrap().contents())
            .collect();
        assert!(separator.contains("surf  ⌃]"), "{separator}");
        for r in 32..40u16 {
            assert_eq!(
                screen.cell(r, flow.col - 1).unwrap().contents(),
                "│",
                "row {r}"
            );
            for c in flow.col..total_cols {
                assert_eq!(
                    screen.cell(r, c).unwrap().contents(),
                    "▓",
                    "row {r} col {c}"
                );
            }
        }
    }

    #[test]
    fn with_flow_off_the_column_goes_and_the_separator_names_the_key() {
        let (parser, layout) = draw_with_flow(FlowView::Off);
        let screen = parser.screen();
        let separator: String = (0..layout.total_cols)
            .map(|c| screen.cell(31, c).unwrap().contents())
            .collect();
        assert!(separator.ends_with("━ flow off  ⌃] ━"), "{separator}");
        // No third separator: the widgets take the whole width again.
        let flow = flow_column_rect(&layout).unwrap();
        for r in 32..40u16 {
            assert_ne!(screen.cell(r, flow.col - 1).unwrap().contents(), "│");
        }
    }

    fn assert_screens_equal(a: &vt100::Screen, b: &vt100::Screen, context: &str) {
        assert_eq!(a.size(), b.size(), "{context}: screen size");
        assert_eq!(
            a.cursor_position(),
            b.cursor_position(),
            "{context}: cursor position"
        );
        let (rows, cols) = a.size();
        for r in 0..rows {
            for c in 0..cols {
                let ca = a.cell(r, c).unwrap();
                let cb = b.cell(r, c).unwrap();
                assert_eq!(
                    (
                        ca.contents(),
                        ca.fgcolor(),
                        ca.bgcolor(),
                        ca.bold(),
                        ca.italic(),
                        ca.underline()
                    ),
                    (
                        cb.contents(),
                        cb.fgcolor(),
                        cb.bgcolor(),
                        cb.bold(),
                        cb.italic(),
                        cb.underline()
                    ),
                    "{context}: mismatch at row {r} col {c}"
                );
            }
        }
    }

    #[test]
    fn incremental_redraws_match_full_redraw_cell_for_cell() {
        use crate::git::FileStatus;
        use crate::parsers::{ChangeNode, ChangeType, FileChanges, NodeKind};
        use crate::platforms::SessionState;

        let (cols, rows) = (180u16, 48u16);
        let mut stats = SessionStats::new();
        stats.work_seconds = 95;
        stats.platform_stats.state = SessionState::Complete;
        stats.platform_stats.prompts = 2;
        stats.platform_stats.completions = 2;

        let mut git = GitState::default();
        git.is_repo = true;
        git.branch = "feat/perf".to_string();
        git.files = (0..12)
            .map(|i| FileStatus {
                status: "M".to_string(),
                path: format!("src/mod_{i}.rs"),
                additions: 4 + i,
                deletions: 1,
                is_folder: false,
                file_count: 0,
            })
            .collect();

        let mut diff = DiffSummary::from_files(vec![FileChanges {
            path: "src/mod_0.rs".to_string(),
            language: "Rust".to_string(),
            changes: vec![ChangeNode {
                kind: NodeKind::Function,
                name: "draw_fast".to_string(),
                scope: vec!["Renderer".to_string()],
                change_type: ChangeType::Modified,
                additions: 10,
                deletions: 2,
                file_path: Some("src/mod_0.rs".to_string()),
                line_number: Some(42),
                children: Vec::new(),
            }],
        }]);

        let pairing = PairingState::default();
        let update = UpdateState::default();
        let recap = RecapState::default();
        let mut cooldowns = Cooldowns::default();
        let mark = SessionMark::from_seed("equiv-test");
        let cwd = Path::new("/tmp/project");

        let mut flow_col = crate::flow::FlowColumn::new(
            "equiv-test",
            mark,
            None,
            &crate::config::FlowPreferences::default(),
        );
        let status_rows = compute_dynamic_status_rows(
            rows,
            cols,
            &stats,
            &git,
            &diff,
            Some("Equivalence test"),
            &[],
            &[],
            0,
            true,
        );
        let layout = Layout {
            pty_rows: rows - status_rows,
            total_cols: cols,
            status_rows,
            handoff_rows: 0,
        };
        let rect = flow_column_rect(&layout).unwrap();
        flow_col.set_geometry(Some(rect));
        flow_col.sync_state(SessionState::Thinking);
        flow_col.tick();

        let mut incremental_vt = vt100::Parser::new(rows, cols, 0);
        let mut renderer = StatusBarRenderer::new();
        let mut git_ver = 1u64;
        let mut diff_ver = 1u64;
        let mut now_ms = 1_700_000_000_000u64;

        let check_step = |label: &str,
                          renderer: &mut StatusBarRenderer,
                          inc_vt: &mut vt100::Parser,
                          stats: &SessionStats,
                          git: &GitState,
                          git_ver: u64,
                          diff: &DiffSummary,
                          diff_ver: u64,
                          flow_col: &crate::flow::FlowColumn,
                          cooldowns: &Cooldowns,
                          now_ms: u64,
                          expect_incremental: bool| {
            let mut inc_out = Vec::new();
            let exact = flow_col.frame_exact_width() == Some(rect.width);
            renderer
                .draw(
                    &mut inc_out,
                    &layout,
                    stats,
                    git,
                    git_ver,
                    diff,
                    diff_ver,
                    Some("Equivalence test"),
                    &[],
                    IdeKind::VsCode,
                    cwd,
                    None,
                    &pairing,
                    &update,
                    &recap,
                    false,
                    &[],
                    "",
                    Some((3, 5)),
                    b"",
                    cooldowns,
                    now_ms,
                    mark,
                    false,
                    flow_col.view(),
                    flow_col.frame_cells(),
                    exact,
                )
                .unwrap();
            if expect_incremental {
                assert!(
                    !inc_out.windows(3).any(|w| w == b"\x1b[J"),
                    "{label}: incremental draw should not emit CLEAR_TO_END"
                );
            }
            inc_vt.process(&inc_out);

            let mut full_out = Vec::new();
            draw_status_bar(
                &mut full_out,
                &layout,
                stats,
                git,
                diff,
                Some("Equivalence test"),
                &[],
                IdeKind::VsCode,
                cwd,
                None,
                &pairing,
                &update,
                &recap,
                false,
                &[],
                "",
                Some((3, 5)),
                cooldowns,
                now_ms,
                mark,
                false,
                flow_col.view(),
            )
            .unwrap();
            let mut full_vt = vt100::Parser::new(rows, cols, 0);
            full_vt.process(&full_out);
            assert_screens_equal(inc_vt.screen(), full_vt.screen(), label);
            inc_out.len()
        };

        // Step 1: initial full draw
        let full_bytes = check_step(
            "initial",
            &mut renderer,
            &mut incremental_vt,
            &stats,
            &git,
            git_ver,
            &diff,
            diff_ver,
            &flow_col,
            &cooldowns,
            now_ms,
            false,
        );

        // Step 2: identical state -> 0 bytes
        let noop_bytes = check_step(
            "noop",
            &mut renderer,
            &mut incremental_vt,
            &stats,
            &git,
            git_ver,
            &diff,
            diff_ver,
            &flow_col,
            &cooldowns,
            now_ms,
            true,
        );
        assert_eq!(noop_bytes, 0, "unchanged state should emit 0 bytes");

        // Step 3: only Stats work_seconds + state cooldown changes (single row patch)
        now_ms += 60_000;
        stats.work_seconds += 60;
        cooldowns.observe_session_state(
            cooldown::SESSION_STATE_KEY.to_string(),
            SessionState::Complete,
            now_ms - 1000,
        );
        cooldowns.observe_session_state(
            cooldown::SESSION_STATE_KEY.to_string(),
            SessionState::Interrupted,
            now_ms,
        );
        stats.platform_stats.state = SessionState::Interrupted;
        let stats_patch_bytes = check_step(
            "stats_only",
            &mut renderer,
            &mut incremental_vt,
            &stats,
            &git,
            git_ver,
            &diff,
            diff_ver,
            &flow_col,
            &cooldowns,
            now_ms,
            true,
        );
        assert!(
            stats_patch_bytes < full_bytes / 5,
            "stats-only patch ({stats_patch_bytes}B) should be a tiny fraction of full draw ({full_bytes}B)"
        );

        // Step 4: Flow-only differential frame via draw_flow_cells_only (cell-run diff)
        flow_col.tick();
        let mut flow_out = Vec::new();
        renderer
            .draw_flow_cells_only(
                &mut flow_out,
                rect,
                flow_col.frame_cells().unwrap(),
                Some((3, 5)),
                b"",
            )
            .unwrap();
        incremental_vt.process(&flow_out);
        let mut full_out = Vec::new();
        draw_status_bar(
            &mut full_out,
            &layout,
            &stats,
            &git,
            &diff,
            Some("Equivalence test"),
            &[],
            IdeKind::VsCode,
            cwd,
            None,
            &pairing,
            &update,
            &recap,
            false,
            &[],
            "",
            Some((3, 5)),
            &cooldowns,
            now_ms,
            mark,
            false,
            flow_col.view(),
        )
        .unwrap();
        let mut full_vt = vt100::Parser::new(rows, cols, 0);
        full_vt.process(&full_out);
        assert_screens_equal(incremental_vt.screen(), full_vt.screen(), "after_flow_only");

        // Step 5: Git + Changes update without resizing
        git.files[0].additions = 99;
        git_ver += 1;
        diff = DiffSummary::from_files(vec![FileChanges {
            path: "src/mod_0.rs".to_string(),
            language: "Rust".to_string(),
            changes: vec![ChangeNode {
                kind: NodeKind::Function,
                name: "draw_faster".to_string(),
                scope: vec!["Renderer".to_string()],
                change_type: ChangeType::Added,
                additions: 99,
                deletions: 0,
                file_path: Some("src/mod_0.rs".to_string()),
                line_number: Some(88),
                children: Vec::new(),
            }],
        }]);
        diff_ver += 1;
        check_step(
            "git_and_changes_update",
            &mut renderer,
            &mut incremental_vt,
            &stats,
            &git,
            git_ver,
            &diff,
            diff_ver,
            &flow_col,
            &cooldowns,
            now_ms,
            true,
        );
    }
}
