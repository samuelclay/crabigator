//! Pure utility functions with no side effects
//!
//! This module contains helper functions used across the application
//! for string manipulation, formatting, and other pure computations.

use unicode_width::UnicodeWidthChar;

use crate::escape::{color, fg, RESET};
use crate::parsers::NodeKind;

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

/// Format a number with K/M suffix for large values
pub fn format_number(n: u64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Create a scaled diff bar showing additions (green) and deletions (red)
/// Max width is `max_width` characters, scaled proportionally to `max_changes`
pub fn create_diff_bar(
    additions: usize,
    deletions: usize,
    max_changes: usize,
    max_width: usize,
) -> String {
    let total = additions + deletions;
    if total == 0 {
        return format!("{}{}{}", fg(color::DARK_GRAY), "·".repeat(max_width.min(2)), RESET);
    }

    // Scale to max_width based on max_changes
    let scaled_total = ((total as f64 / max_changes as f64) * max_width as f64).ceil() as usize;
    let bar_width = scaled_total.min(max_width).max(1);

    // Distribute bar width between additions and deletions
    let add_chars = if total > 0 {
        ((additions as f64 / total as f64) * bar_width as f64).round() as usize
    } else {
        0
    };
    let del_chars = bar_width.saturating_sub(add_chars);

    let mut bar = String::new();
    if add_chars > 0 {
        bar.push_str(&format!("{}{}{}", fg(color::GREEN), "+".repeat(add_chars), RESET));
    }
    if del_chars > 0 {
        bar.push_str(&format!("{}{}{}", fg(color::RED), "-".repeat(del_chars), RESET));
    }

    bar
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

/// Get icon and color for a semantic change type.
/// Icons with ambiguous unicode width include a trailing space to prevent overlap.
pub fn get_change_icon_color(kind: &NodeKind) -> (&'static str, u8) {
    match kind {
        NodeKind::Class => ("◆", color::PURPLE),        // Purple - class
        NodeKind::Function => ("ƒ", color::BLUE),       // Blue - function
        NodeKind::Method => ("·", color::LIGHT_BLUE),   // Light blue - method
        NodeKind::Struct => ("▣ ", color::ORANGE),      // Orange - struct (space for ambiguous width)
        NodeKind::Enum => ("◇", color::YELLOW),         // Yellow - enum
        NodeKind::Trait => ("◈", color::PINK),          // Pink - trait
        NodeKind::Impl => ("▸", color::LIGHT_GREEN),    // Green - impl
        NodeKind::Module => ("▢ ", color::GRAY),        // Gray - module (space for ambiguous width)
        NodeKind::Const => ("●", color::DARK_ORANGE),   // Orange - const
        NodeKind::Other => ("•", color::GRAY),          // Gray - other
    }
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
    fn test_format_number() {
        assert_eq!(format_number(500), "500");
        assert_eq!(format_number(1500), "1.5K");
        assert_eq!(format_number(1500000), "1.5M");
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
