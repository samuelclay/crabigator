//! Changes widget - displays semantic code changes grouped by language
//!
//! Shows parsed semantic changes (functions, classes, etc.) from git diffs,
//! organized by programming language with per-change line stats.

use std::io::{Stdout, Write};
use std::path::Path;

use anyhow::Result;

use crate::ide::IdeKind;
use crate::parsers::{ChangeNode, ChangeType, DiffSummary, LanguageChanges, NodeKind};
use crate::terminal::escape::{self, color, fg, hyperlink, RESET};

use super::utils::{digit_count, strip_ansi_len, truncate_middle};
use super::WidgetArea;

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
    del_num: usize,   // width for "−N" column
    add_num: usize,   // width for "+N" column
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
            1  // just a space placeholder
        };
        let add_num = if max_add > 0 {
            1 + digit_count(max_add)
        } else {
            1  // just a space placeholder
        };

        Self { del_num, add_num }
    }

    /// Total width of stats columns
    fn total_width(&self) -> usize {
        // Format: " −N +M" = space + del_num + space + add_num
        1 + self.del_num + 1 + self.add_num
    }
}


/// Draw the changes widget at the given position
pub fn draw_changes_widget(
    stdout: &mut Stdout,
    area: WidgetArea,
    diff_summary: &DiffSummary,
    terminal_title: Option<&str>,
    ide: IdeKind,
    cwd: &Path,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(area.pty_rows + 1 + area.row, area.col + 1))?;

    // Get changes grouped by language
    let by_language = diff_summary.by_language();

    // For row == 1, show header: "Language, N changes" on left, terminal title on right
    if area.row == 1 {
        // Build left side: language + count or loading indicator
        let left = if diff_summary.loading {
            format!("{}Changes{} {}...{}", fg(color::ORANGE), RESET, fg(color::GRAY), RESET)
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
            // No changes
            String::new()
        };
        let left_len = strip_ansi_len(&left);

        // Build right side: terminal title if available (light blue for subtle distinction)
        let right = terminal_title
            .map(|t| format!("{}{}{}", fg(color::LIGHT_BLUE), t, RESET))
            .unwrap_or_default();
        let right_len = strip_ansi_len(&right);

        let pad = (area.width as usize).saturating_sub(left_len + right_len);
        write!(stdout, "{}{:pad$}{}", left, "", right, pad = pad)?;
        return Ok(());
    }

    if by_language.is_empty() {
        write!(stdout, "{:width$}", "", width = area.width as usize)?;
        return Ok(());
    }

    // Build rows to display
    let rows_data = build_rows_for_display(&by_language, area.width, area.height, ide, cwd);

    // Row index (0-based from row 1)
    let row_idx = (area.row - 1) as usize;

    if row_idx < rows_data.len() {
        let content = &rows_data[row_idx];
        write!(stdout, "{}", content)?;
        let content_len = strip_ansi_len(content);
        let pad = (area.width as usize).saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;
    } else {
        write!(stdout, "{:width$}", "", width = area.width as usize)?;
    }

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
    height: u16,
    ide: IdeKind,
    cwd: &Path,
) -> Vec<String> {
    let mut rows = Vec::new();
    let available_rows = height.saturating_sub(2) as usize; // -2 for separator and first row

    for lang_changes in by_language {
        if rows.len() >= available_rows {
            break;
        }

        // Sort changes by kind priority, then by total changes (descending),
        // then by name and file_path for deterministic ordering
        let mut sorted_changes: Vec<&ChangeNode> = lang_changes.changes.iter().collect();
        sorted_changes.sort_by(|a, b| {
            kind_priority(&a.kind).cmp(&kind_priority(&b.kind))
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

    // Wrap name in hyperlink if we have file path info
    let linked_name = if let Some(ref path) = change.file_path {
        let abs_path = cwd.join(path).to_string_lossy().to_string();
        let url = ide.file_url(&abs_path, change.line_number);
        hyperlink(&url, &name)
    } else {
        name.to_string()
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
        fg(modifier_color), modifier, RESET,
        fg(icon_color), icon, RESET,
        linked_name, "", stats,
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

    // Wrap name in hyperlink if we have file path info
    let linked_name = if let Some(ref path) = change.file_path {
        let abs_path = cwd.join(path).to_string_lossy().to_string();
        let url = ide.file_url(&abs_path, change.line_number);
        hyperlink(&url, &name)
    } else {
        name.to_string()
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
        fg(modifier_color), modifier, RESET,
        fg(icon_color), icon, RESET,
        linked_name, stats
    );

    // Calculate display width (hyperlink escape sequences don't contribute to visual width)
    let stats_width = if change.additions > 0 || change.deletions > 0 {
        1 + (if change.deletions > 0 { 1 + digit_count(change.deletions) } else { 0 })
          + (if change.additions > 0 { 1 + digit_count(change.additions) } else { 0 })
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
    let del_num_width = if deletions > 0 { 1 + digit_count(deletions) } else { 0 };
    let del_padding = del_width.saturating_sub(del_num_width);

    let add_num_width = if additions > 0 { 1 + digit_count(additions) } else { 0 };
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
        "", del_str, "", add_str,
        del_pad = del_padding, add_pad = add_padding
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
