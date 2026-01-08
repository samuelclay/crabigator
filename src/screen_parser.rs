//! Screen content parser for extracting permission options
//!
//! When Claude Code shows a permission prompt, the options are displayed on screen
//! as numbered items. This module parses the screen content to extract those options.

use regex::Regex;
use std::sync::LazyLock;

use crate::cloud::PromptOption;

/// Regex to match numbered options like "  1. Yes, allow once" or "❯ 1. Yes"
/// Handles optional cursor prefix (❯) and ANSI codes
/// Captures: group 1 = number, group 2 = label text
static OPTION_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[\s❯]*(\d+)\.\s+(.+)$").expect("Invalid regex"));

/// Regex to strip ANSI escape sequences (both full \x1b[...m and partial [...m)
static ANSI_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"\x1b\[[0-9;]*m|\[[0-9;]*m").expect("Invalid ANSI regex"));

/// Strip ANSI escape codes from a string
fn strip_ansi(s: &str) -> String {
    ANSI_REGEX.replace_all(s, "").to_string()
}

/// Parse permission options from screen content
///
/// Scans the screen content for numbered options and returns them as PromptOption structs.
/// Returns an empty vec if no options are found.
pub fn parse_permission_options(screen_content: &str) -> Vec<PromptOption> {
    let mut options = Vec::new();

    // Scan from bottom to top, looking for numbered options
    // Permission prompts typically appear at the bottom of the screen
    for line in screen_content.lines().rev() {
        // Strip ANSI codes before matching to handle colored/styled lines
        let clean_line = strip_ansi(line);
        if let Some(caps) = OPTION_REGEX.captures(&clean_line) {
            let num = caps.get(1).map(|m| m.as_str()).unwrap_or("1");
            let label = caps
                .get(2)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_else(|| "Option".to_string());

            options.push(PromptOption {
                value: num.to_string(),
                label,
                description: None,
            });
        } else if !options.is_empty() {
            // Stop once we hit a non-option line after finding options
            // This prevents picking up random numbered text from earlier in the screen
            break;
        }
    }

    // Reverse to restore original order (1, 2, 3, ...)
    options.reverse();
    options
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_options() {
        let screen = r#"
Allow Bash to run: ls /etc/hosts ?

  1. Yes, allow once
  2. Yes, allow for this session
  3. No, deny
  4. Don't ask again for this session
"#;
        let options = parse_permission_options(screen);
        assert_eq!(options.len(), 4);
        assert_eq!(options[0].value, "1");
        assert_eq!(options[0].label, "Yes, allow once");
        assert_eq!(options[1].value, "2");
        assert_eq!(options[2].value, "3");
        assert_eq!(options[2].label, "No, deny");
        assert_eq!(options[3].value, "4");
    }

    #[test]
    fn test_empty_screen() {
        let options = parse_permission_options("");
        assert!(options.is_empty());
    }

    #[test]
    fn test_no_options() {
        let screen = "Some random text\nwithout any numbered options";
        let options = parse_permission_options(screen);
        assert!(options.is_empty());
    }

    #[test]
    fn test_options_at_bottom_only() {
        let screen = r#"
1. This is some old numbered content
2. That should be ignored

Some other text

  1. Allow
  2. Deny
"#;
        let options = parse_permission_options(screen);
        // Should only capture the bottom options
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Allow");
        assert_eq!(options[1].label, "Deny");
    }

    #[test]
    fn test_strip_ansi_from_labels() {
        // Test with partial ANSI codes like [1m (bold) and [m (reset)
        let screen = "  1. [1mQuestion state [m - description\n  2. [1mPermission state [m - another";
        let options = parse_permission_options(screen);
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Question state  - description");
        assert_eq!(options[1].label, "Permission state  - another");
    }

    #[test]
    fn test_strip_full_ansi_codes() {
        // Test with full ANSI escape sequences
        let screen = "  1. \x1b[1mYes, allow\x1b[0m\n  2. \x1b[31mNo, deny\x1b[0m";
        let options = parse_permission_options(screen);
        assert_eq!(options.len(), 2);
        assert_eq!(options[0].label, "Yes, allow");
        assert_eq!(options[1].label, "No, deny");
    }

    #[test]
    fn test_cursor_prefix() {
        // Test with cursor prefix (❯) like Claude Code shows
        let screen = r#"
Do you want to create test-permission.txt?
❯ 1. Yes
  2. Yes, allow all edits during this session (shift+tab)
  3. Type here to tell Claude what to do differently

Esc to cancel
"#;
        let options = parse_permission_options(screen);
        assert_eq!(options.len(), 3);
        assert_eq!(options[0].value, "1");
        assert_eq!(options[0].label, "Yes");
        assert_eq!(options[1].value, "2");
        assert_eq!(options[1].label, "Yes, allow all edits during this session (shift+tab)");
        assert_eq!(options[2].value, "3");
        assert_eq!(options[2].label, "Type here to tell Claude what to do differently");
    }
}
