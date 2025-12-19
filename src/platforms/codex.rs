//! Codex platform implementation
//!
//! Handles hook installation, version management, and stats reading
//! for the OpenAI Codex CLI.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};

use super::{Platform, PlatformStats};

/// Current hook version - should match Cargo.toml version
const HOOK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Python notify script content for Codex
/// Codex's notify callback receives JSON as argv[1] with event data
const NOTIFY_SCRIPT: &str = r#"#!/usr/bin/env python3
"""
Crabigator notify hook for Codex CLI
Handles: agent-turn-complete events

Codex passes a JSON payload as the first command-line argument containing:
  - type: event type (e.g., "agent-turn-complete")
  - last-assistant-message: the assistant's response
  - input-messages: array of conversation messages
  - thread-id: the thread identifier

State machine:
  - ready: Initial state (nothing happened yet)
  - thinking: Codex is actively processing
  - complete: Codex finished responding
"""
# crabigator-notify-version: {VERSION}

import json
import hashlib
import os
import sys
import time
from pathlib import Path

def get_stats_file(cwd: str) -> Path:
    """Get stats file path based on session ID (from env) or working directory hash."""
    session_id = os.environ.get("CRABIGATOR_SESSION_ID")
    if session_id:
        return Path(f"/tmp/crabigator-stats-{session_id}.json")
    # Fallback to cwd hash if no session ID
    cwd_hash = hashlib.md5(cwd.encode()).hexdigest()[:12]
    return Path(f"/tmp/crabigator-stats-{cwd_hash}.json")

def load_stats(stats_file: Path) -> dict:
    """Load existing stats or return defaults."""
    if stats_file.exists():
        try:
            with open(stats_file) as f:
                return json.load(f)
        except (json.JSONDecodeError, IOError):
            pass
    return {
        "messages": 0,
        "subagent_messages": 0,
        "compressions": 0,
        "tools": {},
        "state": "ready",
        "thread_id": None,
        "last_updated": None,
    }

def save_stats(stats_file: Path, stats: dict):
    """Atomically save stats to file."""
    stats["last_updated"] = time.time()

    # Write to temp file then rename for atomicity
    temp_file = stats_file.with_suffix(f'.{os.getpid()}.tmp')
    try:
        with open(temp_file, 'w') as f:
            json.dump(stats, f)
        temp_file.rename(stats_file)
    except OSError:
        try:
            temp_file.unlink(missing_ok=True)
        except Exception:
            pass

def main():
    # Codex passes JSON as first command-line argument
    if len(sys.argv) < 2:
        sys.exit(0)

    try:
        data = json.loads(sys.argv[1])
    except json.JSONDecodeError:
        sys.exit(0)

    event_type = data.get("type", "")
    if event_type != "agent-turn-complete":
        sys.exit(0)

    cwd = os.getcwd()
    stats_file = get_stats_file(cwd)
    stats = load_stats(stats_file)

    # Increment message count
    stats["messages"] += 1
    stats["state"] = "complete"

    # Store thread ID if present
    if "thread-id" in data:
        stats["thread_id"] = data["thread-id"]

    # Count tool calls from input-messages if present
    # Codex message format includes tool calls in the conversation
    input_messages = data.get("input-messages", [])
    for msg in input_messages:
        if isinstance(msg, dict):
            # Look for tool_calls in assistant messages
            tool_calls = msg.get("tool_calls", [])
            for tool_call in tool_calls:
                if isinstance(tool_call, dict):
                    tool_name = tool_call.get("function", {}).get("name", "unknown")
                    stats["tools"][tool_name] = stats["tools"].get(tool_name, 0) + 1

    save_stats(stats_file, stats)
    sys.exit(0)

if __name__ == "__main__":
    main()
"#;

/// Metadata about installed hooks
#[derive(Debug, Serialize, Deserialize)]
struct HooksMeta {
    installed_version: String,
    installed_at: String,
    script_path: String,
}

/// Codex platform implementation
pub struct CodexPlatform {
    /// Path to ~/.codex directory
    codex_dir: PathBuf,
    /// Path to ~/.codex/crabigator directory
    crabigator_dir: PathBuf,
}

impl CodexPlatform {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        let codex_dir = home.join(".codex");
        let crabigator_dir = codex_dir.join("crabigator");

        Self {
            codex_dir,
            crabigator_dir,
        }
    }

    /// Get path to hooks metadata file
    fn meta_path(&self) -> PathBuf {
        self.crabigator_dir.join("hooks-meta.json")
    }

    /// Get path to notify script
    fn script_path(&self) -> PathBuf {
        self.crabigator_dir.join("notify.py")
    }

    /// Get path to Codex config.toml
    fn config_path(&self) -> PathBuf {
        self.codex_dir.join("config.toml")
    }

    fn atomic_write(&self, path: &PathBuf, contents: &str) -> Result<()> {
        let tmp_path = path.with_extension("tmp");
        let mut file = fs::File::create(&tmp_path)
            .with_context(|| format!("Failed to create temp file {}", tmp_path.display()))?;
        file.write_all(contents.as_bytes())
            .with_context(|| format!("Failed to write temp file {}", tmp_path.display()))?;
        file.sync_all()
            .with_context(|| format!("Failed to flush temp file {}", tmp_path.display()))?;
        fs::rename(&tmp_path, path)
            .with_context(|| format!("Failed to rename {} to {}", tmp_path.display(), path.display()))?;
        Ok(())
    }

    /// Check if hooks are installed and current version
    fn is_current_version(&self) -> bool {
        let meta_path = self.meta_path();
        let script_path = self.script_path();
        if !meta_path.exists() {
            return false;
        }
        if !script_path.exists() {
            return false;
        }

        match fs::read_to_string(&meta_path) {
            Ok(content) => {
                match serde_json::from_str::<HooksMeta>(&content) {
                    Ok(meta) => meta.installed_version == HOOK_VERSION,
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    /// Check if our notify hook is registered in config.toml
    fn hooks_registered(&self) -> Result<bool> {
        let config_path = self.config_path();
        if !config_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&config_path)
            .with_context(|| format!("Failed to read {}", config_path.display()))?;

        let config: toml::Value = toml::from_str(&content).with_context(|| {
            format!(
                "{} contains invalid TOML; refusing to overwrite",
                config_path.display()
            )
        })?;

        let script_path_str = self.script_path().to_string_lossy().to_string();

        // Check if notify contains our script
        if let Some(notify) = config.get("notify") {
            if let Some(arr) = notify.as_array() {
                // notify = ["python3", "/path/to/script.py"]
                if arr.len() >= 2 {
                    if let Some(path) = arr.get(1).and_then(|v| v.as_str()) {
                        return Ok(path == script_path_str);
                    }
                }
            }
        }

        Ok(false)
    }

    /// Install or update hooks
    fn install_hooks(&self) -> Result<()> {
        // Create crabigator directory
        fs::create_dir_all(&self.crabigator_dir)
            .context("Failed to create crabigator directory")?;

        // Write notify script with version embedded
        let script_content = NOTIFY_SCRIPT.replace("{VERSION}", HOOK_VERSION);
        let script_path = self.script_path();
        fs::write(&script_path, &script_content)
            .context("Failed to write notify script")?;

        // Make script executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        // Update config.toml
        self.merge_config()?;

        // Write metadata
        let meta = HooksMeta {
            installed_version: HOOK_VERSION.to_string(),
            installed_at: Utc::now().to_rfc3339(),
            script_path: script_path.to_string_lossy().to_string(),
        };
        let meta_content = serde_json::to_string_pretty(&meta)?;
        fs::write(self.meta_path(), meta_content)
            .context("Failed to write hooks metadata")?;

        Ok(())
    }

    /// Merge our notify configuration into config.toml
    fn merge_config(&self) -> Result<()> {
        let config_path = self.config_path();
        let script_path = self.script_path();
        let script_path_str = script_path.to_string_lossy().to_string();

        // Ensure codex directory exists
        fs::create_dir_all(&self.codex_dir)
            .context("Failed to create codex directory")?;

        // Load existing config or create new
        let mut config: toml::Value = if config_path.exists() {
            let content = fs::read_to_string(&config_path)
                .with_context(|| format!("Failed to read {}", config_path.display()))?;
            toml::from_str(&content).with_context(|| {
                format!(
                    "{} contains invalid TOML; refusing to overwrite",
                    config_path.display()
                )
            })?
        } else {
            toml::Value::Table(toml::map::Map::new())
        };

        let table = config.as_table_mut()
            .context("config.toml root must be a table")?;

        // Set notify = ["python3", "/path/to/notify.py"]
        let notify_value = toml::Value::Array(vec![
            toml::Value::String("python3".to_string()),
            toml::Value::String(script_path_str),
        ]);

        // Check if notify is already set correctly
        let needs_update = match table.get("notify") {
            Some(existing) => existing != &notify_value,
            None => true,
        };

        if needs_update {
            table.insert("notify".to_string(), notify_value);

            // Write back config
            let config_content = toml::to_string_pretty(&config)?;
            self.atomic_write(&config_path, &config_content)
                .with_context(|| format!("Failed to write {}", config_path.display()))?;
        }

        Ok(())
    }

    /// Generate MD5 hash prefix for a string (matches Python implementation)
    fn md5_hash_prefix(input: &str, len: usize) -> String {
        let mut hasher = Md5::new();
        hasher.update(input.as_bytes());
        let result = hasher.finalize();
        let hex_string: String = result.iter().map(|b| format!("{:02x}", b)).collect();
        hex_string[..len.min(hex_string.len())].to_string()
    }

    /// Get stats file path - uses session ID from env var if available, otherwise cwd hash
    fn stats_file_path(cwd: &str) -> PathBuf {
        if let Ok(session_id) = std::env::var("CRABIGATOR_SESSION_ID") {
            PathBuf::from(format!("/tmp/crabigator-stats-{}.json", session_id))
        } else {
            // Fallback to cwd hash if no session ID
            let hash = Self::md5_hash_prefix(cwd, 12);
            PathBuf::from(format!("/tmp/crabigator-stats-{}.json", hash))
        }
    }
}

impl Default for CodexPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for CodexPlatform {
    fn ensure_hooks_installed(&self) -> Result<()> {
        if self.is_current_version() {
            match self.hooks_registered() {
                Ok(true) => return Ok(()),
                Ok(false) => {}
                Err(e) => return Err(e),
            }
        }
        self.install_hooks()?;
        Ok(())
    }

    fn load_stats(&self, cwd: &str) -> Result<PlatformStats> {
        let stats_path = Self::stats_file_path(cwd);

        if !stats_path.exists() {
            return Ok(PlatformStats::default());
        }

        let content = fs::read_to_string(&stats_path)
            .context("Failed to read stats file")?;

        let stats: PlatformStats = serde_json::from_str(&content)
            .unwrap_or_default();

        Ok(stats)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hash_prefix() {
        let hash = CodexPlatform::md5_hash_prefix("/Users/test/project", 12);
        assert_eq!(hash.len(), 12);
    }

    #[test]
    fn test_stats_file_path() {
        let path = CodexPlatform::stats_file_path("/Users/test/project");
        assert!(path.to_string_lossy().starts_with("/tmp/crabigator-stats-"));
        assert!(path.to_string_lossy().ends_with(".json"));
    }
}
