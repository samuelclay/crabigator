//! Git widget - displays git repository status
//!
//! Shows branch name and file status with change bars.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::git::GitState;
use crate::utils::{create_diff_bar, get_filename, strip_ansi_len, truncate_path};

/// Draw the git widget at the given position
pub fn draw_git_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    git_state: &GitState,
) -> Result<()> {
    write!(stdout, "\x1b[{};{}H", pty_rows + 1 + row, col + 1)?;

    let files = &git_state.files;

    if row == 1 {
        // Header with branch name
        let branch = if git_state.branch.is_empty() {
            "Git"
        } else {
            &git_state.branch
        };
        let header = format!("\x1b[38;5;114m {}\x1b[0m", truncate_path(branch, 15));
        write!(stdout, "{}", header)?;
        let content_len = strip_ansi_len(&header);
        let pad = (width as usize).saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;
        return Ok(());
    }

    if files.is_empty() {
        if row == 2 {
            let content = "\x1b[38;5;83m✓ Clean\x1b[0m";
            write!(stdout, "{}", content)?;
            let pad = (width as usize).saturating_sub(strip_ansi_len(content));
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            write!(stdout, "{:width$}", "", width = width as usize)?;
        }
        return Ok(());
    }

    // Calculate max changes for scaling the bar graph
    let max_changes = files
        .iter()
        .map(|f| f.total_changes())
        .max()
        .unwrap_or(1)
        .max(1);

    // Get file for this row (row 2 = index 0, etc.)
    let file_idx = (row - 2) as usize;
    if let Some(file) = files.get(file_idx) {
        // File name (truncated)
        let name = get_filename(&file.path);
        let name_width = (width as usize).saturating_sub(12); // Leave room for bar
        let truncated_name = truncate_path(name, name_width);

        // Status icon
        let (icon, icon_color) = match file.status.as_str() {
            "M" => ("●", "38;5;220"), // Yellow
            "A" => ("+", "38;5;83"),  // Green
            "D" => ("−", "38;5;203"), // Red
            "?" => ("?", "38;5;245"), // Gray
            _ => ("•", "38;5;250"),
        };

        // Create scaled bar (max 8 chars)
        let bar = create_diff_bar(file.additions, file.deletions, max_changes, 8);

        let content = format!(
            "\x1b[{}m{}\x1b[0m {} {}",
            icon_color, icon, truncated_name, bar
        );

        write!(stdout, "{}", content)?;
        let content_len = strip_ansi_len(&content);
        let pad = (width as usize).saturating_sub(content_len);
        write!(stdout, "{:pad$}", "", pad = pad)?;
    } else {
        write!(stdout, "{:width$}", "", width = width as usize)?;
    }

    Ok(())
}
