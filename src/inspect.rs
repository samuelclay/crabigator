//! Inspect command implementation
//!
//! Discovers and displays state from other running crabigator instances.

use std::fs::{self, metadata};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use chrono::{Local, TimeZone};
use serde_json::Value;

use crate::platforms::PlatformStats;
use crate::terminal::escape::{ansi, CLEAR_SCREEN_HOME, DIM, RESET};

/// Get file status with size info
fn get_file_status(path: &str) -> String {
    match metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            if size == 0 {
                format!("{}(empty){}", ansi::YELLOW, RESET)
            } else {
                let size_str = format_size(size);
                format!("{}({size_str}){}", ansi::GREEN, RESET)
            }
        }
        Err(_) => format!("{DIM}(not found){RESET}"),
    }
}

/// Format file size human-readable
fn format_size(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{} B", bytes)
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}

/// Run the inspect command
pub fn run_inspect(dir_filter: Option<String>, watch: bool, raw: bool, history: bool) -> Result<()> {
    loop {
        let instances = discover_instances(&dir_filter)?;

        if raw {
            for (path, data) in &instances {
                println!("--- {} ---", path.display());
                println!("{}", serde_json::to_string_pretty(data)?);
            }
        } else if history {
            print_history(&instances)?;
        } else {
            print_pretty(&instances)?;
        }

        if !watch {
            break;
        }

        // Clear screen and wait before next update
        print!("{CLEAR_SCREEN_HOME}");
        thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

/// Load stats file for a session to get event history
fn load_stats_for_session(session_id: &str) -> Option<PlatformStats> {
    let stats_path = format!("/tmp/crabigator-stats-{}.json", session_id);
    if let Ok(content) = fs::read_to_string(&stats_path) {
        serde_json::from_str(&content).ok()
    } else {
        None
    }
}

/// Format Unix timestamp as local time string
fn format_timestamp(ts: f64) -> String {
    let secs = ts as i64;
    let nanos = ((ts - secs as f64) * 1_000_000_000.0) as u32;
    if let Some(dt) = Local.timestamp_opt(secs, nanos).single() {
        dt.format("%H:%M:%S%.3f").to_string()
    } else {
        format!("{:.3}", ts)
    }
}

/// Print event history for debugging
fn print_history(instances: &[(PathBuf, Value)]) -> Result<()> {
    if instances.is_empty() {
        println!("No active crabigator instances found.");
        return Ok(());
    }

    for (_path, data) in instances {
        let session_id = data
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let cwd = data.get("cwd").and_then(|v| v.as_str()).unwrap_or("unknown");

        println!("\n=== Session {} ===", session_id);
        println!("Directory: {}", cwd);

        // Load stats file to get event history
        if let Some(stats) = load_stats_for_session(session_id) {
            let current_state = format!("{:?}", stats.state);
            println!("Current state: {}", current_state);

            if stats.event_history.is_empty() {
                println!("\n[Event History] {DIM}(empty - hooks may need reinstalling){RESET}");
            } else {
                println!("\n[Event History] ({} events)", stats.event_history.len());
                println!(
                    "  {:<15} {:<20} {:<12} Details",
                    "Time", "Event", "State Before"
                );
                println!("  {}", "-".repeat(70));

                for event in &stats.event_history {
                    let time_str = format_timestamp(event.ts);
                    let details_str = event
                        .details
                        .as_ref()
                        .map(|d| {
                            d.iter()
                                .map(|(k, v)| format!("{}={}", k, v))
                                .collect::<Vec<_>>()
                                .join(", ")
                        })
                        .unwrap_or_default();

                    println!(
                        "  {:<15} {:<20} {:<12} {}",
                        time_str,
                        event.event,
                        event.state_before,
                        details_str
                    );
                }
            }
        } else {
            println!("\n[Event History] {DIM}(stats file not found){RESET}");
        }

        // Show hooks log if it exists
        let hooks_log = format!("/tmp/crabigator-{}/hooks.log", session_id);
        if let Ok(content) = fs::read_to_string(&hooks_log) {
            let lines: Vec<&str> = content.lines().collect();
            let recent = if lines.len() > 20 {
                &lines[lines.len() - 20..]
            } else {
                &lines[..]
            };

            println!("\n[Hooks Log] (last {} of {} lines)", recent.len(), lines.len());
            for line in recent {
                println!("  {}", line);
            }
        }

        println!();
    }

    Ok(())
}

fn discover_instances(dir_filter: &Option<String>) -> Result<Vec<(PathBuf, Value)>> {
    let pattern = "/tmp/crabigator-*/inspect.json";
    let mut instances = vec![];

    for entry in glob::glob(pattern)? {
        let path = entry?;
        if let Ok(content) = fs::read_to_string(&path) {
            if let Ok(data) = serde_json::from_str::<Value>(&content) {
                // Apply directory filter
                if let Some(filter) = dir_filter {
                    if let Some(cwd) = data.get("cwd").and_then(|v| v.as_str()) {
                        if !cwd.contains(filter) {
                            continue;
                        }
                    }
                }
                instances.push((path, data));
            }
        }
    }

    Ok(instances)
}

fn print_pretty(instances: &[(PathBuf, Value)]) -> Result<()> {
    if instances.is_empty() {
        println!("No active crabigator instances found.");
        return Ok(());
    }

    for (path, data) in instances {
        let session_id = data
            .get("session_id")
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");
        let cwd = data.get("cwd").and_then(|v| v.as_str()).unwrap_or("unknown");

        println!("\n=== Session {} ===", session_id);
        println!("Directory: {}", cwd);
        println!("Mirror: {}", path.display());

        // Show launch timing
        if let Some(timing) = data.get("launch_timing") {
            let uptime = timing.get("uptime_secs").and_then(|v| v.as_u64()).unwrap_or(0);
            let git_ms = timing.get("git_time_ms").and_then(|v| v.as_u64());
            let diff_ms = timing.get("diff_time_ms").and_then(|v| v.as_u64());

            print!("Uptime: {}s", uptime);

            match (git_ms, diff_ms) {
                (Some(g), Some(d)) => {
                    let total = g + d;
                    let color = if total > 1000 {
                        ansi::YELLOW
                    } else {
                        ansi::GREEN
                    };
                    println!(" | Initial load: {color}{}ms{RESET} (git: {}ms, diff: {}ms)", total, g, d);
                }
                _ => println!(" | Initial load: {DIM}pending...{RESET}"),
            }
        }

        // Show capture info
        if let Some(capture) = data.get("capture") {
            let enabled = capture.get("enabled").and_then(|v| v.as_bool()).unwrap_or(false);

            if enabled {
                println!("\n[Capture]");

                if let Some(scrollback) = capture.get("scrollback_path").and_then(|v| v.as_str()) {
                    let status = get_file_status(scrollback);
                    println!("  Scrollback: {} {}", scrollback, status);
                }

                if let Some(screen) = capture.get("screen_path").and_then(|v| v.as_str()) {
                    let status = get_file_status(screen);
                    println!("  Screen:     {} {}", screen, status);
                }
            } else {
                println!("\n[Capture] {DIM}disabled{RESET}");
            }
        }

        if let Some(widgets) = data.get("widgets") {
            // Stats
            if let Some(stats) = widgets.get("stats") {
                println!("\n[Stats]");
                if let Some(rendered) = stats.get("rendered").and_then(|v| v.as_array()) {
                    for line in rendered {
                        if let Some(s) = line.as_str() {
                            println!("  {}", s);
                        }
                    }
                }
            }

            // Git
            if let Some(git) = widgets.get("git") {
                println!("\n[Git]");
                if let Some(rendered) = git.get("rendered").and_then(|v| v.as_array()) {
                    for line in rendered {
                        if let Some(s) = line.as_str() {
                            println!("  {}", s);
                        }
                    }
                }
            }

            // Changes
            if let Some(changes) = widgets.get("changes") {
                println!("\n[Changes]");
                if let Some(rendered) = changes.get("rendered").and_then(|v| v.as_array()) {
                    for line in rendered {
                        if let Some(s) = line.as_str() {
                            println!("  {}", s);
                        }
                    }
                }
            }
        }

        println!();
    }

    Ok(())
}
