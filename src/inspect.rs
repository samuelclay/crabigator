//! Inspect command implementation
//!
//! Discovers and displays state from other running crabigator instances.

use std::fs;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

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
    let pattern = "/tmp/crabigator-mirror-*.json";
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
        println!("File: {}", path.display());

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
