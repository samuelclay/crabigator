//! Git widget - displays git repository status
//!
//! Shows branch name and file status with change bars.
//! Automatically uses multiple columns when there are more files than rows.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::git::{FileStatus, GitState};
use crate::utils::{compute_unique_display_names, create_diff_bar, create_folder_bar, get_filename, strip_ansi_len, truncate_path};

/// Draw the git widget at the given position
pub fn draw_git_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    height: u16,
    git_state: &GitState,
) -> Result<()> {
    write!(stdout, "\x1b[{};{}H", pty_rows + 1 + row, col + 1)?;

    let files = &git_state.files;

    if row == 1 {
        // Header with branch name on left, status on right
        let branch = if git_state.branch.is_empty() {
            "Git"
        } else {
            &git_state.branch
        };
        let left = format!("\x1b[38;5;114m {}\x1b[0m", truncate_path(branch, 15));
        let left_len = strip_ansi_len(&left);

        // Right side: loading, "✓ Clean", or file count
        let right = if git_state.loading {
            "\x1b[38;5;245m...\x1b[0m".to_string()
        } else if files.is_empty() {
            "\x1b[38;5;83m✓ Clean\x1b[0m".to_string()
        } else {
            let count = files.len();
            let label = if count == 1 { "file" } else { "files" };
            format!("\x1b[38;5;220m{} {}\x1b[0m", count, label)
        };
        let right_len = strip_ansi_len(&right);

        let pad = (width as usize).saturating_sub(left_len + right_len);
        write!(stdout, "{}{:pad$}{}", left, "", right, pad = pad)?;
        return Ok(());
    }

    if files.is_empty() {
        // No files to display, just clear the row
        write!(stdout, "{:width$}", "", width = width as usize)?;
        return Ok(());
    }

    // Compute unique display names for all files
    let paths: Vec<&str> = files.iter().map(|f| f.path.as_str()).collect();
    let display_names = compute_unique_display_names(&paths);

    // Calculate max changes for scaling the bar graph
    let max_changes = files
        .iter()
        .map(|f| if f.is_folder { f.file_count } else { f.total_changes() })
        .max()
        .unwrap_or(1)
        .max(1);

    // Available data rows (subtract 2: one for separator row 0, one for header row 1)
    let available_rows = height.saturating_sub(2) as usize;
    let num_files = files.len();

    // Row index (0-based, row 2 = index 0)
    let row_idx = (row - 2) as usize;

    // Decide layout: columns or single-line
    if available_rows > 0 && num_files <= available_rows {
        // Single column - simple case
        if row_idx < num_files {
            let file = &files[row_idx];
            let display_name = &display_names[row_idx];
            let item = format_file_entry(file, display_name, width as usize, max_changes);
            write!(stdout, "{}", item)?;
            let content_len = strip_ansi_len(&item);
            let pad = (width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            write!(stdout, "{:width$}", "", width = width as usize)?;
        }
    } else if available_rows > 0 {
        // Multi-column layout
        let num_cols = (num_files + available_rows - 1) / available_rows;

        // Pre-compute natural widths for all entries (without truncation)
        let natural_widths: Vec<usize> = files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let entry = format_file_entry_natural(file, &display_names[i], max_changes);
                strip_ansi_len(&entry)
            })
            .collect();

        // Calculate column widths based on actual content
        let mut col_widths: Vec<usize> = vec![0; num_cols];
        for col_idx in 0..num_cols {
            let start = col_idx * available_rows;
            let end = (start + available_rows).min(num_files);
            for file_idx in start..end {
                col_widths[col_idx] = col_widths[col_idx].max(natural_widths[file_idx]);
            }
            col_widths[col_idx] += 1; // Add margin
        }

        let total_width_needed: usize = col_widths.iter().sum();

        if total_width_needed <= width as usize {
            // Columns fit - distribute extra space proportionally to allow wider names
            let extra_space = width as usize - total_width_needed;
            let extra_per_col = extra_space / num_cols;

            // Apply extra space to column widths (cap name portion at 30 chars)
            for col_idx in 0..num_cols {
                let max_name_in_col = (col_idx * available_rows..((col_idx + 1) * available_rows).min(num_files))
                    .map(|i| display_names[i].chars().count())
                    .max()
                    .unwrap_or(0);
                // Name width = col_width - 11 (icon + spaces + max bar)
                // So to allow up to 30 char names: col_width = 30 + 11 = 41 max
                let current_name_width = col_widths[col_idx].saturating_sub(11);
                let desired_name_width = max_name_in_col.min(30);
                if desired_name_width > current_name_width {
                    let needed = desired_name_width - current_name_width;
                    col_widths[col_idx] += needed.min(extra_per_col);
                }
            }

            // Render with proper alignment
            let mut output = String::new();
            for col_idx in 0..num_cols {
                let file_idx = col_idx * available_rows + row_idx;
                if file_idx < num_files {
                    let file = &files[file_idx];
                    let display_name = &display_names[file_idx];
                    let item = format_file_entry(file, display_name, col_widths[col_idx], max_changes);
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
            // Build all compact items with their widths
            let items: Vec<String> = files
                .iter()
                .enumerate()
                .map(|(i, file)| format_file_compact(file, &display_names[i], max_changes))
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

/// Format a file entry compactly (icon + name + short bar) for wrapped mode
fn format_file_compact(file: &FileStatus, display_name: &str, max_changes: usize) -> String {
    let (icon, icon_color) = match file.status.as_str() {
        "M" => ("●", "38;5;220"),
        "A" => ("+", "38;5;83"),
        "D" => ("−", "38;5;203"),
        "??" | "?" => ("?", "38;5;45"),
        _ => ("•", "38;5;250"),
    };

    if file.is_folder {
        let folder_name = get_filename(file.path.trim_end_matches('/'));
        let bar = create_folder_bar(file.file_count, max_changes, 4);
        format!("\x1b[{}m{}\x1b[0m{}/ {}", icon_color, icon, folder_name, bar)
    } else {
        let bar = create_diff_bar(file.additions, file.deletions, max_changes, 4);
        format!("\x1b[{}m{}\x1b[0m{} {}", icon_color, icon, display_name, bar)
    }
}

/// Format a file entry at its natural width (no truncation) to measure actual size
fn format_file_entry_natural(file: &FileStatus, display_name: &str, max_changes: usize) -> String {
    let (icon, icon_color) = match file.status.as_str() {
        "M" => ("●", "38;5;220"),
        "A" => ("+", "38;5;83"),
        "D" => ("−", "38;5;203"),
        "??" | "?" => ("?", "38;5;45"),
        _ => ("•", "38;5;250"),
    };

    if file.is_folder {
        let folder_name = get_filename(file.path.trim_end_matches('/'));
        let count_display = if file.file_count == 0 {
            format!("\x1b[38;5;240m0 files\x1b[0m")
        } else {
            format!("\x1b[38;5;245m{} files\x1b[0m", file.file_count)
        };
        let bar = create_folder_bar(file.file_count, max_changes, 8);
        format!(
            "\x1b[{}m{}\x1b[0m {}/ {} {}",
            icon_color, icon, folder_name, count_display, bar
        )
    } else {
        let bar = create_diff_bar(file.additions, file.deletions, max_changes, 8);
        format!(
            "\x1b[{}m{}\x1b[0m {} {}",
            icon_color, icon, display_name, bar
        )
    }
}

/// Format a single file entry for display
fn format_file_entry(file: &FileStatus, display_name: &str, col_width: usize, max_changes: usize) -> String {
    // Status icon
    let (icon, icon_color) = match file.status.as_str() {
        "M" => ("●", "38;5;220"),  // Yellow
        "A" => ("+", "38;5;83"),   // Green
        "D" => ("−", "38;5;203"),  // Red
        "??" | "?" => ("?", "38;5;45"), // Cyan for untracked
        _ => ("•", "38;5;250"),
    };

    if file.is_folder {
        // Folder display: "? folder_name/ N files +++++"
        let folder_name = get_filename(file.path.trim_end_matches('/'));
        let folder_display = format!("{}/", folder_name);

        // File count in gray, or dim if 0
        let count_display = if file.file_count == 0 {
            format!("\x1b[38;5;240m0 files\x1b[0m")
        } else {
            format!("\x1b[38;5;245m{} files\x1b[0m", file.file_count)
        };

        // Create folder bar (cyan) scaled relative to max
        let bar = create_folder_bar(file.file_count, max_changes, 8);

        // Calculate available width for folder name
        let count_len = if file.file_count == 0 { 7 } else { format!("{} files", file.file_count).len() };
        let name_width = col_width.saturating_sub(12 + count_len);
        let truncated_folder = truncate_path(&folder_display, name_width);

        format!(
            "\x1b[{}m{}\x1b[0m {} {} {}",
            icon_color, icon, truncated_folder, count_display, bar
        )
    } else {
        // Regular file display - use the pre-computed unique display name
        // Overhead: icon(1) + space(1) + space(1) + bar(8 max) = 11
        let name_width = col_width.saturating_sub(11);
        let truncated_name = truncate_path(display_name, name_width);

        // Create scaled bar (max 8 chars)
        let bar = create_diff_bar(file.additions, file.deletions, max_changes, 8);

        format!(
            "\x1b[{}m{}\x1b[0m {} {}",
            icon_color, icon, truncated_name, bar
        )
    }
}
