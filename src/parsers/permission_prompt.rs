//! Permission prompt parser
//!
//! Parses Claude Code terminal screen content to extract permission menu options.
//! This allows the web dashboard to display the exact same options shown in the terminal.
//! Also detects interrupted state from screen content.

use regex::Regex;
use serde::{Deserialize, Serialize};

/// Check if the screen shows an interrupted/cancelled state
/// (user hit Escape during permission or thinking)
pub fn is_interrupted(screen_content: &str) -> bool {
    let stripped = strip_ansi_codes(screen_content);

    // Look for various interrupted/cancelled messages from Claude Code
    // "Interrupted · What should Claude do instead?" - during thinking
    // "User rejected" - when user rejects a permission
    // Also check for empty prompt with no permission dialog visible
    if stripped.contains("Interrupted") && stripped.contains("What should Claude do instead") {
        return true;
    }
    if stripped.contains("User rejected") {
        return true;
    }

    false
}

/// A single permission option extracted from the screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOption {
    /// Option number (1, 2, 3, etc.)
    pub number: u32,
    /// Full text of the option
    pub text: String,
    /// Whether this option is currently selected (has ❯ indicator)
    pub selected: bool,
}

/// Parsed permission prompt from screen content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionPrompt {
    /// List of available options
    pub options: Vec<PermissionOption>,
}

impl PermissionPrompt {
    /// Parse permission options from screen content
    ///
    /// The screen content may contain ANSI escape codes which will be stripped.
    /// Looks for numbered options in the format:
    ///   ❯ 1. Option text
    ///     2. Another option
    ///     3. No
    pub fn parse(screen_content: &str) -> Option<Self> {
        let stripped = strip_ansi_codes(screen_content);

        // Look for numbered options pattern
        // Options are in format: [❯ or space] N. text
        // where N is 1, 2, 3, etc.
        let option_re = Regex::new(r"(?m)^[\s❯>]*(\d+)\.\s+(.+)$").ok()?;

        let mut options = Vec::new();
        let mut current_number: Option<u32> = None;
        let mut current_text = String::new();
        let mut current_selected = false;

        for line in stripped.lines() {
            // Check if this line starts a new option
            if let Some(caps) = option_re.captures(line) {
                // Save previous option if we have one
                if let Some(num) = current_number {
                    let text = current_text.trim().to_string();
                    if !text.is_empty() {
                        options.push(PermissionOption {
                            number: num,
                            text,
                            selected: current_selected,
                        });
                    }
                }

                // Start new option
                let num: u32 = caps.get(1)?.as_str().parse().ok()?;
                let text = caps.get(2)?.as_str().to_string();
                let selected = line.contains('❯') || line.trim_start().starts_with('>');

                current_number = Some(num);
                current_text = text;
                current_selected = selected;
            } else if current_number.is_some() {
                // Check if this is a continuation line (indented text that's part of previous option)
                let trimmed = line.trim();

                // Stop at footer lines
                if trimmed.starts_with("Esc to cancel")
                    || trimmed.starts_with("Tab to add")
                    || trimmed.is_empty()
                    || trimmed.starts_with('─') // Box drawing character, end of section
                {
                    break;
                }

                // Check if this looks like a continuation (not a new option number)
                if !trimmed.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
                    // Append to current option
                    if !current_text.is_empty() {
                        current_text.push(' ');
                    }
                    current_text.push_str(trimmed);
                } else {
                    // Looks like might be a new section, stop parsing
                    break;
                }
            }
        }

        // Don't forget the last option
        if let Some(num) = current_number {
            let text = current_text.trim().to_string();
            if !text.is_empty() {
                options.push(PermissionOption {
                    number: num,
                    text,
                    selected: current_selected,
                });
            }
        }

        if options.is_empty() {
            None
        } else {
            Some(Self { options })
        }
    }

    /// Check if this looks like a valid permission prompt
    /// (should have at least 2 options including a "Yes" and something else)
    pub fn is_valid(&self) -> bool {
        self.options.len() >= 2
            && self.options.iter().any(|o| o.text.to_lowercase().starts_with("yes"))
    }
}

/// Strip ANSI escape codes from text
fn strip_ansi_codes(text: &str) -> String {
    // Match ANSI escape sequences: ESC [ ... m (SGR) and ESC [ ... other codes
    let ansi_re = Regex::new(r"\x1b\[[0-9;]*[a-zA-Z]").unwrap();
    ansi_re.replace_all(text, "").to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_permission() {
        let screen = r#"
Do you want to proceed?
❯ 1. Yes
   2. Yes, and don't ask again
   3. No

Esc to cancel · Tab to add additional instructions
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        assert_eq!(prompt.options.len(), 3);
        assert_eq!(prompt.options[0].number, 1);
        assert_eq!(prompt.options[0].text, "Yes");
        assert!(prompt.options[0].selected);
        assert_eq!(prompt.options[1].number, 2);
        assert_eq!(prompt.options[1].text, "Yes, and don't ask again");
        assert!(!prompt.options[1].selected);
        assert_eq!(prompt.options[2].number, 3);
        assert_eq!(prompt.options[2].text, "No");
    }

    #[test]
    fn test_parse_long_option_text() {
        let screen = r#"
Do you want to proceed?
❯ 1. Yes
   2. Yes, and don't ask again for ./utils/ssh_hz.sh commands in /Users/sclay/projects/newsblur
   3. No

Esc to cancel
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        assert_eq!(prompt.options.len(), 3);
        assert!(prompt.options[1].text.contains("ssh_hz.sh"));
        assert!(prompt.options[1].text.contains("newsblur"));
    }

    #[test]
    fn test_strip_ansi() {
        let with_ansi = "\x1b[38;2;255;255;255m❯\x1b[m 1. Yes";
        let stripped = strip_ansi_codes(with_ansi);
        assert_eq!(stripped, "❯ 1. Yes");
    }

    #[test]
    fn test_is_valid() {
        let valid = PermissionPrompt {
            options: vec![
                PermissionOption { number: 1, text: "Yes".to_string(), selected: true },
                PermissionOption { number: 2, text: "No".to_string(), selected: false },
            ],
        };
        assert!(valid.is_valid());

        let invalid = PermissionPrompt {
            options: vec![
                PermissionOption { number: 1, text: "Something".to_string(), selected: true },
            ],
        };
        assert!(!invalid.is_valid());
    }
}
