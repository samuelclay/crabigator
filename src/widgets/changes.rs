//! Changes widget - displays semantic code changes
//!
//! Shows parsed semantic changes (functions, classes, etc.) from git diffs.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::parsers::DiffSummary;
use crate::utils::{get_change_icon_color, strip_ansi_len, truncate_path};

/// Draw the changes widget at the given position
pub fn draw_changes_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    diff_summary: &DiffSummary,
) -> Result<()> {
    write!(stdout, "\x1b[{};{}H", pty_rows + 1 + row, col + 1)?;

    if row == 1 {
        // Header
        let header = "\x1b[38;5;179m Changes\x1b[0m".to_string();
        write!(stdout, "{}", header)?;
        let pad = (width as usize).saturating_sub(strip_ansi_len(&header));
        write!(stdout, "{:pad$}", "", pad = pad)?;
        return Ok(());
    }

    // Collect all semantic changes across files
    let all_changes: Vec<_> = diff_summary
        .files
        .iter()
        .flat_map(|f| f.changes.iter().map(move |c| (f, c)))
        .collect();

    if all_changes.is_empty() {
        if row == 2 {
            let content = "\x1b[38;5;245mNo semantic changes\x1b[0m";
            write!(stdout, "{}", content)?;
            let pad = (width as usize).saturating_sub(strip_ansi_len(content));
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            write!(stdout, "{:width$}", "", width = width as usize)?;
        }
        return Ok(());
    }

    // Multi-column layout: calculate how many items per column
    let item_width = 20usize; // Each item takes ~20 chars
    let num_cols = (width as usize / item_width).max(1);
    let items_per_row = num_cols;

    // Row 2 onwards shows changes
    let row_idx = (row - 2) as usize;
    let start_idx = row_idx * items_per_row;

    let mut output = String::new();
    for col_idx in 0..items_per_row {
        let idx = start_idx + col_idx;
        if idx >= all_changes.len() {
            break;
        }

        let (_file, change) = &all_changes[idx];
        let (icon, color) = get_change_icon_color(&change.kind);
        let name = truncate_path(&change.name, item_width - 3);

        output.push_str(&format!("\x1b[{}m{}\x1b[0m{} ", color, icon, name));
    }

    write!(stdout, "{}", output)?;
    let content_len = strip_ansi_len(&output);
    let pad = (width as usize).saturating_sub(content_len);
    write!(stdout, "{:pad$}", "", pad = pad)?;

    Ok(())
}
