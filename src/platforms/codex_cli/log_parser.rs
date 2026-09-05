//! Codex CLI log parsing
//!
//! Parses JSONL log files from Codex CLI sessions to extract statistics.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde_json::Value;

use crate::platforms::{ActivePrompt, PermissionDetails, PlatformStats, Question, SessionState};

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
    /// True when `session_path` is a conversation adopted after an in-pane
    /// resume rather than the rollout that started with this pane.
    pub resume_followed: bool,
    pub resume_requested: bool,
    pub resume_all: bool,
    pub explicit_resume_id: Option<String>,
    /// The rollout that started with this pane, kept while following a resume
    /// so a resume back to the original conversation is preferred.
    pub native_session_path: Option<PathBuf>,
    pub native_session_started_at: Option<SystemTime>,
    /// The native rollout's mtime when the pane switched away from it; a
    /// newer write means the pane resumed back.
    pub native_mtime_at_switch: Option<SystemTime>,
    /// When the user last sent input to this pane. An in-pane resume always
    /// starts with input, so the resume-follow scan requires input newer
    /// than the tracked rollout's last write before it adopts anything.
    pub last_user_input: Option<SystemTime>,
    pub prompt_counts: MessageCounters,
    pub completion_counts: MessageCounters,
    pub stats: PlatformStats,
    active_prompt_call_id: Option<String>,
}

impl Default for CodexState {
    fn default() -> Self {
        Self {
            session_path: None,
            session_offset: 0,
            last_scan: None,
            app_start: SystemTime::now(),
            session_started_at: None,
            resume_followed: false,
            resume_requested: false,
            resume_all: false,
            explicit_resume_id: None,
            native_session_path: None,
            native_session_started_at: None,
            native_mtime_at_switch: None,
            last_user_input: None,
            prompt_counts: MessageCounters::default(),
            completion_counts: MessageCounters::default(),
            stats: PlatformStats::default(),
            active_prompt_call_id: None,
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
        "turn_context" => handle_turn_context(state, &value),
        "session_meta" => {
            set_state(state, SessionState::Ready);
            if let Some(id) = value
                .pointer("/payload/id")
                .and_then(Value::as_str)
                .filter(|id| !id.is_empty())
            {
                state.stats.native_session_id = Some(id.to_string());
            }
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
                Some("user") if !is_bootstrap_message(payload) => {
                    record_prompt(state, MessageSource::ResponseItem);
                }
                _ => {}
            }
        }
        Some("function_call") => {
            if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                let call_id = payload.get("call_id").and_then(|v| v.as_str());
                let tool_input = parse_function_call_input(payload);
                record_tool_call(state, name, call_id, tool_input);
            }
        }
        Some("function_call_output") => {
            if let Some(call_id) = payload.get("call_id").and_then(|v| v.as_str()) {
                record_tool_output(state, call_id);
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

/// Handle a turn_context log entry.
fn handle_turn_context(state: &mut CodexState, value: &Value) {
    let payload = value.get("payload").and_then(|v| v.as_object());
    let Some(payload) = payload else {
        return;
    };
    if let Some(model) = payload
        .get("model")
        .and_then(|v| v.as_str())
        .filter(|model| !model.trim().is_empty())
    {
        state.stats.model = Some(model.to_string());
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
        clear_active_prompt(state);
        set_state(state, SessionState::Thinking);
    }
}

/// Record an assistant completion
fn record_completion(state: &mut CodexState, source: MessageSource) {
    state.completion_counts.record(source);
    state.stats.completions = state.completion_counts.effective();
    clear_active_prompt(state);
    set_state(state, SessionState::Complete);
}

/// Record a tool call
fn record_tool_call(
    state: &mut CodexState,
    name: &str,
    call_id: Option<&str>,
    tool_input: Option<Value>,
) {
    let entry = state.stats.tools.entry(name.to_string()).or_insert(0);
    *entry = entry.saturating_add(1);
    state.stats.tool_timestamps.push(now_unix());

    if is_question_tool(name) {
        set_question_prompt(state, call_id, tool_input.as_ref());
        set_state(state, SessionState::Question);
    } else if is_permission_tool_call(tool_input.as_ref()) {
        set_permission_prompt(state, name, call_id, tool_input);
        set_state(state, SessionState::Permission);
    } else {
        clear_active_prompt(state);
        set_state(state, SessionState::Thinking);
    }
}

/// Record tool output and clear pending prompt state when the prompt is answered.
fn record_tool_output(state: &mut CodexState, call_id: &str) {
    if state.active_prompt_call_id.as_deref() == Some(call_id) {
        clear_active_prompt(state);
        set_state(state, SessionState::Thinking);
    }
}

/// Check if a tool is a question tool
fn is_question_tool(name: &str) -> bool {
    matches!(name, "AskUserQuestion" | "ask_user" | "request_user_input")
}

/// Parse function_call arguments from payload.
fn parse_function_call_input(payload: &serde_json::Map<String, Value>) -> Option<Value> {
    match payload.get("arguments") {
        Some(Value::String(raw)) => serde_json::from_str(raw).ok(),
        Some(value) => Some(value.clone()),
        None => None,
    }
}

/// Check if a tool call is requesting an escalated permission prompt.
fn is_permission_tool_call(tool_input: Option<&Value>) -> bool {
    let Some(input) = tool_input else {
        return false;
    };

    if input
        .get("sandbox_permissions")
        .and_then(|v| v.as_str())
        .is_some_and(|mode| mode == "require_escalated")
    {
        return true;
    }

    input
        .get("with_escalated_permissions")
        .and_then(|v| v.as_bool())
        .unwrap_or(false)
}

/// Set active prompt state for a question tool.
fn set_question_prompt(state: &mut CodexState, call_id: Option<&str>, tool_input: Option<&Value>) {
    clear_active_prompt(state);

    let questions = tool_input
        .and_then(|input| input.get("questions"))
        .and_then(|questions| serde_json::from_value::<Vec<Question>>(questions.clone()).ok())
        .unwrap_or_default();

    state.stats.active_prompt = Some(ActivePrompt::Question { questions });
    state.stats.permission = None;
    state.active_prompt_call_id = call_id.map(|id| id.to_string());
}

/// Set active prompt state for an escalated permission tool.
fn set_permission_prompt(
    state: &mut CodexState,
    name: &str,
    call_id: Option<&str>,
    tool_input: Option<Value>,
) {
    clear_active_prompt(state);

    let permission_input = tool_input.unwrap_or(Value::Null);
    state.stats.active_prompt = Some(ActivePrompt::Permission {
        tool_name: name.to_string(),
        tool_input: Some(permission_input.clone()),
    });
    state.stats.permission = Some(PermissionDetails {
        tool: name.to_string(),
        input: permission_input,
        suggestions: vec![],
    });
    state.active_prompt_call_id = call_id.map(|id| id.to_string());
}

/// Clear active prompt and associated permission details.
fn clear_active_prompt(state: &mut CodexState) {
    state.active_prompt_call_id = None;
    state.stats.active_prompt = None;
    state.stats.permission = None;
}

/// Check if a user-role message was written by Codex itself (should not count as a user prompt)
fn is_bootstrap_message(payload: &serde_json::Map<String, Value>) -> bool {
    response_item_text(payload).is_some_and(|text| is_injected_user_message(&text))
}

/// Opening tags of the user-role messages Codex writes on its own behalf
/// during a conversation. Most arrive once at session start; `<skill>` is
/// appended right after any prompt that names a `$skill`, carrying the skill
/// file so the model can follow it.
const INJECTED_MESSAGE_TAGS: &[&str] = &[
    "<skill>",
    "<skills_instructions>",
    "<apps_instructions>",
    "<plugins_instructions>",
    "<permissions instructions>",
    "<collaboration_mode>",
    "<multi_agent_mode>",
];

/// Whether a user-role message in a Codex rollout was written by Codex rather
/// than typed by the user: bootstrap instructions, environment context, mode
/// switches, and skill bodies. Shared by the stats parser, the transcript
/// writer, and the recap turn collector so all three agree on what a prompt is.
pub(crate) fn is_injected_user_message(text: &str) -> bool {
    text.contains("<INSTRUCTIONS>")
        || text.contains("<environment_context>")
        || text.contains("# AGENTS.md instructions")
        || INJECTED_MESSAGE_TAGS
            .iter()
            .any(|tag| text.trim_start().starts_with(tag))
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
    state.active_prompt_call_id = None;
    set_last_updated(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn user_message(text: &str) -> String {
        json!({
            "type": "response_item",
            "payload": {
                "type": "message",
                "role": "user",
                "content": [{"type": "input_text", "text": text}]
            }
        })
        .to_string()
    }

    /// Codex appends the body of any `$skill` the prompt names as a second
    /// user-role message. Only the typed prompt counts.
    #[test]
    fn skill_bodies_and_mode_notices_do_not_count_as_prompts() {
        let mut state = CodexState::default();
        update_from_log(
            &mut state,
            &user_message("<permissions instructions>\nsandbox"),
        );
        update_from_log(&mut state, &user_message("<collaboration_mode># Default"));
        update_from_log(&mut state, &user_message("fix the hover, use $commit-pr"));
        update_from_log(
            &mut state,
            &user_message(
                "<skill>\n<name>commit-pr</name>\n<path>/skills/commit-pr/SKILL.md</path>",
            ),
        );
        assert_eq!(state.stats.prompts, 1);
    }

    #[test]
    fn injected_user_messages_are_recognized_by_their_opening_tag() {
        assert!(is_injected_user_message("<skill>\n<name>x</name>"));
        assert!(is_injected_user_message("  <multi_agent_mode>off"));
        assert!(is_injected_user_message(
            "# AGENTS.md instructions for /repo\n<INSTRUCTIONS>"
        ));
        assert!(!is_injected_user_message("write a <skill> tag parser"));
        assert!(!is_injected_user_message(
            "<image name=[Image #1] path=/tmp/a.png>"
        ));
    }

    #[test]
    fn tracks_permission_prompt_for_escalated_tool_calls() {
        let mut state = CodexState::default();

        update_from_log(
            &mut state,
            &json!({
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "exec_command",
                    "call_id": "call_perm",
                    "arguments": "{\"cmd\":\"tmux ls\",\"sandbox_permissions\":\"require_escalated\",\"justification\":\"Allow?\"}"
                }
            })
            .to_string(),
        );

        assert_eq!(state.stats.state, SessionState::Permission);
        assert_eq!(state.active_prompt_call_id.as_deref(), Some("call_perm"));
        assert!(matches!(
            state.stats.active_prompt.as_ref(),
            Some(ActivePrompt::Permission { tool_name, .. }) if tool_name == "exec_command"
        ));

        let permission = state.stats.permission.as_ref().expect("permission details");
        assert_eq!(permission.tool, "exec_command");
        assert_eq!(
            permission
                .input
                .get("sandbox_permissions")
                .and_then(|v| v.as_str()),
            Some("require_escalated")
        );

        update_from_log(
            &mut state,
            &json!({
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "call_perm",
                    "output": "approved"
                }
            })
            .to_string(),
        );

        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.active_prompt.is_none());
        assert!(state.stats.permission.is_none());
        assert!(state.active_prompt_call_id.is_none());
    }

    #[test]
    fn keeps_permission_prompt_for_unrelated_tool_outputs() {
        let mut state = CodexState::default();

        update_from_log(
            &mut state,
            &json!({
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "exec_command",
                    "call_id": "call_perm",
                    "arguments": "{\"cmd\":\"tmux ls\",\"sandbox_permissions\":\"require_escalated\"}"
                }
            })
            .to_string(),
        );

        update_from_log(
            &mut state,
            &json!({
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "call_other",
                    "output": "done"
                }
            })
            .to_string(),
        );

        assert_eq!(state.stats.state, SessionState::Permission);
        assert_eq!(state.active_prompt_call_id.as_deref(), Some("call_perm"));
        assert!(matches!(
            state.stats.active_prompt.as_ref(),
            Some(ActivePrompt::Permission { .. })
        ));
    }

    #[test]
    fn tracks_question_prompt_for_request_user_input_calls() {
        let mut state = CodexState::default();

        update_from_log(
            &mut state,
            &json!({
                "type": "response_item",
                "payload": {
                    "type": "function_call",
                    "name": "request_user_input",
                    "call_id": "call_question",
                    "arguments": "{\"questions\":[{\"header\":\"Scope\",\"id\":\"scope\",\"question\":\"Which scope?\",\"options\":[{\"label\":\"A\",\"description\":\"Option A\"},{\"label\":\"B\",\"description\":\"Option B\"}]}]}"
                }
            })
            .to_string(),
        );

        assert_eq!(state.stats.state, SessionState::Question);
        assert_eq!(
            state.active_prompt_call_id.as_deref(),
            Some("call_question")
        );

        match state.stats.active_prompt.as_ref() {
            Some(ActivePrompt::Question { questions }) => {
                assert_eq!(questions.len(), 1);
                assert_eq!(questions[0].question, "Which scope?");
                assert_eq!(questions[0].options.len(), 2);
                assert_eq!(questions[0].options[0].label, "A");
            }
            other => panic!("expected question prompt, got {other:?}"),
        }

        update_from_log(
            &mut state,
            &json!({
                "type": "response_item",
                "payload": {
                    "type": "function_call_output",
                    "call_id": "call_question",
                    "output": "1"
                }
            })
            .to_string(),
        );

        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.active_prompt.is_none());
        assert!(state.stats.permission.is_none());
    }

    #[test]
    fn tracks_model_from_turn_context() {
        let mut state = CodexState::default();

        update_from_log(
            &mut state,
            &json!({
                "type": "turn_context",
                "payload": {
                    "cwd": "/Users/example/project",
                    "model": "gpt-5.5"
                }
            })
            .to_string(),
        );

        assert_eq!(state.stats.model.as_deref(), Some("gpt-5.5"));
    }
}
