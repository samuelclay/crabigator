//! CLI argument parsing and startup utilities
//!
//! Handles command-line argument parsing, platform resolution,
//! and debug timing infrastructure.

use std::env;
use std::sync::atomic::AtomicU8;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use crate::config::Config;
use crate::platforms::PlatformKind;

/// Subcommand to run
#[derive(Clone, Default)]
pub enum Command {
    /// Run the main crabigator application
    #[default]
    Run,
    /// Inspect other running instances
    Inspect {
        dir_filter: Option<String>,
        watch: bool,
        raw: bool,
        /// Show hook event history for debugging
        history: bool,
    },
}

/// Parsed command-line arguments
#[derive(Clone)]
pub struct Args {
    pub platform: Option<PlatformKind>,
    pub platform_args: Vec<String>,
    pub profile: bool,
    pub command: Command,
    /// Whether to capture output (default: true)
    pub capture: bool,
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

/// Parse command-line arguments
pub fn parse_args() -> Args {
    let mut args = Args::default();
    let mut iter = env::args().skip(1).peekable(); // Skip the binary name
    let mut platform_selected = false;

    // Check for subcommand first
    if let Some(first) = iter.peek() {
        match first.as_str() {
            "inspect" => {
                iter.next(); // consume "inspect"
                let mut dir_filter = None;
                let mut watch = false;
                let mut raw = false;
                let mut history = false;

                for arg in iter {
                    match arg.as_str() {
                        "--watch" | "-w" => watch = true,
                        "--raw" | "-r" => raw = true,
                        "--history" | "-H" => history = true,
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
                    history,
                };
                return args;
            }
            "continue" | "c" => {
                iter.next(); // consume the subcommand
                args.platform_args.push("--continue".to_string());
            }
            "resume" | "r" => {
                iter.next(); // consume the subcommand
                args.platform_args.push("--resume".to_string());
            }
            _ => {}
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
pub fn resolve_platform(explicit: Option<PlatformKind>) -> PlatformKind {
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
pub fn save_platform_preference(platform: PlatformKind) -> anyhow::Result<()> {
    let mut config = Config::load().unwrap_or_default();
    config.set_default_platform(platform.as_str())
}

/// Startup trace for measuring performance.
/// Enabled with --profile. Dumps to stdout after terminal restore.
#[derive(Clone)]
pub struct DebugTimer {
    enabled: bool,
    start: Instant,
    logs: Arc<Mutex<Vec<String>>>,
    pub hook_state: Arc<AtomicU8>,
    pub hook_error: Arc<Mutex<Option<String>>>,
}

impl DebugTimer {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            start: Instant::now(),
            logs: Arc::new(Mutex::new(Vec::new())),
            hook_state: Arc::new(AtomicU8::new(0)),
            hook_error: Arc::new(Mutex::new(None)),
        }
    }

    pub fn log(&self, msg: &str) {
        if !self.enabled {
            return;
        }
        self.push_line(format!(
            "+{:>6}ms  {}",
            self.start.elapsed().as_millis(),
            msg
        ));
    }

    pub fn duration(&self, label: &str, duration: Duration) {
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

    pub fn set_hook_error(&self, error: String) {
        *self.hook_error.lock().unwrap_or_else(|p| p.into_inner()) = Some(error);
    }

    fn push_line(&self, line: String) {
        let mut guard = self.logs.lock().unwrap_or_else(|p| p.into_inner());
        guard.push(line);
    }

    pub fn dump(&self) {
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
