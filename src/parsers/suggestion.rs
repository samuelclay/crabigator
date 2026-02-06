//! Autocomplete suggestion parser
//!
//! Extracts autocomplete/suggestion text from Claude Code's PTY output.
//! Claude Code shows suggestions as dim/gray text on the input line (e.g., "push it").
//! The user can press Enter to accept or Tab to edit.
//!
//! The suggestion text appears in two forms:
//! 1. Raw PTY bytes from ink: `❯ \x1b[7m{first}\x1b[27m\x1b[2m{rest}\x1b[22m`
//! 2. vt100 regenerated: `❯ \x1b[7m{first}\x1b[2;27m{rest}\x1b[m`
//!
//! The vt100 crate combines SGR parameters (e.g., `\x1b[2;27m` instead of
//! `\x1b[27m\x1b[2m`) and uses short reset (`\x1b[m` instead of `\x1b[22m`).
//! We handle both formats.

use regex::Regex;
use std::sync::OnceLock;

/// Tracks autocomplete suggestions from raw PTY output.
///
/// Claude Code renders suggestions using this ANSI pattern:
/// `❯ \x1b[7m{first_char}` + dim/no-reverse transition + `{rest}` + reset
///
/// - `❯` prompt marker (with non-breaking space)
/// - Reverse video (`\x1b[7m`) on the first character (cursor highlight)
/// - Dim + no-reverse for remaining text (separate or combined SGR)
/// - Reset to end (dim-off or full reset)
pub struct SuggestionTracker {
    /// Current suggestion text (None if no suggestion visible)
    current: Option<String>,
    /// Buffer for incomplete PTY chunks (suggestion may span chunks)
    buffer: Vec<u8>,
}

/// Regex matching the suggestion pattern in both raw PTY and vt100 output.
/// Handles two ANSI formats:
/// - Raw PTY: `\x1b[27m\x1b[2m` (separate) + `\x1b[22m` (dim-off)
/// - vt100:   `\x1b[2;27m` (combined)       + `\x1b[m` or `\x1b[0m` (reset)
fn suggestion_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(
            r"❯[\x20\xa0]+\x1b\[7m([\x20-\x7e])(?:\x1b\[27m\x1b\[2m|\x1b\[2;27m)([^\x1b]+)\x1b\[(?:22|0?)m"
        ).unwrap()
    })
}

/// Regex matching any ❯ prompt line (with or without content).
/// Used to detect when the suggestion should be cleared — if the ❯ prompt
/// appears but doesn't match the suggestion pattern, the user started typing
/// or the prompt is empty.
fn prompt_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        // ❯ + space or nbsp, followed by any ANSI sequence
        Regex::new(r"❯[\x20\xa0]+\x1b\[").unwrap()
    })
}

impl SuggestionTracker {
    pub fn new() -> Self {
        Self {
            current: None,
            buffer: Vec::new(),
        }
    }

    /// Clear the current suggestion (e.g., when state leaves ready/complete).
    pub fn clear(&mut self) {
        self.current = None;
    }

    /// Process a chunk of raw PTY output bytes.
    /// Returns true if the suggestion changed.
    pub fn process(&mut self, data: &[u8]) -> bool {
        let text = match std::str::from_utf8(data) {
            Ok(s) => s.to_string(),
            Err(_) => {
                // Handle partial UTF-8 by buffering
                self.buffer.extend_from_slice(data);
                match std::str::from_utf8(&self.buffer) {
                    Ok(s) => {
                        let result = s.to_string();
                        self.buffer.clear();
                        result
                    }
                    Err(_) => {
                        // Limit buffer size to prevent unbounded growth
                        if self.buffer.len() > 4096 {
                            self.buffer.clear();
                        }
                        return false;
                    }
                }
            }
        };

        // Find the LAST suggestion match and LAST prompt match in this chunk.
        // A single PTY chunk may contain multiple ❯ lines (e.g., scrollback re-render
        // plus current input). We use the last occurrence to determine current state.
        let last_suggestion = suggestion_regex()
            .find_iter(&text)
            .last()
            .map(|m| (m.start(), m));

        let last_prompt = prompt_regex()
            .find_iter(&text)
            .last()
            .map(|m| m.start());

        // If we found a suggestion, check if it's the last ❯ in the chunk
        if let Some((suggestion_pos, _)) = &last_suggestion {
            let prompt_is_later = last_prompt.map(|p| p > *suggestion_pos).unwrap_or(false);

            if !prompt_is_later {
                // Suggestion is the last ❯ — extract it
                if let Some(caps) = suggestion_regex().captures_at(&text, *suggestion_pos) {
                    let first_char = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                    let rest = caps.get(2).map(|m| m.as_str()).unwrap_or("");
                    let suggestion = format!("{}{}", first_char, rest).trim().to_string();
                    if !suggestion.is_empty() {
                        let changed = self.current.as_ref() != Some(&suggestion);
                        self.current = Some(suggestion);
                        return changed;
                    }
                }
            }
        }

        // If the last ❯ prompt didn't match the suggestion pattern,
        // the suggestion was cleared (user typed something or prompt is empty)
        if self.current.is_some() && last_prompt.is_some() {
            self.current = None;
            return true;
        }

        false
    }

    /// Get the current suggestion text.
    pub fn current(&self) -> Option<&str> {
        self.current.as_deref()
    }

    /// Parse suggestion from vt100 screen content (rows_formatted output).
    /// This is the authoritative source for suggestion state — the vt100 screen
    /// buffer always reflects what's actually visible on the terminal.
    ///
    /// The vt100 crate optimizes runs of spaces into cursor-forward sequences
    /// (`\x1b[C`), so we replace those back to spaces before matching.
    ///
    /// Returns true if the suggestion state changed (appeared, updated, or cleared).
    pub fn parse_screen(&mut self, screen: &str) -> bool {
        // Replace cursor-forward sequences with spaces: ESC[C (1) and ESC[nC (n)
        let cuf_re = regex::Regex::new(r"\x1b\[(\d*)C").unwrap();
        let normalized = cuf_re.replace_all(screen, |caps: &regex::Captures| {
            let n: usize = caps.get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(1);
            " ".repeat(n)
        });
        if let Some(caps) = suggestion_regex().captures(&normalized) {
            let first_char = caps.get(1).map(|m| m.as_str()).unwrap_or("");
            let rest = caps.get(2).map(|m| m.as_str()).unwrap_or("");
            let suggestion = format!("{}{}", first_char, rest).trim().to_string();
            if !suggestion.is_empty() {
                let changed = self.current.as_ref() != Some(&suggestion);
                self.current = Some(suggestion);
                return changed;
            }
        }
        // No suggestion pattern found. If the ❯ prompt is on screen but without
        // a suggestion, clear the tracked suggestion (user typed or prompt is empty).
        // Don't clear if the prompt isn't visible at all (e.g., during thinking).
        if self.current.is_some() && prompt_regex().is_match(&normalized) {
            self.current = None;
            return true;
        }
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tracker_detects_suggestion() {
        let mut tracker = SuggestionTracker::new();
        // Real raw PTY format from E2E testing
        let data = "❯ \x1b[7mT\x1b[27m\x1b[2mry \"how do I log an error?\"\x1b[22m";
        assert!(tracker.process(data.as_bytes()));
        assert_eq!(
            tracker.current(),
            Some("Try \"how do I log an error?\"")
        );
    }

    #[test]
    fn test_tracker_detects_suggestion_with_nbsp() {
        let mut tracker = SuggestionTracker::new();
        let data = "❯\u{a0}\x1b[7mH\x1b[27m\x1b[2mello world\x1b[22m";
        assert!(tracker.process(data.as_bytes()));
        assert_eq!(tracker.current(), Some("Hello world"));
    }

    #[test]
    fn test_tracker_no_change_returns_false() {
        let mut tracker = SuggestionTracker::new();
        let data = "❯ \x1b[7mT\x1b[27m\x1b[2mry something\x1b[22m";
        assert!(tracker.process(data.as_bytes()));
        // Same suggestion again
        assert!(!tracker.process(data.as_bytes()));
    }

    #[test]
    fn test_tracker_clears_on_empty_prompt() {
        let mut tracker = SuggestionTracker::new();
        // Set a suggestion first
        let data = "❯ \x1b[7mH\x1b[27m\x1b[2mello\x1b[22m";
        tracker.process(data.as_bytes());
        assert!(tracker.current().is_some());

        // Empty prompt clears it
        let empty = "❯ \x1b[39;7m \x1b[27m   ";
        assert!(tracker.process(empty.as_bytes()));
        assert!(tracker.current().is_none());
    }

    #[test]
    fn test_tracker_clears_on_user_typing() {
        let mut tracker = SuggestionTracker::new();
        // Set a suggestion first
        let data = "❯ \x1b[7mH\x1b[27m\x1b[2mello\x1b[22m";
        tracker.process(data.as_bytes());
        assert!(tracker.current().is_some());

        // User types text (white, non-dim) — clears suggestion
        let typed = "❯ \x1b[38;2;255;255;255mok, sooo...";
        assert!(tracker.process(typed.as_bytes()));
        assert!(tracker.current().is_none());
    }

    #[test]
    fn test_tracker_ignores_non_prompt_data() {
        let mut tracker = SuggestionTracker::new();
        let data = "Some regular terminal output\r\n";
        assert!(!tracker.process(data.as_bytes()));
        assert!(tracker.current().is_none());
    }

    #[test]
    fn test_tracker_suggestion_in_larger_output() {
        let mut tracker = SuggestionTracker::new();
        // Suggestion embedded in larger PTY output (with line separator and other content)
        let data = "\x1b[0m───────\r\n\
                     ❯\u{a0}\x1b[7mf\x1b[27m\x1b[2mix the login bug\x1b[22m\r\n\
                     \x1b[0m───────\r\n";
        assert!(tracker.process(data.as_bytes()));
        assert_eq!(tracker.current(), Some("fix the login bug"));
    }

    #[test]
    fn test_tracker_updates_suggestion() {
        let mut tracker = SuggestionTracker::new();
        let data1 = "❯ \x1b[7mH\x1b[27m\x1b[2mello\x1b[22m";
        tracker.process(data1.as_bytes());
        assert_eq!(tracker.current(), Some("Hello"));

        let data2 = "❯ \x1b[7mW\x1b[27m\x1b[2morld\x1b[22m";
        assert!(tracker.process(data2.as_bytes()));
        assert_eq!(tracker.current(), Some("World"));
    }

    #[test]
    fn test_tracker_vt100_combined_sgr() {
        // vt100 crate combines SGR params: \x1b[2;27m instead of \x1b[27m\x1b[2m
        // and uses \x1b[m (short reset) instead of \x1b[22m
        let mut tracker = SuggestionTracker::new();
        let data = "❯\u{a0}\x1b[7mT\x1b[2;27mry \"create a util logging.py that...\"\x1b[m";
        assert!(tracker.process(data.as_bytes()));
        assert_eq!(
            tracker.current(),
            Some("Try \"create a util logging.py that...\"")
        );
    }

    #[test]
    fn test_tracker_vt100_zero_reset() {
        // vt100 may also use \x1b[0m (explicit zero reset)
        let mut tracker = SuggestionTracker::new();
        let data = "❯ \x1b[7mH\x1b[2;27mello world\x1b[0m";
        assert!(tracker.process(data.as_bytes()));
        assert_eq!(tracker.current(), Some("Hello world"));
    }

    #[test]
    fn test_parse_screen_vt100_format() {
        let mut tracker = SuggestionTracker::new();
        // Real vt100 rows_formatted() output from screen.txt
        let screen = "\x1b[0m───────\r\n\
                       ❯\u{a0}\x1b[7mT\x1b[2;27mry \"create a util logging.py that...\"\x1b[m   \r\n\
                       \x1b[0m───────\r\n";
        assert!(tracker.parse_screen(screen));
        assert_eq!(
            tracker.current(),
            Some("Try \"create a util logging.py that...\"")
        );
    }

    #[test]
    fn test_parse_screen_cursor_forward_spaces() {
        let mut tracker = SuggestionTracker::new();
        // vt100 optimizes spaces to cursor-forward (\x1b[C) sequences
        // Real output: ❯\xa0\x1b[7mT\x1b[2;27mry\x1b[C"refactor\x1b[Capp.rs"\x1b[0m
        let screen = "❯\u{a0}\x1b[7mT\x1b[2;27mry\x1b[C\"refactor\x1b[Capp.rs\"\x1b[0m";
        assert!(tracker.parse_screen(screen));
        assert_eq!(
            tracker.current(),
            Some("Try \"refactor app.rs\"")
        );
    }

    #[test]
    fn test_parse_screen_clears_when_prompt_has_no_suggestion() {
        let mut tracker = SuggestionTracker::new();
        // First set a suggestion
        let screen_with = "❯\u{a0}\x1b[7mT\x1b[2;27mry something\x1b[m";
        assert!(tracker.parse_screen(screen_with));
        assert_eq!(tracker.current(), Some("Try something"));

        // Screen now shows ❯ prompt without suggestion (user typed or empty prompt)
        let screen_without = "❯ \x1b[38;2;255;255;255mhello world";
        assert!(tracker.parse_screen(screen_without));
        assert!(tracker.current().is_none());
    }

    #[test]
    fn test_parse_screen_does_not_clear_when_no_prompt_visible() {
        let mut tracker = SuggestionTracker::new();
        // Set a suggestion
        let screen_with = "❯\u{a0}\x1b[7mT\x1b[2;27mry something\x1b[m";
        tracker.parse_screen(screen_with);
        assert_eq!(tracker.current(), Some("Try something"));

        // Screen shows thinking output with no ❯ prompt at all
        let screen_thinking = "\x1b[38;2;78;186;101m⏺\x1b[m Working on it...\x1b[0m\n";
        assert!(!tracker.parse_screen(screen_thinking));
        // Suggestion should be preserved (prompt not visible, can't determine state)
        assert_eq!(tracker.current(), Some("Try something"));
    }
}
