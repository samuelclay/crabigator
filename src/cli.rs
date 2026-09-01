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
    /// Generate a pairing code for Chrome MCP auto-login
    Pair,
    /// Cross-session PR board: every tracked PR from every live session
    Prs {
        /// Print one frame and exit instead of the live view
        once: bool,
    },
    /// Install the URL scheme handler for crabigator:// URLs
    InstallLauncher,
    /// Manage automatic turn recaps
    Recap(RecapCommand),
    /// Save an Anthropic API key for recap generation
    Key { api_key: Option<String> },
    /// Configure the Crabigator cloud service
    Cloud(CloudCommand),
}

#[derive(Clone)]
pub enum CloudCommand {
    /// Save and verify a compatible cloud origin
    Set { url: String, force: bool },
    /// Print the configured cloud origin and local state directory
    Status,
    /// Return to the official Crabigator service
    Reset,
}

/// Recap configuration subcommands
#[derive(Clone)]
pub enum RecapCommand {
    /// Enable automatic recap generation with an Anthropic API key
    Enable {
        api_key: Option<String>,
        model: Option<String>,
    },
    /// Disable recap generation and remove the stored API key
    Disable,
    /// Print current recap configuration status
    Status,
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
            "pair" => {
                iter.next(); // consume "pair"
                args.command = Command::Pair;
                return args;
            }
            "prs" => {
                iter.next(); // consume "prs"
                let once = iter.any(|arg| arg == "--once" || arg == "-1");
                args.command = Command::Prs { once };
                return args;
            }
            "install-launcher" => {
                iter.next(); // consume "install-launcher"
                args.command = Command::InstallLauncher;
                return args;
            }
            "recap" => {
                iter.next(); // consume "recap"
                args.command = Command::Recap(parse_recap_command(iter.collect()));
                return args;
            }
            "key" => {
                iter.next(); // consume "key"
                let api_key = iter.next().filter(|v| !v.starts_with('-'));
                args.command = Command::Key { api_key };
                return args;
            }
            "cloud" => {
                iter.next(); // consume "cloud"
                args.command = Command::Cloud(parse_cloud_command(iter.collect()));
                return args;
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
                        eprintln!(
                            "Unknown platform: {}. Use 'claude', 'codex', 'opencode', or 'grok'.",
                            value
                        );
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

fn parse_cloud_command(args: Vec<String>) -> CloudCommand {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        Some("set") => {
            let Some(url) = iter.next().filter(|value| !value.starts_with('-')) else {
                eprintln!("Usage: crabigator cloud set <origin> [--force]");
                std::process::exit(1);
            };
            let force = iter.any(|arg| arg == "--force");
            CloudCommand::Set { url, force }
        }
        Some("reset") => CloudCommand::Reset,
        Some("status") | None => CloudCommand::Status,
        Some(other) => {
            eprintln!("Unknown cloud command: {other}. Use set, status, or reset.");
            std::process::exit(1);
        }
    }
}

fn parse_recap_command(args: Vec<String>) -> RecapCommand {
    let mut iter = args.into_iter();
    match iter.next().as_deref() {
        Some("disable") => RecapCommand::Disable,
        Some("status") | None => RecapCommand::Status,
        Some("enable") => {
            let mut api_key = None;
            let mut model = None;
            while let Some(arg) = iter.next() {
                match arg.as_str() {
                    "--model" => {
                        model = iter.next();
                    }
                    "--key" => {
                        api_key = iter.next();
                    }
                    value if !value.starts_with('-') && api_key.is_none() => {
                        api_key = Some(value.to_string());
                    }
                    _ => {}
                }
            }
            RecapCommand::Enable { api_key, model }
        }
        Some(other) => {
            eprintln!(
                "Unknown recap command: {}. Use enable, disable, or status.",
                other
            );
            std::process::exit(1);
        }
    }
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
