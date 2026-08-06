//! Shared PR row rendering.
//!
//! The eight-column PR cell engine — column sizing with graceful width
//! degradation, per-cell styling, OSC 8 links, and the primary/secondary
//! glyphs — used by the handoff strip and the cross-session PR board.

use unicode_width::UnicodeWidthStr;

use crate::pr::SessionPr;
use crate::terminal::escape::{self, color, fg, BOLD, RESET, RESET_FG};

pub(crate) const PR_COLUMN_GAP: usize = 2;
pub(crate) const PR_LEFT_PADDING: usize = 1;
pub(crate) const PR_RIGHT_PADDING: usize = 1;

pub(crate) const PR_IDENTITY_MAX: usize = 32;
const PR_DIFF_MAX: usize = 18;
const PR_FILES_MAX: usize = 11;
pub(crate) const PR_BRANCH_MAX: usize = 40;
const PR_STATE_MAX: usize = 6;
const PR_CI_MAX: usize = 10;
/// `💬` is two cells wide, leaving room for a four-digit thread count.
const PR_COMMENTS_MAX: usize = 6;
const PR_MERGE_MAX: usize = 9;
pub(crate) const PR_IDENTITY_MIN: usize = 10;
const PR_BRANCH_MIN: usize = 8;

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
    pub(crate) diff: usize,
    pub(crate) files: usize,
    pub(crate) branch: usize,
    pub(crate) state: usize,
    pub(crate) ci: usize,
    pub(crate) comments: usize,
    pub(crate) merge: usize,
    /// The `✕` dismiss action at the right edge (1 cell when shown).
    pub(crate) dismiss: usize,
}

impl PrColumnWidths {
    #[cfg(test)]
    pub(crate) fn from_prs(prs: &[SessionPr], total_width: usize) -> Self {
        Self::from_pr_refs(&prs.iter().collect::<Vec<_>>(), total_width)
    }

    pub(crate) fn from_pr_refs(prs: &[&SessionPr], total_width: usize) -> Self {
        let mut widths = Self::default();
        for pr in prs {
            widths.identity = widths
                .identity
                .max(pr_identity_text(pr).width().min(PR_IDENTITY_MAX));
            widths.diff = widths.diff.max(pr_diff_text(pr).width().min(PR_DIFF_MAX));
            widths.files = widths
                .files
                .max(pr_files_text(pr).width().min(PR_FILES_MAX));
            widths.branch = widths
                .branch
                .max(pr_branch_text(pr).width().min(PR_BRANCH_MAX));
            widths.state = widths
                .state
                .max(pr_state_label(pr).0.width().min(PR_STATE_MAX));
            widths.ci = widths.ci.max(pr_ci_label(pr).0.width().min(PR_CI_MAX));
            widths.comments = widths
                .comments
                .max(pr_comments_label(pr).0.width().min(PR_COMMENTS_MAX));
            widths.merge = widths
                .merge
                .max(pr_merge_label(pr).0.width().min(PR_MERGE_MAX));
            if !pr_action_url(pr, "dismissed").is_empty() {
                widths.dismiss = 1;
            }
        }

        widths.fit_right(total_width);
        widths.fit_left(total_width);
        widths
    }

    pub(crate) fn fit_right(&mut self, total_width: usize) {
        let right_budget = total_width
            .saturating_sub(PR_LEFT_PADDING + PR_IDENTITY_MIN + PR_COLUMN_GAP + PR_RIGHT_PADDING);
        // Dropped least-essential first: the dismiss action, merge cleanliness,
        // the unresolved thread count, then CI, leaving the PR's own state as
        // the last survivor.
        while self.right_width() > right_budget {
            if self.dismiss > 0 {
                self.dismiss = 0;
            } else if self.merge > 0 {
                self.merge = 0;
            } else if self.comments > 0 {
                self.comments = 0;
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
        files: &str,
        branch: &str,
        total_width: usize,
    ) {
        self.identity = self.identity.max(identity.width().min(PR_IDENTITY_MAX));
        self.diff = self.diff.max(diff.width().min(PR_DIFF_MAX));
        self.files = self.files.max(files.width().min(PR_FILES_MAX));
        self.branch = self.branch.max(branch.width().min(PR_BRANCH_MAX));
        self.fit_right(total_width);
        self.fit_left(total_width);
    }

    fn fit_left(&mut self, total_width: usize) {
        let right = self.right_width();
        let cluster_gap = usize::from(right > 0) * PR_COLUMN_GAP;
        let left_budget = total_width
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(right)
            .saturating_sub(cluster_gap);

        while self.left_width() > left_budget {
            if self.branch > PR_BRANCH_MIN {
                let overflow = self.left_width() - left_budget;
                self.branch -= overflow.min(self.branch - PR_BRANCH_MIN);
            } else if self.branch > 0 {
                self.branch = 0;
            } else if self.files > 0 {
                self.files = 0;
            } else if self.diff > 0 {
                self.diff = 0;
            } else {
                self.identity = self.identity.min(left_budget);
                break;
            }
        }

        // The branch is the flexible middle column. Once every required column
        // fits, give it all remaining room so the status columns stay anchored
        // at the right edge without becoming right-aligned themselves.
        if self.branch > 0 {
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
        table_width(&[self.identity, self.diff, self.files, self.branch])
    }

    pub(crate) fn right_width(&self) -> usize {
        table_width(&[self.state, self.ci, self.comments, self.merge, self.dismiss])
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
    let active = columns.iter().filter(|&&width| width > 0).count();
    columns.iter().sum::<usize>() + active.saturating_sub(1) * PR_COLUMN_GAP
}

/// One styled row: left padding, the left columns, then a gap wide enough to
/// anchor the status columns against the right edge.
pub(crate) fn pr_row_text(width: u16, pr: &SessionPr, widths: &PrColumnWidths) -> String {
    pr_row_text_with_optional_activity(width, pr, widths, None)
}

/// Render the PR row with a board-only activity cell before the GitHub status.
/// The shared handoff row calls `pr_row_text` and keeps the original eight
/// columns.
pub(crate) fn pr_row_text_with_activity(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    styled: String,
    visible: usize,
    column_width: usize,
) -> String {
    pr_row_text_with_optional_activity(width, pr, widths, Some((styled, visible, column_width)))
}

/// Render an active session without a PR in the same columns as PR rows.
#[allow(clippy::too_many_arguments)]
pub(crate) fn session_row_text_with_activity(
    width: u16,
    title: &str,
    additions: i64,
    deletions: i64,
    files: &str,
    branch: &str,
    widths: &PrColumnWidths,
    styled_activity: String,
    activity_visible: usize,
    activity_width: usize,
) -> String {
    let identity = truncate_to_width(&format!("◇ {title}"), widths.identity);
    let diff = colored_diff_cell(
        additions,
        deletions,
        color::GREEN,
        color::RED,
        color::GRAY,
        widths.diff,
    );
    let left_cells = [
        colored_cell(&identity, color::GRAY, widths.identity),
        diff,
        colored_cell(files, color::DARK_GRAY, widths.files),
        colored_cell_capped(branch, color::DARK_GRAY, widths.branch, PR_BRANCH_MAX),
    ];
    let mut row = " ".repeat(PR_LEFT_PADDING);
    row.push_str(&cells_text(&left_cells));

    let right_cells = [
        (styled_activity, activity_visible, activity_width),
        (String::new(), 0, widths.right_width()),
    ];
    let right_width = cells_width(&right_cells);
    if right_width > 0 {
        let gap = (width as usize)
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(widths.left_width())
            .saturating_sub(right_width);
        row.push_str(&" ".repeat(gap));
        row.push_str(&cells_text(&right_cells));
    }
    row
}

fn pr_row_text_with_optional_activity(
    width: u16,
    pr: &SessionPr,
    widths: &PrColumnWidths,
    activity: Option<PrCell>,
) -> String {
    let mut row = " ".repeat(PR_LEFT_PADDING);
    row.push_str(&cells_text(&pr_left_cells(pr, widths)));

    let mut right_cells = pr_right_cells(pr, widths).to_vec();
    if let Some(activity) = activity {
        right_cells.insert(0, activity);
    }
    let right_width = cells_width(&right_cells);
    if right_width > 0 {
        let gap = (width as usize)
            .saturating_sub(PR_LEFT_PADDING)
            .saturating_sub(PR_RIGHT_PADDING)
            .saturating_sub(widths.left_width())
            .saturating_sub(right_width);
        row.push_str(&" ".repeat(gap));
        row.push_str(&cells_text(&right_cells));
    }
    row
}

type PrCell = (String, usize, usize);

pub(crate) fn pr_left_cells(pr: &SessionPr, widths: &PrColumnWidths) -> [PrCell; 4] {
    let identity = truncate_identity(pr, widths.identity);
    // The glyph is its own click target — it flips the PR's disposition —
    // while `repo #num` still opens the PR on GitHub. A column too narrow to
    // hold the glyph drops it and links the whole label instead.
    let glyph = pr_glyph(pr);
    let (glyph_kept, identity_label) = match identity.strip_prefix(&format!("{glyph} ")) {
        Some(label) => (true, label),
        None => (false, identity.as_str()),
    };
    let flip = if pr.primary { "secondary" } else { "primary" };
    let glyph_styled = if glyph_kept {
        format!(
            "{} ",
            link_text(&pr_action_url(pr, flip), glyph.to_string(), 1)
        )
    } else {
        String::new()
    };
    let identity_styled = format!(
        "{}{}{}{}",
        fg(pr_identity_color(pr)),
        glyph_styled,
        escape::hyperlink(&pr.url, identity_label),
        RESET_FG
    );

    // Both diff columns open the PR's Files-changed tab — the actual changes.
    let files_url = pr_files_url(pr);

    let diff = colored_diff_cell(
        pr.additions,
        pr.deletions,
        row_color(pr, color::GREEN),
        row_color(pr, color::RED),
        row_color(pr, color::GRAY),
        widths.diff,
    );

    [
        (identity_styled, identity.width(), widths.identity),
        (link_text(&files_url, diff.0, diff.1), diff.1, diff.2),
        linked_cell(
            &pr_files_text(pr),
            row_color(pr, color::DARK_GRAY),
            widths.files,
            &files_url,
        ),
        colored_cell_capped(
            &pr_branch_text(pr),
            row_color(pr, color::DARK_GRAY),
            widths.branch,
            PR_BRANCH_MAX,
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

pub(crate) fn pr_right_cells(pr: &SessionPr, widths: &PrColumnWidths) -> [PrCell; 5] {
    let (state_label, state_color) = pr_state_label(pr);
    let (ci_label, ci_color) = pr_ci_label(pr);
    let (comments_label, comments_color) = pr_comments_label(pr);
    let (merge_label, merge_color) = pr_merge_label(pr);
    [
        colored_cell(state_label, row_color(pr, state_color), widths.state),
        // Failing CI points at the failing job; anything else at the Checks tab.
        linked_cell(&ci_label, row_color(pr, ci_color), widths.ci, &pr.ci_url),
        // Unresolved threads point at the first one's comment.
        linked_cell(
            &comments_label,
            row_color(pr, comments_color),
            widths.comments,
            &pr.comments_url,
        ),
        colored_cell(merge_label, row_color(pr, merge_color), widths.merge),
        // Dismiss action: removes the PR from every list in the group.
        linked_cell(
            "✕",
            row_color(pr, color::DARK_GRAY),
            widths.dismiss,
            &pr_action_url(pr, "dismissed"),
        ),
    ]
}

/// A cell color as the PR's row should render it: primaries keep the full
/// palette, secondaries go one notch dimmer so the PR the session is actually
/// driving stands out.
fn row_color(pr: &SessionPr, base: u8) -> u8 {
    if pr.primary {
        base
    } else {
        color::dimmed(base)
    }
}

fn colored_cell(label: &str, color: u8, width: usize) -> PrCell {
    colored_cell_capped(label, color, width, width)
}

/// A colored cell whose visible text is also an OSC 8 link.
fn linked_cell(label: &str, color: u8, width: usize, url: &str) -> PrCell {
    let (styled, visible, width) = colored_cell(label, color, width);
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

/// Lay cells out side by side, padding each to its column and separating
/// adjacent ones. Cells with a zero width are skipped entirely.
fn cells_text(cells: &[PrCell]) -> String {
    let mut out = String::new();
    for (styled, visible, width) in cells.iter().filter(|(_, _, width)| *width > 0) {
        if !out.is_empty() {
            out.push_str(&" ".repeat(PR_COLUMN_GAP));
        }
        out.push_str(styled);
        out.push_str(&" ".repeat(width.saturating_sub(*visible)));
    }
    out
}

fn cells_width(cells: &[PrCell]) -> usize {
    let active = cells.iter().filter(|(_, _, width)| *width > 0).count();
    cells.iter().map(|(_, _, width)| width).sum::<usize>()
        + active.saturating_sub(1) * PR_COLUMN_GAP
}

/// `★` marks the session's primary PR; `☆` everything else.
fn pr_glyph(pr: &SessionPr) -> &'static str {
    if pr.primary {
        "★"
    } else {
        "☆"
    }
}

/// Web action link that stores a disposition for the whole device group.
/// The page posts the override with the dashboard's stored auth, confirms,
/// and closes its tab; the desktop picks the change up within a minute.
pub(crate) fn pr_action_url(pr: &SessionPr, disposition: &str) -> String {
    if pr.owner.is_empty() || pr.repo.is_empty() {
        return String::new();
    }
    format!(
        "https://drinkcrabigator.com/pr-action?owner={}&repo={}&number={}&disposition={}",
        pr.owner, pr.repo, pr.number, disposition
    )
}

/// Primaries keep the PR purple; secondaries recede into dim gray.
fn pr_identity_color(pr: &SessionPr) -> u8 {
    if pr.primary {
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
    let suffix = format!(" #{}", pr.number);
    let fixed = 2 + suffix.width(); // glyph + space + number
    if width <= fixed {
        return truncate_to_width(&format!("{glyph} {repo}{suffix}"), width);
    }
    let repo = truncate_to_width(repo, width - fixed);
    format!("{glyph} {repo}{suffix}")
}

/// The identity column at full width, for column sizing.
pub(crate) fn pr_identity_text(pr: &SessionPr) -> String {
    format!("{} {} #{}", pr_glyph(pr), pr_repo_label(pr), pr.number)
}

fn pr_diff_text(pr: &SessionPr) -> String {
    if pr.additions == 0 && pr.deletions == 0 {
        String::new()
    } else {
        format!("+{} -{}", pr.additions, pr.deletions)
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
        format!("⎇ {}", pr.branch)
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
