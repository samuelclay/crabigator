//! Inspect command implementation
//!
//! Discovers and displays state from other running crabigator instances.

use std::fs::{self, metadata};
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

// ANSI colors
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const DIM: &str = "\x1b[2m";
const RESET: &str = "\x1b[0m";

/// Get file status with size info
fn get_file_status(path: &str) -> String {
    match metadata(path) {
        Ok(meta) => {
            let size = meta.len();
            if size == 0 {
                format!("{YELLOW}(empty){RESET}")
            } else {
                let size_str = format_size(size);
                format!("{GREEN}({size_str}){RESET}")
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
pub fn run_inspect(dir_filter: Option<String>, watch: bool, raw: bool) -> Result<()> {
    loop {
        let instances = discover_instances(&dir_filter)?;

        if raw {
            for (path, data) in &instances {
                println!("--- {} ---", path.display());
                println!("{}", serde_json::to_string_pretty(data)?);
            }
        } else {
            print_pretty(&instances)?;
        }

        if !watch {
            break;
        }

        // Clear screen and wait before next update
        print!("\x1b[2J\x1b[H"); // Clear screen, move cursor home
        thread::sleep(Duration::from_millis(500));
    }

    Ok(())
}

fn discover_instances(dir_filter: &Option<String>) -> Result<Vec<(PathBuf, Value)>> {
    let pattern = "/tmp/crabigator-*/mirror.json";
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
