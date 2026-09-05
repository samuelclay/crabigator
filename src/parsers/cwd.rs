//! Current-working-directory detection from assistant CLI status lines.
//!
//! Claude Code and Codex both render the active directory in their bottom
//! status area. Crabigator watches that rendered status line so its own Git and
//! Changes widgets can follow directory changes made by the assistant.

use std::path::PathBuf;

use super::permission_prompt::strip_ansi_for_debug;

/// Detect the current working directory from a formatted PTY screen.
pub fn detect_status_line_cwd(screen_content: &str) -> Option<PathBuf> {
    let stripped = strip_ansi_for_debug(screen_content).replace('\u{00a0}', " ");

    stripped
        .lines()
        .rev()
        .take(12)
        .filter_map(extract_status_line_path)
        .find_map(resolve_existing_dir)
}

fn extract_status_line_path(line: &str) -> Option<String> {
    let line = line.trim();
    if line.is_empty() {
        return None;
    }

    extract_claude_status_path(line).or_else(|| extract_codex_status_path(line))
}

fn extract_claude_status_path(line: &str) -> Option<String> {
    let colon = line.find(':')?;
    let prompt_prefix = &line[..colon];
    if !prompt_prefix.contains('@') {
        return None;
    }

    let rest = line[colon + 1..].trim_start();
    if !looks_like_path_start(rest) {
        return None;
    }

    let end = [" ‹", " ·"]
        .iter()
        .filter_map(|delimiter| rest.find(delimiter))
        .min()
        .unwrap_or(rest.len());

    Some(rest[..end].trim().to_string()).filter(|path| !path.is_empty())
}

fn extract_codex_status_path(line: &str) -> Option<String> {
    line.split('·')
        .skip(1)
        .map(str::trim)
        .find(|candidate| looks_like_path_start(candidate))
        .map(str::to_string)
}

fn looks_like_path_start(value: &str) -> bool {
    value == "~" || value.starts_with('/') || value.starts_with("~/")
}

fn resolve_existing_dir(candidate: String) -> Option<PathBuf> {
    let mut value = candidate.trim();

    loop {
        let path = expand_home(value)?;
        if path.is_dir() {
            return Some(std::fs::canonicalize(&path).unwrap_or(path));
        }

        let idx = value.rfind(char::is_whitespace)?;
        let prefix = &value[..idx];
        value = prefix.trim_end();
        if !looks_like_path_start(value) {
            return None;
        }
    }
}

fn expand_home(value: &str) -> Option<PathBuf> {
    if value == "~" {
        return dirs::home_dir();
    }
    if let Some(rest) = value.strip_prefix("~/") {
        return dirs::home_dir().map(|home| home.join(rest));
    }
    Some(PathBuf::from(value))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extracts_codex_status_line_path() {
        let line = "gpt-5.5 xhigh fast · ~/projects/crabigator";

        assert_eq!(
            extract_status_line_path(line).as_deref(),
            Some("~/projects/crabigator")
        );
    }

    #[test]
    fn extracts_codex_path_before_branch_and_mode() {
        assert_eq!(
            extract_status_line_path("gpt-6-astra xhigh · ~/projects/portal · Main [default]")
                .as_deref(),
            Some("~/projects/portal")
        );
    }

    #[test]
    fn extracts_claude_status_line_path_with_branch() {
        let line = "sclay@claybook-tavus:~/projects/developer-portal ‹main› · 162,457 tokens";

        assert_eq!(
            extract_status_line_path(line).as_deref(),
            Some("~/projects/developer-portal")
        );
    }

    #[test]
    fn extracts_claude_status_line_path_without_branch() {
        let line = "sclay@claybook-tavus:/tmp/example · 105,853 tokens";

        assert_eq!(
            extract_status_line_path(line).as_deref(),
            Some("/tmp/example")
        );
    }

    #[test]
    fn extracts_home_directory_status_line_path() {
        let line = "gpt-5.5 xhigh fast · ~";

        assert_eq!(extract_status_line_path(line).as_deref(), Some("~"));
    }

    #[test]
    fn ignores_claude_shell_reset_output() {
        let line = "  Shell cwd was reset to /Users/sclay/projects/developer-portal";

        assert_eq!(extract_status_line_path(line), None);
    }

    #[test]
    fn detects_existing_absolute_codex_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let expected = std::fs::canonicalize(dir.path()).expect("canonicalize");
        let screen = format!(
            "some output\ngpt-5.5 xhigh fast · {}\n",
            dir.path().display()
        );

        assert_eq!(detect_status_line_cwd(&screen), Some(expected));
    }

    #[test]
    fn detects_existing_absolute_claude_cwd() {
        let dir = tempfile::tempdir().expect("tempdir");
        let expected = std::fs::canonicalize(dir.path()).expect("canonicalize");
        let screen = format!(
            "content\nsclay@host:{} ‹branch-name› · 42 tokens\n",
            dir.path().display()
        );

        assert_eq!(detect_status_line_cwd(&screen), Some(expected));
    }
}
