mod app;
mod banner;
mod capture;
mod cli;
mod cloud;
mod config;
mod git;
mod hooks;
mod ide;
mod inspect;
mod launcher;
mod mirror;
mod mode;
mod pair;
mod parsers;
mod platforms;
mod pr;
mod pr_rank;
mod prs_board;
mod recap;
mod terminal;
mod terminal_spawner;
mod title;
mod ui;
mod update;

#[cfg(test)]
mod fixtures_tests;

use anyhow::Result;
use crossterm::{
    cursor::Show,
    event::{DisableBracketedPaste, EnableBracketedPaste},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, size as terminal_size},
};
use std::env;
use std::io::{stdout, Write};
use std::panic;
use std::sync::atomic::Ordering;
use std::time::{Duration, Instant};

use crate::app::App;
use crate::banner::{print_session_banner, print_session_end_line};
use crate::cli::{parse_args, resolve_platform, Command, DebugTimer};
use crate::config::Config;
use crate::update::{
    check_for_update, detect_install_method, dismiss_version, get_cli_version, UpdateState,
};

fn setup_terminal() -> Result<(u16, u16)> {
    let mut stdout = stdout();
    let (cols, rows) = terminal_size()?;

    // Don't clear the screen - let the CLI start at the bottom
    // and scroll up naturally, preserving existing terminal content above.
    // The scroll region setup will position cursor at the bottom.

    enable_raw_mode()?;

    // Enable bracketed paste
    // Primary screen buffer (no alternate screen) - allows native scrollback
    // Disable mouse capture to allow native text selection
    execute!(stdout, EnableBracketedPaste)?;

    Ok((cols, rows))
}

fn restore_terminal(total_rows: u16) -> Result<()> {
    let mut stdout = stdout();
    // Reset scroll region to full screen
    write!(stdout, "{}", terminal::escape::SCROLL_REGION_RESET)?;
    // Move cursor to the bottom of the screen, then down one more line
    // This ensures we're below all content (CLI output + status widgets)
    write!(stdout, "{}", terminal::escape::cursor_to(total_rows, 1))?;
    stdout.flush()?;

    disable_raw_mode()?;
    execute!(stdout, DisableBracketedPaste, Show)?;
    stdout.flush()?;
    Ok(())
}

thread_local! {
    /// When set, the installed panic hook becomes a no-op. Code paths that wrap
    /// known-panic-prone calls (e.g. the vt100 parser) in `catch_unwind` toggle
    /// this on to suppress the full-backtrace dump the default hook emits, which
    /// otherwise floods the terminal.
    static SUPPRESS_PANIC_HOOK: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

/// RAII guard: restores SUPPRESS_PANIC_HOOK to false on drop so the flag can't
/// leak if `catch_quiet`'s body is ever restructured.
struct SuppressGuard;
impl Drop for SuppressGuard {
    fn drop(&mut self) {
        SUPPRESS_PANIC_HOOK.with(|c| c.set(false));
    }
}

/// Run `f` inside `catch_unwind`, suppressing the panic-hook output. Intended for
/// guarding third-party code (vt100) that panics on malformed input; we treat
/// those panics as recoverable and don't want the user's terminal flooded with
/// backtraces.
pub fn catch_quiet<F, R>(f: F) -> std::thread::Result<R>
where
    F: FnOnce() -> R,
{
    SUPPRESS_PANIC_HOOK.with(|c| c.set(true));
    let _guard = SuppressGuard;
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f))
}

/// Run `parser.process(data)` with panic recovery. vt100 can panic on malformed
/// byte sequences and leaves its internal state corrupted afterward, so on panic
/// we replace the parser with a fresh instance preserving dimensions and the
/// caller-supplied scrollback size.
pub fn guarded_process(parser: &mut vt100::Parser, data: &[u8], scrollback: usize) {
    let ptr = parser as *mut vt100::Parser;
    // SAFETY: we only dereference `ptr` inside the closure; the borrow on `parser`
    // is not live during the call, and catch_unwind returns before we resume it.
    let result = catch_quiet(|| unsafe { (*ptr).process(data) });
    if result.is_err() {
        let (rows, cols) = parser.screen().size();
        *parser = vt100::Parser::new(rows, cols, scrollback);
    }
}

/// Run `parser.screen_mut().set_size(rows, cols)` with panic recovery.
pub fn guarded_resize(parser: &mut vt100::Parser, cols: u16, rows: u16) {
    let ptr = parser as *mut vt100::Parser;
    // SAFETY: same reasoning as guarded_process.
    let _ = catch_quiet(|| unsafe { (*ptr).screen_mut().set_size(rows, cols) });
}

fn setup_panic_handler() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        if SUPPRESS_PANIC_HOOK.with(|c| c.get()) {
            return;
        }
        // First disable raw mode to ensure newlines work
        let _ = disable_raw_mode();
        // Reset scroll region and flush
        let mut stdout = stdout();
        let _ = write!(stdout, "{}", terminal::escape::SCROLL_REGION_RESET);
        let _ = stdout.flush();
        let _ = execute!(stdout, DisableBracketedPaste, Show);
        // Ensure we're on a fresh line
        println!();
        original_hook(panic_info);
    }));
}

/// Generate a unique session ID for this instance
fn generate_session_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    format!("{:x}{:x}", pid, timestamp % 0xFFFFFFFF)
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = parse_args();

    // Handle subcommands that don't need the full app setup
    match args.command {
        Command::Inspect {
            dir_filter,
            watch,
            raw,
            history,
        } => {
            return inspect::run_inspect(dir_filter, watch, raw, history);
        }
        Command::Pair => {
            return pair::run_pair().await;
        }
        Command::Prs { once } => {
            return prs_board::run_prs_board(once).await;
        }
        Command::InstallLauncher => {
            return launcher::install_launcher().map(|()| {
                println!("URL scheme handler installed.");
                println!("crabigator:// URLs will now open in your terminal.");
            });
        }
        Command::Recap(command) => {
            return recap::run_recap_command(command);
        }
        Command::Key { api_key } => {
            return recap::run_key_command(api_key);
        }
        Command::Run => {}
    }

    // Resolve platform from args, env, or config
    let platform_kind = resolve_platform(args.platform);

    // Generate and set session ID before anything else
    // This ensures the CLI and our stats loading use the same ID
    let session_id = generate_session_id();
    env::set_var("CRABIGATOR_SESSION_ID", &session_id);
    env::set_var("CRABIGATOR_PLATFORM", platform_kind.as_str());
    if args.profile {
        env::set_var("CRABIGATOR_PROFILE", "1");
    }

    let timer = DebugTimer::new(args.profile);

    timer.log("args parsed");
    timer.log(&format!("session_id={}", session_id));
    timer.log(&format!("platform={}", platform_kind.display_name()));

    // Install/update URL scheme handler in background (macOS only, fire and forget)
    #[cfg(target_os = "macos")]
    std::thread::spawn(launcher::ensure_installed);

    // Install/update platform hooks in background thread (fire and forget)
    // Don't block startup - hooks will be ready by the time the CLI needs them
    {
        let timer = timer.clone();
        std::thread::spawn(move || {
            timer.hook_state.store(1, Ordering::SeqCst);
            let begin = Instant::now();
            timer.log("hook install started");

            let result = std::panic::catch_unwind(|| {
                let platform = platforms::platform_for(platform_kind);
                platform.ensure_hooks_installed()
            });

            match result {
                Ok(Ok(())) => {
                    timer.hook_state.store(2, Ordering::SeqCst);
                    timer.duration("hook install finished", begin.elapsed());
                }
                Ok(Err(e)) => {
                    timer.hook_state.store(3, Ordering::SeqCst);
                    timer.set_hook_error(e.to_string());
                    timer.duration("hook install failed", begin.elapsed());
                }
                Err(_) => {
                    timer.hook_state.store(4, Ordering::SeqCst);
                    timer.duration("hook install panicked", begin.elapsed());
                }
            };
        });
    }

    timer.log("Hook installation spawned (background)");

    let begin = Instant::now();
    setup_panic_handler();
    timer.duration("setup panic handler", begin.elapsed());

    // Get terminal size and print session banner BEFORE raw mode
    let (cols, _) = terminal_size()?;
    print_session_banner(&session_id, platform_kind, cols);

    // Get CLI version for telemetry (quick, runs in ~10ms)
    let cli_version = get_cli_version(platform_kind.command());

    // Check for updates before entering raw mode (non-blocking modal prompt)
    let update_state = check_and_prompt_for_update(cli_version).await;

    let begin = Instant::now();
    let (cols, rows) = match setup_terminal() {
        Ok(size) => size,
        Err(e) => {
            let _ = disable_raw_mode();
            let _ = execute!(stdout(), DisableBracketedPaste, Show);
            return Err(e);
        }
    };
    timer.duration("setup terminal", begin.elapsed());

    let (result, final_rows) = {
        let begin = Instant::now();
        let platform = platforms::platform_for(platform_kind);
        let app_result = App::new(
            cols,
            rows,
            platform,
            args.platform_args,
            args.capture,
            update_state,
        )
        .await;
        timer.duration("App::new", begin.elapsed());

        match app_result {
            Ok(mut app) => {
                timer.log("Starting main loop");

                let begin = Instant::now();
                let run_result = app.run().await;
                timer.duration("app.run", begin.elapsed());
                let total_rows = app.total_rows;
                (run_result, total_rows)
            }
            Err(e) => {
                let _ = restore_terminal(rows);
                return Err(e);
            }
        }
    };

    let begin = Instant::now();
    let _ = disable_raw_mode();
    let restore_result = restore_terminal(final_rows);
    timer.duration("restore terminal", begin.elapsed());

    if restore_result.is_err() {
        let _ = execute!(stdout(), DisableBracketedPaste, Show);
    }

    println!();

    // Dump startup trace after terminal restore (to stdout, visible in scrollback)
    timer.dump();

    // Surface background hook state/errors after terminal restore.
    match timer.hook_state.load(Ordering::SeqCst) {
        0 => {}
        1 => println!("Warning: Hook installation still running in background."),
        2 => {}
        3 => {
            if let Some(err) = timer
                .hook_error
                .lock()
                .unwrap_or_else(|p| p.into_inner())
                .take()
            {
                println!("Warning: Failed to install hooks: {}", err);
            } else {
                println!("Warning: Failed to install hooks.");
            }
        }
        4 => println!("Warning: Hook installation thread panicked."),
        _ => {}
    }

    // Print session end line with platform and date (get fresh terminal width)
    let end_cols = terminal_size().map(|(c, _)| c).unwrap_or(cols);
    print_session_end_line(platform_kind, end_cols);

    result
}

/// Check for updates and show modal prompt if available
/// Returns UpdateState to pass to the app for banner display
async fn check_and_prompt_for_update(cli_version: Option<String>) -> UpdateState {
    // Load config to check if updates are enabled
    let config = Config::load().unwrap_or_default();
    if !config.check_for_updates {
        return UpdateState::default();
    }

    // Spawn background task to check for updates
    let check_handle = tokio::spawn(async move { check_for_update(cli_version).await });

    // Wait briefly for result (don't block startup too long)
    let result = tokio::time::timeout(Duration::from_millis(500), check_handle).await;

    let check_result = match result {
        Ok(Ok(Ok(r))) => r,
        _ => return UpdateState::default(), // Timeout or error, skip update check
    };

    // No update available
    if !check_result.update_available {
        return UpdateState::default();
    }

    // User already dismissed this version
    if check_result.was_dismissed {
        return UpdateState::from_check(&check_result, true);
    }

    // Show modal prompt
    let version = check_result.new_version.as_deref().unwrap_or("unknown");
    let current = crate::update::CURRENT_VERSION;
    let install_method = detect_install_method();
    let update_cmd = install_method.update_command();

    // Print update prompt
    println!();
    println!(
        "{}  {}Update available v{} → v{}{}",
        terminal::escape::fg(terminal::escape::color::CYAN),
        terminal::escape::BOLD,
        current,
        version,
        terminal::escape::RESET
    );
    println!(
        "{}  Run: {}{}{}",
        terminal::escape::fg(terminal::escape::color::GRAY),
        terminal::escape::fg(terminal::escape::color::GREEN),
        update_cmd,
        terminal::escape::RESET
    );
    println!();
    print!(
        "{}  Update now? [y/N] {}",
        terminal::escape::fg(terminal::escape::color::GRAY),
        terminal::escape::RESET
    );
    let _ = stdout().flush();

    // Read single character response (non-blocking with timeout)
    let response = read_single_char_with_timeout(Duration::from_secs(10));

    println!(); // Move to next line after input

    match response {
        Some('y') | Some('Y') => {
            // Run update command
            println!(
                "{}  Running: {}{}{}",
                terminal::escape::fg(terminal::escape::color::GRAY),
                terminal::escape::fg(terminal::escape::color::GREEN),
                update_cmd,
                terminal::escape::RESET
            );
            println!();

            // Execute the update command
            let status = if cfg!(windows) {
                std::process::Command::new("cmd")
                    .arg("/C")
                    .arg(update_cmd)
                    .status()
            } else {
                std::process::Command::new("sh")
                    .arg("-c")
                    .arg(update_cmd)
                    .status()
            };

            match status {
                Ok(s) if s.success() => {
                    println!();
                    println!(
                        "{}  Update complete! Please restart crabigator.{}",
                        terminal::escape::fg(terminal::escape::color::GREEN),
                        terminal::escape::RESET
                    );
                    std::process::exit(0);
                }
                Ok(s) => {
                    println!();
                    println!(
                        "{}  Update failed (exit code: {:?}). Continuing with current version.{}",
                        terminal::escape::fg(terminal::escape::color::RED),
                        s.code(),
                        terminal::escape::RESET
                    );
                }
                Err(e) => {
                    println!();
                    println!(
                        "{}  Update failed: {}. Continuing with current version.{}",
                        terminal::escape::fg(terminal::escape::color::RED),
                        e,
                        terminal::escape::RESET
                    );
                }
            }
            // Continue with current version, show banner
            UpdateState::from_check(&check_result, true)
        }
        _ => {
            // User said no or timeout - dismiss this version and show banner
            let _ = dismiss_version(version);
            UpdateState::from_check(&check_result, true)
        }
    }
}

/// Read a single character with timeout (for update prompt)
fn read_single_char_with_timeout(timeout: Duration) -> Option<char> {
    use crossterm::event::{self, Event, KeyCode, KeyEventKind};
    use crossterm::terminal::{disable_raw_mode, enable_raw_mode};

    struct RawModeGuard {
        enabled: bool,
    }

    impl RawModeGuard {
        fn new() -> Self {
            Self {
                enabled: enable_raw_mode().is_ok(),
            }
        }
    }

    impl Drop for RawModeGuard {
        fn drop(&mut self) {
            if self.enabled {
                let _ = disable_raw_mode();
            }
        }
    }

    let raw_guard = RawModeGuard::new();
    if !raw_guard.enabled {
        return None;
    }

    let deadline = Instant::now() + timeout;
    let mut result = None;

    loop {
        let remaining = deadline
            .checked_duration_since(Instant::now())
            .unwrap_or(Duration::ZERO);
        if remaining.is_zero() {
            break;
        }

        match event::poll(remaining) {
            Ok(true) => match event::read() {
                Ok(Event::Key(key)) if key.kind == KeyEventKind::Press => match key.code {
                    KeyCode::Char(c) => {
                        result = Some(c);
                        break;
                    }
                    KeyCode::Enter => {
                        result = Some('\n');
                        break;
                    }
                    _ => {}
                },
                Ok(_) => {}
                Err(_) => break,
            },
            Ok(false) => break,
            Err(_) => break,
        }
    }

    // Drain any queued input so it doesn't leak into the main app.
    while event::poll(Duration::from_millis(0)).unwrap_or(false) {
        let _ = event::read();
    }

    result
}
