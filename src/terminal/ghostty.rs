//! Read the current Ghostty tab identity on macOS.
//!
//! Ghostty exposes stable window, tab, and terminal IDs through its official
//! AppleScript dictionary. Crabigator captures the focused surface once at
//! startup, while the terminal that launched it is still focused.

use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct GhosttyContext {
    pub terminal_id: String,
    pub tab_id: String,
    pub tab_name: String,
    pub window_id: String,
}

impl GhosttyContext {
    #[cfg(all(target_os = "macos", not(test)))]
    pub fn focused() -> Option<Self> {
        if !std::env::var("TERM_PROGRAM").is_ok_and(|value| value.eq_ignore_ascii_case("ghostty")) {
            return None;
        }
        let output = std::process::Command::new("osascript")
            .args([
                "-e",
                "tell application \"Ghostty\"",
                "-e",
                "set w to front window",
                "-e",
                "set t to selected tab of w",
                "-e",
                "set term to focused terminal of t",
                "-e",
                "set d to ASCII character 9",
                "-e",
                "return (id of term) & d & (id of t) & d & (name of t) & d & (id of w)",
                "-e",
                "end tell",
            ])
            .output()
            .ok()?;
        output
            .status
            .success()
            .then(|| parse_context(&String::from_utf8_lossy(&output.stdout)))?
    }

    #[cfg(any(not(target_os = "macos"), test))]
    pub fn focused() -> Option<Self> {
        None
    }
}

fn parse_context(value: &str) -> Option<GhosttyContext> {
    let mut fields = value.trim_end().splitn(4, '\t');
    let context = GhosttyContext {
        terminal_id: fields.next()?.to_string(),
        tab_id: fields.next()?.to_string(),
        tab_name: fields.next()?.to_string(),
        window_id: fields.next()?.to_string(),
    };
    (!context.terminal_id.is_empty() && !context.tab_id.is_empty() && !context.window_id.is_empty())
        .then_some(context)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ghostty_context_fields() {
        let context = parse_context(
            "1DF2EA01-A522-482B-A24D-D93981B9680B\ttab-8962c0e00\tdeveloper-portal\ttab-group-894aa57c0\n",
        )
        .unwrap();
        assert_eq!(context.tab_id, "tab-8962c0e00");
        assert_eq!(context.tab_name, "developer-portal");
        assert_eq!(context.window_id, "tab-group-894aa57c0");
    }

    #[test]
    fn rejects_incomplete_context() {
        assert!(parse_context("terminal\ttab\tonly-three").is_none());
    }
}
