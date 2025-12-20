//! Codex CLI platform implementation
//!
//! Reads Codex session logs under ~/.codex/sessions and derives session stats.

use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result};
use chrono::{Datelike, Local};
use serde_json::Value;

use super::{Platform, PlatformKind, PlatformStats, SessionState};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AssistantMessageMode {
    Unknown,
    ResponseItem,
    AgentMessage,
}

struct CodexState {
    session_path: Option<PathBuf>,
    session_offset: u64,
    last_scan: Option<SystemTime>,
    assistant_message_mode: AssistantMessageMode,
    stats: PlatformStats,
}

impl Default for CodexState {
    fn default() -> Self {
        Self {
            session_path: None,
            session_offset: 0,
            last_scan: None,
            assistant_message_mode: AssistantMessageMode::Unknown,
            stats: PlatformStats::default(),
        }
    }
}

pub struct CodexPlatform {
    sessions_dir: PathBuf,
    state: Mutex<CodexState>,
}

impl CodexPlatform {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        let sessions_dir = home.join(".codex").join("sessions");

        Self {
            sessions_dir,
            state: Mutex::new(CodexState::default()),
        }
    }

    fn session_path_override(&self) -> Option<PathBuf> {
        if let Ok(path) = std::env::var("CRABIGATOR_CODEX_SESSION_PATH") {
            return Some(PathBuf::from(path));
        }
        if let Ok(path) = std::env::var("CODEX_SESSION_PATH") {
            return Some(PathBuf::from(path));
        }
        None
    }

    fn should_rescan(state: &CodexState) -> bool {
        let Some(last_scan) = state.last_scan else {
            return true;
        };
        last_scan.elapsed().unwrap_or(Duration::from_secs(0)) >= Duration::from_secs(2)
    }

    fn resolve_session_path(&self, cwd: &str, state: &mut CodexState) -> Result<Option<PathBuf>> {
        if let Some(path) = self.session_path_override() {
            return Ok(Some(path));
        }

        if let Some(path) = &state.session_path {
            if path.exists() {
                return Ok(Some(path.clone()));
            }
        }

        if !Self::should_rescan(state) {
            return Ok(state.session_path.clone());
        }

        state.last_scan = Some(SystemTime::now());

        let mut candidates = Vec::new();
        let today = Local::now();
        for offset in 0..=1 {
            let date = today - chrono::Duration::days(offset);
            let dir = self.sessions_dir_for_date(date);
            if !dir.exists() {
                continue;
            }
            self.collect_candidates(&dir, cwd, &mut candidates)?;
        }

        candidates.sort_by_key(|(_, modified)| *modified);
        let path = candidates.last().map(|(path, _)| path.clone());
        Ok(path)
    }

    fn sessions_dir_for_date(&self, date: chrono::DateTime<Local>) -> PathBuf {
        self.sessions_dir
            .join(format!("{:04}", date.year()))
            .join(format!("{:02}", date.month()))
            .join(format!("{:02}", date.day()))
    }

    fn collect_candidates(
        &self,
        dir: &Path,
        cwd: &str,
        candidates: &mut Vec<(PathBuf, SystemTime)>,
    ) -> Result<()> {
        for entry in fs::read_dir(dir).with_context(|| format!("read {}", dir.display()))? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl") {
                continue;
            }
            let metadata = entry.metadata()?;
            let Some(modified) = metadata.modified().ok() else {
                continue;
            };
            if Self::session_meta_matches(&path, cwd)? {
                candidates.push((path, modified));
            }
        }
        Ok(())
    }

    fn session_meta_matches(path: &Path, cwd: &str) -> Result<bool> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        for _ in 0..5 {
            line.clear();
            if reader.read_line(&mut line)? == 0 {
                break;
            }
            let value: Value = match serde_json::from_str(&line) {
                Ok(value) => value,
                Err(_) => continue,
            };
            let entry_type = value.get("type").and_then(|v| v.as_str());
            if !matches!(entry_type, Some("session_meta") | Some("turn_context")) {
                continue;
            }
            let payload = value.get("payload").and_then(|v| v.as_object());
            let Some(payload) = payload else {
                continue;
            };
            if payload
                .get("cwd")
                .and_then(|v| v.as_str())
                .is_some_and(|entry_cwd| entry_cwd == cwd)
            {
                return Ok(true);
            }
        }
        Ok(false)
    }

    fn reset_state(state: &mut CodexState, path: PathBuf) {
        state.session_path = Some(path);
        state.session_offset = 0;
        state.assistant_message_mode = AssistantMessageMode::Unknown;
        state.stats = PlatformStats::default();
        Self::set_last_updated(state);
    }

    fn update_from_log(state: &mut CodexState, line: &str) {
        let value: Value = match serde_json::from_str(line) {
            Ok(value) => value,
            Err(_) => return,
        };
        let entry_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
        match entry_type {
            "response_item" => Self::handle_response_item(state, &value),
            "event_msg" => Self::handle_event_msg(state, &value),
            "session_meta" => {
                state.stats.state = SessionState::Ready;
            }
            _ => {}
        }
    }

    fn handle_response_item(state: &mut CodexState, value: &Value) {
        let payload = value.get("payload").and_then(|v| v.as_object());
        let Some(payload) = payload else {
            return;
        };
        match payload.get("type").and_then(|v| v.as_str()) {
            Some("message") => {
                let role = payload.get("role").and_then(|v| v.as_str());
                match role {
                    Some("assistant") => {
                        if Self::should_count_response_item_assistant(state) {
                            state.stats.completions = state.stats.completions.saturating_add(1);
                        }
                        state.stats.state = SessionState::Complete;
                    }
                    Some("user") => {
                        state.stats.prompts = state.stats.prompts.saturating_add(1);
                        state.stats.state = SessionState::Thinking;
                    }
                    _ => {}
                }
            }
            Some("function_call") => {
                if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                    let entry = state.stats.tools.entry(name.to_string()).or_insert(0);
                    *entry = entry.saturating_add(1);
                    if matches!(name, "AskUserQuestion" | "ask_user" | "request_user_input") {
                        state.stats.state = SessionState::Question;
                    }
                }
            }
            _ => {}
        }
    }

    fn handle_event_msg(state: &mut CodexState, value: &Value) {
        let payload = value.get("payload").and_then(|v| v.as_object());
        let Some(payload) = payload else {
            return;
        };
        match payload.get("type").and_then(|v| v.as_str()) {
            Some("user_message") => {
                state.stats.prompts = state.stats.prompts.saturating_add(1);
                state.stats.state = SessionState::Thinking;
            }
            Some("agent_message") => {
                if Self::should_count_agent_message_assistant(state) {
                    state.stats.completions = state.stats.completions.saturating_add(1);
                }
                state.stats.state = SessionState::Complete;
            }
            _ => {}
        }
    }

    fn should_count_response_item_assistant(state: &mut CodexState) -> bool {
        match state.assistant_message_mode {
            AssistantMessageMode::Unknown => {
                state.assistant_message_mode = AssistantMessageMode::ResponseItem;
                true
            }
            AssistantMessageMode::ResponseItem => true,
            AssistantMessageMode::AgentMessage => false,
        }
    }

    fn should_count_agent_message_assistant(state: &mut CodexState) -> bool {
        match state.assistant_message_mode {
            AssistantMessageMode::Unknown => {
                state.assistant_message_mode = AssistantMessageMode::AgentMessage;
                true
            }
            AssistantMessageMode::AgentMessage => true,
            AssistantMessageMode::ResponseItem => false,
        }
    }

    fn set_last_updated(state: &mut CodexState) {
        let timestamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        state.stats.last_updated = Some(timestamp);
    }
}

impl Default for CodexPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for CodexPlatform {
    fn kind(&self) -> PlatformKind {
        PlatformKind::Codex
    }

    fn command(&self) -> &'static str {
        PlatformKind::Codex.command()
    }

    fn ensure_hooks_installed(&self) -> Result<()> {
        // Codex CLI does not currently support Crabigator hooks.
        Ok(())
    }

    fn load_stats(&self, cwd: &str) -> Result<PlatformStats> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let session_path = match self.resolve_session_path(cwd, &mut state)? {
            Some(path) => path,
            None => return Ok(PlatformStats::default()),
        };

        let needs_reset = match state.session_path.as_ref() {
            Some(existing) => existing != &session_path,
            None => true,
        };
        if needs_reset {
            Self::reset_state(&mut state, session_path.clone());
        }

        let mut file = File::open(&session_path)
            .with_context(|| format!("open {}", session_path.display()))?;
        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if file_len < state.session_offset {
            Self::reset_state(&mut state, session_path.clone());
        }

        file.seek(SeekFrom::Start(state.session_offset))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut saw_update = false;
        while reader.read_line(&mut line)? > 0 {
            Self::update_from_log(&mut state, line.trim_end());
            saw_update = true;
            line.clear();
        }

        if saw_update {
            state.session_offset = reader.stream_position()?;
            Self::set_last_updated(&mut state);
        }

        Ok(state.stats.clone())
    }
}
