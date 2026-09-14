//! Terminal spawner - opens a new Crabigator session in the user's terminal.
//!
//! On Ghostty this creates a tab in an existing window (matched by working
//! directory, then by the calling session's window id). Terminal.app still
//! opens a new window.

use std::path::Path;

#[cfg(target_os = "macos")]
use std::io::Write;
#[cfg(target_os = "macos")]
use std::process::{Command, Stdio};

use anyhow::{bail, Result};

#[cfg(target_os = "macos")]
use anyhow::Context;

use crate::platforms::PlatformKind;

#[derive(Debug, Clone, Copy, PartialEq)]
enum TerminalApp {
    Terminal,
    Ghostty,
}

/// Detect which terminal emulator to spawn into.
///
/// Order: config override, `$TERM_PROGRAM` / `$__CFBundleIdentifier`, a
/// running Ghostty app, then Terminal.app.
fn detect_terminal(config_override: Option<&str>) -> TerminalApp {
    let term_program = std::env::var("TERM_PROGRAM").ok();
    let bundle_id = std::env::var("__CFBundleIdentifier").ok();

    let candidates = [
        config_override,
        term_program.as_deref(),
        bundle_id.as_deref(),
    ];

    for candidate in candidates.into_iter().flatten() {
        match candidate {
            "terminal" | "Apple_Terminal" | "com.apple.Terminal" => return TerminalApp::Terminal,
            "ghostty" | "com.mitchellh.ghostty" => return TerminalApp::Ghostty,
            _ => {}
        }
    }

    if ghostty_is_running() {
        return TerminalApp::Ghostty;
    }

    TerminalApp::Terminal
}

#[cfg(target_os = "macos")]
fn ghostty_is_running() -> bool {
    Command::new("osascript")
        .args(["-e", r#"application "Ghostty" is running"#])
        .output()
        .ok()
        .and_then(|output| String::from_utf8(output.stdout).ok())
        .is_some_and(|stdout| stdout.trim().eq_ignore_ascii_case("true"))
}

#[cfg(not(target_os = "macos"))]
fn ghostty_is_running() -> bool {
    false
}

/// Replace single quotes with shell-safe escaping for use in AppleScript commands.
#[cfg(target_os = "macos")]
fn shell_escape(path: &str) -> String {
    path.replace('\'', "'\\''")
}

#[cfg(any(test, target_os = "macos"))]
fn applescript_string(value: &str) -> String {
    format!("\"{}\"", value.replace('\\', "\\\\").replace('"', "\\\""))
}

fn resolve_platform_name(platform: Option<&str>, default_platform: &str) -> Result<String> {
    let value = platform.unwrap_or(default_platform);
    match PlatformKind::parse(value) {
        Some(kind) => Ok(kind.as_str().to_string()),
        None => bail!("Unknown platform: {value}. Use claude, codex, opencode, or grok."),
    }
}

/// Path to this crabigator binary, or `"crabigator"` if it cannot be found.
pub(crate) fn find_crabigator_binary() -> String {
    std::env::current_exe()
        .ok()
        .filter(|path| path.exists())
        .map(|path| path.to_string_lossy().into_owned())
        .unwrap_or_else(|| "crabigator".to_string())
}

/// Spawn a new terminal with crabigator in the given directory.
pub fn spawn_terminal(cwd: &str, platform: Option<&str>) -> Result<()> {
    spawn_terminal_in_window(cwd, platform, None)
}

/// Spawn a new terminal, preferring `window_id` when no Ghostty window already
/// shows `cwd`.
pub fn spawn_terminal_in_window(
    cwd: &str,
    platform: Option<&str>,
    window_id: Option<&str>,
) -> Result<()> {
    let cwd_path = Path::new(cwd);
    if !cwd_path.exists() {
        bail!("Directory does not exist: {}", cwd_path.display());
    }
    if !cwd_path.is_dir() {
        bail!("Path is not a directory: {}", cwd_path.display());
    }

    let cwd_str = cwd_path.to_string_lossy();
    let binary = find_crabigator_binary();
    let config = crate::config::Config::load().unwrap_or_default();
    let platform_arg = resolve_platform_name(platform, &config.default_platform)?;
    let terminal = detect_terminal(config.terminal.as_deref());

    match terminal {
        #[cfg(target_os = "macos")]
        TerminalApp::Terminal => spawn_in_terminal_app(&cwd_str, &binary, &platform_arg),
        #[cfg(target_os = "macos")]
        TerminalApp::Ghostty => spawn_in_ghostty(&cwd_str, &binary, &platform_arg, window_id),
        #[cfg(not(target_os = "macos"))]
        _ => {
            let _ = (cwd_str, binary, platform_arg, window_id);
            bail!("Spawning a new session is only supported on macOS")
        }
    }
}

#[cfg(target_os = "macos")]
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

    run_osascript(&script).context("Failed to spawn Terminal.app via osascript")?;
    Ok(())
}

#[cfg(any(test, target_os = "macos"))]
fn ghostty_spawn_script(
    cwd: &str,
    binary: &str,
    platform: &str,
    window_id: Option<&str>,
) -> String {
    let cwd_literal = applescript_string(cwd);
    let binary_literal = applescript_string(binary);
    let platform_literal = applescript_string(platform);
    let window_literal = applescript_string(window_id.unwrap_or(""));

    format!(
        r#"set targetCwd to {cwd_literal}
set preferredWindowId to {window_literal}
set binaryPath to {binary_literal}
set platformName to {platform_literal}
set cfgCommand to "/bin/zsh -lic " & quoted form of ("exec " & quoted form of binaryPath & " " & quoted form of platformName)

tell application "Ghostty"
    set cfg to new surface configuration
    set initial working directory of cfg to targetCwd
    set command of cfg to cfgCommand
    set wait after command of cfg to true

    set win to missing value
    set relatedWin to missing value
    set prefixNeedle to targetCwd & "/"
    repeat with w in windows
        repeat with t in tabs of w
            try
                set termCwd to working directory of focused terminal of t
                if termCwd is targetCwd then
                    set win to w
                    exit repeat
                else if relatedWin is missing value then
                    if termCwd starts with prefixNeedle or targetCwd starts with (termCwd & "/") then
                        set relatedWin to w
                    end if
                end if
            end try
        end repeat
        if win is not missing value then exit repeat
    end repeat
    if win is missing value then set win to relatedWin

    if win is missing value and preferredWindowId is not "" then
        try
            set win to first window whose id is preferredWindowId
        end try
    end if

    if win is missing value then
        try
            set win to front window
        end try
    end if

    if win is missing value then
        set created to new window with configuration cfg
        activate window created
        return "new-window:" & (id of created)
    else
        set created to new tab in win with configuration cfg
        activate window win
        return "new-tab:" & (id of created) & " window:" & (id of win)
    end if
end tell
"#
    )
}

/// Spawn a Ghostty tab in an existing window using Ghostty's AppleScript
/// dictionary. Setting `command` on the surface configuration is required on
/// Ghostty 1.3.1, where a bare `new tab` creates an empty surface.
#[cfg(target_os = "macos")]
fn spawn_in_ghostty(
    cwd: &str,
    binary: &str,
    platform: &str,
    window_id: Option<&str>,
) -> Result<()> {
    let script = ghostty_spawn_script(cwd, binary, platform, window_id);
    let output = run_osascript(&script).context("Failed to spawn Ghostty tab via AppleScript")?;
    if output.trim().is_empty() {
        bail!("Ghostty did not create a tab");
    }
    Ok(())
}

#[cfg(target_os = "macos")]
fn run_osascript(script: &str) -> Result<String> {
    let mut child = Command::new("osascript")
        .arg("-")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .context("Failed to start osascript")?;

    {
        let mut stdin = child
            .stdin
            .take()
            .context("Failed to open osascript stdin")?;
        stdin
            .write_all(script.as_bytes())
            .context("Failed to write AppleScript")?;
    }

    let output = child
        .wait_with_output()
        .context("Failed to wait for osascript")?;
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        bail!("osascript failed ({}): {}", output.status, stderr.trim());
    }
    Ok(String::from_utf8_lossy(&output.stdout).into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ghostty_script_opens_a_tab_with_the_requested_session() {
        let script = ghostty_spawn_script(
            "/Users/sclay/projects/crabigator",
            "/usr/local/bin/crabigator",
            "grok",
            Some("tab-group-abc"),
        );
        assert!(script.contains("new tab in win"));
        assert!(script.contains("new window with configuration"));
        assert!(script.contains("/Users/sclay/projects/crabigator"));
        assert!(script.contains("/usr/local/bin/crabigator"));
        assert!(script.contains("grok"));
        assert!(script.contains("tab-group-abc"));
        assert!(script.contains("wait after command"));
        assert!(!script.contains("keystroke"));
        assert!(!script.contains("System Events"));
        assert!(!script.contains("command down"));

        let exact = script.find("termCwd is targetCwd").unwrap();
        let related = script.find("starts with prefixNeedle").unwrap();
        assert!(exact < related);
    }

    #[test]
    fn applescript_string_escapes_quotes_and_backslashes() {
        assert_eq!(applescript_string(r#"say "hi""#), r#""say \"hi\"""#);
        assert_eq!(applescript_string(r"C:\tmp"), r#""C:\\tmp""#);
    }

    #[test]
    fn resolve_platform_name_accepts_aliases_and_defaults() {
        assert_eq!(resolve_platform_name(None, "codex").unwrap(), "codex");
        assert_eq!(
            resolve_platform_name(Some("grok-build"), "codex").unwrap(),
            "grok"
        );
        assert!(resolve_platform_name(Some("nope"), "codex").is_err());
    }

    #[test]
    fn detect_terminal_honors_config_override() {
        assert_eq!(detect_terminal(Some("ghostty")), TerminalApp::Ghostty);
        assert_eq!(detect_terminal(Some("terminal")), TerminalApp::Terminal);
    }
}
