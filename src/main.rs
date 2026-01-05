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
mod mirror;
mod parsers;
mod platforms;
mod terminal;
mod ui;

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
use std::time::Instant;

use crate::app::App;
use crate::banner::{print_session_banner, print_session_end_line};
use crate::cli::{parse_args, resolve_platform, Command, DebugTimer};

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
    execute!(
        stdout,
        EnableBracketedPaste
    )?;

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
    execute!(
        stdout,
        DisableBracketedPaste,
        Show
    )?;
    stdout.flush()?;
    Ok(())
}

fn setup_panic_handler() {
    let original_hook = panic::take_hook();
    panic::set_hook(Box::new(move |panic_info| {
        // First disable raw mode to ensure newlines work
        let _ = disable_raw_mode();
        // Reset scroll region and flush
        let mut stdout = stdout();
        let _ = write!(stdout, "{}", terminal::escape::SCROLL_REGION_RESET);
        let _ = stdout.flush();
        let _ = execute!(
            stdout,
            DisableBracketedPaste,
            Show
        );
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
        let app_result = App::new(cols, rows, platform, args.platform_args, args.capture).await;
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
            if let Some(err) = timer.hook_error.lock().unwrap_or_else(|p| p.into_inner()).take() {
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
