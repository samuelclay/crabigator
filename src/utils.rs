//! Pure utility functions with no side effects
//!
//! This module contains helper functions used across the application
//! for string manipulation, formatting, and other pure computations.

use crate::parsers::NodeKind;

/// Truncate a path string, showing the end with ellipsis if too long
pub fn truncate_path(path: &str, max_len: usize) -> String {
    if path.len() <= max_len {
        path.to_string()
    } else if max_len <= 3 {
        "...".to_string()
    } else {
        // Show end of path (more useful)
        format!("…{}", &path[path.len() - (max_len - 1)..])
    }
}

/// Extract just the filename from a path
pub fn get_filename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
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
        return format!("\x1b[38;5;240m{}\x1b[0m", "·".repeat(max_width.min(2)));
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
        bar.push_str(&format!("\x1b[38;5;83m{}\x1b[0m", "+".repeat(add_chars)));
    }
    if del_chars > 0 {
        bar.push_str(&format!("\x1b[38;5;203m{}\x1b[0m", "-".repeat(del_chars)));
    }

    bar
}

/// Get icon and color for a semantic change type
pub fn get_change_icon_color(kind: &NodeKind) -> (&'static str, &'static str) {
    match kind {
        NodeKind::Class => ("◆", "38;5;141"),    // Purple - class
        NodeKind::Function => ("ƒ", "38;5;39"),  // Blue - function
        NodeKind::Method => ("·", "38;5;75"),    // Light blue - method
        NodeKind::Struct => ("▣", "38;5;179"),   // Orange - struct
        NodeKind::Enum => ("◇", "38;5;220"),     // Yellow - enum
        NodeKind::Trait => ("◈", "38;5;213"),    // Pink - trait
        NodeKind::Impl => ("▸", "38;5;114"),     // Green - impl
        NodeKind::Module => ("▢", "38;5;245"),   // Gray - module
        NodeKind::Const => ("●", "38;5;208"),    // Orange - const
        NodeKind::Other => ("•", "38;5;245"),    // Gray - other
    }
}

/// Calculate string length excluding ANSI escape sequences
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
            len += 1;
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
}
