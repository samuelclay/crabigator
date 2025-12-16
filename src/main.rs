mod app;
mod git;
mod hooks;
mod parsers;
mod platforms;
mod terminal;
mod ui;

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
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::app::App;

#[derive(Clone, Default)]
struct Args {
    claude_args: Vec<String>,
    profile: bool,
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut iter = env::args().skip(1); // Skip the binary name

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--debug-startup" => {
                args.profile = true;
            }
            "--profile" => {
                args.profile = true;
            }
            "-r" | "--resume" => {
                args.claude_args.push("--resume".to_string());
            }
            "-c" | "--continue" => {
                args.claude_args.push("--continue".to_string());
            }
            _ => {
                // Pass through any other arguments to claude
                args.claude_args.push(arg);
            }
        }
    }

    args
}

/// Startup trace for measuring performance.
/// Enabled with --profile. Dumps to stdout after terminal restore.
#[derive(Clone)]
struct DebugTimer {
    enabled: bool,
    start: Instant,
    logs: Arc<Mutex<Vec<String>>>,
    hook_state: Arc<AtomicU8>,
    hook_error: Arc<Mutex<Option<String>>>,
}

impl DebugTimer {
    fn new(enabled: bool) -> Self {
        Self {
            enabled,
            start: Instant::now(),
            logs: Arc::new(Mutex::new(Vec::new())),
            hook_state: Arc::new(AtomicU8::new(0)),
            hook_error: Arc::new(Mutex::new(None)),
        }
    }

    fn log(&self, msg: &str) {
        if !self.enabled {
            return;
        }
        self.push_line(format!(
            "+{:>6}ms  {}",
            self.start.elapsed().as_millis(),
            msg
        ));
    }

    fn duration(&self, label: &str, duration: Duration) {
        if !self.enabled {
            return;
        }
        self.push_line(format!(
            "+{:>6}ms  {:<28} {:>6}ms",
            self.start.elapsed().as_millis(),
            label,
            duration.as_millis()
        ));
    }

    fn set_hook_error(&self, error: String) {
        *self.hook_error.lock().unwrap_or_else(|p| p.into_inner()) = Some(error);
    }

    fn push_line(&self, line: String) {
        let mut guard = self.logs.lock().unwrap_or_else(|p| p.into_inner());
        guard.push(line);
    }

    fn dump(&self) {
        if !self.enabled {
            return;
        }
        let lines = self.logs.lock().unwrap_or_else(|p| p.into_inner()).clone();
        if lines.is_empty() {
            return;
        }
        println!("\nStartup trace:");
        for line in &lines {
            println!("  {}", line);
        }
    }
}

fn setup_terminal() -> Result<(u16, u16)> {
    let mut stdout = stdout();
    let (cols, rows) = terminal_size()?;

    // Push existing content up into scrollback by printing newlines
    // This preserves the command that launched us in the scrollback buffer
    for _ in 0..rows {
        writeln!(stdout)?;
    }
    stdout.flush()?;

    enable_raw_mode()?;

    // Move cursor to top of screen and enable bracketed paste
    // Primary screen buffer (no alternate screen) - allows native scrollback
    // Disable mouse capture to allow native text selection
    write!(stdout, "{}", terminal::escape::CURSOR_HOME)?;
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
    // This ensures we're below all content (Claude output + status widgets)
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
        let _ = disable_raw_mode();
        // Reset scroll region
        print!("{}", terminal::escape::SCROLL_REGION_RESET);
        let _ = execute!(
            stdout(),
            DisableBracketedPaste
        );
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
    // Generate and set session ID before anything else
    // This ensures Claude Code and our stats loading use the same ID
    let session_id = generate_session_id();
    env::set_var("CRABIGATOR_SESSION_ID", &session_id);

    let args = parse_args();
    let timer = DebugTimer::new(args.profile);

    timer.log("args parsed");
    timer.log(&format!("session_id={}", session_id));

    // Install/update Claude Code hooks in background thread (fire and forget)
    // Don't block startup - hooks will be ready by the time Claude needs them
    {
        let timer = timer.clone();
        std::thread::spawn(move || {
            timer.hook_state.store(1, Ordering::SeqCst);
            let begin = Instant::now();
            timer.log("hook install started");

            let result = std::panic::catch_unwind(|| {
                let platform = platforms::current_platform();
                platforms::Platform::ensure_hooks_installed(&platform)
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

    let begin = Instant::now();
    let (cols, rows) = setup_terminal()?;
    timer.duration("setup terminal", begin.elapsed());

    let begin = Instant::now();
    let mut app = App::new(cols, rows, args.claude_args).await?;
    timer.duration("App::new", begin.elapsed());

    timer.log("Starting main loop");

    let begin = Instant::now();
    let result = app.run().await;
    timer.duration("app.run", begin.elapsed());

    // Capture stats and layout before restoring terminal
    let stats = app.claude_stats.clone();
    let total_rows = app.total_rows;

    let begin = Instant::now();
    restore_terminal(total_rows)?;
    timer.duration("restore terminal", begin.elapsed());

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

    // Print session summary after exit
    println!();
    println!(
        "Session: {} messages, {} tool calls",
        stats.platform_stats.messages,
        stats.platform_stats.total_tool_calls()
    );

    result
}
