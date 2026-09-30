//! Terminal spawner - opens a new Crabigator session in the user's terminal.
//!
//! On Ghostty this creates a tab in an existing window (matched by working
//! directory, then by the calling session's window id). Terminal.app still
//! opens a new window. `spawn_checkout` is the stricter open used by the PR
//! board: a tab only when a window is already in that folder, otherwise a
//! new window.

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
#[cfg(any(test, target_os = "macos"))]
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

/// Where Ghostty puts the new session.
#[derive(Clone, Copy)]
enum GhosttyPlacement {
    /// Match this checkout, then a parent or child path, then `window_id`,
    /// then the front window. Otherwise open a new window.
    PreferExisting,
    /// Match a window that already has this checkout open. Otherwise open a
    /// new window, leaving unrelated windows alone.
    ExactOrNew,
}

/// Start the remote launch in its own process so waiting for display wake
/// survives the requesting session ending. It never holds up the app loop.
pub fn spawn_terminal_detached(
    cwd: &str,
    platform: Option<&str>,
    window_id: Option<&str>,
) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        use std::os::unix::process::CommandExt;
        let log_path =
            std::env::temp_dir().join(format!("crabigator-spawn-{}.log", uuid::Uuid::new_v4()));
        let log = std::fs::File::create(&log_path)?;
        let mut command = Command::new(find_crabigator_binary());
        command.args(["spawn", "--cwd", cwd]);
        if let Some(platform) = platform {
            command.args(["--platform", platform]);
        }
        if let Some(window_id) = window_id {
            command.args(["--window-id", window_id]);
        }
        let mut child = command
            .process_group(0)
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(log)
            .spawn()
            .context("Failed to start the session launcher")?;
        std::thread::spawn(move || match child.wait() {
            Ok(status) if status.success() => {
                let _ = std::fs::remove_file(log_path);
            }
            _ => eprintln!("Session launch failed; see {}", log_path.display()),
        });
        Ok(())
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (cwd, platform, window_id);
        bail!("Spawning a new session is only supported on macOS")
    }
}

/// Spawn a new terminal, preferring `window_id` when no Ghostty window already
/// shows `cwd`.
pub fn spawn_terminal_in_window(
    cwd: &str,
    platform: Option<&str>,
    window_id: Option<&str>,
) -> Result<()> {
    spawn_placed(cwd, platform, window_id, GhosttyPlacement::PreferExisting)
}

/// Open a session in `cwd` on `platform` (claude, codex, opencode, or grok).
///
/// Ghostty adds a tab when a window is already in that folder, and opens a
/// new window when none is. Terminal.app opens a new window either way.
pub fn spawn_checkout(cwd: &str, platform: &str) -> Result<()> {
    spawn_placed(cwd, Some(platform), None, GhosttyPlacement::ExactOrNew)
}

fn spawn_placed(
    cwd: &str,
    platform: Option<&str>,
    window_id: Option<&str>,
    placement: GhosttyPlacement,
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
        TerminalApp::Ghostty => {
            spawn_in_ghostty(&cwd_str, &binary, &platform_arg, window_id, placement)
        }
        #[cfg(not(target_os = "macos"))]
        _ => {
            let _ = (cwd_str, binary, platform_arg, window_id, placement);
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
    placement: GhosttyPlacement,
) -> String {
    let command = format!(
        "exec '{}' '{}'",
        shell_escape(binary),
        shell_escape(platform)
    );
    ghostty_command_script(cwd, &command, window_id, placement)
}

#[cfg(any(test, target_os = "macos"))]
fn ghostty_command_script(
    cwd: &str,
    command: &str,
    window_id: Option<&str>,
    placement: GhosttyPlacement,
) -> String {
    let cwd_literal = applescript_string(cwd);
    let command_literal = applescript_string(command);
    let window_literal = applescript_string(window_id.unwrap_or(""));
    let preferred = match placement {
        GhosttyPlacement::PreferExisting => format!("set preferredWindowId to {window_literal}\n"),
        GhosttyPlacement::ExactOrNew => String::new(),
    };
    // ExactOrNew stops after an exact checkout match. A parent or child path,
    // a remembered window id, and the front window would put the session in
    // some other folder's window.
    let scan = match placement {
        GhosttyPlacement::PreferExisting => {
            r#"    set win to missing value
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
"#
        }
        GhosttyPlacement::ExactOrNew => {
            r#"    set win to missing value
    repeat with w in windows
        repeat with t in tabs of w
            try
                set termCwd to working directory of focused terminal of t
                if termCwd is targetCwd then
                    set win to w
                    exit repeat
                end if
            end try
        end repeat
        if win is not missing value then exit repeat
    end repeat
"#
        }
    };

    format!(
        r#"set targetCwd to {cwd_literal}
{preferred}set cfgCommand to "/bin/zsh -lic " & quoted form of {command_literal}

tell application "Ghostty"
    set cfg to new surface configuration
    set initial working directory of cfg to targetCwd
    set command of cfg to cfgCommand
    set wait after command of cfg to true

{scan}
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
    placement: GhosttyPlacement,
) -> Result<()> {
    // Ghostty needs a drawable display to create its Metal surface. Start the
    // assistant in a detached terminal first, then attach that same terminal
    // when the display wakes. The dashboard can use it during the wait.
    let script = if display_is_asleep() {
        let session = BackgroundSession::start(cwd, binary, platform)?;
        wait_for_drawable_display();
        if !session.is_running() {
            return Ok(());
        }
        ghostty_command_script(cwd, &session.attach_command(), window_id, placement)
    } else {
        ghostty_spawn_script(cwd, binary, platform, window_id, placement)
    };
    let output = run_osascript(&script).context("Failed to spawn Ghostty tab via AppleScript")?;
    if output.trim().is_empty() {
        bail!("Ghostty did not create a tab");
    }
    Ok(())
}

/// A separate tmux server keeps the session alive without a GUI and avoids
/// inheriting the user's tmux configuration or changing their existing server.
#[cfg(target_os = "macos")]
struct BackgroundSession {
    tmux: std::path::PathBuf,
    socket: String,
}

#[cfg(target_os = "macos")]
impl BackgroundSession {
    fn start(cwd: &str, binary: &str, platform: &str) -> Result<Self> {
        let tmux = std::env::split_paths(&std::env::var_os("PATH").unwrap_or_default())
            .map(|dir| dir.join("tmux"))
            .chain(["/opt/homebrew/bin/tmux".into(), "/usr/local/bin/tmux".into()])
            .find(|path| path.is_file())
            .context("Starting a session while the display sleeps requires tmux. Install it with brew install tmux.")?;
        let session = Self {
            tmux,
            socket: format!("crabigator-{}", uuid::Uuid::new_v4()),
        };
        let command = format!(
            "exec '{}' '{}'",
            shell_escape(binary),
            shell_escape(platform)
        );
        let output = session
            .command()
            .args([
                "-f",
                "/dev/null",
                "new-session",
                "-d",
                "-s",
                "crabigator",
                "-x",
                "120",
                "-y",
                "40",
                "-c",
                cwd,
                "/bin/zsh",
                "-lic",
                &command,
                ";",
                "set-option",
                "-g",
                "status",
                "off",
                ";",
                "set-option",
                "-g",
                "prefix",
                "None",
            ])
            .output()
            .context("Failed to start a background terminal")?;
        if !output.status.success() {
            bail!(
                "Failed to start a background terminal: {}",
                String::from_utf8_lossy(&output.stderr).trim()
            );
        }
        Ok(session)
    }

    fn command(&self) -> Command {
        let mut command = Command::new(&self.tmux);
        command.args(["-L", &self.socket]);
        // These belong to the caller, not the new terminal or conversation.
        for name in [
            "TMUX",
            "TMUX_PANE",
            "CODEX_THREAD_ID",
            "CODEX_ROLLOUT_PATH",
            "CODEX_SESSION_PATH",
            "CRABIGATOR_CODEX_SESSION_PATH",
        ] {
            command.env_remove(name);
        }
        command
    }

    fn is_running(&self) -> bool {
        self.command()
            .args(["has-session", "-t", "crabigator"])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .is_ok_and(|status| status.success())
    }

    fn attach_command(&self) -> String {
        format!(
            "exec '{}' -L '{}' attach-session -t crabigator",
            shell_escape(&self.tmux.to_string_lossy()),
            shell_escape(&self.socket)
        )
    }
}

/// True when macOS has no drawable display. That is the state Ghostty 1.3.1
/// cannot open a terminal in.
#[cfg(target_os = "macos")]
fn display_is_asleep() -> bool {
    #[link(name = "CoreGraphics", kind = "framework")]
    extern "C" {
        fn CGGetActiveDisplayList(
            max_displays: u32,
            active_displays: *mut u32,
            display_count: *mut u32,
        ) -> i32;
    }

    let mut count = 0u32;
    let err = unsafe { CGGetActiveDisplayList(0, std::ptr::null_mut(), &mut count) };
    err != 0 || count == 0
}

/// Poll until `asleep` is false. `pause` runs between checks.
#[cfg(any(test, target_os = "macos"))]
fn wait_while_asleep(mut asleep: impl FnMut() -> bool, mut pause: impl FnMut()) {
    if !asleep() {
        return;
    }
    while asleep() {
        pause();
    }
}

#[cfg(target_os = "macos")]
fn wait_for_drawable_display() {
    if !display_is_asleep() {
        return;
    }
    eprintln!(
        "The session is running in the background. Ghostty will attach when the Mac display wakes."
    );
    wait_while_asleep(display_is_asleep, || {
        std::thread::sleep(std::time::Duration::from_secs(1));
    });
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
            GhosttyPlacement::PreferExisting,
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
    fn ghostty_checkout_script_skips_unrelated_windows() {
        let script = ghostty_spawn_script(
            "/Users/sclay/projects/crabigator",
            "/usr/local/bin/crabigator",
            "codex",
            Some("tab-group-abc"),
            GhosttyPlacement::ExactOrNew,
        );
        assert!(script.contains("termCwd is targetCwd"));
        assert!(script.contains("new tab in win"));
        assert!(script.contains("new window with configuration"));
        assert!(script.contains("codex"));
        assert!(!script.contains("front window"));
        assert!(!script.contains("relatedWin"));
        assert!(!script.contains("preferredWindowId"));
        assert!(!script.contains("tab-group-abc"));
    }

    /// Exercises a real detached terminal and then attaches a PTY to it.
    /// Run explicitly on a Mac with tmux installed.
    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires tmux and starts a detached terminal"]
    fn background_session_accepts_input_before_and_after_attach() {
        use portable_pty::{native_pty_system, CommandBuilder, PtySize};
        use std::time::{Duration, Instant};

        let dir = tempfile::tempdir().unwrap();
        let script = dir.path().join("assistant with 'quotes'.sh");
        std::fs::write(
            &script,
            r#"
while IFS= read -r line; do
    printf '%s:%s\n' "$$" "$line" >> received
 done
"#,
        )
        .unwrap();
        let session = BackgroundSession::start(
            dir.path().to_str().unwrap(),
            "/bin/sh",
            script.to_str().unwrap(),
        )
        .unwrap();
        struct Cleanup<'a>(&'a BackgroundSession);
        impl Drop for Cleanup<'_> {
            fn drop(&mut self) {
                let _ = self.0.command().arg("kill-server").status();
            }
        }
        let _cleanup = Cleanup(&session);
        let received = dir.path().join("received");
        let wait_for = |text: &str| {
            let deadline = Instant::now() + Duration::from_secs(15);
            loop {
                let content = std::fs::read_to_string(&received).unwrap_or_default();
                if content.contains(text) {
                    return content;
                }
                assert!(Instant::now() < deadline, "Missing {text}: {content}");
                std::thread::sleep(Duration::from_millis(20));
            }
        };
        assert!(session
            .command()
            .args(["send-keys", "-t", "crabigator", "before-attach", "Enter"])
            .status()
            .unwrap()
            .success());
        let before = wait_for(":before-attach");
        let pid = before.trim().split(':').next().unwrap().to_string();

        let pair = native_pty_system()
            .openpty(PtySize {
                rows: 40,
                cols: 120,
                pixel_width: 0,
                pixel_height: 0,
            })
            .unwrap();
        let mut cmd = CommandBuilder::new("/bin/sh");
        cmd.args(["-c", &session.attach_command()]);
        cmd.env("TERM", "xterm-256color");
        let mut child = pair.slave.spawn_command(cmd).unwrap();
        let mut reader = pair.master.try_clone_reader().unwrap();
        let output = std::thread::spawn(move || {
            let _ = std::io::copy(&mut reader, &mut std::io::sink());
        });
        let mut writer = pair.master.take_writer().unwrap();
        writer.write_all(b"after-attach\r").unwrap();
        writer.flush().unwrap();
        let after = wait_for(":after-attach");
        assert!(after
            .lines()
            .all(|line| line.starts_with(&format!("{pid}:"))));
        assert!(session.is_running());
        child.kill().unwrap();
        drop(writer);
        drop(pair);
        child.wait().unwrap();
        output.join().unwrap();
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

    #[test]
    fn wait_while_asleep_returns_immediately_when_awake() {
        let mut pauses = 0;
        wait_while_asleep(|| false, || pauses += 1);
        assert_eq!(pauses, 0);
    }

    #[test]
    fn wait_while_asleep_pauses_until_the_display_wakes() {
        let mut checks = 0;
        let mut pauses = 0;
        wait_while_asleep(
            || {
                checks += 1;
                checks < 3
            },
            || pauses += 1,
        );
        assert_eq!(pauses, 1);
    }
}
