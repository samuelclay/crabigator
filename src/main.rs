mod app;
mod capture;
mod config;
mod git;
mod hooks;
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
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::app::App;
use crate::config::Config;
use crate::platforms::PlatformKind;

// ANSI color codes (256-color palette)
const RESET: &str = "\x1b[0m";
const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";

// Colors matching terminal/escape.rs palette
const FG_CYAN: &str = "\x1b[38;5;45m";
const FG_BLUE: &str = "\x1b[38;5;39m";
const FG_PURPLE: &str = "\x1b[38;5;141m";
const FG_ORANGE: &str = "\x1b[38;5;179m";
const FG_GRAY: &str = "\x1b[38;5;240m";

/// Print session info banner with file paths
fn print_session_banner(session_id: &str, platform: PlatformKind, capture_enabled: bool, cols: u16) {
    let width = cols as usize;

    println!();

    // Header line: 🦀 CRABIGATOR ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━ Platform
    let platform_name = platform.display_name();
    let title = format!("{FG_ORANGE}🦀{RESET} {BOLD}{FG_CYAN}CRABIGATOR{RESET}");
    let platform_label = format!("{FG_PURPLE}{platform_name}{RESET}");
    // Plain lengths: "🦀" (2 cells) + " " (1) + "CRABIGATOR" (10)
    let title_plain_len = 2 + 1 + 10;
    let platform_plain_len = platform_name.len();
    let rule_len = width.saturating_sub(title_plain_len + platform_plain_len + 2);
    let rule = format!("{FG_GRAY}{}{RESET}", "━".repeat(rule_len));
    println!("{title} {rule} {platform_label}");

    // File paths
    if capture_enabled {
        let capture_dir = format!("/tmp/crabigator-capture-{}", session_id);
        let scrollback_path = format!("{}/scrollback.log", capture_dir);
        let screen_path = format!("{}/screen.txt", capture_dir);

        print_path_line("Log", &scrollback_path, FG_CYAN);
        print_path_line("Screen", &screen_path, FG_BLUE);
    } else {
        println!("   {DIM}(capture disabled){RESET}");
    }

    let mirror_path = format!("/tmp/crabigator-mirror-{}.json", session_id);
    print_path_line("Mirror", &mirror_path, FG_PURPLE);

    // Footer rule
    println!("{FG_GRAY}{}{RESET}", "━".repeat(width));
    println!();
}

fn print_path_line(label: &str, path: &str, label_color: &str) {
    // Format: "   Label   /path/to/file"
    let label_width: usize = 8;
    let label_padding = label_width.saturating_sub(label.len());
    println!(
        "   {label_color}{label}{RESET}{}{DIM}{path}{RESET}",
        " ".repeat(label_padding)
    );
}

#[derive(Clone)]
enum Command {
    /// Run the main crabigator application
    Run,
    /// Inspect other running instances
    Inspect {
        dir_filter: Option<String>,
        watch: bool,
        raw: bool,
    },
}

impl Default for Command {
    fn default() -> Self {
        Command::Run
    }
}

#[derive(Clone)]
struct Args {
    platform: Option<PlatformKind>,
    platform_args: Vec<String>,
    profile: bool,
    command: Command,
    /// Whether to capture output (default: true)
    capture: bool,
}

impl Default for Args {
    fn default() -> Self {
        Self {
            platform: None,
            platform_args: Vec::new(),
            profile: false,
            command: Command::default(),
            capture: true, // On by default
        }
    }
}

fn parse_args() -> Args {
    let mut args = Args::default();
    let mut iter = env::args().skip(1).peekable(); // Skip the binary name
    let mut platform_selected = false;

    // Check for subcommand first
    if let Some(first) = iter.peek() {
        if first == "inspect" {
            iter.next(); // consume "inspect"
            let mut dir_filter = None;
            let mut watch = false;
            let mut raw = false;

            for arg in iter {
                match arg.as_str() {
                    "--watch" | "-w" => watch = true,
                    "--raw" | "-r" => raw = true,
                    _ if !arg.starts_with('-') && dir_filter.is_none() => {
                        dir_filter = Some(arg);
                    }
                    _ => {}
                }
            }

            args.command = Command::Inspect {
                dir_filter,
                watch,
                raw,
            };
            return args;
        }
    }

    while let Some(arg) = iter.next() {
        match arg.as_str() {
            "--debug-startup" => {
                args.profile = true;
            }
            "--profile" => {
                args.profile = true;
            }
            "--platform" | "-p" => {
                if let Some(value) = iter.next() {
                    if let Some(platform) = PlatformKind::parse(&value) {
                        args.platform = Some(platform);
                        platform_selected = true;
                    } else {
                        eprintln!("Unknown platform: {}. Use 'claude' or 'codex'.", value);
                        std::process::exit(1);
                    }
                }
            }
            "-r" | "--resume" => {
                args.platform_args.push("--resume".to_string());
            }
            "-c" | "--continue" => {
                args.platform_args.push("--continue".to_string());
            }
            "--no-capture" => {
                args.capture = false;
            }
            _ => {
                if !platform_selected && !arg.starts_with('-') {
                    if let Some(platform) = PlatformKind::parse(&arg) {
                        args.platform = Some(platform);
                        platform_selected = true;
                        continue;
                    }
                }
                // Pass through any other arguments to the platform CLI
                args.platform_args.push(arg);
            }
        }
    }

    args
}

/// Resolve platform from explicit arg, env var, config file, or default
/// If explicitly selected, saves preference to config for future use.
fn resolve_platform(explicit: Option<PlatformKind>) -> PlatformKind {
    if let Some(kind) = explicit {
        let _ = save_platform_preference(kind);
        return kind;
    }

    if let Ok(env_platform) = env::var("CRABIGATOR_PLATFORM") {
        if let Some(kind) = PlatformKind::parse(&env_platform) {
            return kind;
        }
    }

    if let Ok(config) = Config::load() {
        if let Some(kind) = PlatformKind::parse(&config.default_platform) {
            return kind;
        }
    }

    PlatformKind::Claude
}

/// Save platform preference to config file
fn save_platform_preference(platform: PlatformKind) -> anyhow::Result<()> {
    let mut config = Config::load().unwrap_or_default();
    config.set_default_platform(platform.as_str())
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
        } => {
            return inspect::run_inspect(dir_filter, watch, raw);
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
        let platform_kind = platform_kind;
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
    print_session_banner(&session_id, platform_kind, args.capture, cols);

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

    let (result, stats, final_rows) = {
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
                let stats = app.session_stats.clone();
                let total_rows = app.total_rows;
                (run_result, Some(stats), total_rows)
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

    // Print session summary after exit
    if let Some(stats) = stats {
        println!();
        println!(
            "Session: {} prompts, {} completions, {} tool calls",
            stats.platform_stats.prompts,
            stats.platform_stats.completions,
            stats.platform_stats.total_tool_calls()
        );
    }

    result
}
