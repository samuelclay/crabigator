//! Terminal spawner - opens a new terminal window with crabigator
//!
//! macOS only: detects the user's terminal emulator and opens a new
//! window with crabigator running in the specified directory.

use std::path::Path;
use std::process::Command;

use anyhow::{bail, Context, Result};

#[derive(Debug, Clone, Copy, PartialEq)]
enum TerminalApp {
    Terminal,
    Ghostty,
}

/// Detect which terminal emulator the user is running in.
///
/// Checks (in order): config override, $TERM_PROGRAM, $__CFBundleIdentifier.
/// Falls back to Terminal.app if no match is found.
fn detect_terminal(config_override: Option<&str>) -> TerminalApp {
    let term_program = std::env::var("TERM_PROGRAM").ok();
    let bundle_id = std::env::var("__CFBundleIdentifier").ok();

    let candidates = [config_override, term_program.as_deref(), bundle_id.as_deref()];

    for candidate in candidates.into_iter().flatten() {
        match candidate {
            "terminal" | "Apple_Terminal" | "com.apple.Terminal" => return TerminalApp::Terminal,
            "ghostty" | "com.mitchellh.ghostty" => return TerminalApp::Ghostty,
            _ => {}
        }
    }

    TerminalApp::Terminal
}

/// Replace single quotes with shell-safe escaping for use in AppleScript commands.
fn shell_escape(path: &str) -> String {
    path.replace('\'', "'\\''")
}

/// Returns the path to the current crabigator binary, or "crabigator" if unavailable.
fn find_crabigator_binary() -> String {
    std::env::current_exe()
        .ok()
        .filter(|p| p.exists())
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_else(|| "crabigator".to_string())
}

/// Spawn a new terminal window with crabigator in the given directory
pub fn spawn_terminal(cwd: &str, platform: Option<&str>) -> Result<()> {
    let cwd = Path::new(cwd);
    if !cwd.exists() {
        bail!("Directory does not exist: {}", cwd.display());
    }
    if !cwd.is_dir() {
        bail!("Path is not a directory: {}", cwd.display());
    }

    let cwd_str = cwd.to_string_lossy();
    let platform_arg = platform.unwrap_or("claude");
    let binary = find_crabigator_binary();

    let config = crate::config::Config::load().unwrap_or_default();
    let terminal = detect_terminal(config.terminal.as_deref());

    match terminal {
        TerminalApp::Terminal => spawn_in_terminal_app(&cwd_str, &binary, platform_arg),
        TerminalApp::Ghostty => spawn_in_ghostty(&cwd_str, &binary, platform_arg),
    }
}

/// Spawn in Terminal.app via osascript
fn spawn_in_terminal_app(cwd: &str, binary: &str, platform: &str) -> Result<()> {
    let escaped_cwd = shell_escape(cwd);
    let escaped_binary = shell_escape(binary);
    let script = format!(
        r#"tell application "Terminal"
    activate
    do script "cd '{}' && '{}' {}"
end tell"#,
        escaped_cwd, escaped_binary, platform
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .context("Failed to spawn Terminal.app via osascript")?;

    Ok(())
}

/// Spawn in Ghostty by opening a new window and typing the command.
///
/// Avoids Ghostty's `-e` execution prompt entirely by using System Events
/// to keystroke the command into a fresh window.
fn spawn_in_ghostty(cwd: &str, binary: &str, platform: &str) -> Result<()> {
    let escaped_cwd = shell_escape(cwd);
    let escaped_binary = shell_escape(binary);
    let cmd = format!("cd '{}' && '{}' {}", escaped_cwd, escaped_binary, platform);

    // AppleScript: activate Ghostty, open new window (Cmd+N), type command, press Enter
    let script = format!(
        r#"tell application "Ghostty" to activate
delay 0.3
tell application "System Events"
    tell process "Ghostty"
        keystroke "n" using command down
        delay 0.3
        keystroke "{}"
        delay 0.1
        keystroke return
    end tell
end tell"#,
        cmd.replace('\\', "\\\\").replace('"', "\\\"")
    );

    Command::new("osascript")
        .arg("-e")
        .arg(&script)
        .spawn()
        .context("Failed to spawn Ghostty via System Events")?;

    Ok(())
}
