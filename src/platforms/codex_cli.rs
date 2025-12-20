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
enum MessageSource {
    ResponseItem,
    EventMsg,
}

#[derive(Default)]
struct MessageCounters {
    response_item: u32,
    event_msg: u32,
    prefer_event: bool,
}

impl MessageCounters {
    fn record(&mut self, source: MessageSource) {
        match source {
            MessageSource::ResponseItem => {
                self.response_item = self.response_item.saturating_add(1);
            }
            MessageSource::EventMsg => {
                self.event_msg = self.event_msg.saturating_add(1);
                self.prefer_event = true;
            }
        }
    }

    fn effective(&self) -> u32 {
        if self.prefer_event {
            self.event_msg
        } else {
            self.response_item
        }
    }
}

struct CodexState {
    session_path: Option<PathBuf>,
    session_offset: u64,
    last_scan: Option<SystemTime>,
    app_start: SystemTime,
    session_started_at: Option<SystemTime>,
    prompt_counts: MessageCounters,
    completion_counts: MessageCounters,
    stats: PlatformStats,
}

impl Default for CodexState {
    fn default() -> Self {
        Self {
            session_path: None,
            session_offset: 0,
            last_scan: None,
            app_start: SystemTime::now(),
            session_started_at: None,
            prompt_counts: MessageCounters::default(),
            completion_counts: MessageCounters::default(),
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
        for var in ["CRABIGATOR_CODEX_SESSION_PATH", "CODEX_SESSION_PATH"] {
            if let Ok(path) = std::env::var(var) {
                return Some(PathBuf::from(path));
            }
        }
        None
    }

    fn should_rescan(state: &CodexState) -> bool {
        let Some(last_scan) = state.last_scan else {
            return true;
        };
        last_scan.elapsed().unwrap_or(Duration::from_secs(0)) >= Duration::from_secs(2)
    }

    fn resolve_session_path(
        &self,
        cwd: &str,
        state: &mut CodexState,
    ) -> Result<Option<(PathBuf, Option<SystemTime>)>> {
        if let Some(path) = self.session_path_override() {
            return Ok(Some((path, None)));
        }

        let threshold = state
            .app_start
            .checked_sub(Duration::from_secs(2))
            .unwrap_or(state.app_start);

        if let (Some(path), Some(session_start)) =
            (state.session_path.as_ref(), state.session_started_at)
        {
            if path.exists() && session_start >= threshold {
                return Ok(Some((path.clone(), Some(session_start))));
            }
        }

        if !Self::should_rescan(state) {
            return Ok(state.session_path.clone().map(|path| (path, state.session_started_at)));
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

        let choice = Self::choose_candidate(&candidates, threshold, state.app_start);
        if let Some((path, session_start)) = choice {
            return Ok(Some((path, Some(session_start))));
        }

        if let Some(path) = &state.session_path {
            if path.exists() {
                return Ok(Some((path.clone(), state.session_started_at)));
            }
        }

        Ok(None)
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
        candidates: &mut Vec<SessionCandidate>,
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
            let meta = Self::session_meta_info(&path, cwd)?;
            if meta.matches {
                candidates.push(SessionCandidate {
                    path,
                    modified,
                    session_start: meta.session_start,
                });
            }
        }
        Ok(())
    }

    fn session_meta_info(path: &Path, cwd: &str) -> Result<SessionMetaInfo> {
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
                let session_start =
                    payload.get("timestamp").and_then(|v| v.as_str()).and_then(Self::parse_timestamp);
                return Ok(SessionMetaInfo {
                    matches: true,
                    session_start,
                });
            }
        }
        Ok(SessionMetaInfo {
            matches: false,
            session_start: None,
        })
    }

    fn reset_state(state: &mut CodexState, path: PathBuf, session_started_at: Option<SystemTime>) {
        state.session_path = Some(path);
        state.session_offset = 0;
        state.session_started_at = session_started_at;
        state.prompt_counts = MessageCounters::default();
        state.completion_counts = MessageCounters::default();
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
                Self::set_state(state, SessionState::Ready);
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
                        Self::record_completion(state, MessageSource::ResponseItem);
                    }
                    Some("user") => {
                        if !Self::is_bootstrap_message(payload) {
                            Self::record_prompt(state, MessageSource::ResponseItem);
                        }
                    }
                    _ => {}
                }
            }
            Some("function_call") => {
                if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                    Self::record_tool_call(state, name);
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
            Some("user_message") => Self::record_prompt(state, MessageSource::EventMsg),
            Some("agent_message") => {
                Self::record_completion(state, MessageSource::EventMsg);
            }
            _ => {}
        }
    }

    fn set_last_updated(state: &mut CodexState) {
        state.stats.last_updated = Some(Self::now_unix());
    }

    fn parse_timestamp(value: &str) -> Option<SystemTime> {
        let parsed = chrono::DateTime::parse_from_rfc3339(value).ok()?;
        let millis = parsed.timestamp_millis();
        if millis < 0 {
            return None;
        }
        Some(UNIX_EPOCH + Duration::from_millis(millis as u64))
    }

    fn choose_candidate(
        candidates: &[SessionCandidate],
        threshold: SystemTime,
        app_start: SystemTime,
    ) -> Option<(PathBuf, SystemTime)> {
        let mut best: Option<(PathBuf, SystemTime, Duration)> = None;
        for candidate in candidates {
            let session_time = candidate.session_start.unwrap_or(candidate.modified);
            if session_time < threshold {
                continue;
            }
            let delta = if session_time >= app_start {
                session_time.duration_since(app_start).unwrap_or_default()
            } else {
                app_start.duration_since(session_time).unwrap_or_default()
            };
            let is_better = best
                .as_ref()
                .map(|(_, _, best_delta)| delta < *best_delta)
                .unwrap_or(true);
            if is_better {
                best = Some((candidate.path.clone(), session_time, delta));
            }
        }
        if let Some((path, session_time, _)) = best {
            return Some((path, session_time));
        }
        candidates
            .iter()
            .max_by_key(|candidate| candidate.modified)
            .map(|candidate| {
                (
                    candidate.path.clone(),
                    candidate.session_start.unwrap_or(candidate.modified),
                )
            })
    }

    fn now_unix() -> f64 {
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64()
    }

    fn set_state(state: &mut CodexState, new_state: SessionState) {
        state.stats.state = new_state;
        match new_state {
            SessionState::Complete | SessionState::Question => {
                state.stats.idle_since = Some(Self::now_unix());
            }
            _ => {
                state.stats.idle_since = None;
            }
        }
    }

    fn record_prompt(state: &mut CodexState, source: MessageSource) {
        state.prompt_counts.record(source);
        state.stats.prompts = state.prompt_counts.effective();
        let should_set_state = if state.prompt_counts.prefer_event {
            matches!(source, MessageSource::EventMsg)
        } else {
            matches!(source, MessageSource::ResponseItem)
        };
        if should_set_state {
            Self::set_state(state, SessionState::Thinking);
        }
    }

    fn record_completion(state: &mut CodexState, source: MessageSource) {
        state.completion_counts.record(source);
        state.stats.completions = state.completion_counts.effective();
        Self::set_state(state, SessionState::Complete);
    }

    fn record_tool_call(state: &mut CodexState, name: &str) {
        let entry = state.stats.tools.entry(name.to_string()).or_insert(0);
        *entry = entry.saturating_add(1);
        state.stats.tool_timestamps.push(Self::now_unix());
        if Self::is_question_tool(name) {
            Self::set_state(state, SessionState::Question);
        } else {
            Self::set_state(state, SessionState::Thinking);
        }
    }

    fn is_question_tool(name: &str) -> bool {
        matches!(name, "AskUserQuestion" | "ask_user" | "request_user_input")
    }

    fn is_bootstrap_message(payload: &serde_json::Map<String, Value>) -> bool {
        let Some(text) = Self::response_item_text(payload) else {
            return false;
        };
        text.contains("<INSTRUCTIONS>")
            || text.contains("<environment_context>")
            || text.contains("# AGENTS.md instructions")
    }

    fn response_item_text(payload: &serde_json::Map<String, Value>) -> Option<String> {
        let content = payload.get("content")?.as_array()?;
        let mut combined = String::new();
        for entry in content {
            let Some(entry) = entry.as_object() else {
                continue;
            };
            let entry_type = entry.get("type").and_then(|v| v.as_str());
            if matches!(entry_type, Some("input_text") | Some("output_text")) {
                if let Some(text) = entry.get("text").and_then(|v| v.as_str()) {
                    combined.push_str(text);
                }
            }
        }
        if combined.is_empty() {
            None
        } else {
            Some(combined)
        }
    }
}

struct SessionCandidate {
    path: PathBuf,
    modified: SystemTime,
    session_start: Option<SystemTime>,
}

struct SessionMetaInfo {
    matches: bool,
    session_start: Option<SystemTime>,
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
        let (session_path, session_started_at) = match self.resolve_session_path(cwd, &mut state)? {
            Some((path, started_at)) => (path, started_at),
            None => return Ok(PlatformStats::default()),
        };

        let needs_reset = match state.session_path.as_ref() {
            Some(existing) => existing != &session_path,
            None => true,
        };
        if needs_reset {
            Self::reset_state(&mut state, session_path.clone(), session_started_at);
        } else if state.session_started_at.is_none() && session_started_at.is_some() {
            state.session_started_at = session_started_at;
        }

        let mut file = File::open(&session_path)
            .with_context(|| format!("open {}", session_path.display()))?;
        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if file_len < state.session_offset {
            let session_started_at = state.session_started_at;
            Self::reset_state(&mut state, session_path.clone(), session_started_at);
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
