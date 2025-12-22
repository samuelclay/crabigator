//! Codex CLI log parsing
//!
//! Parses JSONL log files from Codex CLI sessions to extract statistics.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::platforms::{PlatformStats, SessionState};

/// Source of a message count
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MessageSource {
    ResponseItem,
    EventMsg,
}

/// Tracks message counts from different sources
#[derive(Default)]
pub struct MessageCounters {
    response_item: u32,
    event_msg: u32,
    prefer_event: bool,
}

impl MessageCounters {
    pub fn record(&mut self, source: MessageSource) {
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

    pub fn effective(&self) -> u32 {
        if self.prefer_event {
            self.event_msg
        } else {
            self.response_item
        }
    }

    pub fn prefer_event(&self) -> bool {
        self.prefer_event
    }
}

/// Internal state for tracking a Codex session
pub struct CodexState {
    pub session_path: Option<PathBuf>,
    pub session_offset: u64,
    pub last_scan: Option<SystemTime>,
    pub app_start: SystemTime,
    pub session_started_at: Option<SystemTime>,
    pub prompt_counts: MessageCounters,
    pub completion_counts: MessageCounters,
    pub stats: PlatformStats,
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

/// Candidate session file discovered during directory scan
pub struct SessionCandidate {
    pub path: PathBuf,
    pub modified: SystemTime,
    pub session_start: Option<SystemTime>,
}

/// Metadata extracted from first few lines of a session file
pub struct SessionMetaInfo {
    pub matches: bool,
    pub session_start: Option<SystemTime>,
}

/// Get current time as Unix timestamp
pub fn now_unix() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

/// Parse RFC3339 timestamp to SystemTime
pub fn parse_timestamp(value: &str) -> Option<SystemTime> {
    let parsed = chrono::DateTime::parse_from_rfc3339(value).ok()?;
    let millis = parsed.timestamp_millis();
    if millis < 0 {
        return None;
    }
    Some(UNIX_EPOCH + Duration::from_millis(millis as u64))
}

/// Update stats from a log line
pub fn update_from_log(state: &mut CodexState, line: &str) {
    let value: Value = match serde_json::from_str(line) {
        Ok(value) => value,
        Err(_) => return,
    };
    let entry_type = value.get("type").and_then(|v| v.as_str()).unwrap_or("");
    match entry_type {
        "response_item" => handle_response_item(state, &value),
        "event_msg" => handle_event_msg(state, &value),
        "session_meta" => {
            set_state(state, SessionState::Ready);
        }
        _ => {}
    }
}

/// Handle a response_item log entry
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
                    record_completion(state, MessageSource::ResponseItem);
                }
                Some("user") => {
                    if !is_bootstrap_message(payload) {
                        record_prompt(state, MessageSource::ResponseItem);
                    }
                }
                _ => {}
            }
        }
        Some("function_call") => {
            if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                record_tool_call(state, name);
            }
        }
        _ => {}
    }
}

/// Handle an event_msg log entry
fn handle_event_msg(state: &mut CodexState, value: &Value) {
    let payload = value.get("payload").and_then(|v| v.as_object());
    let Some(payload) = payload else {
        return;
    };
    match payload.get("type").and_then(|v| v.as_str()) {
        Some("user_message") => record_prompt(state, MessageSource::EventMsg),
        Some("agent_message") => {
            record_completion(state, MessageSource::EventMsg);
        }
        _ => {}
    }
}

/// Set session state and update idle timer
pub fn set_state(state: &mut CodexState, new_state: SessionState) {
    state.stats.state = new_state;
    match new_state {
        SessionState::Complete | SessionState::Question => {
            state.stats.idle_since = Some(now_unix());
        }
        _ => {
            state.stats.idle_since = None;
        }
    }
}

/// Record a user prompt
fn record_prompt(state: &mut CodexState, source: MessageSource) {
    state.prompt_counts.record(source);
    state.stats.prompts = state.prompt_counts.effective();
    let should_set_state = if state.prompt_counts.prefer_event() {
        matches!(source, MessageSource::EventMsg)
    } else {
        matches!(source, MessageSource::ResponseItem)
    };
    if should_set_state {
        set_state(state, SessionState::Thinking);
    }
}

/// Record an assistant completion
fn record_completion(state: &mut CodexState, source: MessageSource) {
    state.completion_counts.record(source);
    state.stats.completions = state.completion_counts.effective();
    set_state(state, SessionState::Complete);
}

/// Record a tool call
fn record_tool_call(state: &mut CodexState, name: &str) {
    let entry = state.stats.tools.entry(name.to_string()).or_insert(0);
    *entry = entry.saturating_add(1);
    state.stats.tool_timestamps.push(now_unix());
    if is_question_tool(name) {
        set_state(state, SessionState::Question);
    } else {
        set_state(state, SessionState::Thinking);
    }
}

/// Check if a tool is a question tool
fn is_question_tool(name: &str) -> bool {
    matches!(name, "AskUserQuestion" | "ask_user" | "request_user_input")
}

/// Check if a message is a bootstrap message (should not count as user prompt)
fn is_bootstrap_message(payload: &serde_json::Map<String, Value>) -> bool {
    let Some(text) = response_item_text(payload) else {
        return false;
    };
    text.contains("<INSTRUCTIONS>")
        || text.contains("<environment_context>")
        || text.contains("# AGENTS.md instructions")
}

/// Extract text content from a response_item payload
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

/// Set the last_updated timestamp
pub fn set_last_updated(state: &mut CodexState) {
    state.stats.last_updated = Some(now_unix());
}

/// Reset state for a new session
pub fn reset_state(state: &mut CodexState, path: PathBuf, session_started_at: Option<SystemTime>) {
    state.session_path = Some(path);
    state.session_offset = 0;
    state.session_started_at = session_started_at;
    state.prompt_counts = MessageCounters::default();
    state.completion_counts = MessageCounters::default();
    state.stats = PlatformStats::default();
    set_last_updated(state);
}
