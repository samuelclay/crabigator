//! Shared PR row rendering.
//!
//! The shared PR cell engine — column sizing with graceful width
//! degradation, per-cell styling, OSC 8 links, the primary/secondary
//! glyphs, and the promote/demote and dismiss actions — used by the handoff
//! strip and the cross-session PR board.

use unicode_width::UnicodeWidthStr;

use crate::pr::SessionPr;
use crate::terminal::escape::{
    self, color, fg, BOLD, RESET, RESET_BOLD, RESET_FG, RESET_UNDERLINE, UNDERLINE,
};
use crate::ui::cooldown::{tint_text, StatusTints, Tint};

pub(crate) const PR_COLUMN_GAP: usize = 2;
pub(crate) const PR_RIGHT_COLUMN_GAP: usize = 1;
pub(crate) const PR_LEFT_PADDING: usize = 1;
pub(crate) const PR_RIGHT_PADDING: usize = 1;

pub(crate) const PR_IDENTITY_MAX: usize = 44;
const PR_IDENTITY_EXTRA: usize = 12;
const PR_DIFF_MAX: usize = 18;
const PR_FILES_MAX: usize = 11;
/// One cell wider than the other caps: the branch label spends three of its
/// cells on [`BRANCH_PREFIX`], so this keeps the name itself as long as it was.
pub(crate) const PR_BRANCH_MAX: usize = 29;
const PR_STATE_MAX: usize = 6;
const PR_CI_MAX: usize = 10;
/// `💬` is two cells wide, leaving room for a four-digit thread count.
const PR_COMMENTS_MAX: usize = 6;
/// The review column is a single state glyph.
const PR_REVIEW_MAX: usize = 1;
const PR_MERGE_MAX: usize = 9;
pub(crate) const PR_IDENTITY_MIN: usize = 10;
const PR_BRANCH_MIN: usize = 9;

/// The branch glyph and the gap before the branch name. Most terminals draw
/// `⎇` wider than the one cell it claims, so a single space reads as none.
pub(crate) const BRANCH_PREFIX: &str = "⎇  ";

/// PRs the handoff renders: dismissed ones hidden, primaries above
/// secondaries, arrival order kept within each group.
pub(crate) fn display_prs(prs: &[SessionPr]) -> Vec<&SessionPr> {
    let mut out: Vec<&SessionPr> = prs.iter().filter(|pr| !pr.dismissed).collect();
    out.sort_by_key(|pr| !pr.primary);
    out
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct PrColumnWidths {
    pub(crate) identity: usize,
    pub(crate) number: usize,
    pub(crate) diff: usize,
    pub(crate) files: usize,
    pub(crate) branch: usize,
    pub(crate) state: usize,
    pub(crate) ci: usize,
    pub(crate) comments: usize,
    pub(crate) review: usize,
    pub(crate) merge: usize,
    /// The `↑`/`↓` promote/demote action beside the dismiss (1 cell when shown).
    pub(crate) flip: usize,
    /// The `✕` dismiss action at the right edge (1 cell when shown).
    pub(crate) dismiss: usize,
    /// Space reserved for PR-board detail links that replace GitHub status.
    pub(crate) detail_right: usize,
    /// PR-view layout: the diff sits in the right cluster beside the GitHub
    /// status instead of between the title and the branch, and the title
    /// takes every spare cell on the left.
    pub(crate) stats_right: bool,
    /// The flexible columns as measured, before any fitting pass grew or
    /// dropped them. The PR-view layout refits from these so the title, not
    /// the branch, ends up with the slack.
    pub(crate) natural_identity: usize,
    pub(crate) natural_branch: usize,
    pub(crate) natural_diff: usize,
}

impl PrColumnWidths {
    #[cfg(test)]
    pub(crate) fn from_prs(prs: &[SessionPr], total_width: usize) -> Self {
        Self::from_pr_refs(&prs.iter().collect::<Vec<_>>(), total_width)
    }

    pub(crate) fn from_pr_refs(prs: &[&SessionPr], total_width: usize) -> Self {
        let mut widths = Self::default();
        for pr in prs {
            widths.measure_identity(pr_identity_text(pr).width());
            widths.number = widths.number.max(pr_number_text(pr).width());
            widths.measure_stats(pr_diff_text(pr).width(), pr_files_text(pr).width());
            widths.measure_branch(pr_branch_text(pr).width());
            widths.state = widths
                .state
                .max(pr_state_label(pr).0.width().min(PR_STATE_MAX));
            widths.ci = widths.ci.max(pr_ci_label(pr).0.width().min(PR_CI_MAX));
            widths.comments = widths
                .comments
                .max(pr_comments_label(pr).0.width().min(PR_COMMENTS_MAX));
            widths.review = widths
                .review
                .max(pr_review_label(pr).0.width().min(PR_REVIEW_MAX));
            widths.merge = widths
                .merge
                .max(pr_merge_label(pr).0.width().min(PR_MERGE_MAX));
            if !pr_action_url(pr, "dismissed", "").is_empty() {
                widths.flip = 1;
                widths.dismiss = 1;
            }
        }

        widths.fit_right(total_width);
        widths.fit_left(total_width);
        widths
    }

    fn measure_identity(&mut self, width: usize) {
        let width = width.min(PR_IDENTITY_MAX);
        self.identity = self.identity.max(width);
        self.natural_identity = self.natural_identity.max(width);
    }

    fn measure_branch(&mut self, width: usize) {
        let width = width.min(PR_BRANCH_MAX);
        self.branch = self.branch.max(width);
        self.natural_branch = self.natural_branch.max(width);
    }

    fn measure_stats(&mut self, diff: usize, files: usize) {
        let diff = diff.min(PR_DIFF_MAX);
        let files = files.min(PR_FILES_MAX);
        self.diff = self.diff.max(diff);
        self.natural_diff = self.natural_diff.max(diff);
        self.files = self.files.max(files);
    }

    pub(crate) fn fit_right(&mut self, total_width: usize) {
        let essential_left = table_width(&[PR_IDENTITY_MIN, self.number]);
        let right_budget = total_width
            .saturating_sub(PR_LEFT_PADDING + essential_left + PR_COLUMN_GAP + PR_RIGHT_PADDING);
        self.detail_right = self.detail_right.min(right_budget);
        // Dropped least-essential first: the promote/demote action, the
        // dismiss action, the board diff, merge cleanliness, the unresolved
        // thread count, the review glyph, then CI, leaving the PR's own state
        // as the last survivor.
        while self.status_right_width() > right_budget {
            if self.flip > 0 {
                self.flip = 0;
            } else if self.dismiss > 0 {
                self.dismiss = 0;
            } else if self.stats_right && self.diff > 0 {
                self.diff = 0;
            } else if self.merge > 0 {
                self.merge = 0;
            } else if self.comments > 0 {
                self.comments = 0;
            } else if self.review > 0 {
                self.review = 0;
            } else if self.ci > 0 {
                self.ci = 0;
            } else {
                self.state = self.state.min(right_budget);
                break;
            }
        }
    }

    /// Include a non-PR board row in the shared left-column measurements.
    pub(crate) fn include_board_row(
        &mut self,
        identity: &str,
        diff: &str,
        branch: &str,
        total_width: usize,
    ) {
        self.measure_identity(identity.width());
        self.measure_stats(diff.width(), 0);
        self.measure_branch(branch.width());
        self.fit_right(total_width);
        self.fit_left(total_width);
    }

    /// Reserve enough of the right edge for expanded board metadata while
    /// keeping compact GitHub status anchored to the same edge.
    pub(crate) fn include_board_detail_right(&mut self, width: usize, total_width: usize) {
        self.detail_right = self.detail_right.max(width);
        self.fit_right(total_width);
        self.fit_left(total_width);
    }

    /// Switch to the board's row shape: the number rides inside the
    /// identity cell (`★ #142: title`) so the standalone number column
    /// collapses, and the diff moves beside the GitHub status so the title
    /// keeps the width it used to take.
    pub(crate) fn use_pr_view_layout(&mut self, total_width: usize) {
        self.number = 0;
        self.stats_right = true;
        // The board drops promote/demote but keeps dismiss so unwanted PRs
        // can still be removed. The in-session handoff strip carries both.
        self.flip = 0;
        // Earlier passes fitted the stats on the left and may have dropped
        // them or grown the branch into the slack. Start over from the
        // measured widths so the fit below reflects this layout's priorities.
        self.identity = self.natural_identity;
        self.branch = self.natural_branch;
        self.diff = self.natural_diff;
        self.files = 0;
        self.fit_right(total_width);
        self.fit_left(total_width);
    }

    /// Include the board's session title in the identity measurement.
    pub(crate) fn include_board_identity(
        &mut self,
        pr: &SessionPr,
        title: &str,
        total_width: usize,
    ) {
        self.measure_identity(board_pr_identity_text(pr, title).width());
        self.fit_right(total_width);
        self.fit_left(total_width);
    }

    /// Cells left for the left columns once the right cluster is placed.
    fn left_budget(&self, total_width: usize) -> usize {
        let right = self.right_width();
        let cluster_gap = usize::from(right > 0) * PR_COLUMN_GAP;
        total_width
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(right)
            .saturating_sub(cluster_gap)
    }

    fn fit_left(&mut self, total_width: usize) {
        // Dropping stats frees the left side directly, or shrinks the right
        // cluster when they sit beside the status, so the budget is
        // recomputed after every cut.
        let mut left_budget = self.left_budget(total_width);
        while self.left_width() > left_budget {
            if self.stats_right && self.branch > 0 {
                // The PR view shows the branch whole or not at all: a
                // truncated branch is worth less than more of the title.
                self.branch = 0;
            } else if self.branch > PR_BRANCH_MIN {
                let overflow = self.left_width() - left_budget;
                self.branch -= overflow.min(self.branch - PR_BRANCH_MIN);
            } else if self.branch > 0 {
                self.branch = 0;
            } else if self.files > 0 {
                self.files = 0;
            } else if self.diff > 0 {
                self.diff = 0;
            } else {
                let number_space =
                    self.number + usize::from(self.identity > 0 && self.number > 0) * PR_COLUMN_GAP;
                self.identity = self.identity.min(left_budget.saturating_sub(number_space));
                if self.left_width() > left_budget {
                    self.number = self.number.min(left_budget);
                }
                break;
            }
            left_budget = self.left_budget(total_width);
        }

        if self.stats_right {
            // The PR view's row is its title; it takes every spare cell.
            self.identity += left_budget.saturating_sub(self.left_width());
        } else if self.branch > 0 {
            // Favor session titles over branches, whose distinguishing text is
            // preserved at the right edge when the label is truncated.
            let identity_growth = left_budget
                .saturating_sub(self.left_width())
                .min(PR_IDENTITY_EXTRA)
                .min(PR_IDENTITY_MAX.saturating_sub(self.identity));
            self.identity += identity_growth;
            self.branch += left_budget.saturating_sub(self.left_width());
        } else {
            // A branch too narrow to be worth reading is dropped whole, which
            // leaves slack behind. Spend it on the identity so the repo name
            // survives instead of stranding the cells.
            let slack = left_budget.saturating_sub(self.left_width());
            self.identity = (self.identity + slack).min(PR_IDENTITY_MAX);
        }
    }

    fn left_width(&self) -> usize {
        if self.stats_right {
            return table_width(&[self.identity, self.number, self.branch]);
        }
        table_width(&[
            self.identity,
            self.number,
            self.diff,
            self.files,
            self.branch,
        ])
    }

    /// The board's diff column, plus one leading and one trailing space so it
    /// reads apart from activity and GitHub status.
    fn stats_cell_width(&self) -> usize {
        if !self.stats_right {
            return 0;
        }
        match self.diff {
            0 => 0,
            stats => stats + 2,
        }
    }

    pub(crate) fn board_left_width(&self) -> usize {
        self.left_width()
    }

    pub(crate) fn right_width(&self) -> usize {
        self.status_right_width().max(self.detail_right)
    }

    fn status_right_width(&self) -> usize {
        table_width_with_gap(
            &[
                self.stats_cell_width(),
                self.state,
                self.ci,
                self.comments,
                self.review,
                self.merge,
                self.flip,
                self.dismiss,
            ],
            PR_RIGHT_COLUMN_GAP,
        )
    }

    #[cfg(test)]
    pub(crate) fn total_width(&self) -> usize {
        let left = self.left_width();
        let right = self.right_width();
        PR_LEFT_PADDING
            + left
            + usize::from(left > 0 && right > 0) * PR_COLUMN_GAP
            + right
            + PR_RIGHT_PADDING
    }
}

fn table_width(columns: &[usize]) -> usize {
    table_width_with_gap(columns, PR_COLUMN_GAP)
}

fn table_width_with_gap(columns: &[usize], gap: usize) -> usize {
    let active = columns.iter().filter(|&&width| width > 0).count();
    columns.iter().sum::<usize>() + active.saturating_sub(1) * gap
}

#[cfg(test)]
pub(crate) fn pr_row_text(width: u16, pr: &SessionPr, widths: &PrColumnWidths) -> String {
    pr_row_text_tinted(width, pr, widths, StatusTints::default(), "")
}

/// One styled row: left padding, the left columns, then a gap wide enough to
/// anchor the status columns against the right edge, with cooldown tints on
/// the status cells that changed recently. `scope` is where the row's action
/// links store dispositions — see [`pr_action_url`].
pub(crate) fn pr_row_text_tinted(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    tints: StatusTints,
    scope: &str,
) -> String {
    pr_row_text_with_optional_activity(width, pr, widths, None, None, tints, scope)
}

/// Render the PR view's header row: identity (`★ #142: title`) and branch on
/// the left, then the sessions' activity, diff, and GitHub status anchored
/// right. The block stands for the PR across every session, so its action
/// links act group-wide.
pub(crate) fn pr_view_row_text(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    title: &str,
    activity: PrCell,
    tints: StatusTints,
) -> String {
    pr_row_text_with_optional_activity(width, pr, widths, Some(title), Some(activity), tints, "")
}

/// Render a session-view PR sub-row: the PR-view row anatomy (identity,
/// branch, and diff beside the GitHub status) indented beneath its session,
/// with the activity column left empty — the session's header row carries
/// the state and ages. Its action links act in that session's scope.
#[allow(clippy::too_many_arguments)]
pub(crate) fn session_view_pr_row_text(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    title: &str,
    activity_width: usize,
    tints: StatusTints,
    scope: &str,
) -> String {
    const INDENT: &str = "  ";
    let budget = widths.identity.saturating_sub(INDENT.width());
    let identity = truncate_board_pr_identity(pr, title, budget);
    let mut left_cells = pr_left_cells_with_identity(pr, widths, identity, scope);
    left_cells[0].0.insert_str(0, INDENT);
    left_cells[0].1 += INDENT.width();
    pr_row_text_with_left_cells(
        width,
        pr,
        widths,
        &left_cells,
        Some((String::new(), 0, activity_width)),
        tints,
        scope,
    )
}

/// Render an active session without a PR in the same columns as PR rows.
#[allow(clippy::too_many_arguments)]
pub(crate) fn session_row_text_with_activity(
    width: u16,
    title: &str,
    identity_color: u8,
    additions: i64,
    deletions: i64,
    branch: &str,
    widths: &PrColumnWidths,
    styled_activity: String,
    activity_visible: usize,
    activity_width: usize,
) -> String {
    let identity = truncate_to_width(&format!("◇ {title}"), widths.identity);
    // The diff sits where a PR row's diff does, at the head of an otherwise
    // empty status cluster.
    let status = if widths.stats_cell_width() == 0 {
        (String::new(), 0, widths.right_width())
    } else {
        let diff = workspace_diff_text(additions, deletions);
        let diff_cell = colored_diff_cell(
            additions,
            deletions,
            color::GREEN,
            color::RED,
            color::GRAY,
            diff.width().min(PR_DIFF_MAX),
        );
        stats_right_cell(
            &diff_cell,
            widths.right_width() - widths.status_right_width(),
            widths.right_width(),
        )
    };
    let left_cells = [
        colored_cell(&identity, identity_color, widths.identity),
        (String::new(), 0, widths.number),
        (String::new(), 0, 0),
        (String::new(), 0, 0),
        colored_branch_cell(branch, color::DARK_GRAY, widths.branch),
    ];
    let mut row = " ".repeat(PR_LEFT_PADDING);
    row.push_str(&cells_text(&left_cells));

    let right_cells = [(styled_activity, activity_visible, activity_width), status];
    let right_width = right_cells_width(&right_cells);
    if right_width > 0 {
        let gap = (width as usize)
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(widths.left_width())
            .saturating_sub(right_width);
        row.push_str(&" ".repeat(gap));
        row.push_str(&right_cells_text(&right_cells));
    }
    row
}

/// Render a PR-board detail row against the same left, activity, and GitHub
/// status columns as its compact PR row.
#[allow(clippy::too_many_arguments)]
pub(crate) fn board_detail_row_text(
    width: u16,
    widths: &PrColumnWidths,
    left_styled: String,
    left_visible: usize,
    activity_styled: String,
    activity_visible: usize,
    activity_width: usize,
    right_styled: String,
    right_visible: usize,
) -> String {
    let mut row = " ".repeat(PR_LEFT_PADDING);
    row.push_str(&cells_text(&[(
        left_styled,
        left_visible,
        widths.left_width(),
    )]));

    let right_cells = [
        (activity_styled, activity_visible, activity_width),
        (right_styled, right_visible, widths.right_width()),
    ];
    let right_width = right_cells_width(&right_cells);
    if right_width > 0 {
        let gap = (width as usize)
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(widths.left_width())
            .saturating_sub(right_width);
        row.push_str(&" ".repeat(gap));
        row.push_str(&right_cells_text(&right_cells));
    }
    row
}

fn pr_row_text_with_optional_activity(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    board_title: Option<&str>,
    activity: Option<PrCell>,
    tints: StatusTints,
    scope: &str,
) -> String {
    let left_cells = board_title.map_or_else(
        || pr_left_cells_scoped(pr, widths, scope),
        |title| pr_left_cells_with_board_title(pr, widths, title, scope),
    );
    pr_row_text_with_left_cells(width, pr, widths, &left_cells, activity, tints, scope)
}

#[allow(clippy::too_many_arguments)]
fn pr_row_text_with_left_cells(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    left_cells: &[PrCell],
    activity: Option<PrCell>,
    tints: StatusTints,
    scope: &str,
) -> String {
    let mut row = " ".repeat(PR_LEFT_PADDING);
    row.push_str(&cells_text(left_cells));

    let mut right_cells = pr_right_cells_tinted(pr, widths, tints, scope).to_vec();
    if widths.stats_right {
        right_cells.insert(0, pr_stats_right_cell(pr, widths));
    }
    let status_width = right_cells_width(&right_cells);
    let filler = widths.right_width().saturating_sub(status_width);
    if filler > 0 {
        if let Some((styled, visible, width)) = right_cells
            .iter_mut()
            .find(|(_, _, cell_width)| *cell_width > 0)
        {
            styled.insert_str(0, &" ".repeat(filler));
            *visible += filler;
            *width += filler;
        }
    }
    if let Some(activity) = activity {
        right_cells.insert(0, activity);
    }
    let right_width = right_cells_width(&right_cells);
    if right_width > 0 {
        let gap = (width as usize)
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(widths.left_width())
            .saturating_sub(right_width);
        row.push_str(&" ".repeat(gap));
        row.push_str(&right_cells_text(&right_cells));
    }
    row
}

fn styled_pr_identity(pr: &SessionPr, identity: &str, scope: &str) -> String {
    let glyph = pr_glyph(pr);
    let (glyph_kept, identity_label) = match identity.strip_prefix(&format!("{glyph} ")) {
        Some(label) => (true, label),
        None => (false, identity),
    };
    // The glyph flips a session PR's classification; on a pure watch it
    // removes the watch instead (watches are group-level).
    let (flip, flip_scope) = if pr.watched && !pr.primary {
        ("unwatched", "")
    } else {
        (pr_flip_disposition(pr), scope)
    };
    let glyph_styled = if glyph_kept {
        format!(
            "{} ",
            link_text(&pr_action_url(pr, flip, flip_scope), glyph.to_string(), 1)
        )
    } else {
        String::new()
    };
    format!(
        "{}{}{}{}",
        fg(pr_identity_color(pr)),
        glyph_styled,
        escape::hyperlink(
            &pr.url,
            &emphasize_first(identity_label.to_string(), &pr_number_text(pr))
        ),
        RESET_FG
    )
}

/// The PR view's diff as a right-cluster cell, padded so it sits apart from
/// the activity before it and the status after it.
fn pr_stats_right_cell(pr: &SessionPr, widths: &PrColumnWidths) -> PrCell {
    let width = widths.stats_cell_width();
    if width == 0 {
        return (String::new(), 0, 0);
    }
    stats_right_cell(&pr_stats_cells(pr, widths.diff, 0)[0], 0, width)
}

/// A diff cell at the head of the right cluster. One leading space keeps it
/// off the activity column; `filler` is the shift a PR row's status takes when
/// detail links are wider than the status itself, so rows without a status
/// line up with it.
fn stats_right_cell(cell: &PrCell, filler: usize, width: usize) -> PrCell {
    let lead = filler + 1;
    (
        format!("{}{}", " ".repeat(lead), cell.0),
        lead + cell.1,
        width,
    )
}

/// The diff and file-count cells. Both open the PR's Files-changed tab — the
/// actual changes.
fn pr_stats_cells(pr: &SessionPr, diff_width: usize, files_width: usize) -> [PrCell; 2] {
    let files_url = pr_files_url(pr);
    let diff = colored_diff_cell(
        pr.additions,
        pr.deletions,
        row_color(pr, color::GREEN),
        row_color(pr, color::RED),
        row_color(pr, color::GRAY),
        diff_width,
    );
    [
        (link_text(&files_url, diff.0, diff.1), diff.1, diff.2),
        linked_cell(
            &pr_files_text(pr),
            row_color(pr, color::DARK_GRAY),
            files_width,
            &files_url,
        ),
    ]
}

/// A rendered cell: styled text, its visible width, and the column width.
pub(crate) type PrCell = (String, usize, usize);

#[cfg(test)]
pub(crate) fn pr_left_cells(pr: &SessionPr, widths: &PrColumnWidths) -> [PrCell; 5] {
    pr_left_cells_scoped(pr, widths, "")
}

pub(crate) fn pr_left_cells_scoped(
    pr: &SessionPr,
    widths: &PrColumnWidths,
    scope: &str,
) -> [PrCell; 5] {
    let identity = truncate_identity(pr, widths.identity);
    pr_left_cells_with_identity(pr, widths, identity, scope)
}

fn pr_left_cells_with_board_title(
    pr: &SessionPr,
    widths: &PrColumnWidths,
    title: &str,
    scope: &str,
) -> [PrCell; 5] {
    let identity = truncate_board_pr_identity(pr, title, widths.identity);
    pr_left_cells_with_identity(pr, widths, identity, scope)
}

fn pr_left_cells_with_identity(
    pr: &SessionPr,
    widths: &PrColumnWidths,
    identity: String,
    scope: &str,
) -> [PrCell; 5] {
    // The glyph is its own click target; the remaining identity opens GitHub.
    let identity_styled = styled_pr_identity(pr, &identity, scope);

    let mut number = linked_cell(
        &pr_number_text(pr),
        row_color(pr, color::PURPLE),
        widths.number,
        &pr.url,
    );
    number.0 = emphasize_first(number.0, &pr_number_text(pr));

    // In the PR view the stats render beside the status instead.
    let [diff, files] = if widths.stats_right {
        pr_stats_cells(pr, 0, 0)
    } else {
        pr_stats_cells(pr, widths.diff, widths.files)
    };

    [
        (identity_styled, identity.width(), widths.identity),
        number,
        diff,
        files,
        colored_branch_cell(
            &pr_branch_text(pr),
            row_color(pr, color::DARK_GRAY),
            widths.branch,
        ),
    ]
}

/// Render additions and deletions with the same emphasis on PR and session
/// rows. A diff that cannot fit keeps its shape but drops the split colors.
fn colored_diff_cell(
    additions: i64,
    deletions: i64,
    additions_color: u8,
    deletions_color: u8,
    fallback_color: u8,
    width: usize,
) -> PrCell {
    let full = if additions == 0 && deletions == 0 {
        String::new()
    } else {
        format!("+{additions} -{deletions}")
    };
    let visible = truncate_to_width(&full, width);
    let styled = if visible.is_empty() {
        String::new()
    } else if visible.width() == full.width() {
        format!(
            "{BOLD}{}+{} {}-{}{}",
            fg(additions_color),
            additions,
            fg(deletions_color),
            deletions,
            RESET,
        )
    } else {
        format!("{BOLD}{}{}{}", fg(fallback_color), visible, RESET)
    };
    (styled, visible.width(), width)
}

#[cfg(test)]
pub(crate) fn pr_right_cells(pr: &SessionPr, widths: &PrColumnWidths) -> [PrCell; 7] {
    pr_right_cells_tinted(pr, widths, StatusTints::default(), "")
}

/// What each GitHub status column shows, as `(column, text)`. The board
/// compares the text between frames to spot changes.
pub(crate) fn pr_status_signature(pr: &SessionPr) -> [(&'static str, String); 4] {
    [
        ("state", pr_state_label(pr).0.to_string()),
        ("ci", pr_ci_label(pr).0),
        ("review", pr_review_label(pr).0.to_string()),
        ("merge", pr_merge_label(pr).0.to_string()),
    ]
}

/// The status cells with cooldown tints on the columns that changed recently.
pub(crate) fn pr_right_cells_tinted(
    pr: &SessionPr,
    widths: &PrColumnWidths,
    tints: StatusTints,
    scope: &str,
) -> [PrCell; 7] {
    let (state_label, state_color) = pr_state_label(pr);
    let (ci_label, ci_color) = pr_ci_label(pr);
    let (comments_label, comments_color) = pr_comments_label(pr);
    let (review_label, review_color) = pr_review_label(pr);
    let (merge_label, merge_color) = pr_merge_label(pr);
    [
        colored_cell_tinted(
            state_label,
            row_color(pr, state_color),
            widths.state,
            tints.state,
        ),
        // Failing CI points at the failing job; anything else at the Checks tab.
        linked_cell_tinted(
            &ci_label,
            row_color(pr, ci_color),
            widths.ci,
            &pr.ci_url,
            tints.ci,
        ),
        // Unresolved threads point at the first one's comment.
        linked_cell_tinted(
            &comments_label,
            row_color(pr, comments_color),
            widths.comments,
            &pr.comments_url,
            tints.comments,
        ),
        colored_cell_tinted(
            review_label,
            row_color(pr, review_color),
            widths.review,
            tints.review,
        ),
        colored_cell_tinted(
            merge_label,
            row_color(pr, merge_color),
            widths.merge,
            tints.merge,
        ),
        // Promote/demote action: ↑ makes a secondary PR primary, ↓ makes a
        // primary secondary. Unlike the identity glyph, it works on watched
        // PRs too, so a watch can be promoted instead of only unwatched.
        linked_cell(
            pr_flip_glyph(pr),
            row_color(pr, color::DARK_GRAY),
            widths.flip,
            &pr_action_url(pr, pr_flip_disposition(pr), scope),
        ),
        // Dismiss action. With a session or worktree scope it removes the PR
        // from that session's lists only; with no scope (PR-view blocks,
        // watched PRs) it removes the PR from every list in the group.
        linked_cell(
            "✕",
            row_color(pr, color::DARK_GRAY),
            widths.dismiss,
            &pr_action_url(pr, "dismissed", scope),
        ),
    ]
}

/// A cell color as the PR's row should render it: primaries and watched PRs
/// keep the full palette, secondaries go one notch dimmer so the PRs that
/// matter stand out.
fn row_color(pr: &SessionPr, base: u8) -> u8 {
    if pr.primary || pr.watched {
        base
    } else {
        color::dimmed(base)
    }
}

fn colored_cell(label: &str, color: u8, width: usize) -> PrCell {
    colored_cell_capped(label, color, width, width)
}

/// A colored cell painted on a cooldown tint when one is given. The label
/// keeps its normal color; only the background glows.
fn colored_cell_tinted(label: &str, color: u8, width: usize, tint: Option<Tint>) -> PrCell {
    let (styled, visible, width) = colored_cell(label, color, width);
    match tint {
        // An empty cell has nothing to light up.
        Some(tint) if visible > 0 => (
            tint_text(&truncate_to_width(label, width), tint, color),
            visible,
            width,
        ),
        _ => (styled, visible, width),
    }
}

/// A colored cell whose visible text is also an OSC 8 link.
fn linked_cell(label: &str, color: u8, width: usize, url: &str) -> PrCell {
    linked_cell_tinted(label, color, width, url, None)
}

fn linked_cell_tinted(
    label: &str,
    color: u8,
    width: usize,
    url: &str,
    tint: Option<Tint>,
) -> PrCell {
    let (styled, visible, width) = colored_cell_tinted(label, color, width, tint);
    (link_text(url, styled, visible), visible, width)
}

/// Wrap already-styled cell text in a link, skipping empty cells and empty
/// targets so neither produces a clickable blank. Column padding is written
/// outside the cell, so the link never covers dead space.
fn link_text(url: &str, styled: String, visible: usize) -> String {
    if url.is_empty() || visible == 0 {
        styled
    } else {
        escape::hyperlink(url, &styled)
    }
}

fn colored_cell_capped(label: &str, color: u8, width: usize, max_content: usize) -> PrCell {
    let label = truncate_to_width(label, width.min(max_content));
    let visible = label.width();
    (
        format!("{}{}{}", fg(color), label, RESET_FG),
        visible,
        width,
    )
}

fn colored_branch_cell(label: &str, color: u8, width: usize) -> PrCell {
    let label = truncate_branch_to_width(label, width.min(PR_BRANCH_MAX));
    let visible = label.width();
    (
        format!("{}{}{}", fg(color), label, RESET_FG),
        visible,
        width,
    )
}

/// Keep the branch suffix because it usually carries the ticket or feature
/// name that distinguishes similar worktrees.
fn truncate_branch_to_width(label: &str, max_width: usize) -> String {
    let Some(branch) = label.strip_prefix(BRANCH_PREFIX) else {
        return truncate_left_to_width(label, max_width);
    };
    let prefix_width = BRANCH_PREFIX.width();
    if label.width() <= max_width {
        return label.to_string();
    }
    if max_width <= prefix_width {
        return truncate_to_width(BRANCH_PREFIX, max_width);
    }
    format!(
        "{BRANCH_PREFIX}{}",
        truncate_left_to_width(branch, max_width - prefix_width)
    )
}

fn truncate_left_to_width(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if text.width() <= max_width {
        return text.to_string();
    }
    if max_width == 0 {
        return String::new();
    }
    if max_width == 1 {
        return "…".to_string();
    }

    let suffix_width = max_width - 1;
    let mut width = 0;
    let mut suffix_start = text.len();
    for (index, ch) in text.char_indices().rev() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > suffix_width {
            break;
        }
        width += ch_width;
        suffix_start = index;
    }
    format!("…{}", &text[suffix_start..])
}

/// Lay cells out side by side, padding each to its column and separating
/// adjacent ones. Cells with a zero width are skipped entirely.
fn cells_text(cells: &[PrCell]) -> String {
    cells_text_with_gap(cells, PR_COLUMN_GAP)
}

fn cells_text_with_gap(cells: &[PrCell], gap: usize) -> String {
    let mut out = String::new();
    for (styled, visible, width) in cells.iter().filter(|(_, _, width)| *width > 0) {
        if !out.is_empty() {
            out.push_str(&" ".repeat(gap));
        }
        out.push_str(styled);
        out.push_str(&" ".repeat(width.saturating_sub(*visible)));
    }
    out
}

fn right_cells_text(cells: &[PrCell]) -> String {
    cells_text_with_gap(cells, PR_RIGHT_COLUMN_GAP)
}

fn right_cells_width(cells: &[PrCell]) -> usize {
    let active = cells.iter().filter(|(_, _, width)| *width > 0).count();
    cells.iter().map(|(_, _, width)| width).sum::<usize>()
        + active.saturating_sub(1) * PR_RIGHT_COLUMN_GAP
}

/// `★` marks the session's primary PR, `◉` an explicitly watched one that no
/// session claims, and `☆` everything else.
fn pr_glyph(pr: &SessionPr) -> &'static str {
    if pr.primary {
        "★"
    } else if pr.watched {
        "◉"
    } else {
        "☆"
    }
}

/// The promote/demote action's glyph: `↑` promotes a secondary PR, `↓`
/// demotes a primary one.
fn pr_flip_glyph(pr: &SessionPr) -> &'static str {
    if pr.primary {
        "↓"
    } else {
        "↑"
    }
}

/// The disposition the promote/demote action stores: the opposite of the
/// PR's current classification.
fn pr_flip_disposition(pr: &SessionPr) -> &'static str {
    if pr.primary {
        "secondary"
    } else {
        "primary"
    }
}

/// Web action link that stores a disposition. `scope` picks where it applies:
/// empty for the whole device group (PR-view blocks, watched PRs),
/// `session:<id>` for one session's rows, or `path:<cwd>` for a worktree —
/// sticky for future sessions started there. The page posts the override with
/// the dashboard's stored auth, confirms, and closes its tab; the Worker
/// nudges every live session over its WebSocket so the desktop refetches at
/// once (its 60s poll is the fallback).
pub(crate) fn pr_action_url(pr: &SessionPr, disposition: &str, scope: &str) -> String {
    if pr.owner.is_empty() || pr.repo.is_empty() {
        return String::new();
    }
    let mut url = format!(
        "https://drinkcrabigator.com/pr-action?owner={}&repo={}&number={}&disposition={}",
        pr.owner, pr.repo, pr.number, disposition
    );
    if !scope.is_empty() {
        url.push_str("&scope=");
        url.push_str(&urlencode(scope));
    }
    url
}

/// Percent-encode a query value; scopes carry `:` and filesystem paths.
fn urlencode(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for byte in value.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' | b'/' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// Primaries and watched PRs keep the PR purple; secondaries recede into
/// dim gray.
fn pr_identity_color(pr: &SessionPr) -> u8 {
    if pr.primary || pr.watched {
        color::PURPLE
    } else {
        color::dimmed(color::GRAY)
    }
}

/// The repo name the identity column shows; PRs tracked before their repo is
/// known fall back to a bare `PR`.
fn pr_repo_label(pr: &SessionPr) -> &str {
    if pr.repo.is_empty() {
        "PR"
    } else {
        pr.repo.as_str()
    }
}

fn truncate_identity(pr: &SessionPr, width: usize) -> String {
    let repo = pr_repo_label(pr);
    let glyph = pr_glyph(pr);
    truncate_to_width(&format!("{glyph} {repo}"), width)
}

fn truncate_board_pr_identity(pr: &SessionPr, title: &str, width: usize) -> String {
    truncate_to_width(&board_pr_identity_text(pr, title), width)
}

/// The identity column at full width, for column sizing.
pub(crate) fn pr_identity_text(pr: &SessionPr) -> String {
    format!("{} {}", pr_glyph(pr), pr_repo_label(pr))
}

fn board_pr_identity_text(pr: &SessionPr, title: &str) -> String {
    format!("{} {}: {title}", pr_glyph(pr), pr_number_text(pr))
}

fn pr_number_text(pr: &SessionPr) -> String {
    format!("#{}", pr.number)
}

/// Bold and underline the first occurrence of `text` (the PR's `#number`
/// handle) in a styled cell, so the number stands out and reads as the link
/// it is. Cells truncated into the number stay plain.
fn emphasize_first(styled: String, text: &str) -> String {
    styled.replacen(
        text,
        &format!("{BOLD}{UNDERLINE}{text}{RESET_UNDERLINE}{RESET_BOLD}"),
        1,
    )
}

fn pr_diff_text(pr: &SessionPr) -> String {
    if pr.additions == 0 && pr.deletions == 0 {
        String::new()
    } else {
        format!("+{} -{}", pr.additions, pr.deletions)
    }
}

fn workspace_diff_text(additions: i64, deletions: i64) -> String {
    if additions == 0 && deletions == 0 {
        String::new()
    } else {
        format!("+{additions} -{deletions}")
    }
}

/// The PR's Files-changed tab — where the diff columns point.
fn pr_files_url(pr: &SessionPr) -> String {
    if pr.url.is_empty() {
        String::new()
    } else {
        format!("{}/files", pr.url)
    }
}

fn pr_files_text(pr: &SessionPr) -> String {
    if pr.changed_files == 0 {
        String::new()
    } else {
        let word = if pr.changed_files == 1 {
            "file"
        } else {
            "files"
        };
        format!("{} {}", pr.changed_files, word)
    }
}

fn pr_branch_text(pr: &SessionPr) -> String {
    if pr.branch.is_empty() {
        String::new()
    } else {
        format!("{BRANCH_PREFIX}{}", pr.branch)
    }
}

fn pr_ci_label(pr: &SessionPr) -> (String, u8) {
    if pr.checks_total == 0 {
        (String::new(), color::GRAY)
    } else if pr.checks_failed > 0 {
        (format!("✗{} CI", pr.checks_failed), color::RED)
    } else if pr.checks_pending > 0 {
        (format!("●{} CI", pr.checks_pending), color::YELLOW)
    } else {
        ("✓ CI".to_string(), color::GREEN)
    }
}

/// `💬N` for a PR carrying unresolved review threads; blank when the
/// conversation is settled (or the PR is no longer open, which clears it).
pub(crate) fn pr_comments_label(pr: &SessionPr) -> (String, u8) {
    if pr.unresolved_comments <= 0 {
        (String::new(), color::GRAY)
    } else {
        (format!("💬{}", pr.unresolved_comments), color::ORANGE)
    }
}

/// Review approval state, open PRs only: `✓` approved, `✗` changes requested,
/// `⊘` a review was dismissed by new commits and never redone, `◌` awaiting
/// review.
fn pr_review_label(pr: &SessionPr) -> (&'static str, u8) {
    if pr.state != "OPEN" {
        ("", color::GRAY)
    } else if pr.review_decision == "APPROVED" {
        ("✓", color::GREEN)
    } else if pr.review_decision == "CHANGES_REQUESTED" {
        ("✗", color::RED)
    } else if pr.review_dismissed {
        ("⊘", color::ORANGE)
    } else {
        ("◌", color::DARK_GRAY)
    }
}

fn pr_merge_label(pr: &SessionPr) -> (&'static str, u8) {
    match pr.mergeable.as_str() {
        "CONFLICTING" => ("conflicts", color::RED),
        "MERGEABLE" if pr.merge_state_status == "BEHIND" => ("behind", color::YELLOW),
        "MERGEABLE" => ("clean", color::GREEN),
        _ => ("", color::GRAY),
    }
}

/// `(label, color)` for a PR's state. A PR that has never enriched shows
/// fetch progress or the failure instead of a silently bare row — retries
/// run automatically, so "error" means "failing, still trying".
fn pr_state_label(pr: &SessionPr) -> (&'static str, u8) {
    if pr.state == "MERGED" {
        ("merged", color::PURPLE)
    } else if pr.state == "CLOSED" {
        ("closed", color::RED)
    } else if pr.is_draft {
        ("draft", color::GRAY)
    } else if pr.state == "OPEN" {
        // Match the softer green used for the dir path in the git widget.
        ("open", color::LIGHT_GREEN)
    } else if !pr.fetch_error.is_empty() {
        ("error", color::RED)
    } else if pr.refreshed_at == 0 {
        ("fetch…", color::GRAY)
    } else {
        ("", color::GRAY)
    }
}

/// Cut `text` to `max_width` display cells, ellipsizing when it doesn't fit.
pub(crate) fn truncate_to_width(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 {
        return String::new();
    }
    let mut width = 0;
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    for (i, ch) in chars.iter().enumerate() {
        let ch_width = ch.width().unwrap_or(0);
        let last = i + 1 == total;
        if width + ch_width > max_width {
            if !out.is_empty() {
                out.pop();
            }
            out.push('…');
            return out;
        }
        if !last && width + ch_width == max_width {
            // Reserve the final cell for the ellipsis when there's more text.
            out.push('…');
            return out;
        }
        width += ch_width;
        out.push(*ch);
    }
    out
}
