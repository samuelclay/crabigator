//! Claude Code platform implementation
//!
//! Handles hook installation, version management, and stats reading
//! for the Claude Code CLI.

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{Platform, PlatformKind, PlatformStats};

/// Current hook version - should match Cargo.toml version
const HOOK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Python hook script content
const HOOK_SCRIPT: &str = r#"#!/usr/bin/env python3
"""
Crabigator stats hook for Claude Code
Handles: PermissionRequest, PostToolUse, Stop, SubagentStop, PreCompact, UserPromptSubmit

State machine:
  - ready: Initial state (nothing happened yet)
  - thinking: Claude is actively processing
  - permission: Claude is waiting for permission approval
  - question: Claude asked a question (AskUserQuestion tool)
  - complete: Claude finished responding
"""
# crabigator-hook-version: {VERSION}

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
        "prompts": 0,
        "completions": 0,
        "subagent_messages": 0,
        "compressions": 0,
        "tools": {},
        "tool_timestamps": [],
        "state": "ready",
        "pending_question": False,
        "idle_since": None,
        "last_updated": None,
    }

def save_stats(stats_file: Path, stats: dict):
    """Atomically save stats to file."""
    stats["last_updated"] = time.time()

    # Write to temp file then rename for atomicity
    # Use unique temp file name to avoid race conditions between concurrent hooks
    temp_file = stats_file.with_suffix(f'.{os.getpid()}.tmp')
    try:
        with open(temp_file, 'w') as f:
            json.dump(stats, f)
        temp_file.rename(stats_file)
    except OSError:
        # If rename fails, try to clean up temp file
        try:
            temp_file.unlink(missing_ok=True)
        except Exception:
            pass

def main():
    try:
        data = json.load(sys.stdin)
    except json.JSONDecodeError:
        sys.exit(0)

    cwd = data.get("cwd", os.getcwd())
    event = data.get("hook_event_name", "")

    stats_file = get_stats_file(cwd)
    stats = load_stats(stats_file)

    if event == "PermissionRequest":
        # Permission dialog is being shown to user
        stats["state"] = "permission"

    elif event == "PostToolUse":
        tool_name = data.get("tool_name", "unknown")
        stats["tools"][tool_name] = stats["tools"].get(tool_name, 0) + 1
        if "tool_timestamps" not in stats:
            stats["tool_timestamps"] = []
        stats["tool_timestamps"].append(time.time())
        # Mark if this was a question tool
        if tool_name == "AskUserQuestion":
            stats["pending_question"] = True
        # Tool completed - back to thinking (more tools may follow)
        stats["state"] = "thinking"

    elif event == "Stop":
        stats["completions"] = stats.get("completions", 0) + 1
        # Transition to question or complete based on pending flag
        if stats.get("pending_question"):
            stats["state"] = "question"
            stats["pending_question"] = False
        else:
            stats["state"] = "complete"
        # Start idle timer
        stats["idle_since"] = time.time()

    elif event == "SubagentStop":
        stats["subagent_messages"] += 1

    elif event == "PreCompact":
        stats["compressions"] += 1

    elif event == "UserPromptSubmit":
        # User submitted input, Claude starts thinking
        stats["prompts"] = stats.get("prompts", 0) + 1
        stats["state"] = "thinking"
        stats["pending_question"] = False
        stats["idle_since"] = None

    save_stats(stats_file, stats)
    sys.exit(0)

if __name__ == "__main__":
    main()
"#;

/// Metadata about installed hooks
#[derive(Debug, Serialize, Deserialize)]
struct HooksMeta {
    installed_version: String,
    /// MD5 hash of the hook script content for change detection
    #[serde(default)]
    script_hash: String,
    installed_at: String,
    script_path: String,
}

/// Claude Code platform implementation
pub struct ClaudeCodePlatform {
    /// Path to ~/.claude directory
    claude_dir: PathBuf,
    /// Path to ~/.claude/crabigator directory
    crabigator_dir: PathBuf,
}

impl ClaudeCodePlatform {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        let claude_dir = home.join(".claude");
        let crabigator_dir = claude_dir.join("crabigator");

        Self {
            claude_dir,
            crabigator_dir,
        }
    }

    /// Get path to hooks metadata file
    fn meta_path(&self) -> PathBuf {
        self.crabigator_dir.join("hooks-meta.json")
    }

    /// Get path to hook script
    fn script_path(&self) -> PathBuf {
        self.crabigator_dir.join("stats-hook.py")
    }

    /// Get path to Claude Code settings.json
    fn settings_path(&self) -> PathBuf {
        self.claude_dir.join("settings.json")
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

    /// Compute hash of the hook script content for change detection
    fn script_content_hash() -> String {
        let script_content = HOOK_SCRIPT.replace("{VERSION}", HOOK_VERSION);
        Self::md5_hash_prefix(&script_content, 32)
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
                    Ok(meta) => {
                        // Check both version and script hash
                        meta.installed_version == HOOK_VERSION
                            && meta.script_hash == Self::script_content_hash()
                    }
                    Err(_) => false,
                }
            }
            Err(_) => false,
        }
    }

    fn settings_has_our_hook(settings: &Value, event: &str, script_path_str: &str) -> bool {
        let Some(hooks) = settings.get("hooks").and_then(|h| h.as_object()) else {
            return false;
        };
        let Some(event_arr) = hooks.get(event).and_then(|v| v.as_array()) else {
            return false;
        };

        let hooks_contains_cmd = |hooks_value: &Value| {
            hooks_value.as_array().is_some_and(|hooks_arr| {
                hooks_arr.iter().any(|hook| {
                    hook.get("command")
                        .and_then(|c| c.as_str())
                        .is_some_and(|cmd| cmd == script_path_str)
                })
            })
        };

        // Events that require matcher="*" to catch all tool types
        let events_with_matcher = ["PermissionRequest", "PostToolUse"];
        if events_with_matcher.contains(&event) {
            event_arr.iter().any(|entry| {
                entry.get("matcher")
                    .and_then(|m| m.as_str())
                    .is_some_and(|m| m == "*")
                    && entry.get("hooks").is_some_and(hooks_contains_cmd)
            })
        } else {
            event_arr.iter().any(|entry| entry.get("hooks").is_some_and(hooks_contains_cmd))
        }
    }

    fn hooks_registered(&self) -> Result<bool> {
        let settings_path = self.settings_path();
        if !settings_path.exists() {
            return Ok(false);
        }

        let content = fs::read_to_string(&settings_path)
            .with_context(|| format!("Failed to read {}", settings_path.display()))?;
        let settings: Value = serde_json::from_str(&content).with_context(|| {
            format!(
                "{} contains invalid JSON; refusing to overwrite",
                settings_path.display()
            )
        })?;

        let script_path_str = self.script_path().to_string_lossy().to_string();
        let hook_events = ["PermissionRequest", "PostToolUse", "Stop", "SubagentStop", "PreCompact", "UserPromptSubmit"];

        Ok(hook_events
            .iter()
            .all(|event| Self::settings_has_our_hook(&settings, event, &script_path_str)))
    }

    /// Install or update hooks
    fn install_hooks(&self) -> Result<()> {
        // Create crabigator directory
        fs::create_dir_all(&self.crabigator_dir)
            .context("Failed to create crabigator directory")?;

        // Write hook script with version embedded
        let script_content = HOOK_SCRIPT.replace("{VERSION}", HOOK_VERSION);
        let script_path = self.script_path();
        fs::write(&script_path, &script_content)
            .context("Failed to write hook script")?;

        // Make script executable on Unix
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let mut perms = fs::metadata(&script_path)?.permissions();
            perms.set_mode(0o755);
            fs::set_permissions(&script_path, perms)?;
        }

        // Update settings.json
        self.merge_settings()?;

        // Write metadata
        let meta = HooksMeta {
            installed_version: HOOK_VERSION.to_string(),
            script_hash: Self::script_content_hash(),
            installed_at: Utc::now().to_rfc3339(),
            script_path: script_path.to_string_lossy().to_string(),
        };
        let meta_content = serde_json::to_string_pretty(&meta)?;
        fs::write(self.meta_path(), meta_content)
            .context("Failed to write hooks metadata")?;

        Ok(())
    }

    /// Merge our hook configuration into settings.json
    fn merge_settings(&self) -> Result<()> {
        let settings_path = self.settings_path();
        let script_path = self.script_path();
        let script_path_str = script_path.to_string_lossy().to_string();

        let mut changed = false;

        // Load existing settings or create new. If settings.json is invalid, refuse to overwrite.
        let mut settings: Value = if settings_path.exists() {
            let content = fs::read_to_string(&settings_path)
                .with_context(|| format!("Failed to read {}", settings_path.display()))?;
            serde_json::from_str(&content).with_context(|| {
                format!(
                    "{} contains invalid JSON; refusing to overwrite",
                    settings_path.display()
                )
            })?
        } else {
            changed = true;
            json!({})
        };

        settings
            .as_object_mut()
            .context("settings.json root must be a JSON object")?;

        // Ensure hooks object exists
        if settings.get("hooks").is_none() {
            settings["hooks"] = json!({});
            changed = true;
        }
        if !settings["hooks"].is_object() {
            anyhow::bail!("settings.json hooks field must be a JSON object; refusing to overwrite");
        }

        // Hook events we need to register
        let hook_events = ["PermissionRequest", "PostToolUse", "Stop", "SubagentStop", "PreCompact", "UserPromptSubmit"];
        // Events that require matcher="*" to catch all tool types
        let events_with_matcher = ["PermissionRequest", "PostToolUse"];

        // Our hook configuration
        let our_hook = json!({
            "type": "command",
            "command": script_path_str
        });

        // For each event type, ensure our hook is registered.
        // We identify our hook by its `command` path and never remove other hooks.
        for event in hook_events {
            if !settings["hooks"].as_object().unwrap().contains_key(event) {
                settings["hooks"][event] = json!([]);
                changed = true;
            }

            let arr = settings["hooks"][event]
                .as_array_mut()
                .with_context(|| format!("settings.json hooks.{} must be a JSON array", event))?;

            let is_our_hook = |hook: &Value| {
                hook.get("command")
                    .and_then(|c| c.as_str())
                    .is_some_and(|cmd| cmd == script_path_str)
            };

            // Determine preferred placement and ensure our hook exists there.
            if events_with_matcher.contains(&event) {
                // For tool-related events, we need matcher="*" to catch all tool types.
                let mut star_idx = arr.iter().position(|entry| {
                    entry.get("matcher")
                        .and_then(|m| m.as_str())
                        .is_some_and(|m| m == "*")
                });

                // If a matcher="*" entry exists and has a hooks array, add our hook there if missing.
                let mut placed = false;
                if let Some(idx) = star_idx {
                    if let Some(hooks_value) = arr[idx].get_mut("hooks") {
                        if let Some(hooks_arr) = hooks_value.as_array_mut() {
                            if !hooks_arr.iter().any(is_our_hook) {
                                hooks_arr.push(our_hook.clone());
                                changed = true;
                            }
                            placed = true;
                        }
                    }
                }

                if !placed {
                    // Create a dedicated matcher="*" entry.
                    arr.push(json!({
                        "matcher": "*",
                        "hooks": [our_hook.clone()]
                    }));
                    changed = true;
                    star_idx = Some(arr.len() - 1);
                }

                // Remove our hook from any other entries (or duplicates in the matcher="*" entry), without touching other hooks.
                let primary_idx = star_idx.expect("star_idx must exist after placement");
                for (entry_idx, entry) in arr.iter_mut().enumerate() {
                    let Some(hooks_value) = entry.get_mut("hooks") else { continue };
                    let Some(hooks_arr) = hooks_value.as_array_mut() else { continue };
                    let mut kept_primary = 0u32;
                    let before = hooks_arr.len();
                    hooks_arr.retain(|hook| {
                        if is_our_hook(hook) {
                            if entry_idx == primary_idx && kept_primary == 0 {
                                kept_primary = 1;
                                true
                            } else {
                                false
                            }
                        } else {
                            true
                        }
                    });
                    if hooks_arr.len() != before {
                        changed = true;
                    }
                }
            } else {
                // Other events: ensure our hook exists in at least one entry.
                let mut found_at: Option<(usize, usize)> = None;
                for (entry_idx, entry) in arr.iter().enumerate() {
                    let Some(hooks_arr) = entry.get("hooks").and_then(|h| h.as_array()) else { continue };
                    for (hook_idx, hook) in hooks_arr.iter().enumerate() {
                        if is_our_hook(hook) {
                            found_at = Some((entry_idx, hook_idx));
                            break;
                        }
                    }
                    if found_at.is_some() {
                        break;
                    }
                }

                if found_at.is_none() {
                    arr.push(json!({
                        "hooks": [our_hook.clone()]
                    }));
                    changed = true;
                }

                // Deduplicate: keep the first occurrence and remove the rest, without touching other hooks.
                let mut kept_one = false;
                for entry in arr.iter_mut() {
                    let Some(hooks_value) = entry.get_mut("hooks") else { continue };
                    let Some(hooks_arr) = hooks_value.as_array_mut() else { continue };
                    let before = hooks_arr.len();
                    hooks_arr.retain(|hook| {
                        if is_our_hook(hook) {
                            if kept_one {
                                false
                            } else {
                                kept_one = true;
                                true
                            }
                        } else {
                            true
                        }
                    });
                    if hooks_arr.len() != before {
                        changed = true;
                    }
                }
            }

            // Drop any entries whose hooks array became empty (these were our-only entries).
            let before_len = arr.len();
            arr.retain(|entry| {
                entry.get("hooks")
                    .and_then(|h| h.as_array())
                    .map(|hooks_arr| !hooks_arr.is_empty())
                    .unwrap_or(true)
            });
            if arr.len() != before_len {
                changed = true;
            }
        }

        if !changed {
            return Ok(());
        }

        // Write back settings
        let settings_content = serde_json::to_string_pretty(&settings)?;
        self.atomic_write(&settings_path, &settings_content)
            .with_context(|| format!("Failed to write {}", settings_path.display()))?;

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

impl Default for ClaudeCodePlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for ClaudeCodePlatform {
    fn kind(&self) -> PlatformKind {
        PlatformKind::Claude
    }

    fn command(&self) -> &'static str {
        PlatformKind::Claude.command()
    }

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

    fn cleanup_stats(&self, cwd: &str) {
        let stats_path = Self::stats_file_path(cwd);
        let _ = fs::remove_file(stats_path);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_md5_hash_prefix() {
        // Test that our hash matches Python's hashlib.md5
        let hash = ClaudeCodePlatform::md5_hash_prefix("/Users/test/project", 12);
        assert_eq!(hash.len(), 12);
        // The actual hash value would need to be verified against Python
    }

    #[test]
    fn test_stats_file_path() {
        let path = ClaudeCodePlatform::stats_file_path("/Users/test/project");
        assert!(path.to_string_lossy().starts_with("/tmp/crabigator-stats-"));
        assert!(path.to_string_lossy().ends_with(".json"));
    }
}
