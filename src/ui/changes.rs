//! Changes widget - displays semantic code changes grouped by language
//!
//! Shows parsed semantic changes (functions, classes, etc.) from git diffs,
//! organized by programming language.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::parsers::{ChangeNode, ChangeType, DiffSummary, LanguageChanges};
use crate::terminal::escape::{self, color, fg, RESET};

use super::utils::{get_change_icon_color, strip_ansi_len, truncate_middle};

/// Draw the changes widget at the given position
pub fn draw_changes_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    height: u16,
    diff_summary: &DiffSummary,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(pty_rows + 1 + row, col + 1))?;

    // Get changes grouped by language
    let by_language = diff_summary.by_language();

    // For row == 1, we need to show either loading or the first language header
    if row == 1 {
        if diff_summary.loading {
            let left = format!("{}Changes{}", fg(color::ORANGE), RESET);
            let left_len = strip_ansi_len(&left);
            let right = format!("{}...{}", fg(color::GRAY), RESET);
            let right_len = strip_ansi_len(&right);
            let pad = (width as usize).saturating_sub(left_len + right_len);
            write!(stdout, "{}{:pad$}{}", left, "", right, pad = pad)?;
            return Ok(());
        }

        if by_language.is_empty() {
            // No changes at all - show empty
            write!(stdout, "{:width$}", "", width = width as usize)?;
            return Ok(());
        }
    }

    if by_language.is_empty() {
        write!(stdout, "{:width$}", "", width = width as usize)?;
        return Ok(());
    }

    // Build a flat list of rows to render:
    // Each language has a header row, then item rows
    let rows_data = build_rows_for_display(&by_language, width, height);

    // Row index (0-based from row 1)
    let row_idx = (row - 1) as usize;

    if row_idx < rows_data.len() {
        let content = &rows_data[row_idx];
        write!(stdout, "{}", content)?;
        let content_len = strip_ansi_len(content);
        let pad = (width as usize).saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;
    } else {
        write!(stdout, "{:width$}", "", width = width as usize)?;
    }

    Ok(())
}

struct FormattedItem {
    text: String,
    width: usize,
}

/// Build rows for display, respecting available height
fn build_rows_for_display(
    by_language: &[LanguageChanges],
    width: u16,
    height: u16,
) -> Vec<String> {
    let mut rows = Vec::new();
    let available_rows = height.saturating_sub(2) as usize; // -2 for separator and first row

    for lang_changes in by_language {
        if rows.len() >= available_rows {
            break;
        }

        // Add language header
        let count = lang_changes.changes.len();
        let label = if count == 1 { "change" } else { "changes" };
        let header = format_header(&lang_changes.language, count, label, width as usize);
        rows.push(header);

        if rows.len() >= available_rows {
            break;
        }

        // Format items for this language
        let items: Vec<FormattedItem> = lang_changes
            .changes
            .iter()
            .map(|c| format_change_item(c))
            .collect();

        // Pack items into rows that fit within width
        let item_rows = pack_items_into_rows(&items, width as usize);

        for item_row in item_rows {
            if rows.len() >= available_rows {
                break;
            }
            rows.push(item_row);
        }
    }

    rows
}

/// Format a language header row
fn format_header(language: &str, count: usize, label: &str, width: usize) -> String {
    let left = format!("{}{}{}", fg(color::ORANGE), language, RESET);
    let left_len = strip_ansi_len(&left);

    let right = format!("{}{} {}{}", fg(color::ORANGE), count, label, RESET);
    let right_len = strip_ansi_len(&right);

    let pad = width.saturating_sub(left_len + right_len);
    format!("{}{:pad$}{}", left, "", right, pad = pad)
}

/// Format a single change item with icon and modifier indicator
fn format_change_item(change: &ChangeNode) -> FormattedItem {
    let (icon, icon_color) = get_change_icon_color(&change.kind);

    // Add modifier for change type: + for added, ~ for modified
    let modifier = match change.change_type {
        ChangeType::Added => format!("{}+{}", fg(color::GREEN), RESET),
        ChangeType::Modified => format!("{}~{}", fg(color::YELLOW), RESET),
        ChangeType::Deleted => format!("{}-{}", fg(color::RED), RESET),
    };

    let name = truncate_middle(&change.name, 25);
    let text = format!(
        "{}{}{}{} {}",
        modifier,
        fg(icon_color),
        icon,
        RESET,
        name
    );

    // Calculate display width (modifier is 1 char, icon varies, space, name)
    let width = 1 + strip_ansi_len(&format!("{}{}{}", fg(icon_color), icon, RESET)) + 1 + name.chars().count();

    FormattedItem { text, width }
}

/// Pack items into rows that fit within the given width
fn pack_items_into_rows(items: &[FormattedItem], max_width: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current_row = String::new();
    let mut current_width = 0usize;

    for item in items {
        let needed = if current_row.is_empty() {
            item.width
        } else {
            item.width + 1 // +1 for space separator
        };

        if current_width + needed <= max_width {
            if !current_row.is_empty() {
                current_row.push(' ');
            }
            current_row.push_str(&item.text);
            current_width += needed;
        } else {
            if !current_row.is_empty() {
                rows.push(current_row);
            }
            current_row = item.text.clone();
            current_width = item.width;
        }
    }

    if !current_row.is_empty() {
        rows.push(current_row);
    }

    rows
}
