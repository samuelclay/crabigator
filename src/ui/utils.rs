//! Pure utility functions with no side effects
//!
//! This module contains helper functions used across the application
//! for string manipulation, formatting, and other pure computations.

use unicode_width::UnicodeWidthChar;

use crate::terminal::escape::{color, fg, RESET};

/// Truncate a path string, showing the end with ellipsis if too long
pub fn truncate_path(path: &str, max_len: usize) -> String {
    let char_count = path.chars().count();
    if char_count <= max_len {
        path.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        // Show end of path (more useful)
        // Skip (char_count - (max_len - 1)) characters to show the last (max_len - 1) chars
        let skip = char_count - (max_len - 1);
        let suffix: String = path.chars().skip(skip).collect();
        format!("…{}", suffix)
    }
}

/// Truncate a string with ellipsis at ~30% from the beginning
/// e.g., "very_long_function_name_here" -> "very_lon…name_here"
pub fn truncate_middle(s: &str, max_len: usize) -> String {
    let char_count = s.chars().count();
    if char_count <= max_len {
        s.to_string()
    } else if max_len <= 1 {
        "…".to_string()
    } else {
        // Put ellipsis at 30% from start
        let available = max_len - 1; // -1 for the ellipsis
        let prefix_len = (available * 30) / 100;
        let suffix_len = available - prefix_len;

        let prefix: String = s.chars().take(prefix_len).collect();
        let suffix: String = s.chars().skip(char_count - suffix_len).collect();
        format!("{}…{}", prefix, suffix)
    }
}

/// Extract just the filename from a path
pub fn get_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// Given a list of paths, compute minimal unique display names for each.
/// Adds parent directory components only when needed to disambiguate duplicates.
/// Returns a vec of display names in the same order as input paths.
pub fn compute_unique_display_names(paths: &[&str]) -> Vec<String> {
    use std::collections::HashMap;

    // Start with just filenames
    let mut display_names: Vec<String> = paths
        .iter()
        .map(|p| get_filename(p).to_string())
        .collect();

    // Track how many path components we've used for each (0 = just filename)
    let mut depths: Vec<usize> = vec![0; paths.len()];

    // Keep expanding until all names are unique
    loop {
        // Find duplicates
        let mut name_counts: HashMap<&str, Vec<usize>> = HashMap::new();
        for (i, name) in display_names.iter().enumerate() {
            name_counts.entry(name.as_str()).or_default().push(i);
        }

        // Find indices that need more components
        let mut needs_expansion: Vec<usize> = Vec::new();
        for indices in name_counts.values() {
            if indices.len() > 1 {
                needs_expansion.extend(indices);
            }
        }

        if needs_expansion.is_empty() {
            break;
        }

        // Expand each duplicate by adding one more parent component
        for &i in &needs_expansion {
            depths[i] += 1;
            display_names[i] = get_path_suffix(paths[i], depths[i] + 1);
        }

        // Safety: if we've used the entire path, stop
        if depths.iter().all(|&d| d >= 10) {
            break;
        }
    }

    display_names
}

/// Get the last N components of a path (e.g., n=2 gives "parent/file.rs")
fn get_path_suffix(path: &str, n: usize) -> String {
    let parts: Vec<&str> = path.rsplit('/').take(n).collect();
    parts.into_iter().rev().collect::<Vec<_>>().join("/")
}

/// Format diff statistics with colored numbers and logarithmic-scaled bars
/// Returns formatted string like "−12 ▓▓████ +42" with appropriate colors
/// Layout: deletions | bar | additions (for visual balance)
/// Bar width scales logarithmically: 1-9=1, 10-99=2, 100-999=3, etc.
pub fn format_diff_stats(
    additions: usize,
    deletions: usize,
    _max_changes: usize,
    show_bar: usize,
) -> String {
    let total = additions + deletions;
    if total == 0 {
        return format!("{}·{}", fg(color::DARK_GRAY), RESET);
    }

    let mut result = String::new();

    // Calculate bar widths using log scale (order of magnitude + 1)
    let del_bar = if deletions > 0 {
        (deletions as f64).log10().floor() as usize + 1
    } else {
        0
    };
    let add_bar = if additions > 0 {
        (additions as f64).log10().floor() as usize + 1
    } else {
        0
    };

    // Format: −N ▓▓████ +M
    // Deletions on left
    if deletions > 0 {
        result.push_str(&format!("{}−{}{}", fg(color::RED), deletions, RESET));
    }

    // Bar in middle (if requested)
    if show_bar > 0 {
        if deletions > 0 {
            result.push(' ');
        }
        if del_bar > 0 {
            result.push_str(&format!("{}{}{}", fg(color::RED), "▓".repeat(del_bar), RESET));
        }
        if add_bar > 0 {
            result.push_str(&format!("{}{}{}", fg(color::GREEN), "█".repeat(add_bar), RESET));
        }
        if additions > 0 {
            result.push(' ');
        }
    } else if deletions > 0 && additions > 0 {
        result.push(' ');
    }

    // Additions on right
    if additions > 0 {
        result.push_str(&format!("{}+{}{}", fg(color::GREEN), additions, RESET));
    }

    result
}

/// Format diff statistics with column alignment
/// Columns: [del_num] [del_bar|add_bar] [add_num]
/// Each column is padded to specified widths for alignment across rows
/// Bar columns have separate widths: del_bar extends LEFT, add_bar extends RIGHT
pub fn format_diff_stats_aligned(
    additions: usize,
    deletions: usize,
    show_bar: bool,
    del_num_width: usize,   // width for deletion number column (including −)
    del_bar_width: usize,   // width for left bar column (deletions)
    add_bar_width: usize,   // width for right bar column (additions)
    add_num_width: usize,   // width for addition number column (including +)
) -> String {
    let total = additions + deletions;
    if total == 0 {
        // Center a dot in the total width
        let total_width = del_num_width + 1 + del_bar_width + add_bar_width + 1 + add_num_width;
        let pad = total_width / 2;
        return format!("{:>pad$}{}·{}{:pad$}", "", fg(color::DARK_GRAY), RESET, "", pad = pad);
    }

    let mut result = String::new();

    // Calculate actual bar sizes for this file
    let del_bar = if deletions > 0 {
        (deletions as f64).log10().floor() as usize + 1
    } else {
        0
    };
    let add_bar = if additions > 0 {
        (additions as f64).log10().floor() as usize + 1
    } else {
        0
    };

    // Deletion number column (right-aligned)
    if deletions > 0 {
        let del_str = format!("{}−{}{}", fg(color::RED), deletions, RESET);
        let actual_width = 1 + digit_count(deletions);
        let pad = del_num_width.saturating_sub(actual_width);
        result.push_str(&format!("{:pad$}{}", "", del_str, pad = pad));
    } else {
        result.push_str(&format!("{:del_num_width$}", "", del_num_width = del_num_width));
    }

    result.push(' ');

    // Bar columns: red extends LEFT from center, green extends RIGHT from center
    if show_bar {
        // Left bar: red right-aligned (grows leftward from center)
        let left_pad = del_bar_width.saturating_sub(del_bar);
        result.push_str(&format!("{:left_pad$}", "", left_pad = left_pad));
        if del_bar > 0 {
            result.push_str(&format!("{}{}{}", fg(color::RED), "▓".repeat(del_bar.min(del_bar_width)), RESET));
        }

        // Right bar: green left-aligned (grows rightward from center)
        if add_bar > 0 {
            result.push_str(&format!("{}{}{}", fg(color::GREEN), "█".repeat(add_bar.min(add_bar_width)), RESET));
        }
        let right_pad = add_bar_width.saturating_sub(add_bar);
        result.push_str(&format!("{:right_pad$}", "", right_pad = right_pad));
    } else {
        let bar_width = del_bar_width + add_bar_width;
        result.push_str(&format!("{:bar_width$}", "", bar_width = bar_width));
    }

    result.push(' ');

    // Addition number column (left-aligned)
    if additions > 0 {
        let add_str = format!("{}+{}{}", fg(color::GREEN), additions, RESET);
        result.push_str(&add_str);
        let actual_width = 1 + digit_count(additions);
        let pad = add_num_width.saturating_sub(actual_width);
        result.push_str(&format!("{:pad$}", "", pad = pad));
    } else {
        result.push_str(&format!("{:add_num_width$}", "", add_num_width = add_num_width));
    }

    result
}

/// Count digits in a number
fn digit_count(n: usize) -> usize {
    if n == 0 {
        1
    } else {
        (n as f64).log10().floor() as usize + 1
    }
}

/// Create a bar showing folder size (cyan colored for untracked folders)
/// Scaled relative to max_count (which could be file changes or file counts)
pub fn create_folder_bar(file_count: usize, max_count: usize, max_width: usize) -> String {
    if file_count == 0 {
        return format!("{}{}{}", fg(color::DARK_GRAY), "·".repeat(max_width.min(2)), RESET);
    }

    let max_count = max_count.max(1);
    let scaled = ((file_count as f64 / max_count as f64) * max_width as f64).ceil() as usize;
    let bar_width = scaled.min(max_width).max(1);

    format!("{}{}{}", fg(color::CYAN), "+".repeat(bar_width), RESET)
}

/// Calculate display width excluding ANSI escape sequences
/// Uses Unicode width to properly handle wide characters (e.g., ▣ = 2 columns)
pub fn strip_ansi_len(s: &str) -> usize {
    let mut len = 0;
    let mut in_escape = false;
    for c in s.chars() {
        if c == '\x1b' {
            in_escape = true;
        } else if in_escape {
            if c == 'm' {
                in_escape = false;
            }
        } else {
            len += c.width().unwrap_or(0);
        }
    }
    len
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate_path_short() {
        assert_eq!(truncate_path("foo.rs", 10), "foo.rs");
    }

    #[test]
    fn test_truncate_path_exact() {
        assert_eq!(truncate_path("foo.rs", 6), "foo.rs");
    }

    #[test]
    fn test_truncate_path_long() {
        // The ellipsis (…) counts as 1 char, so we get 9 more chars from the end
        assert_eq!(truncate_path("very_long_filename.rs", 10), "…lename.rs");
    }

    #[test]
    fn test_truncate_path_tiny_max() {
        assert_eq!(truncate_path("foo.rs", 2), "...");
    }

    #[test]
    fn test_truncate_path_multibyte() {
        // Test with multi-byte UTF-8 characters like 'ƒ' (used for function icons)
        let path = "ƒfoo ƒbar ƒbaz";
        let result = truncate_path(path, 8);
        // Should be "…ar ƒbaz" (7 chars + ellipsis)
        assert_eq!(result.chars().count(), 8);
        assert!(result.starts_with('…'));
    }

    #[test]
    fn test_get_filename() {
        assert_eq!(get_filename("src/app.rs"), "app.rs");
        assert_eq!(get_filename("app.rs"), "app.rs");
        assert_eq!(get_filename("a/b/c/d.txt"), "d.txt");
    }

    #[test]
    fn test_strip_ansi_len() {
        assert_eq!(strip_ansi_len("hello"), 5);
        assert_eq!(strip_ansi_len("\x1b[31mhello\x1b[0m"), 5);
        assert_eq!(strip_ansi_len("\x1b[38;5;141mtest\x1b[0m"), 4);
    }

    #[test]
    fn test_compute_unique_display_names_no_duplicates() {
        let paths = vec!["src/app.rs", "src/main.rs", "src/lib.rs"];
        let names = compute_unique_display_names(&paths);
        assert_eq!(names, vec!["app.rs", "main.rs", "lib.rs"]);
    }

    #[test]
    fn test_compute_unique_display_names_with_duplicates() {
        let paths = vec!["src/git/mod.rs", "src/parsers/mod.rs", "src/widgets/mod.rs", "src/app.rs"];
        let names = compute_unique_display_names(&paths);
        assert_eq!(names, vec!["git/mod.rs", "parsers/mod.rs", "widgets/mod.rs", "app.rs"]);
    }

    #[test]
    fn test_compute_unique_display_names_deeper_duplicates() {
        let paths = vec!["a/b/mod.rs", "a/c/mod.rs", "x/b/mod.rs"];
        let names = compute_unique_display_names(&paths);
        // a/b/mod.rs and x/b/mod.rs both have b/mod.rs, so need more context
        assert_eq!(names, vec!["a/b/mod.rs", "c/mod.rs", "x/b/mod.rs"]);
    }
}
