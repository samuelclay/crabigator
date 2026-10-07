//! Claude Code platform implementation
//!
//! Handles hook installation, version management, and stats reading
//! for the Claude Code CLI.

mod hook_script;
pub mod transcript;

use std::fs;
use std::io::Write;
use std::path::PathBuf;

use anyhow::{Context, Result};
use chrono::Utc;
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use super::{Platform, PlatformKind, PlatformStats};
use hook_script::{script_with_version, HOOK_VERSION};

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

/// A hook event crabigator listens to, and how its hook is registered.
struct HookSpec {
    event: &'static str,
    /// Tool events register under matcher "*" to hear every tool.
    tool_matcher: bool,
    /// Run without Claude Code waiting on it.
    run_async: bool,
}

const fn hook(event: &'static str) -> HookSpec {
    HookSpec {
        event,
        tool_matcher: false,
        run_async: false,
    }
}

const fn tool_hook(event: &'static str) -> HookSpec {
    HookSpec {
        event,
        tool_matcher: true,
        run_async: false,
    }
}

/// Every hook event Claude Code offers, except three:
/// - WorktreeCreate and WorktreeRemove: a hook there replaces git's own
///   worktree handling, so one that only listens would break worktrees.
/// - FileChanged: it watches only files a hook names, so it never fires.
///
/// MessageDisplay sits on the path that shows streamed text, so it runs
/// async: the text never waits on it.
const HOOK_EVENTS: &[HookSpec] = &[
    hook("SessionStart"),
    hook("SessionEnd"),
    hook("Setup"),
    hook("UserPromptSubmit"),
    hook("UserPromptExpansion"),
    tool_hook("PreToolUse"),
    tool_hook("PermissionRequest"),
    tool_hook("PermissionDenied"),
    tool_hook("PostToolUse"),
    tool_hook("PostToolUseFailure"),
    hook("PostToolBatch"),
    HookSpec {
        event: "MessageDisplay",
        tool_matcher: false,
        run_async: true,
    },
    hook("Notification"),
    hook("SubagentStart"),
    hook("SubagentStop"),
    hook("TaskCreated"),
    hook("TaskCompleted"),
    hook("TeammateIdle"),
    hook("Stop"),
    hook("StopFailure"),
    hook("PreCompact"),
    hook("PostCompact"),
    hook("PreModelSwitch"),
    hook("PostModelSwitch"),
    hook("Elicitation"),
    hook("ElicitationResult"),
    hook("InstructionsLoaded"),
    hook("ConfigChange"),
    hook("CwdChanged"),
    hook("DirectoryAdded"),
];

/// Where the hook appends a line for every hook event of a crabigator
/// session (`activity_log_path` in stats_hook.py), for the flow column.
pub fn activity_log_path(session_id: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/crabigator-{session_id}/activity.jsonl"))
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
        fs::rename(&tmp_path, path).with_context(|| {
            format!(
                "Failed to rename {} to {}",
                tmp_path.display(),
                path.display()
            )
        })?;
        Ok(())
    }

    /// Compute hash of the hook script content for change detection
    fn script_content_hash() -> String {
        Self::md5_hash_prefix(&script_with_version(), 32)
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

    /// Our hook entry for an event: the script, async where it must not hold Claude up.
    fn our_hook(spec: &HookSpec, script_path_str: &str) -> Value {
        let mut hook = json!({
            "type": "command",
            "command": script_path_str
        });
        if spec.run_async {
            hook["async"] = json!(true);
        }
        hook
    }

    fn settings_has_our_hook(settings: &Value, spec: &HookSpec, script_path_str: &str) -> bool {
        let Some(hooks) = settings.get("hooks").and_then(|h| h.as_object()) else {
            return false;
        };
        let Some(event_arr) = hooks.get(spec.event).and_then(|v| v.as_array()) else {
            return false;
        };

        let ours = Self::our_hook(spec, script_path_str);
        let hooks_contains_ours = |hooks_value: &Value| {
            hooks_value
                .as_array()
                .is_some_and(|hooks_arr| hooks_arr.contains(&ours))
        };

        if spec.tool_matcher {
            event_arr.iter().any(|entry| {
                entry
                    .get("matcher")
                    .and_then(|m| m.as_str())
                    .is_some_and(|m| m == "*")
                    && entry.get("hooks").is_some_and(hooks_contains_ours)
            })
        } else {
            event_arr
                .iter()
                .any(|entry| entry.get("hooks").is_some_and(hooks_contains_ours))
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
        Ok(HOOK_EVENTS
            .iter()
            .all(|spec| Self::settings_has_our_hook(&settings, spec, &script_path_str)))
    }

    /// Install or update hooks
    fn install_hooks(&self) -> Result<()> {
        // Create crabigator directory
        fs::create_dir_all(&self.crabigator_dir)
            .context("Failed to create crabigator directory")?;

        // Write hook script with version embedded
        let script_path = self.script_path();
        fs::write(&script_path, script_with_version()).context("Failed to write hook script")?;

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
        fs::write(self.meta_path(), meta_content).context("Failed to write hooks metadata")?;

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

        // For each event type, ensure our hook is registered.
        // We identify our hook by its `command` path and never remove other hooks.
        for spec in HOOK_EVENTS {
            let event = spec.event;
            let our_hook = Self::our_hook(spec, &script_path_str);
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
            if spec.tool_matcher {
                // For tool-related events, we need matcher="*" to catch all tool types.
                let mut star_idx = arr.iter().position(|entry| {
                    entry
                        .get("matcher")
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
                    let Some(hooks_value) = entry.get_mut("hooks") else {
                        continue;
                    };
                    let Some(hooks_arr) = hooks_value.as_array_mut() else {
                        continue;
                    };
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
                    let Some(hooks_arr) = entry.get("hooks").and_then(|h| h.as_array()) else {
                        continue;
                    };
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
                    let Some(hooks_value) = entry.get_mut("hooks") else {
                        continue;
                    };
                    let Some(hooks_arr) = hooks_value.as_array_mut() else {
                        continue;
                    };
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

            // Bring the hook we kept up to date (an older install lacks `async`).
            for hook in arr
                .iter_mut()
                .filter_map(|entry| entry.get_mut("hooks").and_then(|h| h.as_array_mut()))
                .flatten()
            {
                if is_our_hook(hook) && *hook != our_hook {
                    *hook = our_hook.clone();
                    changed = true;
                }
            }

            // Drop any entries whose hooks array became empty (these were our-only entries).
            let before_len = arr.len();
            arr.retain(|entry| {
                entry
                    .get("hooks")
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

        let content = fs::read_to_string(&stats_path).context("Failed to read stats file")?;

        let mut stats: PlatformStats = serde_json::from_str(&content).unwrap_or_default();

        // Cap tool_timestamps to prevent unbounded growth in long sessions.
        // The Python hook caps at 1000, but existing stats files may be larger.
        if stats.tool_timestamps.len() > 1000 {
            let start = stats.tool_timestamps.len() - 1000;
            stats.tool_timestamps = stats.tool_timestamps[start..].to_vec();
        }

        Ok(stats)
    }

    fn cleanup_stats(&self, cwd: &str) {
        let stats_path = Self::stats_file_path(cwd);
        let _ = fs::remove_file(stats_path.with_extension("lock"));
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

    /// Run Python `body` against the hook script, in a temp dir as `root`:
    /// `run(event)` feeds it one hook input, and `stats()` and `activity()`
    /// read the stats file and activity log it wrote there.
    fn run_hook_script(body: &str) {
        let tmp = tempfile::tempdir().unwrap();
        fs::write(tmp.path().join("stats_hook.py"), script_with_version()).unwrap();
        let prelude = r#"
import io, json, pathlib, runpy, sys
root = pathlib.Path(sys.argv[1])
hook = runpy.run_path(str(root / 'stats_hook.py'))
main = hook['main']
main.__globals__['get_stats_file'] = lambda cwd: root / 'stats.json'
main.__globals__['activity_log_path'] = lambda session_id: root / 'activity.jsonl'
main.__globals__['debug_log'] = lambda *args: None
main.__globals__['create_claude_session_symlink'] = lambda *args: None
def run(event):
    sys.stdin = io.StringIO(json.dumps({'cwd': str(root), 'session_id': 'conversation', **event}))
    main()
def stats():
    return json.loads((root / 'stats.json').read_text())
def activity():
    return [json.loads(line) for line in (root / 'activity.jsonl').read_text().splitlines()]
"#;
        let output = std::process::Command::new("python3")
            .args(["-c", &format!("{prelude}{body}")])
            .arg(tmp.path())
            .env("CRABIGATOR_SESSION_ID", "hook-test")
            .output()
            .expect("Python 3 runs the Claude hook");
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    #[test]
    fn resume_hook_restores_transcript_and_directory_before_the_first_prompt() {
        run_hook_script(
            r#"
work = root / 'worktree'
work.mkdir()
transcript = root / 'session.jsonl'
transcript.write_text(json.dumps({'cwd': str(root)}) + '\n' + json.dumps({'cwd': str(work)}) + '\n' + json.dumps({'cwd': str(root), 'isSidechain': True}) + '\n')
event = {'hook_event_name': 'SessionStart', 'source': 'resume', 'transcript_path': str(transcript), 'session_id': 'resumed-conversation'}
run(event)
resumed = stats()
assert resumed['transcript_path'] == str(transcript)
assert resumed['working_directory'] == str(work)
assert resumed['claude_session_id'] == 'resumed-conversation'
assert resumed['state'] == 'ready'
assert resumed['prompts'] == 0
resumed.update(state='thinking', active_prompt='Keep working')
(root / 'stats.json').write_text(json.dumps(resumed))
run({**event, 'source': 'compact'})
assert stats()['state'] == 'thinking'
assert stats()['active_prompt'] == 'Keep working'
"#,
        );
    }

    #[test]
    fn every_event_reaches_the_activity_log_and_only_state_changes_write_stats() {
        run_hook_script(
            r#"
effort = {'level': 'xhigh'}
run({'hook_event_name': 'PreToolUse', 'tool_name': 'Edit', 'tool_use_id': 't1', 'effort': effort,
     'tool_input': {'old_string': 'a', 'new_string': 'one\ntwo\nthree'}})
run({'hook_event_name': 'MessageDisplay', 'delta': 'Hello, world\n', 'effort': effort})
run({'hook_event_name': 'PostToolBatch', 'tool_calls': [{}, {}]})
run({'hook_event_name': 'Notification', 'notification_type': 'idle_prompt'})
run({'hook_event_name': 'SubagentStart', 'agent_id': 'a1', 'agent_type': 'Explore'})
run({'hook_event_name': 'PostToolUseFailure', 'tool_name': 'Bash', 'tool_use_id': 't2', 'is_interrupt': True})
assert not (root / 'stats.json').exists(), 'none of those change state'
lines = activity()
assert lines[0]['ev'] == 'PreToolUse' and lines[0]['lines'] == 3 and lines[0]['effort'] == 'xhigh', lines[0]
assert lines[0]['tool'] == 'Edit' and lines[0]['id'] == 't1'
assert lines[1]['chars'] == 13 and lines[2]['calls'] == 2
assert lines[3]['notification_type'] == 'idle_prompt'
assert lines[4]['agent'] == 'a1' and lines[4]['agent_type'] == 'Explore'
assert lines[5]['interrupt'] is True
run({'hook_event_name': 'PostToolUseFailure', 'tool_name': 'Bash', 'tool_use_id': 't3', 'error': 'exit 1'})
assert stats()['tools'] == {'Bash': 1} and stats()['state'] == 'thinking'
"#,
        );
    }

    #[test]
    fn a_background_agent_never_restarts_a_finished_turn() {
        run_hook_script(
            r#"
run({'hook_event_name': 'UserPromptSubmit', 'prompt': 'go'})
run({'hook_event_name': 'Stop'})
assert stats()['state'] == 'complete'
run({'hook_event_name': 'PostToolUse', 'tool_name': 'Read', 'tool_use_id': 't1', 'agent_id': 'bg'})
assert stats()['state'] == 'complete', 'a subagent tool call leaves the turn finished'
assert stats()['tools'] == {'Read': 1}
# A background agent asks, the person answers: back to complete, not thinking.
run({'hook_event_name': 'PermissionRequest', 'tool_name': 'Bash', 'tool_input': {}, 'agent_id': 'bg'})
assert stats()['state'] == 'permission'
run({'hook_event_name': 'PostToolUse', 'tool_name': 'Read', 'tool_use_id': 't2', 'agent_id': 'other'})
assert stats()['state'] == 'permission', "another agent's call doesn't answer it"
run({'hook_event_name': 'PostToolUse', 'tool_name': 'Bash', 'tool_use_id': 't3', 'agent_id': 'bg'})
assert stats()['state'] == 'complete' and stats()['active_prompt'] is None
assert 'permission' not in stats()
# During a turn, a subagent's answered permission goes back to thinking.
run({'hook_event_name': 'UserPromptSubmit', 'prompt': 'again'})
run({'hook_event_name': 'PermissionRequest', 'tool_name': 'Bash', 'tool_input': {}, 'agent_id': 'fg'})
run({'hook_event_name': 'PostToolUse', 'tool_name': 'Bash', 'tool_use_id': 't4', 'agent_id': 'fg'})
assert stats()['state'] == 'thinking'
"#,
        );
    }

    #[test]
    fn an_api_error_ends_the_turn_and_a_model_switch_is_recorded() {
        run_hook_script(
            r#"
run({'hook_event_name': 'SessionStart', 'source': 'startup', 'model': 'claude-opus-5-5[1m]'})
assert stats()['model'] == 'claude-opus-5-5'
run({'hook_event_name': 'UserPromptSubmit', 'prompt': 'go'})
run({'hook_event_name': 'StopFailure', 'error': 'overloaded'})
assert stats()['state'] == 'complete' and stats()['idle_since'] is not None
assert stats()['completions'] == 0
run({'hook_event_name': 'PostModelSwitch', 'to_model': 'claude-sonnet-5-5'})
assert stats()['model'] == 'claude-sonnet-5-5'
"#,
        );
    }

    #[test]
    fn registers_every_event_and_upgrades_an_older_install() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = ClaudeCodePlatform {
            claude_dir: tmp.path().to_path_buf(),
            crabigator_dir: tmp.path().join("crabigator"),
        };
        let script = platform.script_path().to_string_lossy().to_string();
        // An older install: two events, MessageDisplay not async, beside the user's own hook.
        let older = json!({"hooks": {
            "Stop": [{"hooks": [{"type": "command", "command": script}]}],
            "MessageDisplay": [{"hooks": [{"type": "command", "command": script}]}],
            "PreToolUse": [{"matcher": "Bash", "hooks": [{"type": "command", "command": "mine.sh"}]}],
        }});
        fs::write(platform.settings_path(), older.to_string()).unwrap();
        assert!(!platform.hooks_registered().unwrap());
        platform.merge_settings().unwrap();
        assert!(platform.hooks_registered().unwrap());
        let settings: Value =
            serde_json::from_str(&fs::read_to_string(platform.settings_path()).unwrap()).unwrap();
        let hooks = settings["hooks"].as_object().unwrap();
        assert_eq!(hooks.len(), HOOK_EVENTS.len());
        for never in ["WorktreeCreate", "WorktreeRemove", "FileChanged"] {
            assert!(!hooks.contains_key(never), "{never} must not be registered");
        }
        assert_eq!(
            hooks["MessageDisplay"],
            json!([{"hooks": [{"type": "command", "command": script, "async": true}]}])
        );
        assert_eq!(
            hooks["PreToolUse"],
            json!([
                {"matcher": "Bash", "hooks": [{"type": "command", "command": "mine.sh"}]},
                {"matcher": "*", "hooks": [{"type": "command", "command": script}]}
            ])
        );
    }

    #[test]
    fn reads_claude_session_id_as_native_session_id() {
        let stats: crate::platforms::PlatformStats = serde_json::from_str(
            r#"{"prompts":1,"completions":0,"subagent_messages":0,"compressions":0,"claude_session_id":"abc-uuid","state":"thinking"}"#,
        )
        .unwrap();
        assert_eq!(stats.native_session_id.as_deref(), Some("abc-uuid"));
        assert_eq!(stats.state, crate::platforms::SessionState::Thinking);
    }
}
