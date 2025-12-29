//! Git widget - displays git repository status
//!
//! Shows branch name and file status with change bars.
//! Automatically uses multiple columns when there are more files than rows.

use std::io::{Stdout, Write};
use std::path::Path;

use anyhow::Result;

use crate::ide::IdeKind;
use crate::terminal::escape::{self, color, fg, hyperlink, RESET};
use crate::git::{FileStatus, GitState};
use super::utils::{compute_unique_display_names, create_folder_bar, digit_count, format_diff_stats, format_diff_stats_aligned, get_filename, strip_ansi_len, truncate_path};

/// Dynamic column widths computed from actual file data
#[derive(Clone, Copy)]
struct StatsColumnWidths {
    del_num: usize,   // width for "−N" column
    del_bar: usize,   // width for deletion bar (left half)
    add_bar: usize,   // width for addition bar (right half)
    add_num: usize,   // width for "+N" column
}

impl StatsColumnWidths {
    /// Compute column widths from a list of files
    fn from_files(files: &[FileStatus]) -> Self {
        let mut max_del = 0usize;
        let mut max_add = 0usize;

        for file in files {
            if !file.is_folder {
                max_del = max_del.max(file.deletions);
                max_add = max_add.max(file.additions);
            }
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

        // Bar widths: based on magnitude (log10), minimum 1 if there are any
        let del_bar = if max_del > 0 {
            digit_count(max_del)
        } else {
            1  // minimum space
        };
        let add_bar = if max_add > 0 {
            digit_count(max_add)
        } else {
            1  // minimum space
        };

        Self { del_num, del_bar, add_bar, add_num }
    }

    /// Total width including spaces between columns
    fn total_width(&self) -> usize {
        // del_num + space + del_bar + add_bar + space + add_num
        self.del_num + 1 + self.del_bar + self.add_bar + 1 + self.add_num
    }
}


/// Draw the git widget at the given position
#[allow(clippy::too_many_arguments)]
pub fn draw_git_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    height: u16,
    git_state: &GitState,
    ide: IdeKind,
    cwd: &Path,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(pty_rows + 1 + row, col + 1))?;

    let files = &git_state.files;

    if row == 1 {
        // Header with branch name on left, status on right
        let branch = if git_state.branch.is_empty() {
            "Git"
        } else {
            &git_state.branch
        };
        let left = format!("{} {}{}", fg(color::LIGHT_GREEN), truncate_path(branch, 15), RESET);
        let left_len = strip_ansi_len(&left);

        // Right side: loading, "✓ Clean", or file count
        let right = if git_state.loading {
            format!("{}...{}", fg(color::GRAY), RESET)
        } else if files.is_empty() {
            format!("{}✓ Clean{}", fg(color::GREEN), RESET)
        } else {
            let count = files.len();
            let label = if count == 1 { "file" } else { "files" };
            format!("{}{} {}{}", fg(color::YELLOW), count, label, RESET)
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

    // Compute dynamic stats column widths based on actual data
    let stats_widths = StatsColumnWidths::from_files(files);

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
            let item = format_file_entry(file, display_name, width as usize, max_changes, &stats_widths, ide, cwd);
            write!(stdout, "{}", item)?;
            let content_len = strip_ansi_len(&item);
            let pad = (width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
        } else {
            write!(stdout, "{:width$}", "", width = width as usize)?;
        }
    } else if available_rows > 0 {
        // Multi-column layout
        let num_cols = num_files.div_ceil(available_rows);

        // Pre-compute natural widths for all entries (without truncation)
        let natural_widths: Vec<usize> = files
            .iter()
            .enumerate()
            .map(|(i, file)| {
                let entry = format_file_entry_natural(file, &display_names[i], max_changes, &stats_widths);
                strip_ansi_len(&entry)
            })
            .collect();

        // Calculate column widths based on actual content
        let mut col_widths: Vec<usize> = vec![0; num_cols];
        for (col_idx, col_width) in col_widths.iter_mut().enumerate() {
            let start = col_idx * available_rows;
            let end = (start + available_rows).min(num_files);
            for nw in natural_widths.iter().take(end).skip(start) {
                *col_width = (*col_width).max(*nw);
            }
            *col_width += 1; // Add margin
        }

        let total_width_needed: usize = col_widths.iter().sum();

        if total_width_needed <= width as usize {
            // Columns fit - distribute extra space proportionally to allow wider names
            let extra_space = width as usize - total_width_needed;
            let extra_per_col = extra_space / num_cols;

            // Apply extra space to column widths (cap name portion at 30 chars)
            for (col_idx, col_width) in col_widths.iter_mut().enumerate() {
                let max_name_in_col = (col_idx * available_rows..((col_idx + 1) * available_rows).min(num_files))
                    .map(|i| display_names[i].chars().count())
                    .max()
                    .unwrap_or(0);
                // Name width = col_width - 11 (icon + spaces + max bar)
                // So to allow up to 30 char names: col_width = 30 + 11 = 41 max
                let current_name_width = col_width.saturating_sub(11);
                let desired_name_width = max_name_in_col.min(30);
                if desired_name_width > current_name_width {
                    let needed = desired_name_width - current_name_width;
                    *col_width += needed.min(extra_per_col);
                }
            }

            // Render with proper alignment
            let mut output = String::new();
            for (col_idx, col_width) in col_widths.iter().enumerate() {
                let file_idx = col_idx * available_rows + row_idx;
                if file_idx < num_files {
                    let file = &files[file_idx];
                    let display_name = &display_names[file_idx];
                    let item = format_file_entry(file, display_name, *col_width, max_changes, &stats_widths, ide, cwd);
                    let item_len = strip_ansi_len(&item);
                    output.push_str(&item);
                    // Pad to column width
                    let pad = col_width.saturating_sub(item_len);
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
                .map(|(i, file)| format_file_compact(file, &display_names[i], max_changes, ide, cwd))
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

/// Format a file entry compactly (icon + name + stats) for wrapped mode
fn format_file_compact(file: &FileStatus, display_name: &str, max_changes: usize, ide: IdeKind, cwd: &Path) -> String {
    let (icon, icon_color) = get_status_icon_color(&file.status);

    if file.is_folder {
        let folder_name = get_filename(file.path.trim_end_matches('/'));
        let bar = create_folder_bar(file.file_count, max_changes, 4);
        format!("{}{}{}{}/ {}", fg(icon_color), icon, RESET, folder_name, bar)
    } else {
        // Compact: just numbers, no bar
        let stats = format_diff_stats(file.additions, file.deletions, max_changes, 0);
        // Make file name a clickable hyperlink
        let abs_path = cwd.join(&file.path).to_string_lossy().to_string();
        let url = ide.file_url(&abs_path, None);
        let linked_name = hyperlink(&url, display_name);
        format!("{}{}{}{} {}", fg(icon_color), icon, RESET, linked_name, stats)
    }
}

/// Get icon and color for a git status code
fn get_status_icon_color(status: &str) -> (&'static str, u8) {
    match status {
        "M" => ("●", color::YELLOW),
        "A" => ("+", color::GREEN),
        "D" => ("−", color::RED),
        "??" | "?" => ("?", color::CYAN),
        _ => ("•", color::FAINT),
    }
}

/// Format a file entry at its natural width (no truncation) to measure actual size
#[allow(unused_variables)]
fn format_file_entry_natural(file: &FileStatus, display_name: &str, max_changes: usize, stats_widths: &StatsColumnWidths) -> String {
    let (icon, icon_color) = get_status_icon_color(&file.status);

    if file.is_folder {
        let folder_name = get_filename(file.path.trim_end_matches('/'));
        let count_display = if file.file_count == 0 {
            format!("{}0 files{}", fg(color::DARK_GRAY), RESET)
        } else {
            format!("{}{} files{}", fg(color::GRAY), file.file_count, RESET)
        };
        let bar = create_folder_bar(file.file_count, max_changes, 8);
        format!(
            "{}{}{} {}/ {} {}",
            fg(icon_color), icon, RESET, folder_name, count_display, bar
        )
    } else {
        // Natural width with aligned stats columns
        let stats = format_diff_stats_aligned(
            file.additions,
            file.deletions,
            true,
            stats_widths.del_num,
            stats_widths.del_bar,
            stats_widths.add_bar,
            stats_widths.add_num,
        );
        format!(
            "{}{}{} {} {}",
            fg(icon_color), icon, RESET, display_name, stats
        )
    }
}

/// Format a single file entry for display
#[allow(unused_variables)]
fn format_file_entry(file: &FileStatus, display_name: &str, col_width: usize, max_changes: usize, stats_widths: &StatsColumnWidths, ide: IdeKind, cwd: &Path) -> String {
    // Status icon
    let (icon, icon_color) = get_status_icon_color(&file.status);

    if file.is_folder {
        // Folder display: "? folder_name/ N files +++++"
        let folder_name = get_filename(file.path.trim_end_matches('/'));
        let folder_display = format!("{}/", folder_name);

        // File count in gray, or dim if 0
        let count_display = if file.file_count == 0 {
            format!("{}0 files{}", fg(color::DARK_GRAY), RESET)
        } else {
            format!("{}{} files{}", fg(color::GRAY), file.file_count, RESET)
        };

        // Create folder bar (cyan) scaled relative to max
        let bar = create_folder_bar(file.file_count, max_changes, 8);

        // Calculate available width for folder name
        let count_len = if file.file_count == 0 { 7 } else { format!("{} files", file.file_count).len() };
        let name_width = col_width.saturating_sub(12 + count_len);
        let truncated_folder = truncate_path(&folder_display, name_width);

        format!(
            "{}{}{} {} {} {}",
            fg(icon_color), icon, RESET, truncated_folder, count_display, bar
        )
    } else {
        // Regular file display with aligned stats columns
        // Format: "icon name[padded]  −N ▓▓ ████ +M"
        // Overhead: icon(1) + space(1) + space(1) + stats(dynamic)
        let overhead = 3 + stats_widths.total_width();
        let name_width = col_width.saturating_sub(overhead);
        let truncated_name = truncate_path(display_name, name_width);

        // Pad filename to fixed width so stats columns align
        let name_char_count = truncated_name.chars().count();
        let name_padding = name_width.saturating_sub(name_char_count);

        // Make file name a clickable hyperlink
        let abs_path = cwd.join(&file.path).to_string_lossy().to_string();
        let url = ide.file_url(&abs_path, None);
        let linked_name = hyperlink(&url, &truncated_name);

        // Format stats with aligned columns
        let stats = format_diff_stats_aligned(
            file.additions,
            file.deletions,
            true,  // show bar
            stats_widths.del_num,
            stats_widths.del_bar,
            stats_widths.add_bar,
            stats_widths.add_num,
        );

        format!(
            "{}{}{} {}{:pad$} {}",
            fg(icon_color), icon, RESET, linked_name, "", stats, pad = name_padding
        )
    }
}
