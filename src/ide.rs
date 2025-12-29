//! IDE detection and file URL generation for clickable hyperlinks
//!
//! Detects the user's IDE from environment variables and generates
//! appropriate URL schemes for OSC 8 hyperlinks.

use std::env;
use std::path::Path;

/// Supported IDE types for URL generation
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum IdeKind {
    VsCode,
    Cursor,
    IntelliJ,
    Zed,
    Sublime,
    /// No IDE detected - falls back to file:// URLs
    #[default]
    None,
}

impl IdeKind {
    /// Parse an IDE kind from a config string
    pub fn from_config(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "vscode" | "code" | "vs-code" => Some(Self::VsCode),
            "cursor" => Some(Self::Cursor),
            "intellij" | "idea" | "webstorm" | "pycharm" | "rustrover" | "jetbrains" => {
                Some(Self::IntelliJ)
            }
            "zed" => Some(Self::Zed),
            "sublime" | "subl" => Some(Self::Sublime),
            "none" | "file" | "default" => Some(Self::None),
            _ => None,
        }
    }

    /// Generate a file URL for this IDE
    ///
    /// - `abs_path`: Absolute path to the file
    /// - `line`: Optional line number (1-indexed)
    pub fn file_url(&self, abs_path: &str, line: Option<usize>) -> String {
        match self {
            Self::VsCode => {
                if let Some(ln) = line {
                    format!("vscode://file{}:{}", abs_path, ln)
                } else {
                    format!("vscode://file{}", abs_path)
                }
            }
            Self::Cursor => {
                if let Some(ln) = line {
                    format!("cursor://file{}:{}", abs_path, ln)
                } else {
                    format!("cursor://file{}", abs_path)
                }
            }
            Self::IntelliJ => {
                if let Some(ln) = line {
                    format!("idea://open?file={}&line={}", abs_path, ln)
                } else {
                    format!("idea://open?file={}", abs_path)
                }
            }
            Self::Zed => {
                if let Some(ln) = line {
                    format!("zed://file{}:{}", abs_path, ln)
                } else {
                    format!("zed://file{}", abs_path)
                }
            }
            Self::Sublime => {
                // Sublime uses file:// URI with subl:// wrapper
                if let Some(ln) = line {
                    format!("subl://open?url=file://{}&line={}", abs_path, ln)
                } else {
                    format!("subl://open?url=file://{}", abs_path)
                }
            }
            Self::None => {
                // Plain file:// URL (no line number support)
                format!("file://{}", abs_path)
            }
        }
    }
}

/// Detect the IDE from environment variables
///
/// Priority:
/// 1. TERM_PROGRAM - set by integrated terminals
/// 2. IDE-specific environment variables
/// 3. VISUAL/EDITOR as hints
pub fn detect_ide() -> IdeKind {
    // Check TERM_PROGRAM first (integrated terminal detection)
    if let Ok(term_program) = env::var("TERM_PROGRAM") {
        match term_program.to_lowercase().as_str() {
            "vscode" => return IdeKind::VsCode,
            "cursor" => return IdeKind::Cursor,
            _ => {}
        }
    }

    // Check for VS Code specific env vars
    if env::var("VSCODE_PID").is_ok() || env::var("VSCODE_CWD").is_ok() {
        return IdeKind::VsCode;
    }

    // Check for Cursor specific env vars
    if env::var("CURSOR_TRACE_ID").is_ok() {
        return IdeKind::Cursor;
    }

    // Check for JetBrains IDEs
    if env::var("IDEA_INITIAL_DIRECTORY").is_ok()
        || env::var("JETBRAINS_REMOTE_RUN").is_ok()
        || env::var("TERMINAL_EMULATOR").is_ok_and(|v| v.contains("JetBrains"))
    {
        return IdeKind::IntelliJ;
    }

    // Check for Zed
    if env::var("ZED_TERM").is_ok() {
        return IdeKind::Zed;
    }

    // Check VISUAL/EDITOR as fallback hints
    for var in ["VISUAL", "EDITOR"] {
        if let Ok(editor) = env::var(var) {
            let editor_lower = editor.to_lowercase();
            if editor_lower.contains("code") && !editor_lower.contains("cursor") {
                return IdeKind::VsCode;
            }
            if editor_lower.contains("cursor") {
                return IdeKind::Cursor;
            }
            if editor_lower.contains("idea")
                || editor_lower.contains("webstorm")
                || editor_lower.contains("pycharm")
            {
                return IdeKind::IntelliJ;
            }
            if editor_lower.contains("zed") {
                return IdeKind::Zed;
            }
            if editor_lower.contains("subl") {
                return IdeKind::Sublime;
            }
        }
    }

    IdeKind::None
}

/// Build an absolute path from cwd and a relative path
#[allow(dead_code)]
pub fn make_absolute(cwd: &Path, relative: &str) -> String {
    cwd.join(relative).to_string_lossy().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ide_from_config() {
        assert_eq!(IdeKind::from_config("vscode"), Some(IdeKind::VsCode));
        assert_eq!(IdeKind::from_config("CURSOR"), Some(IdeKind::Cursor));
        assert_eq!(IdeKind::from_config("idea"), Some(IdeKind::IntelliJ));
        assert_eq!(IdeKind::from_config("zed"), Some(IdeKind::Zed));
        assert_eq!(IdeKind::from_config("sublime"), Some(IdeKind::Sublime));
        assert_eq!(IdeKind::from_config("none"), Some(IdeKind::None));
        assert_eq!(IdeKind::from_config("unknown"), None);
    }

    #[test]
    fn test_file_url_vscode() {
        let ide = IdeKind::VsCode;
        assert_eq!(
            ide.file_url("/Users/test/file.rs", Some(42)),
            "vscode://file/Users/test/file.rs:42"
        );
        assert_eq!(
            ide.file_url("/Users/test/file.rs", None),
            "vscode://file/Users/test/file.rs"
        );
    }

    #[test]
    fn test_file_url_intellij() {
        let ide = IdeKind::IntelliJ;
        assert_eq!(
            ide.file_url("/Users/test/file.rs", Some(42)),
            "idea://open?file=/Users/test/file.rs&line=42"
        );
    }

    #[test]
    fn test_file_url_none() {
        let ide = IdeKind::None;
        assert_eq!(
            ide.file_url("/Users/test/file.rs", Some(42)),
            "file:///Users/test/file.rs"
        );
    }
}
