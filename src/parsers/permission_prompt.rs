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

    // Only check the last ~10 lines to avoid false positives from scrollback
    let last_lines: String = stripped
        .lines()
        .rev()
        .take(10)
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>()
        .join("\n");

    // Don't report interrupted if Claude is actively thinking/working
    // These indicators mean we're past the interrupted state
    if last_lines.contains("Running")
        || last_lines.contains("Forging")
        || last_lines.contains("Thinking")
    {
        return false;
    }

    // Look for various interrupted/cancelled messages from Claude Code
    // "Interrupted · What should Claude do instead?" - during thinking
    // "User rejected" - when user rejects a permission
    if last_lines.contains("Interrupted") && last_lines.contains("What should Claude do instead") {
        return true;
    }
    if last_lines.contains("User rejected") {
        return true;
    }

    false
}

/// A single permission option extracted from the screen
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOption {
    /// Option number (1, 2, 3, etc.)
    pub number: u32,
    /// Full text of the option (plain text, no HTML)
    pub text: String,
    /// Whether this option is currently selected (has ❯ indicator)
    pub selected: bool,
}

/// Parsed permission prompt from screen content
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PermissionPrompt {
    /// The question being asked (e.g., "Do you want to create test-permission.txt?")
    pub question: Option<String>,
    /// List of available options
    pub options: Vec<PermissionOption>,
    /// Whether "Tab to add additional instructions" is available
    pub allows_tab_instructions: bool,
}

impl PermissionPrompt {
    /// Parse permission options from screen content
    ///
    /// Extracts the question and numbered options from Claude Code's permission dialog.
    /// Looks for patterns like:
    ///   Do you want to create test-file.txt?
    ///   ❯ 1. Yes
    ///     2. Yes, and always allow...
    ///     3. No
    pub fn parse(screen_content: &str) -> Option<Self> {
        let stripped = strip_ansi_codes(screen_content);

        // Look for numbered options pattern
        let option_re = Regex::new(r"(?m)^[\s❯>]*(\d+)\.\s+(.+)$").ok()?;

        // Find the question line and its position
        // "Do you want to" - standard permission prompts
        // "Would you like to" - Exit Plan Mode and similar
        // Use `contains` not `starts_with` because the question may be prefixed
        // (e.g., "Claude has written up a plan and is ready to execute. Would you like to proceed?")
        let lines: Vec<&str> = stripped.lines().collect();
        let question_idx = lines.iter().position(|line| {
            let trimmed = line.trim();
            trimmed.contains("Do you want to") || trimmed.contains("Would you like to")
        });

        // Fallback: if no question found, use the ❯ cursor line as anchor
        // The ❯ marker is unique to Claude Code's selection UI
        let (start_idx, question) = if let Some(idx) = question_idx {
            (idx + 1, Some(lines[idx].trim().to_string()))
        } else {
            // Find first ❯ line that also matches a numbered option
            let cursor_idx = lines.iter().position(|line| {
                line.contains('❯') && option_re.is_match(line)
            });
            if let Some(ci) = cursor_idx {
                // Scan backwards for the nearest question line (ends with ?)
                let q = lines[..ci]
                    .iter()
                    .rev()
                    .find(|l| l.trim().ends_with('?'))
                    .map(|l| l.trim().to_string());
                (ci, q)
            } else {
                (0, None)
            }
        };

        // Check for "Tab to add additional instructions" in footer
        let allows_tab_instructions = stripped
            .lines()
            .any(|line| line.contains("Tab to add"));

        let mut options = Vec::new();
        let mut current_number: Option<u32> = None;
        let mut current_text = String::new();
        let mut current_selected = false;

        for line in lines.iter().skip(start_idx) {
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

                // Start new option (use continue instead of ? to avoid aborting entire parse)
                let num: u32 = match caps.get(1).and_then(|m| m.as_str().parse().ok()) {
                    Some(n) => n,
                    None => continue,
                };
                let text = match caps.get(2) {
                    Some(m) => m.as_str().to_string(),
                    None => continue,
                };
                let selected = line.contains('❯') || line.trim_start().starts_with('>');

                current_number = Some(num);
                current_text = text;
                current_selected = selected;
            } else if current_number.is_some() {
                // Check if this is a continuation line
                let trimmed = line.trim();

                // Stop at footer lines
                if trimmed.starts_with("Esc to cancel")
                    || trimmed.starts_with("Tab to add")
                    || trimmed.starts_with("ctrl-g")
                    || trimmed.is_empty()
                    || trimmed.starts_with('─')
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
            Some(Self {
                question,
                options,
                allows_tab_instructions,
            })
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

/// Public wrapper for debug logging
pub fn strip_ansi_for_debug(text: &str) -> String {
    strip_ansi_codes(text)
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
            question: Some("Do you want to proceed?".to_string()),
            options: vec![
                PermissionOption { number: 1, text: "Yes".to_string(), selected: true },
                PermissionOption { number: 2, text: "No".to_string(), selected: false },
            ],
            allows_tab_instructions: false,
        };
        assert!(valid.is_valid());

        let invalid = PermissionPrompt {
            question: None,
            options: vec![
                PermissionOption { number: 1, text: "Something".to_string(), selected: true },
            ],
            allows_tab_instructions: false,
        };
        assert!(!invalid.is_valid());
    }

    #[test]
    fn test_detects_tab_instructions() {
        let screen_with_tab = r#"
Do you want to proceed?
❯ 1. Yes
   2. No

Esc to cancel · Tab to add additional instructions
"#;
        let prompt = PermissionPrompt::parse(screen_with_tab).unwrap();
        assert!(prompt.allows_tab_instructions);

        let screen_without_tab = r#"
Do you want to proceed?
❯ 1. Yes
   2. No

Esc to cancel
"#;
        let prompt = PermissionPrompt::parse(screen_without_tab).unwrap();
        assert!(!prompt.allows_tab_instructions);
    }

    #[test]
    fn test_parse_extracts_question() {
        let screen = r#"
Do you want to create test-file.txt?
❯ 1. Yes
   2. No
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        assert_eq!(prompt.question, Some("Do you want to create test-file.txt?".to_string()));
    }

    #[test]
    fn test_parse_would_you_like_question() {
        // Exit Plan Mode uses "Would you like to" instead of "Do you want to"
        let screen = r#"
Would you like to proceed?
❯ 1. Yes, and continue with the planned work
   2. Yes, but let me provide more guidance first
   3. No, let's discuss further
   4. No, cancel this plan

Esc to cancel · Tab to add additional instructions
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        assert_eq!(prompt.question, Some("Would you like to proceed?".to_string()));
        assert_eq!(prompt.options.len(), 4);
        assert_eq!(prompt.options[0].text, "Yes, and continue with the planned work");
        assert!(prompt.options[0].selected);
        assert_eq!(prompt.options[1].text, "Yes, but let me provide more guidance first");
        assert_eq!(prompt.options[2].text, "No, let's discuss further");
        assert_eq!(prompt.options[3].text, "No, cancel this plan");
    }

    #[test]
    fn test_parse_ignores_numbered_list_in_plan_content() {
        // Real-world scenario: Plan content has numbered list above the permission options
        // The parser should only match options AFTER the question line
        let screen = r#"
Ready to code?

Here is Claude's plan:
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
Plan: Create hello.txt

Task

Create a file named hello.txt in the working directory.

Implementation

1. Create /private/tmp/test-plan-mode/hello.txt with content: Hello!
2. Verify the file was created successfully

Verification

- Confirm file exists with ls hello.txt
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌

Would you like to proceed?

❯ 1. Yes, clear context and auto-accept edits (shift+tab)
  2. Yes, auto-accept edits
  3. Yes, manually approve edits
  4. Type here to tell Claude what to change

ctrl-g to edit in VS Code
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        // Should find the question
        assert_eq!(prompt.question, Some("Would you like to proceed?".to_string()));
        // Should have 4 options (not 6 - shouldn't include numbered list items from plan)
        assert_eq!(prompt.options.len(), 4);
        // First option should be the actual permission option, not plan content
        assert_eq!(prompt.options[0].text, "Yes, clear context and auto-accept edits (shift+tab)");
        assert!(prompt.options[0].selected);
        assert_eq!(prompt.options[1].text, "Yes, auto-accept edits");
        assert_eq!(prompt.options[2].text, "Yes, manually approve edits");
        assert_eq!(prompt.options[3].text, "Type here to tell Claude what to change");
        // Should be valid (has "Yes" option)
        assert!(prompt.is_valid());
    }

    #[test]
    fn test_parse_prefixed_question_line() {
        // Real-world: question is prefixed with descriptive text
        // "Claude has written up a plan and is ready to execute. Would you like to proceed?"
        let screen = r#"
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌
Plan: Create world.txt

Task

Create a file called world.txt with the content "hello world".

Steps

1. Create /private/tmp/test-plan-mode/world.txt with content hello world

Verification

- Read the file to confirm it contains "hello world"
╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌╌

Claude has written up a plan and is ready to execute. Would you like to proceed?

❯ 1. Yes, clear context and auto-accept edits (shift+tab)
  2. Yes, auto-accept edits
  3. Yes, manually approve edits
  4. Type here to tell Claude what to change

ctrl-g to edit in VS Code
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        assert_eq!(
            prompt.question,
            Some("Claude has written up a plan and is ready to execute. Would you like to proceed?".to_string())
        );
        assert_eq!(prompt.options.len(), 4);
        assert_eq!(prompt.options[0].text, "Yes, clear context and auto-accept edits (shift+tab)");
        assert!(prompt.options[0].selected);
        assert_eq!(prompt.options[3].text, "Type here to tell Claude what to change");
        assert!(prompt.is_valid());
    }

    #[test]
    fn test_parse_cursor_anchor_fallback() {
        // Fallback: no known question text, but ❯ cursor anchors the options
        let screen = r#"
Some plan content here...

1. Step one of the plan
2. Step two of the plan

Unrecognized prompt text here

❯ 1. Yes, proceed
  2. Yes, auto-accept
  3. No, go back
  4. Type feedback

ctrl-g to edit
"#;

        let prompt = PermissionPrompt::parse(screen).unwrap();
        // Should use ❯ anchor and find question by scanning backwards for ?
        assert_eq!(prompt.options.len(), 4);
        assert_eq!(prompt.options[0].text, "Yes, proceed");
        assert!(prompt.options[0].selected);
        assert!(prompt.is_valid());
    }
}
