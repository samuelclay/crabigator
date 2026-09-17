//! Changes widget - displays semantic code changes grouped by language
//!
//! Shows parsed semantic changes (functions, classes, etc.) from git diffs,
//! organized by programming language with per-change line stats.

use std::io::{Stdout, Write};
use std::path::Path;

use anyhow::Result;

use crate::ide::IdeKind;
use crate::parsers::{ChangeNode, ChangeType, DiffSummary, LanguageChanges, NodeKind};
use crate::pr::SessionPr;
use crate::session_mark::SessionMark;
use crate::slack::{compact_display_label, SlackThread};
use crate::terminal::escape::{
    self, color, fg, hyperlink, BOLD, ITALIC, RESET, RESET_BOLD, RESET_UNDERLINE, UNDERLINE,
};
use crate::title::SessionTitleHierarchy;

use super::pr_cells::truncate_to_width;
use super::utils::{digit_count, strip_ansi_len, truncate_middle, truncate_path};
use super::WidgetArea;

/// Columns the message snippet is indented from the Slack link above it.
const SLACK_SNIPPET_INDENT: usize = 2;

/// Priority order for node kinds (lower = higher priority, appears first)
fn kind_priority(kind: &NodeKind) -> u8 {
    match kind {
        NodeKind::Function => 0,
        NodeKind::Method => 1,
        NodeKind::Class => 2,
        NodeKind::Struct => 3,
        NodeKind::Enum => 4,
        NodeKind::Trait => 5,
        NodeKind::Impl => 6,
        NodeKind::Module => 7,
        NodeKind::Const => 8,
        NodeKind::Other => 9,
    }
}

/// Get icon and color for a node kind
fn get_kind_icon(kind: &NodeKind) -> (&'static str, u8) {
    match kind {
        NodeKind::Function | NodeKind::Method => ("ƒ", color::BLUE),
        NodeKind::Class => ("◆", color::PURPLE),
        NodeKind::Struct => ("◇", color::CYAN),
        NodeKind::Enum => ("▣", color::YELLOW),
        NodeKind::Trait => ("◈", color::PURPLE),
        NodeKind::Impl => ("◊", color::CYAN),
        NodeKind::Module => ("□", color::GRAY),
        NodeKind::Const => ("•", color::GRAY),
        NodeKind::Other => ("·", color::DARK_GRAY),
    }
}

/// Dynamic column widths computed from actual change data
#[derive(Clone, Copy)]
struct StatsColumnWidths {
    del_num: usize, // width for "−N" column
    add_num: usize, // width for "+N" column
}

impl StatsColumnWidths {
    /// Compute column widths from a list of changes
    fn from_changes(changes: &[ChangeNode]) -> Self {
        let mut max_del = 0usize;
        let mut max_add = 0usize;

        for c in changes {
            max_del = max_del.max(c.deletions);
            max_add = max_add.max(c.additions);
        }

        // Number column widths: sign + digits (minimum 1 space if none)
        let del_num = if max_del > 0 {
            1 + digit_count(max_del)
        } else {
            1 // just a space placeholder
        };
        let add_num = if max_add > 0 {
            1 + digit_count(max_add)
        } else {
            1 // just a space placeholder
        };

        Self { del_num, add_num }
    }

    /// Total width of stats columns
    fn total_width(&self) -> usize {
        // Format: " −N +M" = space + del_num + space + add_num
        1 + self.del_num + 1 + self.add_num
    }
}

/// Number of content rows the changes widget actually renders for the given
/// column width — accounting for the packed layout the widget falls back to
/// when per-row display would not fit. The widget header at row 1 already
/// carries the first language's name, so only later languages add a header
/// row of their own.
pub fn changes_natural_rows(
    diff_summary: &DiffSummary,
    available_width: u16,
    title_rows: u16,
    slack_threads: &[SlackThread],
) -> u16 {
    let slack_rows = slack_row_count(slack_threads);
    let langs = diff_summary.by_language();
    if langs.is_empty() {
        return (title_rows + slack_rows).max(1);
    }

    // Average packed-item width: modifier + icon + truncated name (≤20) + " ±N".
    // The packer separates items with a 2-col margin, so each slot consumes
    // `avg + 2` columns. This is a coarse estimate but matches typical Rust
    // changes within ~1 row.
    const AVG_ITEM_WIDTH: u32 = 30;
    let row_w = (available_width as u32).max(1);
    let items_per_row = ((row_w + 2) / (AVG_ITEM_WIDTH + 2)).max(1) as u16;

    let mut rows: u16 = langs.len() as u16; // one header row per language
    for lang in &langs {
        let n = lang.changes.len() as u16;
        if n == 0 {
            continue;
        }
        let packed = n.div_ceil(items_per_row);
        // Pick the smaller of one-per-row vs packed — both are valid layouts
        // and the widget will honour whichever the area allows.
        rows = rows.saturating_add(n.min(packed));
    }
    // Titles claim their own top rows. Without them the first language header
    // doubles as the top row, so no extra row is needed.
    rows = rows.saturating_add(title_rows);
    rows = rows.saturating_add(slack_rows);
    rows
}

/// What the Slack block shows on a given row below the titles: a thread's
/// link, or the message snippet that follows a link when the text is known.
enum SlackRow<'a> {
    Link(&'a SlackThread),
    Snippet(&'a SlackThread, &'a str),
}

/// Every row the Slack block draws, in order: each thread's link, followed
/// by its message snippet when the text is known.
fn slack_rows(threads: &[SlackThread]) -> impl Iterator<Item = SlackRow<'_>> {
    threads.iter().flat_map(|thread| {
        std::iter::once(SlackRow::Link(thread)).chain(
            thread
                .text
                .as_deref()
                .map(|text| SlackRow::Snippet(thread, text)),
        )
    })
}

fn slack_row_count(threads: &[SlackThread]) -> u16 {
    u16::try_from(slack_rows(threads).count()).unwrap_or(u16::MAX)
}

/// Draw the changes widget at the given position
#[allow(clippy::too_many_arguments)]
pub fn draw_changes_widget(
    stdout: &mut Stdout,
    area: WidgetArea,
    diff_summary: &DiffSummary,
    titles: SessionTitleHierarchy<'_>,
    slack_threads: &[SlackThread],
    ide: IdeKind,
    cwd: &Path,
    session_mark: SessionMark,
) -> Result<()> {
    write!(
        stdout,
        "{}",
        escape::cursor_to(area.pty_rows + 1 + area.row, area.col + 1)
    )?;
    // 1-col left margin so content doesn't sit flush against the separator/edge.
    write!(stdout, " ")?;
    let inner_width = area.width.saturating_sub(2);
    let inner_width_usize = inner_width as usize;

    // Get changes grouped by language
    let by_language = diff_summary.by_language();

    let title_rows = titles.row_count();
    let prefix_rows = title_rows.saturating_add(slack_row_count(slack_threads));

    // Session title + chip on row 1; purple `#N: title` under it. The PR
    // takes row 1 when there is no session title.
    if let Some(header) = title_header_row(titles, area.row, inner_width_usize, session_mark) {
        write_padded_row(stdout, &header, inner_width_usize)?;
        return Ok(());
    }

    let slack_start_row = title_rows + 1;
    if area.row >= slack_start_row {
        let offset = (area.row - slack_start_row) as usize;
        if let Some(slack_row) = slack_rows(slack_threads).nth(offset) {
            let row = match slack_row {
                SlackRow::Link(thread) => {
                    let label = compact_display_label(thread, inner_width_usize);
                    format!(
                        "{}{}{}",
                        fg(color::CYAN),
                        hyperlink(&thread.url, &label),
                        RESET
                    )
                }
                SlackRow::Snippet(thread, text) => {
                    slack_snippet_row(thread, text, inner_width_usize)
                }
            };
            write_padded_row(stdout, &row, inner_width_usize)?;
            return Ok(());
        }
    }

    // Without title/link context, the first language header doubles as the
    // top row so the widget never wastes a line.
    if area.row == 1 && prefix_rows == 0 {
        let header = if diff_summary.loading {
            format!(
                "{}Changes{} {}...{}",
                fg(color::ORANGE),
                RESET,
                fg(color::GRAY),
                RESET
            )
        } else if let Some(first_lang) = by_language.first() {
            let total: usize = by_language.iter().map(|l| l.changes.len()).sum();
            let change_word = if total == 1 { "change" } else { "changes" };
            format!(
                "{}{}{} {}{} {}{}",
                fg(color::ORANGE),
                first_lang.language,
                RESET,
                fg(color::GRAY),
                total,
                change_word,
                RESET
            )
        } else {
            String::new()
        };
        write_padded_row(stdout, &header, inner_width_usize)?;
        return Ok(());
    }

    if by_language.is_empty() {
        write!(stdout, "{:width$}", "", width = inner_width_usize)?;
        write!(stdout, " ")?;
        return Ok(());
    }

    // Build rows to display
    let available_rows = area.height.saturating_sub(1 + prefix_rows) as usize;
    let rows_data = build_rows_for_display(&by_language, inner_width, available_rows, ide, cwd);

    // Map the terminal row to a built row. When the title occupies the top row,
    // the first language header (rows_data[0]) shows on the row just below it;
    // otherwise that header already lives on the top row, so we skip it here.
    let row_idx = if prefix_rows > 0 {
        area.row.saturating_sub(prefix_rows + 1) as usize
    } else {
        (area.row - 1) as usize
    };

    if row_idx < rows_data.len() {
        let content = &rows_data[row_idx];
        write!(stdout, "{}", content)?;
        let content_len = strip_ansi_len(content);
        let pad = inner_width_usize.saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;
    } else {
        write!(stdout, "{:width$}", "", width = inner_width_usize)?;
    }

    // 1-col right margin.
    write!(stdout, " ")?;

    Ok(())
}

/// The message snippet under a Slack link: indented, italic, and muted so it
/// reads as a quote, cut at the column edge, and clickable like the link.
fn slack_snippet_row(thread: &SlackThread, text: &str, width: usize) -> String {
    let snippet = truncate_to_width(text, width.saturating_sub(SLACK_SNIPPET_INDENT));
    if snippet.is_empty() {
        return String::new();
    }
    format!(
        "{}{}{}{}{}",
        " ".repeat(SLACK_SNIPPET_INDENT),
        ITALIC,
        fg(color::GRAY),
        hyperlink(&thread.url, &snippet),
        RESET
    )
}

fn title_header_row(
    titles: SessionTitleHierarchy<'_>,
    row: u16,
    width: usize,
    session_mark: SessionMark,
) -> Option<String> {
    match (row, titles.generated_title, titles.official_pr) {
        (1, Some(title), _) => Some(format_session_title_row(title, width, session_mark)),
        (1, None, Some(pr)) => Some(format_official_title_row(pr, width, Some(session_mark))),
        (2, Some(_), Some(pr)) => Some(format_official_title_row(pr, width, None)),
        _ => None,
    }
}

fn title_chip_prefix(mark: Option<SessionMark>) -> (String, usize) {
    match mark {
        Some(mark) => (format!("{} ", mark.chip()), mark.width() + 1),
        None => (String::new(), 0),
    }
}

fn format_session_title_row(title: &str, width: usize, mark: SessionMark) -> String {
    format!(
        "{} {}{}{}",
        mark.chip(),
        fg(color::LIGHT_BLUE),
        truncate_path(title, width.saturating_sub(mark.width() + 1)),
        RESET
    )
}

/// `#N: title` with a GitHub link and an emphasized number, matching the board.
fn format_official_title_row(pr: &SessionPr, width: usize, mark: Option<SessionMark>) -> String {
    let number = format!("#{}", pr.number);
    let (prefix, chip_span) = title_chip_prefix(mark);
    let identity = truncate_path(
        &format!("{number}: {}", pr.title.trim()),
        width.saturating_sub(chip_span),
    );
    // Truncation that cuts into `#N` leaves the number unstyled, same as the board.
    let labeled = identity.replacen(
        &number,
        &format!("{BOLD}{UNDERLINE}{number}{RESET_UNDERLINE}{RESET_BOLD}"),
        1,
    );
    let body = if pr.url.is_empty() {
        labeled
    } else {
        hyperlink(&pr.url, &labeled)
    };
    format!("{}{}{}{}", prefix, fg(color::PURPLE), body, RESET)
}

fn write_padded_row(stdout: &mut Stdout, content: &str, width: usize) -> Result<()> {
    let pad = width.saturating_sub(strip_ansi_len(content));
    write!(stdout, "{}{:pad$} ", content, "", pad = pad)?;
    Ok(())
}

/// Formatted item with its display width
struct FormattedItem {
    text: String,
    width: usize,
}

/// Build rows for display, respecting available height
fn build_rows_for_display(
    by_language: &[LanguageChanges],
    width: u16,
    available_rows: usize,
    ide: IdeKind,
    cwd: &Path,
) -> Vec<String> {
    let mut rows = Vec::new();

    for lang_changes in by_language {
        if rows.len() >= available_rows {
            break;
        }

        // Sort changes by kind priority, then by total changes (descending),
        // then by name and file_path for deterministic ordering
        let mut sorted_changes: Vec<&ChangeNode> = lang_changes.changes.iter().collect();
        sorted_changes.sort_by(|a, b| {
            kind_priority(&a.kind)
                .cmp(&kind_priority(&b.kind))
                .then_with(|| {
                    let a_total = a.additions + a.deletions;
                    let b_total = b.additions + b.deletions;
                    b_total.cmp(&a_total) // descending
                })
                .then_with(|| a.name.cmp(&b.name))
                .then_with(|| a.file_path.cmp(&b.file_path))
        });

        // Add language header
        let count = lang_changes.changes.len();
        let label = if count == 1 { "change" } else { "changes" };
        let header = format_header(&lang_changes.language, count, label, width as usize);
        rows.push(header);

        if rows.len() >= available_rows {
            break;
        }

        // Calculate how many rows we have for items
        let remaining_rows = available_rows - rows.len();
        let num_changes = sorted_changes.len();

        // If all changes fit one-per-row, use column-aligned display
        if num_changes <= remaining_rows {
            let stats_widths = StatsColumnWidths::from_changes(&lang_changes.changes);
            let overhead = 5 + stats_widths.total_width();
            let name_width = (width as usize).saturating_sub(overhead).max(10);

            for change in &sorted_changes {
                if rows.len() >= available_rows {
                    break;
                }
                let item = format_change_entry(change, name_width, &stats_widths, ide, cwd);
                rows.push(item);
            }
        } else {
            // Too many changes - use ragged/wrapped display
            let items: Vec<FormattedItem> = sorted_changes
                .iter()
                .map(|c| format_change_compact(c, ide, cwd))
                .collect();

            // Pack items into rows with 2-space margin
            let packed_rows = pack_items_into_rows(&items, width as usize);

            let mut items_shown = 0usize;
            for packed_row in packed_rows {
                if rows.len() >= available_rows {
                    // Show "and N more" for remaining items
                    let remaining = num_changes.saturating_sub(items_shown);
                    if remaining > 0 {
                        rows.push(format!(
                            "{}  ... and {} more{}",
                            fg(color::DARK_GRAY),
                            remaining,
                            RESET
                        ));
                    }
                    break;
                }
                items_shown += packed_row.item_count;
                rows.push(packed_row.text);
            }
        }
    }

    rows
}

/// Format a language header row
fn format_header(language: &str, count: usize, label: &str, width: usize) -> String {
    // Match the first row format: "Language N changes" with count in gray
    let content = format!(
        "{}{}{} {}{} {}{}",
        fg(color::ORANGE),
        language,
        RESET,
        fg(color::GRAY),
        count,
        label,
        RESET
    );
    let content_len = strip_ansi_len(&content);
    let pad = width.saturating_sub(content_len);
    format!("{}{:pad$}", content, "", pad = pad)
}

/// Render the `parent › ` scope prefix (if any) in gray, leaving the symbol
/// name itself in the default color.
fn colorize_scope_prefix(name: &str) -> String {
    match name.rfind(" › ") {
        Some(idx) => {
            let split = idx + " › ".len();
            format!(
                "{}{}{}{}",
                fg(color::GRAY),
                &name[..split],
                RESET,
                &name[split..]
            )
        }
        None => name.to_string(),
    }
}

/// Format a single change entry with aligned columns (for one-per-row display)
fn format_change_entry(
    change: &ChangeNode,
    name_width: usize,
    stats_widths: &StatsColumnWidths,
    ide: IdeKind,
    cwd: &Path,
) -> String {
    let (icon, icon_color) = get_kind_icon(&change.kind);

    // Modifier for change type: + for added, ~ for modified, - for deleted
    let (modifier, modifier_color) = match change.change_type {
        ChangeType::Added => ("+", color::GREEN),
        ChangeType::Modified => ("~", color::YELLOW),
        ChangeType::Deleted => ("-", color::RED),
    };

    let name = truncate_middle(&change.name, name_width);
    let name_char_count = name.chars().count();
    let name_padding = name_width.saturating_sub(name_char_count);
    let styled_name = colorize_scope_prefix(&name);

    // Wrap name in hyperlink if we have file path info
    let linked_name = if let Some(ref path) = change.file_path {
        let abs_path = cwd.join(path).to_string_lossy().to_string();
        let url = ide.file_url(&abs_path, change.line_number);
        hyperlink(&url, &styled_name)
    } else {
        styled_name
    };

    // Format stats with aligned columns
    let stats = format_change_stats(
        change.additions,
        change.deletions,
        stats_widths.del_num,
        stats_widths.add_num,
    );

    format!(
        "{}{}{}{}{}{} {}{:pad$}{}",
        fg(modifier_color),
        modifier,
        RESET,
        fg(icon_color),
        icon,
        RESET,
        linked_name,
        "",
        stats,
        pad = name_padding
    )
}

/// Format a compact change entry (for ragged/wrapped display)
fn format_change_compact(change: &ChangeNode, ide: IdeKind, cwd: &Path) -> FormattedItem {
    let (icon, icon_color) = get_kind_icon(&change.kind);

    let (modifier, modifier_color) = match change.change_type {
        ChangeType::Added => ("+", color::GREEN),
        ChangeType::Modified => ("~", color::YELLOW),
        ChangeType::Deleted => ("-", color::RED),
    };

    // Truncate name for compact display
    let name = truncate_middle(&change.name, 20);
    let styled_name = colorize_scope_prefix(&name);

    // Wrap name in hyperlink if we have file path info
    let linked_name = if let Some(ref path) = change.file_path {
        let abs_path = cwd.join(path).to_string_lossy().to_string();
        let url = ide.file_url(&abs_path, change.line_number);
        hyperlink(&url, &styled_name)
    } else {
        styled_name
    };

    // Compact stats (no alignment)
    let stats = if change.additions > 0 || change.deletions > 0 {
        let del = if change.deletions > 0 {
            format!("{}−{}{}", fg(color::RED), change.deletions, RESET)
        } else {
            String::new()
        };
        let add = if change.additions > 0 {
            format!("{}+{}{}", fg(color::GREEN), change.additions, RESET)
        } else {
            String::new()
        };
        format!(" {}{}", del, add)
    } else {
        String::new()
    };

    let text = format!(
        "{}{}{}{}{}{}{}{}",
        fg(modifier_color),
        modifier,
        RESET,
        fg(icon_color),
        icon,
        RESET,
        linked_name,
        stats
    );

    // Calculate display width (hyperlink escape sequences don't contribute to visual width)
    let stats_width = if change.additions > 0 || change.deletions > 0 {
        1 + (if change.deletions > 0 {
            1 + digit_count(change.deletions)
        } else {
            0
        }) + (if change.additions > 0 {
            1 + digit_count(change.additions)
        } else {
            0
        })
    } else {
        0
    };
    let width = 1 + 1 + name.chars().count() + stats_width; // modifier + icon + name + stats

    FormattedItem { text, width }
}

/// Format change stats with aligned columns
fn format_change_stats(
    additions: usize,
    deletions: usize,
    del_width: usize,
    add_width: usize,
) -> String {
    if additions == 0 && deletions == 0 {
        // No stats to show - just padding
        return format!("{:width$}", "", width = 1 + del_width + 1 + add_width);
    }

    // For right-aligned columns: padding goes on the left, number on the right
    // When a value is 0, we just use padding (no number string)
    let del_num_width = if deletions > 0 {
        1 + digit_count(deletions)
    } else {
        0
    };
    let del_padding = del_width.saturating_sub(del_num_width);

    let add_num_width = if additions > 0 {
        1 + digit_count(additions)
    } else {
        0
    };
    let add_padding = add_width.saturating_sub(add_num_width);

    let del_str = if deletions > 0 {
        format!("{}−{}{}", fg(color::RED), deletions, RESET)
    } else {
        String::new()
    };

    let add_str = if additions > 0 {
        format!("{}+{}{}", fg(color::GREEN), additions, RESET)
    } else {
        String::new()
    };

    format!(
        " {:del_pad$}{} {:add_pad$}{}",
        "",
        del_str,
        "",
        add_str,
        del_pad = del_padding,
        add_pad = add_padding
    )
}

/// A packed row with its text and how many items it contains
struct PackedRow {
    text: String,
    item_count: usize,
}

/// Pack items into rows with 2-space margin between items
fn pack_items_into_rows(items: &[FormattedItem], max_width: usize) -> Vec<PackedRow> {
    let mut rows = Vec::new();
    let mut current_row = String::new();
    let mut current_width = 0usize;
    let mut current_count = 0usize;
    const MARGIN: usize = 2;

    for item in items {
        let needed = if current_row.is_empty() {
            item.width
        } else {
            item.width + MARGIN
        };

        if current_width + needed <= max_width {
            if !current_row.is_empty() {
                current_row.push_str("  "); // 2-space margin
            }
            current_row.push_str(&item.text);
            current_width += needed;
            current_count += 1;
        } else {
            if !current_row.is_empty() {
                rows.push(PackedRow {
                    text: current_row,
                    item_count: current_count,
                });
            }
            current_row = item.text.clone();
            current_width = item.width;
            current_count = 1;
        }
    }

    if !current_row.is_empty() {
        rows.push(PackedRow {
            text: current_row,
            item_count: current_count,
        });
    }

    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    fn thread(url: &str, text: Option<&str>) -> SlackThread {
        SlackThread {
            url: url.to_string(),
            posted_at: 1_754_404_040,
            channel: Some("builder".to_string()),
            author: Some("Elisa".to_string()),
            text: text.map(str::to_string),
        }
    }

    #[test]
    fn snippet_rows_follow_their_link_and_count_toward_the_widget_height() {
        let threads = vec![
            thread(
                "https://t.slack.com/archives/C1/p1754404040000001",
                Some("first message"),
            ),
            thread("https://t.slack.com/archives/C1/p1754404040000002", None),
            thread(
                "https://t.slack.com/archives/C1/p1754404040000003",
                Some("third message"),
            ),
        ];
        assert_eq!(slack_row_count(&threads), 5);

        let describe = |offset: usize| match slack_rows(&threads).nth(offset) {
            Some(SlackRow::Link(thread)) => format!("link {}", &thread.url[thread.url.len() - 1..]),
            Some(SlackRow::Snippet(_, text)) => format!("snippet {text}"),
            None => "none".to_string(),
        };
        assert_eq!(describe(0), "link 1");
        assert_eq!(describe(1), "snippet first message");
        assert_eq!(describe(2), "link 2");
        assert_eq!(describe(3), "link 3");
        assert_eq!(describe(4), "snippet third message");
        assert_eq!(describe(5), "none");
    }

    #[test]
    fn snippet_row_is_indented_italic_and_cut_at_the_column_edge() {
        let thread = thread(
            "https://t.slack.com/archives/C1/p1754404040000001",
            Some("It looks like phx4.5 faces have their voices displayed as Haiyao"),
        );
        let row = slack_snippet_row(&thread, thread.text.as_deref().unwrap(), 30);
        assert!(row.starts_with(&format!("  {ITALIC}")));
        assert!(row.contains("It looks like phx4.5 faces …"));
        assert!(row.contains(&thread.url));
        assert_eq!(strip_ansi_len(&row), 30);

        assert!(slack_snippet_row(&thread, "text", 2).is_empty());
    }

    #[test]
    fn first_title_row_prefixes_the_identity_chip() {
        let mark = SessionMark::from_seed("changes-title");
        let row = format_session_title_row("Fix the title chip", 40, mark);
        assert!(row.contains(mark.glyph));
        assert!(row.contains("Fix the title chip"));
        assert!(row.contains(&fg(color::LIGHT_BLUE)));
        assert_eq!(
            strip_ansi_len(&row),
            mark.width() + 1 + "Fix the title chip".len()
        );
    }

    #[test]
    fn official_title_row_matches_the_pr_board_identity() {
        let mut pr = SessionPr::test_stub(42, "acme", "widgets");
        pr.primary = true;
        pr.title = "Ship the title chip".to_string();
        pr.url = "https://github.com/acme/widgets/pull/42".to_string();
        let mark = SessionMark::from_seed("changes-pr-title");
        let row = format_official_title_row(&pr, 60, Some(mark));
        assert!(row.contains(mark.glyph));
        assert!(row.contains("#42"));
        assert!(row.contains("Ship the title chip"));
        assert!(row.contains(&pr.url));
        assert!(row.contains(BOLD));
        assert!(row.contains(UNDERLINE));
        assert_eq!(
            strip_ansi_len(&row),
            mark.width() + 1 + "#42: Ship the title chip".len()
        );
    }

    #[test]
    fn title_stack_matches_the_pr_board_session_view() {
        let mut pr = SessionPr::test_stub(1578, "acme", "widgets");
        pr.primary = true;
        pr.title = "preserve typed prompts".to_string();
        pr.url = "https://github.com/acme/widgets/pull/1578".to_string();
        let titles = SessionTitleHierarchy {
            official_pr: Some(&pr),
            generated_title: Some("E2E Test Coverage Verification"),
        };
        let mark = SessionMark::from_seed("changes-stack");

        let row1 = title_header_row(titles, 1, 80, mark).expect("session title");
        assert!(row1.contains(mark.glyph), "{row1}");
        assert!(row1.contains("E2E Test Coverage Verification"), "{row1}");
        assert!(!row1.contains("#1578"), "{row1}");
        assert!(row1.contains(&fg(color::LIGHT_BLUE)), "{row1}");

        let row2 = title_header_row(titles, 2, 80, mark).expect("PR title");
        assert!(!row2.contains(mark.glyph), "{row2}");
        assert!(row2.contains("#1578"), "{row2}");
        assert!(row2.contains("preserve typed prompts"), "{row2}");
        assert!(row2.contains(&fg(color::PURPLE)), "{row2}");
        assert!(row2.contains(&pr.url), "{row2}");
        assert!(title_header_row(titles, 3, 80, mark).is_none());
    }

    #[test]
    fn pr_keeps_the_chip_when_it_is_the_only_title() {
        let mut pr = SessionPr::test_stub(42, "acme", "widgets");
        pr.primary = true;
        pr.title = "Ship the title chip".to_string();
        let titles = SessionTitleHierarchy {
            official_pr: Some(&pr),
            generated_title: None,
        };
        let mark = SessionMark::from_seed("changes-pr-only");
        let row1 = title_header_row(titles, 1, 80, mark).expect("PR title");
        assert!(row1.contains(mark.glyph), "{row1}");
        assert!(row1.contains("#42"), "{row1}");
        assert!(title_header_row(titles, 2, 80, mark).is_none());
    }
}
