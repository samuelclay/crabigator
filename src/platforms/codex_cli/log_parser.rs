//! Codex CLI log parsing
//!
//! Parses JSONL log files from Codex CLI sessions to extract statistics.

use std::path::PathBuf;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Deserialize;
use serde_json::Value;

use crate::platforms::{
    ActivePrompt, PermissionDetails, PlatformStats, Question, QuestionOption, SessionState,
};

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
    pub flow_roster: super::companions::ChildRoster,
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
    pub terminal_title: Option<String>,
    pub title_lookup_pending: bool,
    pub last_title_scan: Option<SystemTime>,
    pub prompt_counts: MessageCounters,
    pub completion_counts: MessageCounters,
    task_lifecycle: bool,
    event_timestamp: Option<f64>,
    pub stats: PlatformStats,
    active_prompt_call_id: Option<String>,
    /// Questions from `request_user_input_async` that wait for a reply. Codex
    /// keeps working while they wait and drops them when the turn ends.
    async_questions: Vec<AsyncQuestion>,
    cwd: super::cwd::CodexCwd,
}

/// One question of a `request_user_input_async` call.
struct AsyncQuestion {
    call_id: String,
    index: usize,
    question: Question,
}

impl Default for CodexState {
    fn default() -> Self {
        Self {
            flow_roster: super::companions::ChildRoster::default(),
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
            terminal_title: None,
            title_lookup_pending: false,
            last_title_scan: None,
            prompt_counts: MessageCounters::default(),
            completion_counts: MessageCounters::default(),
            task_lifecycle: false,
            event_timestamp: None,
            stats: PlatformStats::default(),
            active_prompt_call_id: None,
            async_questions: Vec::new(),
            cwd: super::cwd::CodexCwd::default(),
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
    state.event_timestamp = value
        .get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_timestamp)
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|time| time.as_secs_f64());
    state.cwd.observe(&value);
    state.stats.working_directory = state.cwd.current.clone();
    state.stats.working_directory_from_command = state.cwd.from_command;
    match entry_type {
        "response_item" => handle_response_item(state, &value),
        "event_msg" => handle_event_msg(state, &value),
        "turn_context" => handle_turn_context(state, &value),
        "compacted" => state.stats.compressions = state.stats.compressions.saturating_add(1),
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
                Some("assistant")
                    if !state.task_lifecycle
                        && payload.get("phase").and_then(Value::as_str) != Some("commentary") =>
                {
                    record_completion(state, MessageSource::ResponseItem);
                }
                Some("user") => {
                    let text = response_item_text(payload);
                    if let Some(replies) = text.as_deref().and_then(question_replies) {
                        record_question_replies(state, &replies);
                    } else if !text.as_deref().is_some_and(is_injected_user_message) {
                        record_prompt(state, MessageSource::ResponseItem);
                    }
                }
                _ => {}
            }
        }
        Some("function_call") | Some("custom_tool_call") => {
            if let Some(name) = payload.get("name").and_then(|v| v.as_str()) {
                let call_id = payload.get("call_id").and_then(|v| v.as_str());
                let tool_input = parse_function_call_input(payload);
                record_tool_call(state, name, call_id, tool_input);
            }
        }
        Some("function_call_output") | Some("custom_tool_call_output") => {
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
        Some("user_message")
            if !payload
                .get("message")
                .and_then(Value::as_str)
                .is_some_and(is_injected_user_message) =>
        {
            record_prompt(state, MessageSource::EventMsg)
        }
        Some("agent_message")
            if !state.task_lifecycle
                && payload.get("phase").and_then(Value::as_str) != Some("commentary") =>
        {
            record_completion(state, MessageSource::EventMsg);
        }
        Some("task_started") => {
            begin_task_lifecycle(state);
            end_turn_prompts(state);
            set_state(state, SessionState::Thinking);
        }
        Some("task_complete") => {
            begin_task_lifecycle(state);
            record_completion(state, MessageSource::EventMsg);
        }
        Some("turn_aborted") => {
            end_turn_prompts(state);
            set_state(state, SessionState::Interrupted);
        }
        _ => {}
    }
}

fn begin_task_lifecycle(state: &mut CodexState) {
    if !state.task_lifecycle {
        // Older portions of a resumed log may contain response items only.
        state.completion_counts.event_msg = state.completion_counts.effective();
        state.completion_counts.prefer_event = true;
        state.task_lifecycle = true;
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
            state.stats.idle_since = Some(state.event_timestamp.unwrap_or_else(now_unix));
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
        state.stats.prompts_changed_at = state.event_timestamp;
        settle_prompt(state, SessionState::Thinking);
    }
}

/// Record an assistant completion
fn record_completion(state: &mut CodexState, source: MessageSource) {
    state.completion_counts.record(source);
    state.stats.completions = state.completion_counts.effective();
    state.stats.completions_changed_at = state.event_timestamp;
    end_turn_prompts(state);
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
    state
        .stats
        .tool_timestamps
        .push(state.event_timestamp.unwrap_or_else(now_unix));

    let tool = name.rsplit('.').next().unwrap_or(name);
    if tool == ASYNC_QUESTION_TOOL {
        // Codex answers `{"accepted":true}` at once and keeps working; the
        // answers arrive later as a reply message.
        let call_id = call_id.unwrap_or_default();
        let questions = parse_questions(tool_input.as_ref(), true);
        let pending = questions
            .into_iter()
            .enumerate()
            .map(|(index, question)| AsyncQuestion {
                call_id: call_id.to_string(),
                index,
                question,
            });
        state.async_questions.extend(pending);
        show_async_questions(state);
    } else if is_question_tool(tool) {
        set_question_prompt(state, call_id, tool_input.as_ref());
        set_state(state, SessionState::Question);
    } else if is_permission_tool_call(tool_input.as_ref()) {
        set_permission_prompt(state, name, call_id, tool_input);
        set_state(state, SessionState::Permission);
    } else {
        settle_prompt(state, SessionState::Thinking);
    }
}

/// Record tool output and clear pending prompt state when the prompt is answered.
fn record_tool_output(state: &mut CodexState, call_id: &str) {
    if state.active_prompt_call_id.as_deref() == Some(call_id) {
        settle_prompt(state, SessionState::Thinking);
    }
}

/// Remove the async questions a reply answers. Codex is still working, so
/// the session thinks again once none are left.
fn record_question_replies(state: &mut CodexState, replies: &[QuestionReply]) {
    let before = state.async_questions.len();
    state.async_questions.retain(|pending| {
        !replies.iter().any(|reply| match &reply.item {
            Some((call_id, index)) => pending.call_id == *call_id && pending.index == *index,
            None => pending.question.question == reply.question,
        })
    });
    if state.async_questions.len() != before {
        show_async_questions(state);
    }
}

/// Drop an async question the user skipped with ctrl+]. Codex logs nothing
/// for a skip, so the question shown on screen names it.
pub fn skip_async_question(state: &mut CodexState, shown: &str) {
    fn words(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    let shown = words(shown);
    let Some(index) = state
        .async_questions
        .iter()
        .position(|pending| words(&pending.question.question) == shown)
    else {
        return;
    };
    state.async_questions.remove(index);
    show_async_questions(state);
    set_last_updated(state);
}

/// Show the async questions still waiting, or think again when none are
/// left. A permission or question prompt that blocks Codex stays on top
/// until it is answered.
fn show_async_questions(state: &mut CodexState) {
    if state.active_prompt_call_id.is_none() {
        settle_prompt(state, SessionState::Thinking);
    }
}

/// Codex's question tool that returns at once and takes its answers later.
const ASYNC_QUESTION_TOOL: &str = "request_user_input_async";

/// Check if a tool is a question tool that waits for its answer
fn is_question_tool(name: &str) -> bool {
    matches!(name, "AskUserQuestion" | "ask_user" | "request_user_input")
}

/// Read the questions of a Codex question tool call.
///
/// `request_user_input` sends `question` with `{label, description}`
/// options and shows a "None of the above" row that takes notes, not a
/// typed answer. `request_user_input_async` sends `title` with plain string
/// options, or none for a typed answer, and shows an "Other" row.
fn parse_questions(tool_input: Option<&Value>, typed_answers: bool) -> Vec<Question> {
    let Some(questions) = tool_input
        .and_then(|input| input.get("questions"))
        .and_then(Value::as_array)
    else {
        return Vec::new();
    };
    questions
        .iter()
        .filter_map(|question| {
            let text = question
                .get("question")
                .or_else(|| question.get("title"))
                .and_then(Value::as_str)?;
            let options: Vec<QuestionOption> = question
                .get("options")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(|option| match option {
                    Value::String(label) => Some(QuestionOption {
                        label: label.clone(),
                        description: None,
                    }),
                    _ => QuestionOption::deserialize(option).ok(),
                })
                .collect();
            Some(Question {
                question: text.to_string(),
                header: question
                    .get("header")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                allows_other: (!typed_answers && !options.is_empty()).then_some(false),
                options,
                multi_select: question
                    .get("multiSelect")
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect()
}

/// Opening tag of the message that carries answers to async questions.
const QUESTION_REPLY_TAG: &str = "<send_user_message_question_reply>";

/// One answer from a `<send_user_message_question_reply>` message.
pub(crate) struct QuestionReply {
    pub question: String,
    pub answer: String,
    /// The `(call_id, index)` of the question it answers.
    item: Option<(String, usize)>,
}

/// Read the answers in a question reply message, or `None` when the text is
/// not one. Codex sends one per answered question:
///
/// ```text
/// <send_user_message_question_reply>
/// [{"answer":"Pesto","question":"Which sauce?",
///   "questionItemId":"[\"request_user_input_async\",\"call_5e11\",0]"}]
/// </send_user_message_question_reply>
/// ```
pub(crate) fn question_replies(text: &str) -> Option<Vec<QuestionReply>> {
    let body = text.trim_start().strip_prefix(QUESTION_REPLY_TAG)?;
    let body = body
        .split_once("</send_user_message_question_reply>")
        .map_or(body, |(inner, _)| inner);
    let entries: Vec<Value> = serde_json::from_str(body.trim()).unwrap_or_default();
    Some(
        entries
            .iter()
            .map(|entry| {
                let field = |key: &str| {
                    entry
                        .get(key)
                        .and_then(Value::as_str)
                        .unwrap_or_default()
                        .to_string()
                };
                let item = entry
                    .get("questionItemId")
                    .and_then(Value::as_str)
                    .and_then(|id| serde_json::from_str::<(String, String, usize)>(id).ok())
                    .map(|(_, call_id, index)| (call_id, index));
                QuestionReply {
                    question: field("question"),
                    answer: field("answer"),
                    item,
                }
            })
            .collect(),
    )
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

    let questions = parse_questions(tool_input, false);

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

/// Clear the active prompt and move to `next`, unless async questions still
/// wait for an answer: they keep the session in the question state.
fn settle_prompt(state: &mut CodexState, next: SessionState) {
    clear_active_prompt(state);
    if state.async_questions.is_empty() {
        set_state(state, next);
        return;
    }
    let questions = state
        .async_questions
        .iter()
        .map(|pending| pending.question.clone())
        .collect();
    state.stats.active_prompt = Some(ActivePrompt::Question { questions });
    // Keep the idle time from when the first question arrived.
    if state.stats.state != SessionState::Question {
        set_state(state, SessionState::Question);
    }
}

/// Codex drops unanswered async questions when its turn ends.
fn end_turn_prompts(state: &mut CodexState) {
    state.async_questions.clear();
    clear_active_prompt(state);
}

/// Opening tags of the user-role messages Codex writes on its own behalf
/// during a conversation. Most arrive once at session start; `<skill>` is
/// appended right after any prompt that names a `$skill`, carrying the skill
/// file so the model can follow it, and a question reply carries the answers
/// picked in Codex's question view.
const INJECTED_MESSAGE_TAGS: &[&str] = &[
    "<skill>",
    "<skills_instructions>",
    "<apps_instructions>",
    "<plugins_instructions>",
    "<permissions instructions>",
    "<collaboration_mode>",
    "<multi_agent_mode>",
    QUESTION_REPLY_TAG,
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
    state.flow_roster = super::companions::ChildRoster::default();
    state.session_path = Some(path);
    state.session_offset = 0;
    state.session_started_at = session_started_at;
    state.prompt_counts = MessageCounters::default();
    state.completion_counts = MessageCounters::default();
    state.task_lifecycle = false;
    state.event_timestamp = None;
    state.stats = PlatformStats::default();
    state.active_prompt_call_id = None;
    state.async_questions.clear();
    state.cwd = super::cwd::CodexCwd::default();
    set_last_updated(state);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn modern_turns_count_finishes_not_commentary_and_keep_original_times() {
        let mut state = CodexState::default();
        for entry in [
            json!({"type":"event_msg","payload":{"type":"task_started"}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"text":"Fix it"}]}}),
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","phase":"commentary"}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"functions.exec","call_id":"tool","input":"text(1)"}}),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"tool","output":"1"}}),
            json!({"type":"response_item","payload":{"type":"message","role":"assistant","phase":"final"}}),
        ] {
            let mut entry = entry;
            entry["timestamp"] = json!("2026-10-13T17:00:00Z");
            update_from_log(&mut state, &entry.to_string());
        }
        assert_eq!(state.stats.prompts, 1);
        assert_eq!(state.stats.completions, 0);
        assert_eq!(state.stats.total_tool_calls(), 1);
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert_eq!(state.stats.prompts_changed_at, Some(1791910800.0));
        update_from_log(&mut state, &json!({"timestamp":"2026-10-13T17:01:00Z","type":"event_msg","payload":{"type":"task_complete"}}).to_string());
        assert_eq!(state.stats.completions, 1);
        assert_eq!(state.stats.completions_changed_at, Some(1791910860.0));
        assert_eq!(state.stats.idle_since, Some(1791910860.0));
        assert_eq!(state.stats.state, SessionState::Complete);
    }

    #[test]
    fn modern_turns_preserve_legacy_completion_totals() {
        let mut state = CodexState::default();
        for _ in 0..3 {
            update_from_log(
                &mut state,
                &json!({"type":"response_item","payload":{"type":"message","role":"assistant"}})
                    .to_string(),
            );
        }
        update_from_log(
            &mut state,
            &json!({"type":"event_msg","payload":{"type":"task_started"}}).to_string(),
        );
        update_from_log(&mut state, &json!({"type":"response_item","payload":{"type":"message","role":"assistant","phase":"final"}}).to_string());
        update_from_log(
            &mut state,
            &json!({"type":"event_msg","payload":{"type":"task_complete"}}).to_string(),
        );
        assert_eq!(state.stats.completions, 4);
    }

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
                // "None of the above" takes notes, not a typed answer.
                assert_eq!(questions[0].allows_other, Some(false));
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

    fn question_reply(call_id: &str, index: usize, question: &str, answer: &str) -> String {
        let reply = json!([{
            "answer": answer,
            "question": question,
            "questionItemId": json!(["request_user_input_async", call_id, index]).to_string(),
        }]);
        user_message(&format!(
            "<send_user_message_question_reply>\n{reply}\n</send_user_message_question_reply>"
        ))
    }

    fn prompt_questions(state: &CodexState) -> Vec<String> {
        match state.stats.active_prompt.as_ref() {
            Some(ActivePrompt::Question { questions }) => {
                questions.iter().map(|q| q.question.clone()).collect()
            }
            other => panic!("expected question prompt, got {other:?}"),
        }
    }

    /// Codex 0.162 asks with `request_user_input_async`: `title` and plain
    /// string options, an immediate `{"accepted":true}` output, more work,
    /// then one reply message per answered question.
    #[test]
    fn async_questions_wait_until_each_is_answered() {
        let mut state = CodexState::default();
        for line in [
            json!({"type":"event_msg","payload":{"type":"task_started"}}).to_string(),
            user_message("order a pizza"),
            json!({"type":"response_item","payload":{"type":"function_call","name":"request_user_input_async","call_id":"call_q","arguments":"{\"questions\":[{\"title\":\"Which pizza size?\",\"options\":[\"Small\",\"Medium\",\"Large\"]},{\"title\":\"Any delivery notes?\"}]}"}}).to_string(),
            json!({"type":"response_item","payload":{"type":"function_call_output","call_id":"call_q","output":"{\"accepted\":true}"}}).to_string(),
            json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"call_sleep","input":"sleep 240"}}).to_string(),
            json!({"type":"response_item","payload":{"type":"custom_tool_call_output","call_id":"call_sleep","output":"done"}}).to_string(),
        ] {
            update_from_log(&mut state, &line);
        }

        assert_eq!(state.stats.state, SessionState::Question);
        let Some(ActivePrompt::Question { questions }) = state.stats.active_prompt.as_ref() else {
            panic!("expected question prompt");
        };
        assert_eq!(questions[0].question, "Which pizza size?");
        let labels: Vec<&str> = questions[0]
            .options
            .iter()
            .map(|o| o.label.as_str())
            .collect();
        assert_eq!(labels, ["Small", "Medium", "Large"]);
        assert_eq!(
            questions[0].allows_other, None,
            "the async view has an Other row"
        );
        assert_eq!(questions[1].question, "Any delivery notes?");
        assert!(questions[1].options.is_empty());

        update_from_log(
            &mut state,
            &question_reply("call_q", 0, "Which pizza size?", "extra large"),
        );
        assert_eq!(state.stats.state, SessionState::Question);
        assert_eq!(prompt_questions(&state), ["Any delivery notes?"]);

        update_from_log(
            &mut state,
            &question_reply("call_q", 1, "Any delivery notes?", "ring the bell"),
        );
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.active_prompt.is_none());
        assert_eq!(state.stats.prompts, 1, "replies are not prompts");
    }

    /// ctrl+] skips an async question without a log entry.
    #[test]
    fn skipped_async_questions_stop_waiting() {
        let mut state = CodexState::default();
        update_from_log(
            &mut state,
            &json!({"type":"response_item","payload":{"type":"function_call","name":"request_user_input_async","call_id":"call_q","arguments":"{\"questions\":[{\"title\":\"Which sauce?\",\"options\":[\"Tomato\",\"Pesto\"]},{\"title\":\"Which drink?\",\"options\":[\"Cola\",\"Water\"]}]}"}}).to_string(),
        );

        skip_async_question(&mut state, "Which  drink?");
        assert_eq!(state.stats.state, SessionState::Question);
        assert_eq!(prompt_questions(&state), ["Which sauce?"]);

        skip_async_question(&mut state, "Which sauce?");
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.active_prompt.is_none());
    }

    /// Codex drops an unanswered async question when its turn ends.
    #[test]
    fn async_questions_end_with_the_turn() {
        let mut state = CodexState::default();
        for line in [
            json!({"type":"event_msg","payload":{"type":"task_started"}}).to_string(),
            json!({"type":"response_item","payload":{"type":"function_call","name":"request_user_input_async","call_id":"call_q","arguments":"{\"questions\":[{\"title\":\"Which drink?\",\"options\":[\"Cola\",\"Water\"]}]}"}}).to_string(),
            json!({"type":"event_msg","payload":{"type":"task_complete"}}).to_string(),
        ] {
            update_from_log(&mut state, &line);
        }
        assert_eq!(state.stats.state, SessionState::Complete);
        assert!(state.stats.active_prompt.is_none());

        update_from_log(
            &mut state,
            &json!({"type":"event_msg","payload":{"type":"task_started"}}).to_string(),
        );
        update_from_log(
            &mut state,
            &json!({"type":"response_item","payload":{"type":"custom_tool_call","name":"exec","call_id":"call_ls","input":"ls"}}).to_string(),
        );
        assert_eq!(state.stats.state, SessionState::Thinking);
    }

    #[test]
    fn reads_question_replies() {
        let text = "<send_user_message_question_reply>\n[{\"answer\":\"Pesto\",\"question\":\"Which pizza sauce?\",\"questionItemId\":\"[\\\"request_user_input_async\\\",\\\"call_5e11\\\",0]\"}]\n</send_user_message_question_reply>";
        let replies = question_replies(text).expect("a reply");
        assert_eq!(replies.len(), 1);
        assert_eq!(replies[0].question, "Which pizza sauce?");
        assert_eq!(replies[0].answer, "Pesto");
        assert_eq!(replies[0].item, Some(("call_5e11".to_string(), 0)));
        assert!(question_replies("Pesto, please").is_none());
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
