//! Mode detection for Claude Code
//!
//! Parses terminal screen content to detect the current operating mode
//! (Normal, Auto-Accept, or Plan) from Claude Code's status line.

use crate::platforms::ClaudeMode;

/// Detect Claude Code mode from screen content
///
/// Looks for status line indicators in the terminal output:
/// - "plan mode" -> Plan
/// - "accept edits" -> AutoAccept
/// - Otherwise -> Normal
pub fn detect_mode(screen: &str) -> ClaudeMode {
    let lower = screen.to_lowercase();

    // Check for plan mode indicator
    if lower.contains("plan mode") {
        return ClaudeMode::Plan;
    }

    // Check for auto-accept indicator
    if lower.contains("accept edits") {
        return ClaudeMode::AutoAccept;
    }

    ClaudeMode::Normal
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_normal_mode() {
        let screen = "Claude> Some prompt text here\n> Ready";
        assert_eq!(detect_mode(screen), ClaudeMode::Normal);
    }

    #[test]
    fn test_detect_plan_mode() {
        let screen = "Claude> Working on task\n⏸ plan mode on";
        assert_eq!(detect_mode(screen), ClaudeMode::Plan);
    }

    #[test]
    fn test_detect_plan_mode_case_insensitive() {
        let screen = "Claude> Working\nPLAN MODE ON";
        assert_eq!(detect_mode(screen), ClaudeMode::Plan);
    }

    #[test]
    fn test_detect_auto_accept_mode() {
        let screen = "Claude> Editing files\n⏵⏵ accept edits";
        assert_eq!(detect_mode(screen), ClaudeMode::AutoAccept);
    }

    #[test]
    fn test_detect_auto_accept_case_insensitive() {
        let screen = "Claude> Working\nACCEPT EDITS enabled";
        assert_eq!(detect_mode(screen), ClaudeMode::AutoAccept);
    }
}
