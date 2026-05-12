//! Automatic per-turn recaps.
//!
//! Recaps are generated on the desktop from local assistant transcripts after
//! each completed turn. The cloud/dashboard only receives the finished recap,
//! not the raw transcript used to produce it.

use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

use crate::cli::RecapCommand;
use crate::config::Config;
use crate::git::GitState;
use crate::platforms::{PlatformKind, PlatformStats, SessionState};

pub const DEFAULT_RECAP_MODEL: &str = "claude-haiku-4-5";
const ANTHROPIC_MESSAGES_URL: &str = "https://api.anthropic.com/v1/messages";
const ANTHROPIC_VERSION: &str = "2023-06-01";
const MAX_TRANSCRIPT_CHARS: usize = 28_000;
const MAX_TOOL_RESULT_CHARS: usize = 1_200;
/// How long the "Recaps enabled" startup toast remains visible.
pub const ENABLED_TOAST_DURATION: Duration = Duration::from_secs(10);

/// Current recap status shown in the terminal handoff strip.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum RecapStatus {
    Disabled,
    MissingKey,
    Waiting,
    Updating,
    Ready,
    Failed(String),
}

/// Turn-scoped line delta shown on the right side of the recap strip.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TurnLineDelta {
    pub additions: i64,
    pub deletions: i64,
}

impl TurnLineDelta {
    fn between(before: &GitLineSnapshot, after: &GitLineSnapshot) -> Self {
        Self {
            additions: after.additions as i64 - before.additions as i64,
            deletions: after.deletions as i64 - before.deletions as i64,
        }
    }
}

/// Display-ready generated recap.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
pub struct TurnRecap {
    pub prompt_count: u32,
    pub generated_at: u64,
    pub variant: RecapVariant,
    pub headline: String,
    pub bullets: Vec<String>,
    pub next_prompt_notes: Vec<String>,
    pub artifacts: Vec<String>,
    pub line_delta: TurnLineDelta,
}

/// Compact or bullet recap rendering mode.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, Hash)]
#[serde(rename_all = "lowercase")]
pub enum RecapVariant {
    Brief,
    Bullets,
}

/// State consumed by the terminal renderer.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecapState {
    pub enabled: bool,
    pub status: RecapStatus,
    pub latest: Option<TurnRecap>,
    pub line_delta: Option<TurnLineDelta>,
    pub model: String,
}

impl RecapState {
    /// Whether the handoff strip is hosting a real recap (rather than the
    /// toast or hint paths). Kept as a small helper for tests and future
    /// callers that want to distinguish "actual recap content" from the
    /// transient enable/disable messaging.
    #[allow(dead_code)]
    pub fn prefers_handoff(&self) -> bool {
        self.latest.is_some() || matches!(self.status, RecapStatus::Failed(_))
    }
}

impl Default for RecapState {
    fn default() -> Self {
        Self {
            enabled: false,
            status: RecapStatus::Disabled,
            latest: None,
            line_delta: None,
            model: DEFAULT_RECAP_MODEL.to_string(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
struct GitLineSnapshot {
    additions: usize,
    deletions: usize,
}

impl GitLineSnapshot {
    fn from_git_state(git: &GitState) -> Self {
        Self {
            additions: git.files.iter().map(|f| f.additions).sum(),
            deletions: git.files.iter().map(|f| f.deletions).sum(),
        }
    }
}

#[derive(Clone, Debug)]
struct TurnBaseline {
    prompt_count: u32,
    git_lines: GitLineSnapshot,
}

#[derive(Clone)]
struct RecapJob {
    platform: PlatformKind,
    transcript_path: Option<PathBuf>,
    cwd: PathBuf,
    prompt_count: u32,
    model: String,
    api_key: String,
    line_delta: TurnLineDelta,
}

/// Owns turn detection, background recap generation, and the display cache.
pub struct RecapManager {
    state: RecapState,
    api_key: Option<String>,
    last_prompt_count: u32,
    last_completion_count: u32,
    initialized: bool,
    active_turn: Option<TurnBaseline>,
    pending: Option<mpsc::Receiver<std::result::Result<TurnRecap, String>>>,
    /// Wall-clock anchor used to fade the startup confirmation toast.
    started_at: Instant,
    /// All successful recaps generated in this session, oldest first. Mirrors
    /// the title_history pattern so the dashboard can replay the full timeline.
    history: Vec<TurnRecap>,
}

impl RecapManager {
    pub fn load() -> Self {
        let config = Config::load().unwrap_or_default();
        let api_key = Config::read_recap_api_key()
            .ok()
            .flatten()
            .or_else(read_anthropic_api_key_env);
        let model = config
            .recap_model
            .unwrap_or_else(|| DEFAULT_RECAP_MODEL.to_string());
        let status = if config.recap_enabled {
            if api_key.is_some() {
                RecapStatus::Waiting
            } else {
                RecapStatus::MissingKey
            }
        } else {
            RecapStatus::Disabled
        };

        Self {
            state: RecapState {
                enabled: config.recap_enabled,
                status,
                latest: None,
                line_delta: None,
                model,
            },
            api_key,
            last_prompt_count: 0,
            last_completion_count: 0,
            initialized: false,
            active_turn: None,
            pending: None,
            started_at: Instant::now(),
            history: Vec::new(),
        }
    }

    pub fn state(&self) -> &RecapState {
        &self.state
    }

    /// All successful recaps generated this session, oldest first.
    pub fn history(&self) -> &[TurnRecap] {
        &self.history
    }

    /// Whether the transient "Recaps enabled" toast should be shown.
    ///
    /// True only while recaps are armed with a usable key (Waiting / Updating /
    /// Ready) and within the first `ENABLED_TOAST_DURATION` of the session.
    pub fn enabled_toast_visible(&self) -> bool {
        if self.started_at.elapsed() >= ENABLED_TOAST_DURATION {
            return false;
        }
        matches!(
            self.state.status,
            RecapStatus::Waiting | RecapStatus::Updating | RecapStatus::Ready
        )
    }

    pub fn state_hash(&self) -> u64 {
        use std::hash::{Hash, Hasher};

        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        self.state.enabled.hash(&mut hasher);
        format!("{:?}", self.state.status).hash(&mut hasher);
        self.state.model.hash(&mut hasher);
        self.state.line_delta.hash(&mut hasher);
        self.state.latest.hash(&mut hasher);
        // Time-sensitive: included so the redraw tick clears the toast at
        // the 10-second mark even when no other state has changed.
        self.enabled_toast_visible().hash(&mut hasher);
        hasher.finish()
    }

    /// Optimistically clear the recap card when we know a new prompt was just
    /// submitted, without waiting for the platform's prompts counter to tick.
    ///
    /// Called from the keyboard input path when Enter fires in Ready/Complete
    /// state, before Codex/Claude has flushed its log line. Mirrors the
    /// `stats.prompts > last_prompt_count` branch in `handle_platform_update`,
    /// so when the real platform update lands it's a no-op.
    ///
    /// Returns true if the visible recap state changed.
    pub fn note_user_submitted_prompt(&mut self) -> bool {
        // Nothing to clear if the card was already empty or in an updating /
        // disabled state.
        if self.state.latest.is_none() && !matches!(self.state.status, RecapStatus::Ready) {
            return false;
        }
        self.state.latest = None;
        self.state.line_delta = Some(TurnLineDelta::default());
        self.pending = None;
        self.active_turn = None;
        self.state.status = if self.state.enabled {
            if self.api_key.is_some() {
                RecapStatus::Updating
            } else {
                RecapStatus::MissingKey
            }
        } else {
            RecapStatus::Disabled
        };
        true
    }

    /// Update turn tracking from the latest platform stats.
    ///
    /// Returns true when renderable recap state changed.
    pub fn handle_platform_update(
        &mut self,
        platform: PlatformKind,
        stats: &PlatformStats,
        effective_state: SessionState,
        git_state: &GitState,
        cwd: &Path,
    ) -> bool {
        let mut changed = self.poll_pending(cwd);

        if !self.initialized {
            self.initialized = true;
            self.last_prompt_count = stats.prompts;
            self.last_completion_count = stats.completions;
            if effective_state == SessionState::Thinking && stats.prompts > 0 {
                self.active_turn = Some(TurnBaseline {
                    prompt_count: stats.prompts,
                    git_lines: GitLineSnapshot::from_git_state(git_state),
                });
            }
            return changed;
        }

        if stats.prompts > self.last_prompt_count {
            self.last_prompt_count = stats.prompts;
            self.active_turn = Some(TurnBaseline {
                prompt_count: stats.prompts,
                git_lines: GitLineSnapshot::from_git_state(git_state),
            });
            self.state.line_delta = Some(TurnLineDelta::default());
            self.state.latest = None;
            self.pending = None;
            self.state.status = if self.state.enabled {
                RecapStatus::Updating
            } else {
                RecapStatus::Disabled
            };
            changed = true;
        }

        if let Some(turn) = &self.active_turn {
            let line_delta = TurnLineDelta::between(
                &turn.git_lines,
                &GitLineSnapshot::from_git_state(git_state),
            );
            if self.state.line_delta != Some(line_delta) {
                self.state.line_delta = Some(line_delta);
                changed = true;
            }
        }

        let completed_turn = stats.completions > self.last_completion_count
            && effective_state == SessionState::Complete;
        if completed_turn {
            self.last_completion_count = stats.completions;
            changed |= self.start_recap_if_ready(platform, stats, git_state, cwd);
        }

        changed
    }

    fn start_recap_if_ready(
        &mut self,
        platform: PlatformKind,
        stats: &PlatformStats,
        git_state: &GitState,
        cwd: &Path,
    ) -> bool {
        if !self.state.enabled {
            self.active_turn = None;
            return false;
        }

        let Some(api_key) = self.api_key.clone() else {
            self.state.status = RecapStatus::MissingKey;
            self.active_turn = None;
            return true;
        };

        if self.pending.is_some() {
            return false;
        }

        let Some(turn) = self.active_turn.take() else {
            return false;
        };

        let line_delta =
            TurnLineDelta::between(&turn.git_lines, &GitLineSnapshot::from_git_state(git_state));
        self.state.line_delta = Some(line_delta);
        self.state.status = RecapStatus::Updating;

        let job = RecapJob {
            platform,
            transcript_path: stats.transcript_path.as_ref().map(PathBuf::from),
            cwd: cwd.to_path_buf(),
            prompt_count: turn.prompt_count,
            model: self.state.model.clone(),
            api_key,
            line_delta,
        };

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let result = generate_turn_recap(job).map_err(|e| e.to_string());
            let _ = tx.send(result);
        });
        self.pending = Some(rx);
        true
    }

    fn poll_pending(&mut self, cwd: &Path) -> bool {
        let Some(rx) = &self.pending else {
            return false;
        };

        match rx.try_recv() {
            Ok(Ok(recap)) => {
                let _ = write_recap_cache(cwd, &recap);
                self.state.line_delta = Some(recap.line_delta);
                self.history.push(recap.clone());
                self.state.latest = Some(recap);
                self.state.status = RecapStatus::Ready;
                self.pending = None;
                true
            }
            Ok(Err(error)) => {
                self.state.status = RecapStatus::Failed(error);
                self.pending = None;
                true
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.state.status = RecapStatus::Failed("recap worker stopped".to_string());
                self.pending = None;
                true
            }
        }
    }
}

pub fn run_recap_command(command: RecapCommand) -> Result<()> {
    match command {
        RecapCommand::Enable { api_key, model } => enable_recap(api_key, model),
        RecapCommand::Disable => disable_recap(),
        RecapCommand::Status => print_recap_status(),
    }
}

/// Implements the top-level `crabigator key` shortcut: saves an Anthropic API
/// key (from arg, env var, or prompt) and ensures recaps are enabled.
pub fn run_key_command(api_key: Option<String>) -> Result<()> {
    enable_recap(api_key, None)
}

fn enable_recap(api_key: Option<String>, model: Option<String>) -> Result<()> {
    let key = match api_key {
        Some(key) => key,
        None => std::env::var("ANTHROPIC_API_KEY").unwrap_or_default(),
    };
    let key = if key.trim().is_empty() {
        prompt_for_api_key()?
    } else {
        key
    };

    if key.trim().is_empty() {
        anyhow::bail!("No API key provided");
    }

    Config::write_recap_api_key(&key)?;
    let mut config = Config::load().unwrap_or_default();
    config.recap_enabled = true;
    if let Some(model) = model {
        config.recap_model = Some(model);
    } else if config.recap_model.is_none() {
        config.recap_model = Some(DEFAULT_RECAP_MODEL.to_string());
    }
    config.save()?;

    println!(
        "Recaps enabled with {}.",
        config.recap_model.as_deref().unwrap_or(DEFAULT_RECAP_MODEL)
    );
    Ok(())
}

fn disable_recap() -> Result<()> {
    let mut config = Config::load().unwrap_or_default();
    config.recap_enabled = false;
    config.save()?;
    Config::remove_recap_api_key()?;
    println!("Recaps disabled and stored API key removed.");
    Ok(())
}

fn print_recap_status() -> Result<()> {
    let config = Config::load().unwrap_or_default();
    let stored_key = Config::read_recap_api_key()?.is_some();
    let env_key = read_anthropic_api_key_env().is_some();
    println!("enabled: {}", config.recap_enabled);
    println!(
        "model: {}",
        config.recap_model.as_deref().unwrap_or(DEFAULT_RECAP_MODEL)
    );
    let key_source = if stored_key {
        "stored"
    } else if env_key {
        "env (ANTHROPIC_API_KEY)"
    } else {
        "missing"
    };
    println!("api_key: {}", key_source);
    Ok(())
}

fn read_anthropic_api_key_env() -> Option<String> {
    std::env::var("ANTHROPIC_API_KEY")
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
}

fn prompt_for_api_key() -> Result<String> {
    print!("Anthropic API key: ");
    io::stdout().flush()?;
    let mut key = String::new();
    io::stdin().read_line(&mut key)?;
    Ok(key.trim().to_string())
}

fn generate_turn_recap(job: RecapJob) -> Result<TurnRecap> {
    let transcript = collect_latest_turn(&job)?;
    if transcript.activity.trim().is_empty() {
        return Ok(fallback_recap(
            &job,
            "The agent completed the turn without captured transcript output.",
        ));
    }

    let request = build_anthropic_request(&job, &transcript);
    let response = call_anthropic(&job.api_key, request)?;
    parse_recap_response(&job, &response).or_else(|_| {
        Ok(fallback_recap(
            &job,
            "The agent completed the turn. The recap response could not be parsed, so check the live output for details.",
        ))
    })
}

#[derive(Debug)]
struct TurnTranscript {
    user_prompt: Option<String>,
    activity: String,
}

fn collect_latest_turn(job: &RecapJob) -> Result<TurnTranscript> {
    let path = job
        .transcript_path
        .clone()
        .or_else(|| latest_codex_session_for_cwd(&job.cwd).ok().flatten());
    let Some(path) = path else {
        return Ok(TurnTranscript {
            user_prompt: None,
            activity: String::new(),
        });
    };

    let content = fs::read_to_string(&path)
        .with_context(|| format!("Failed to read transcript {}", path.display()))?;
    let mut transcript = match job.platform {
        PlatformKind::Claude => collect_claude_latest_turn(&content),
        PlatformKind::Codex => collect_codex_latest_turn(&content),
    };
    transcript.activity =
        redact_sensitive(&truncate_start(&transcript.activity, MAX_TRANSCRIPT_CHARS));
    transcript.user_prompt = transcript
        .user_prompt
        .map(|prompt| redact_sensitive(&truncate_start(&prompt, 2_000)));
    Ok(transcript)
}

fn collect_claude_latest_turn(content: &str) -> TurnTranscript {
    let mut user_prompt = None;
    let mut activity = String::new();
    let mut after_user_prompt = false;

    for line in content.lines() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        match entry.get("type").and_then(|v| v.as_str()) {
            Some("user") => {
                let Some(message_content) = entry.get("message").and_then(|m| m.get("content"))
                else {
                    continue;
                };
                if let Some(prompt) = message_content.as_str() {
                    user_prompt = Some(prompt.trim().to_string());
                    activity.clear();
                    after_user_prompt = true;
                } else if after_user_prompt {
                    append_tool_results(&mut activity, message_content);
                }
            }
            Some("assistant") if after_user_prompt => {
                if let Some(message_content) = entry.get("message").and_then(|m| m.get("content")) {
                    append_assistant_content(&mut activity, message_content);
                }
            }
            _ => {}
        }
    }

    TurnTranscript {
        user_prompt,
        activity,
    }
}

fn collect_codex_latest_turn(content: &str) -> TurnTranscript {
    let mut user_prompt = None;
    let mut activity = String::new();
    let mut after_user_prompt = false;

    for line in content.lines() {
        let Ok(entry) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let entry_type = entry.get("type").and_then(|v| v.as_str());
        let payload = entry.get("payload").unwrap_or(&Value::Null);

        match entry_type {
            Some("response_item") => match payload.get("type").and_then(|v| v.as_str()) {
                Some("message") if payload.get("role").and_then(|v| v.as_str()) == Some("user") => {
                    let prompt = codex_message_text(payload);
                    if !prompt.trim().is_empty() && !is_codex_bootstrap(&prompt) {
                        user_prompt = Some(prompt.trim().to_string());
                        activity.clear();
                        after_user_prompt = true;
                    }
                }
                Some("message")
                    if after_user_prompt
                        && payload.get("role").and_then(|v| v.as_str()) == Some("assistant") =>
                {
                    let text = codex_message_text(payload);
                    if !text.trim().is_empty() {
                        push_section(&mut activity, "assistant", text.trim());
                    }
                }
                Some("function_call") if after_user_prompt => {
                    let name = payload
                        .get("name")
                        .and_then(|v| v.as_str())
                        .unwrap_or("tool");
                    let args = payload
                        .get("arguments")
                        .map(format_json_preview)
                        .unwrap_or_default();
                    push_section(&mut activity, "tool", &format!("{} {}", name, args));
                }
                Some("function_call_output") if after_user_prompt => {
                    let output = payload
                        .get("output")
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if !output.trim().is_empty() {
                        push_section(
                            &mut activity,
                            "tool_result",
                            &truncate_end(output.trim(), MAX_TOOL_RESULT_CHARS),
                        );
                    }
                }
                _ => {}
            },
            Some("event_msg") => match payload.get("type").and_then(|v| v.as_str()) {
                Some("user_message") => {
                    let prompt = payload
                        .get("message")
                        .or_else(|| payload.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if !prompt.trim().is_empty() && !is_codex_bootstrap(prompt) {
                        user_prompt = Some(prompt.trim().to_string());
                        activity.clear();
                        after_user_prompt = true;
                    }
                }
                Some("agent_message") if after_user_prompt => {
                    let text = payload
                        .get("message")
                        .or_else(|| payload.get("text"))
                        .and_then(|v| v.as_str())
                        .unwrap_or_default();
                    if !text.trim().is_empty() {
                        push_section(&mut activity, "assistant", text.trim());
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    TurnTranscript {
        user_prompt,
        activity,
    }
}

fn append_assistant_content(activity: &mut String, content: &Value) {
    match content {
        Value::String(text) => push_section(activity, "assistant", text.trim()),
        Value::Array(blocks) => {
            for block in blocks {
                match block.get("type").and_then(|v| v.as_str()) {
                    Some("text") => {
                        if let Some(text) = block.get("text").and_then(|v| v.as_str()) {
                            push_section(activity, "assistant", text.trim());
                        }
                    }
                    Some("tool_use") => {
                        let name = block.get("name").and_then(|v| v.as_str()).unwrap_or("tool");
                        let input = block
                            .get("input")
                            .map(format_json_preview)
                            .unwrap_or_default();
                        push_section(activity, "tool", &format!("{} {}", name, input));
                    }
                    _ => {}
                }
            }
        }
        _ => {}
    }
}

fn append_tool_results(activity: &mut String, content: &Value) {
    let Value::Array(results) = content else {
        return;
    };
    for result in results {
        let Some(result_content) = result.get("content") else {
            continue;
        };
        let text = match result_content {
            Value::String(text) => text.clone(),
            Value::Array(parts) => parts
                .iter()
                .filter_map(|part| part.get("text").and_then(|v| v.as_str()))
                .collect::<Vec<_>>()
                .join("\n"),
            other => format_json_preview(other),
        };
        if !text.trim().is_empty() {
            push_section(
                activity,
                "tool_result",
                &truncate_end(text.trim(), MAX_TOOL_RESULT_CHARS),
            );
        }
    }
}

fn codex_message_text(payload: &Value) -> String {
    let Some(content) = payload.get("content").and_then(|v| v.as_array()) else {
        return String::new();
    };
    content
        .iter()
        .filter_map(|entry| entry.get("text").and_then(|v| v.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

fn is_codex_bootstrap(text: &str) -> bool {
    text.contains("<INSTRUCTIONS>")
        || text.contains("<environment_context>")
        || text.contains("# AGENTS.md instructions")
}

fn push_section(out: &mut String, label: &str, text: &str) {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return;
    }
    out.push_str("\n[");
    out.push_str(label);
    out.push_str("]\n");
    out.push_str(trimmed);
    out.push('\n');
}

fn format_json_preview(value: &Value) -> String {
    match value {
        Value::String(text) => truncate_end(text, 600),
        other => serde_json::to_string(other)
            .map(|s| truncate_end(&s, 900))
            .unwrap_or_default(),
    }
}

fn latest_codex_session_for_cwd(cwd: &Path) -> Result<Option<PathBuf>> {
    let Some(home) = dirs::home_dir() else {
        return Ok(None);
    };
    let sessions_dir = home.join(".codex").join("sessions");
    if !sessions_dir.exists() {
        return Ok(None);
    }

    let mut best: Option<(PathBuf, SystemTime)> = None;
    collect_jsonl_files(&sessions_dir, &mut |path| {
        if codex_session_matches_cwd(path, cwd).unwrap_or(false) {
            if let Ok(metadata) = fs::metadata(path) {
                if let Ok(modified) = metadata.modified() {
                    let replace = best
                        .as_ref()
                        .map(|(_, best_modified)| modified > *best_modified)
                        .unwrap_or(true);
                    if replace {
                        best = Some((path.to_path_buf(), modified));
                    }
                }
            }
        }
    })?;
    Ok(best.map(|(path, _)| path))
}

fn collect_jsonl_files(dir: &Path, visit: &mut dyn FnMut(&Path)) -> Result<()> {
    for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_jsonl_files(&path, visit)?;
        } else if path.extension().and_then(|ext| ext.to_str()) == Some("jsonl") {
            visit(&path);
        }
    }
    Ok(())
}

fn codex_session_matches_cwd(path: &Path, cwd: &Path) -> Result<bool> {
    let content = fs::read_to_string(path)?;
    let cwd = cwd.to_string_lossy();
    for line in content.lines().take(8) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        if !matches!(
            value.get("type").and_then(|v| v.as_str()),
            Some("session_meta" | "turn_context")
        ) {
            continue;
        }
        if value
            .get("payload")
            .and_then(|p| p.get("cwd"))
            .and_then(|v| v.as_str())
            .is_some_and(|entry_cwd| entry_cwd == cwd)
        {
            return Ok(true);
        }
    }
    Ok(false)
}

fn build_anthropic_request(job: &RecapJob, transcript: &TurnTranscript) -> Value {
    let size_hint = transcript.activity.chars().count();
    let preferred_variant = if size_hint < 3_500 {
        "brief"
    } else {
        "bullets"
    };
    let user_prompt = transcript
        .user_prompt
        .as_deref()
        .unwrap_or("(last user prompt unavailable)");

    let prompt = format!(
        "Write a Crabigator handoff recap for agent output since the last user-initiated prompt.\n\
         Preferred variant: {preferred_variant}\n\
         Last user prompt, for context only:\n{user_prompt}\n\n\
         Agent activity:\n{}\n\n\
         Return JSON only with this exact shape:\n\
         {{\"variant\":\"brief|bullets\",\"headline\":\"\",\
         \"bullets\":[],\"next_prompt_notes\":[],\"artifacts\":[]}}\n\
         Hard length budget — terse > complete sentences:\n\
         - headline: ≤72 chars, no trailing period.\n\
         - bullets: 0-2 items, ≤80 chars each. brief: omit bullets.\n\
         - next_prompt_notes: 0-1 items, ≤80 chars.\n\
         - artifacts: 0-2 items, ≤50 chars each.\n\
         Rules:\n\
         - Be terse. Cut filler words. Don't repeat the headline in bullets.\n\
         - Skip code-level details (file names, line numbers, diffs); the Git and Changes widgets cover those.\n\
         - Artifacts = non-code outputs to review (screenshots, logs, URLs, PRs, build reports).\n\
         - If nothing important happened, headline says so plainly and bullets/notes/artifacts are empty.",
        transcript.activity
    );

    json!({
        "model": job.model,
        "max_tokens": 350,
        "temperature": 0.2,
        "messages": [
            {"role": "user", "content": prompt}
        ]
    })
}

fn call_anthropic(api_key: &str, body: Value) -> Result<String> {
    let api_url = std::env::var("CRABIGATOR_RECAP_API_URL")
        .unwrap_or_else(|_| ANTHROPIC_MESSAGES_URL.to_string());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .context("Failed to create recap runtime")?;
    runtime.block_on(async {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(45))
            .build()?;
        let response = client
            .post(api_url)
            .header("x-api-key", api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .json(&body)
            .send()
            .await
            .context("Failed to call Anthropic Messages API")?;

        let status = response.status();
        let text = response.text().await.unwrap_or_default();
        if !status.is_success() {
            anyhow::bail!(
                "Anthropic returned {}: {}",
                status,
                truncate_end(&text, 220)
            );
        }
        extract_anthropic_text(&text)
    })
}

fn extract_anthropic_text(response: &str) -> Result<String> {
    let value: Value = serde_json::from_str(response).context("Anthropic response was not JSON")?;
    let text = value
        .get("content")
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|block| {
            if block.get("type").and_then(|v| v.as_str()) == Some("text") {
                block.get("text").and_then(|v| v.as_str())
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    if text.trim().is_empty() {
        anyhow::bail!("Anthropic response did not contain text");
    }
    Ok(text)
}

fn parse_recap_response(job: &RecapJob, response: &str) -> Result<TurnRecap> {
    let json_text = extract_json_object(response).unwrap_or(response);
    let value: Value = serde_json::from_str(json_text).context("Recap response was not JSON")?;
    let variant = match value.get("variant").and_then(|v| v.as_str()) {
        Some("bullets") => RecapVariant::Bullets,
        _ => RecapVariant::Brief,
    };
    let headline = value
        .get("headline")
        .and_then(|v| v.as_str())
        .map(clean_one_line)
        .filter(|s| !s.is_empty())
        .unwrap_or_else(|| "The agent completed the latest turn.".to_string());
    let bullets = string_array(&value, "bullets", 3);
    let next_prompt_notes = string_array(&value, "next_prompt_notes", 2);
    let artifacts = string_array(&value, "artifacts", 3);

    Ok(TurnRecap {
        prompt_count: job.prompt_count,
        generated_at: now_unix_ms(),
        variant,
        headline,
        bullets,
        next_prompt_notes,
        artifacts,
        line_delta: job.line_delta,
    })
}

fn fallback_recap(job: &RecapJob, headline: &str) -> TurnRecap {
    TurnRecap {
        prompt_count: job.prompt_count,
        generated_at: now_unix_ms(),
        variant: RecapVariant::Brief,
        headline: headline.to_string(),
        bullets: Vec::new(),
        next_prompt_notes: Vec::new(),
        artifacts: Vec::new(),
        line_delta: job.line_delta,
    }
}

fn string_array(value: &Value, key: &str, limit: usize) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter_map(|v| v.as_str())
        .map(clean_one_line)
        .filter(|s| !s.is_empty())
        .take(limit)
        .collect()
}

fn clean_one_line(text: &str) -> String {
    text.split_whitespace().collect::<Vec<_>>().join(" ")
}

fn extract_json_object(text: &str) -> Option<&str> {
    let start = text.find('{')?;
    let end = text.rfind('}')?;
    (end >= start).then_some(&text[start..=end])
}

fn redact_sensitive(text: &str) -> String {
    let mut redacted = text.to_string();
    let patterns = [
        r"sk-ant-[A-Za-z0-9_\-]{12,}",
        r"sk-[A-Za-z0-9_\-]{20,}",
        r"github_pat_[A-Za-z0-9_]{20,}",
        r"gh[pousr]_[A-Za-z0-9_]{20,}",
        r"AKIA[0-9A-Z]{16}",
        r#"(?i)(api[_-]?key|token|secret|password)(['"\s:=]+)[A-Za-z0-9_\-./+=]{12,}"#,
    ];
    for pattern in patterns {
        if let Ok(regex) = Regex::new(pattern) {
            redacted = regex.replace_all(&redacted, "$1$2[REDACTED]").to_string();
        }
    }
    redacted
}

fn truncate_start(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        return text.to_string();
    }
    let keep: String = text.chars().skip(count - max_chars).collect();
    format!(
        "... omitted {} earlier chars ...\n{}",
        count - max_chars,
        keep
    )
}

fn truncate_end(text: &str, max_chars: usize) -> String {
    let count = text.chars().count();
    if count <= max_chars {
        text.to_string()
    } else {
        let kept: String = text.chars().take(max_chars).collect();
        format!("{}...", kept)
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64
}

fn write_recap_cache(cwd: &Path, recap: &TurnRecap) -> Result<()> {
    let session_id = std::env::var("CRABIGATOR_SESSION_ID").unwrap_or_default();
    if session_id.is_empty() {
        return Ok(());
    }
    let session_dir = PathBuf::from(format!("/tmp/crabigator-{}", session_id));
    fs::create_dir_all(&session_dir)?;
    let path = session_dir.join("recap.json");
    let tmp_path = path.with_extension("tmp");
    let json = serde_json::to_string_pretty(&json!({
        "cwd": cwd.to_string_lossy(),
        "recap": recap,
    }))?;
    fs::write(&tmp_path, json)?;
    fs::rename(&tmp_path, path)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_recap_json_with_wrapping_text() {
        let job = RecapJob {
            platform: PlatformKind::Claude,
            transcript_path: None,
            cwd: PathBuf::from("/tmp"),
            prompt_count: 2,
            model: DEFAULT_RECAP_MODEL.to_string(),
            api_key: "key".to_string(),
            line_delta: TurnLineDelta {
                additions: 12,
                deletions: 3,
            },
        };

        let recap = parse_recap_response(
            &job,
            "```json\n{\"variant\":\"bullets\",\"headline\":\"Changed the flow.\",\"bullets\":[\"One\"],\"next_prompt_notes\":[\"Check behavior\"],\"artifacts\":[\"test log\"]}\n```",
        )
        .unwrap();

        assert_eq!(recap.variant, RecapVariant::Bullets);
        assert_eq!(recap.headline, "Changed the flow.");
        assert_eq!(recap.bullets, vec!["One"]);
        assert_eq!(recap.next_prompt_notes, vec!["Check behavior"]);
        assert_eq!(recap.artifacts, vec!["test log"]);
        assert_eq!(recap.line_delta.additions, 12);
    }

    #[test]
    fn claude_collector_resets_at_latest_user_prompt() {
        let transcript = r#"{"type":"user","message":{"content":"old prompt"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"old answer"}]}}
{"type":"user","message":{"content":"new prompt"}}
{"type":"assistant","message":{"content":[{"type":"text","text":"new answer"},{"type":"tool_use","name":"Bash","input":{"command":"cargo test"}}]}}"#;

        let turn = collect_claude_latest_turn(transcript);
        assert_eq!(turn.user_prompt.as_deref(), Some("new prompt"));
        assert!(turn.activity.contains("new answer"));
        assert!(turn.activity.contains("cargo test"));
        assert!(!turn.activity.contains("old answer"));
    }

    #[test]
    fn redacts_common_secrets() {
        let text =
            "api_key = sk-ant-1234567890abcdefghijklmnop and token: ghp_1234567890abcdefghijklmnop";
        let redacted = redact_sensitive(text);
        assert!(redacted.contains("[REDACTED]"));
        assert!(!redacted.contains("sk-ant-1234567890"));
        assert!(!redacted.contains("ghp_1234567890"));
    }

    #[test]
    fn hides_until_a_real_recap_exists() {
        let state = RecapState {
            enabled: true,
            status: RecapStatus::Waiting,
            latest: None,
            line_delta: None,
            model: DEFAULT_RECAP_MODEL.to_string(),
        };

        assert!(!state.prefers_handoff());
    }

    #[test]
    fn failed_status_takes_over_handoff_until_next_turn() {
        // Failed status reserves the handoff strip even without a `latest`.
        let mut state = RecapState {
            enabled: true,
            status: RecapStatus::Failed("Anthropic returned 400".to_string()),
            latest: None,
            line_delta: None,
            model: DEFAULT_RECAP_MODEL.to_string(),
        };
        assert!(state.prefers_handoff());

        // After the next prompt resets status, the strip releases.
        state.status = RecapStatus::Updating;
        assert!(!state.prefers_handoff());
    }

    #[test]
    fn enabled_toast_visibility_obeys_status_and_clock() {
        // Fresh manager with a working key → toast visible.
        let mut manager = RecapManager {
            state: RecapState {
                enabled: true,
                status: RecapStatus::Waiting,
                latest: None,
                line_delta: None,
                model: DEFAULT_RECAP_MODEL.to_string(),
            },
            api_key: Some("test".to_string()),
            last_prompt_count: 0,
            last_completion_count: 0,
            initialized: false,
            active_turn: None,
            pending: None,
            started_at: Instant::now(),
            history: Vec::new(),
        };
        assert!(manager.enabled_toast_visible());

        // Disabled / MissingKey statuses suppress the toast.
        manager.state.status = RecapStatus::Disabled;
        assert!(!manager.enabled_toast_visible());
        manager.state.status = RecapStatus::MissingKey;
        assert!(!manager.enabled_toast_visible());

        // After the toast window elapses, the toast is gone.
        manager.state.status = RecapStatus::Ready;
        manager.started_at = Instant::now()
            .checked_sub(ENABLED_TOAST_DURATION + Duration::from_secs(1))
            .expect("clock arithmetic");
        assert!(!manager.enabled_toast_visible());
    }

    #[test]
    fn prompt_submission_dismisses_existing_recap() {
        let mut manager = RecapManager {
            state: RecapState {
                enabled: true,
                status: RecapStatus::Ready,
                latest: Some(TurnRecap {
                    prompt_count: 1,
                    generated_at: 1,
                    variant: RecapVariant::Brief,
                    headline: "Previous recap".to_string(),
                    bullets: Vec::new(),
                    next_prompt_notes: Vec::new(),
                    artifacts: Vec::new(),
                    line_delta: TurnLineDelta::default(),
                }),
                line_delta: Some(TurnLineDelta::default()),
                model: DEFAULT_RECAP_MODEL.to_string(),
            },
            api_key: Some("test".to_string()),
            last_prompt_count: 1,
            last_completion_count: 1,
            initialized: true,
            active_turn: None,
            pending: None,
            started_at: Instant::now(),
            history: Vec::new(),
        };
        let stats = PlatformStats {
            prompts: 2,
            completions: 1,
            ..PlatformStats::default()
        };

        let changed = manager.handle_platform_update(
            PlatformKind::Codex,
            &stats,
            SessionState::Thinking,
            &GitState::default(),
            Path::new("/tmp"),
        );

        assert!(changed);
        assert!(manager.state.latest.is_none());
        assert!(!manager.state.prefers_handoff());
        assert!(matches!(manager.state.status, RecapStatus::Updating));
    }
}
