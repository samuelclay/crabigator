//! Changes widget - displays semantic code changes
//!
//! Shows parsed semantic changes (functions, classes, etc.) from git diffs.
//! Automatically uses columns when items fit, falls back to single line.

use std::io::{Stdout, Write};

use anyhow::Result;

use unicode_width::UnicodeWidthStr;

use crate::escape::{self, color, fg, RESET};
use crate::parsers::{ChangeNode, DiffSummary};
use crate::utils::{get_change_icon_color, strip_ansi_len, truncate_middle, truncate_path};

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

    // Collect all semantic changes across files
    let all_changes: Vec<_> = diff_summary
        .files
        .iter()
        .flat_map(|f| f.changes.iter())
        .collect();

    if row == 1 {
        // Header with loading indicator or count on right
        let left = format!("{} Changes{}", fg(color::ORANGE), RESET);
        let left_len = strip_ansi_len(&left);

        let right = if diff_summary.loading {
            format!("{}...{}", fg(color::GRAY), RESET)
        } else if all_changes.is_empty() {
            "".to_string()
        } else {
            let count = all_changes.len();
            let label = if count == 1 { "change" } else { "changes" };
            format!("{}{} {}{}", fg(color::ORANGE), count, label, RESET)
        };
        let right_len = strip_ansi_len(&right);

        let pad = (width as usize).saturating_sub(left_len + right_len);
        write!(stdout, "{}{:pad$}{}", left, "", right, pad = pad)?;
        return Ok(());
    }

    if all_changes.is_empty() {
        write!(stdout, "{:width$}", "", width = width as usize)?;
        return Ok(());
    }

    // Available data rows (subtract 2: one for separator row 0, one for header row 1)
    let available_rows = height.saturating_sub(2) as usize;
    let num_changes = all_changes.len();

    // Row index (0-based, row 2 = index 0)
    let row_idx = (row - 2) as usize;

    // Decide layout: single column, multi-column, or single-line
    if available_rows > 0 && num_changes <= available_rows {
        // Single column - all items fit vertically
        if row_idx < num_changes {
            let change = all_changes[row_idx];
            let (icon, icon_color) = get_change_icon_color(&change.kind);
            let name = truncate_path(&change.name, (width as usize).saturating_sub(2));
            let item = format!("{}{}{} {}", fg(icon_color), icon, RESET, name);
            write!(stdout, "{}", item)?;
            let content_len = strip_ansi_len(&item);
            let pad = (width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            write!(stdout, "{:width$}", "", width = width as usize)?;
        }
    } else if available_rows > 0 {
        // Multi-column layout needed
        let num_cols = (num_changes + available_rows - 1) / available_rows;

        // Calculate column widths based on content
        let mut col_widths: Vec<usize> = vec![0; num_cols];
        for col_idx in 0..num_cols {
            let start = col_idx * available_rows;
            let end = (start + available_rows).min(num_changes);
            for idx in start..end {
                // Width: icon + name + space(1)
                let (icon, _) = get_change_icon_color(&all_changes[idx].kind);
                let icon_width = icon.width();
                let name_width = all_changes[idx].name.width();
                let entry_width = icon_width + name_width + 1;
                col_widths[col_idx] = col_widths[col_idx].max(entry_width);
            }
            col_widths[col_idx] += 1; // Add margin
        }

        let total_width_needed: usize = col_widths.iter().sum();

        if total_width_needed <= width as usize {
            // Columns fit - render with proper alignment
            let mut output = String::new();
            for col_idx in 0..num_cols {
                let idx = col_idx * available_rows + row_idx;
                if idx < num_changes {
                    let change = all_changes[idx];
                    let (icon, icon_color) = get_change_icon_color(&change.kind);
                    let max_name_len = col_widths[col_idx].saturating_sub(2);
                    let name = truncate_path(&change.name, max_name_len);
                    let item = format!("{}{}{} {}", fg(icon_color), icon, RESET, name);
                    let item_len = strip_ansi_len(&item);
                    output.push_str(&item);
                    // Pad to column width
                    let pad = col_widths[col_idx].saturating_sub(item_len);
                    output.push_str(&" ".repeat(pad));
                }
            }
            write!(stdout, "{}", output)?;
            let content_len = strip_ansi_len(&output);
            let pad = (width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            // Columns don't fit - wrap items across rows
            let items: Vec<String> = all_changes
                .iter()
                .map(|change| format_change_compact(change))
                .collect();
            let item_widths: Vec<usize> = items.iter().map(|s| strip_ansi_len(s)).collect();

            // Figure out which items go on which row by wrapping
            let mut rows: Vec<Vec<usize>> = Vec::new();
            let mut current_row: Vec<usize> = Vec::new();
            let mut current_width = 0usize;

            for (i, &item_width) in item_widths.iter().enumerate() {
                let needed = if current_row.is_empty() {
                    item_width
                } else {
                    item_width + 1 // +1 for space separator
                };

                if current_width + needed <= width as usize {
                    current_row.push(i);
                    current_width += needed;
                } else {
                    if !current_row.is_empty() {
                        rows.push(current_row);
                    }
                    current_row = vec![i];
                    current_width = item_width;
                }
            }
            if !current_row.is_empty() {
                rows.push(current_row);
            }

            // Render the row for this row_idx
            if row_idx < rows.len() {
                let mut output = String::new();
                for (j, &item_idx) in rows[row_idx].iter().enumerate() {
                    if j > 0 {
                        output.push(' ');
                    }
                    output.push_str(&items[item_idx]);
                }
                write!(stdout, "{}", output)?;
                let content_len = strip_ansi_len(&output);
                let pad = (width as usize).saturating_sub(content_len);
                write!(stdout, "{:pad$}", "", pad = pad)?;
            } else {
                write!(stdout, "{:width$}", "", width = width as usize)?;
            }
        }
    } else {
        write!(stdout, "{:width$}", "", width = width as usize)?;
    }

    Ok(())
}

/// Format a semantic change compactly (icon + name) for wrapped mode
fn format_change_compact(change: &ChangeNode) -> String {
    let (icon, icon_color) = get_change_icon_color(&change.kind);
    let name = truncate_middle(&change.name, 30);
    format!("{}{}{} {}", fg(icon_color), icon, RESET, name)
}
