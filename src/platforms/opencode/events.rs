//! opencode event-stream processing.
//!
//! Translates the opencode server's SSE events into crabigator's
//! [`PlatformStats`] plus normalized transcript entries. All parsing is pure
//! so it can be unit-tested without a live server.

use std::collections::{HashMap, HashSet};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::platforms::{
    ActivePrompt, ClaudeMode, HookEvent, PermissionDetails, PlatformStats, SessionState,
};

/// Rolling cap on the debug event history.
const EVENT_HISTORY_LIMIT: usize = 50;

/// Tool output is truncated before it is written to the transcript log.
/// Large enough to keep `gh pr view` output intact for PR tracking.
const MAX_TOOL_OUTPUT_CHARS: usize = 4_000;

/// One normalized entry in the opencode transcript log (JSONL).
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum TranscriptEntry {
    User {
        text: String,
    },
    Assistant {
        text: String,
    },
    Tool {
        name: String,
        title: String,
        output: String,
    },
}

/// State shared between the event listener thread and `load_stats`.
#[derive(Default)]
pub struct OpencodeState {
    pub stats: PlatformStats,
    /// The top-level session this crabigator instance wraps
    pub main_session: Option<String>,
    /// Model id -> display name, from the server's provider catalog
    pub model_names: HashMap<String, String>,
    /// Sessions spawned as subagent children (have a parentID)
    child_sessions: HashSet<String>,
    /// Message id -> role, so parts can be attributed
    message_roles: HashMap<String, String>,
    /// Assistant text parts buffered until their message completes.
    /// Message id -> ordered (part id, text) pairs.
    text_buffers: HashMap<String, Vec<(String, String)>>,
    counted_prompts: HashSet<String>,
    counted_completions: HashSet<String>,
    counted_tools: HashSet<String>,
    written_parts: HashSet<String>,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn truncate_chars(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let prefix: String = text.chars().take(max_chars).collect();
    format!("{prefix}…")
}

impl OpencodeState {
    fn is_main(&self, session_id: &str) -> bool {
        self.main_session.as_deref() == Some(session_id)
    }

    /// Whether an event belongs to the wrapped session. Events that name no
    /// session apply to it by default.
    fn is_main_event(&self, props: &Value) -> bool {
        session_id(props).is_none_or(|id| self.is_main(id))
    }

    fn set_state(&mut self, state: SessionState) {
        if self.stats.state != state {
            self.stats.state = state;
            if matches!(state, SessionState::Complete) {
                self.stats.idle_since = Some(now_secs());
            } else {
                self.stats.idle_since = None;
            }
        }
    }

    fn clear_permission(&mut self) {
        self.stats.permission = None;
        self.stats.active_prompt = None;
    }

    fn record_history(&mut self, event: &str) {
        self.stats.event_history.push(HookEvent {
            ts: now_secs(),
            event: event.to_string(),
            state_before: format!("{:?}", self.stats.state).to_lowercase(),
            details: None,
        });
        if self.stats.event_history.len() > EVENT_HISTORY_LIMIT {
            let excess = self.stats.event_history.len() - EVENT_HISTORY_LIMIT;
            self.stats.event_history.drain(..excess);
        }
    }

    /// Resolve a model id to its catalog display name, falling back to the
    /// id without its vendor prefix (`stealth/ox-alpha` -> `ox-alpha`).
    fn display_model(&self, model_id: &str) -> String {
        if let Some(name) = self.model_names.get(model_id) {
            return name.clone();
        }
        model_id.rsplit('/').next().unwrap_or(model_id).to_string()
    }

    /// Adopt `session_id` as the wrapped session if none is tracked yet. The
    /// server is private to this pane's TUI, so every non-child session on it
    /// belongs to this instance.
    fn maybe_adopt(&mut self, session_id: &str) {
        if self.main_session.is_some() || self.child_sessions.contains(session_id) {
            return;
        }
        self.main_session = Some(session_id.to_string());
    }

    /// Buffered text for one streaming part, created empty on first sight.
    /// Snapshots and deltas for the same part share this slot.
    fn text_slot(&mut self, message_id: &str, part_id: &str) -> &mut String {
        let buffer = self.text_buffers.entry(message_id.to_string()).or_default();
        let index = match buffer.iter().position(|(id, _)| id == part_id) {
            Some(index) => index,
            None => {
                buffer.push((part_id.to_string(), String::new()));
                buffer.len() - 1
            }
        };
        &mut buffer[index].1
    }

    fn flush_text_buffer(&mut self, message_id: &str, entries: &mut Vec<TranscriptEntry>) {
        let Some(parts) = self.text_buffers.remove(message_id) else {
            return;
        };
        let is_user = self.message_roles.get(message_id).map(String::as_str) == Some("user");
        for (part_id, text) in parts {
            if text.trim().is_empty() || !self.written_parts.insert(part_id) {
                continue;
            }
            if is_user {
                entries.push(TranscriptEntry::User { text });
            } else {
                entries.push(TranscriptEntry::Assistant { text });
            }
        }
    }
}

/// Apply one SSE event to the shared state, returning transcript entries
/// that became final with this event.
pub fn apply_event(state: &mut OpencodeState, event: &Value) -> Vec<TranscriptEntry> {
    let Some(event_type) = event.get("type").and_then(Value::as_str) else {
        return Vec::new();
    };
    let props = event.get("properties").unwrap_or(&Value::Null);
    let mut entries = Vec::new();

    match event_type {
        "session.created" | "session.updated" => on_session_info(state, props),
        "session.status" => on_session_status(state, props),
        "session.idle" => {
            if state.is_main_event(props) {
                state.clear_permission();
                // An aborted turn also goes idle; keep Interrupted visible
                // until the next prompt or turn starts.
                if state.stats.state != SessionState::Interrupted {
                    state.set_state(SessionState::Complete);
                }
                state.record_history("session.idle");
            }
        }
        "session.error" => on_session_error(state, props),
        "session.compacted" => {
            if state.is_main_event(props) {
                state.stats.compressions += 1;
                state.record_history("session.compacted");
            }
        }
        "message.updated" => on_message_updated(state, props, &mut entries),
        "message.part.updated" => on_part_updated(state, props, &mut entries),
        "message.part.delta" => on_part_delta(state, props),
        // The wire name is `permission.asked`; older SDK docs call the same
        // payload `permission.updated`, so accept both.
        "permission.asked" | "permission.updated" => on_permission_asked(state, props),
        "permission.replied" => {
            state.clear_permission();
            state.set_state(SessionState::Thinking);
            state.record_history("permission.replied");
        }
        _ => return entries,
    }

    state.stats.last_updated = Some(now_secs());
    entries
}

fn session_id(props: &Value) -> Option<&str> {
    props.get("sessionID").and_then(Value::as_str)
}

fn on_session_info(state: &mut OpencodeState, props: &Value) {
    let Some(info) = props.get("info") else {
        return;
    };
    let Some(id) = info.get("id").and_then(Value::as_str) else {
        return;
    };
    if info.get("parentID").and_then(Value::as_str).is_some() {
        state.child_sessions.insert(id.to_string());
        return;
    }
    state.maybe_adopt(id);
    if !state.is_main(id) {
        return;
    }
    if let Some(model_id) = info
        .get("model")
        .and_then(|m| m.get("id").or_else(|| m.get("modelID")))
        .and_then(Value::as_str)
    {
        state.stats.model = Some(state.display_model(model_id));
    }
    if let Some(agent) = info.get("agent").and_then(Value::as_str) {
        state.stats.mode = agent_mode(agent);
    }
}

fn on_session_status(state: &mut OpencodeState, props: &Value) {
    let Some(id) = session_id(props) else {
        return;
    };
    if !state.is_main(id) {
        return;
    }
    let status = props
        .get("status")
        .and_then(|s| s.get("type"))
        .and_then(Value::as_str);
    match status {
        // A pending permission prompt keeps priority over busy/retry noise
        Some("busy") | Some("retry") if state.stats.permission.is_none() => {
            state.set_state(SessionState::Thinking);
        }
        Some("idle") => {
            state.clear_permission();
            if state.stats.state != SessionState::Interrupted {
                state.set_state(SessionState::Complete);
            }
        }
        _ => {}
    }
}

fn on_session_error(state: &mut OpencodeState, props: &Value) {
    if !state.is_main_event(props) {
        return;
    }
    let error_name = props
        .get("error")
        .and_then(|e| e.get("name"))
        .and_then(Value::as_str)
        .unwrap_or_default();
    state.clear_permission();
    if error_name == "MessageAbortedError" {
        state.set_state(SessionState::Interrupted);
    } else {
        state.set_state(SessionState::Complete);
    }
    state.record_history(&format!("session.error:{error_name}"));
}

fn on_message_updated(
    state: &mut OpencodeState,
    props: &Value,
    entries: &mut Vec<TranscriptEntry>,
) {
    let Some(info) = props.get("info") else {
        return;
    };
    let Some(message_id) = info.get("id").and_then(Value::as_str) else {
        return;
    };
    let Some(sid) = info
        .get("sessionID")
        .and_then(Value::as_str)
        .or_else(|| session_id(props))
    else {
        return;
    };
    let role = info.get("role").and_then(Value::as_str).unwrap_or_default();
    state
        .message_roles
        .insert(message_id.to_string(), role.to_string());

    // The server is private to this pane, so a user message on any
    // non-child session means the pane is driving that session. This both
    // adopts the first session (covering --continue, where no
    // session.created fires) and follows in-pane session switches
    // (ctrl+x n / ctrl+x l).
    if role == "user" && !state.child_sessions.contains(sid) && !state.is_main(sid) {
        state.main_session = Some(sid.to_string());
    }
    let is_main = state.is_main(sid);
    let is_child = state.child_sessions.contains(sid);

    match role {
        "user" if is_main => {
            if state.counted_prompts.insert(message_id.to_string()) {
                state.stats.prompts += 1;
                state.clear_permission();
                state.set_state(SessionState::Thinking);
                state.record_history("prompt.submitted");
            }
            // User text parts arrive before the role is known on some
            // orderings; flush anything buffered for this message now.
            state.flush_text_buffer(message_id, entries);
        }
        "assistant" => {
            if is_main {
                if let Some(model_id) = info.get("modelID").and_then(Value::as_str).or_else(|| {
                    info.get("model")
                        .and_then(|m| m.get("modelID"))
                        .and_then(Value::as_str)
                }) {
                    state.stats.model = Some(state.display_model(model_id));
                }
                if let Some(agent) = info.get("agent").and_then(Value::as_str) {
                    state.stats.mode = agent_mode(agent);
                }
            }

            let completed = info.get("time").and_then(|t| t.get("completed")).is_some();
            let aborted = info
                .get("error")
                .and_then(|e| e.get("name"))
                .and_then(Value::as_str)
                == Some("MessageAbortedError");
            // opencode creates one assistant message per step; only the
            // final answer of a turn counts as a completion.
            let final_answer = info.get("finish").and_then(Value::as_str) != Some("tool-calls");

            if (completed || aborted) && state.counted_completions.insert(message_id.to_string()) {
                if is_main {
                    state.flush_text_buffer(message_id, entries);
                    if completed && !aborted && final_answer {
                        state.stats.completions += 1;
                    }
                } else if is_child && completed && final_answer {
                    state.stats.subagent_messages += 1;
                }
            }
        }
        _ => {}
    }
}

fn on_part_updated(state: &mut OpencodeState, props: &Value, entries: &mut Vec<TranscriptEntry>) {
    let Some(part) = props.get("part") else {
        return;
    };
    let (Some(part_id), Some(message_id), Some(sid)) = (
        part.get("id").and_then(Value::as_str),
        part.get("messageID").and_then(Value::as_str),
        part.get("sessionID")
            .and_then(Value::as_str)
            .or_else(|| session_id(props)),
    ) else {
        return;
    };
    let is_main = state.is_main(sid);

    match part.get("type").and_then(Value::as_str) {
        Some("text") => {
            if !is_main || state.written_parts.contains(part_id) {
                return;
            }
            let Some(text) = part.get("text").and_then(Value::as_str) else {
                return;
            };
            let role = state.message_roles.get(message_id).map(String::as_str);
            if role == Some("user") {
                if !text.trim().is_empty() && state.written_parts.insert(part_id.to_string()) {
                    entries.push(TranscriptEntry::User {
                        text: text.to_string(),
                    });
                }
                return;
            }
            // Assistant (or not-yet-attributed) text streams as snapshots
            // that can lag behind the delta feed; keep whichever version is
            // longer and flush when the message ends.
            let slot = state.text_slot(message_id, part_id);
            if text.len() > slot.len() {
                slot.clear();
                slot.push_str(text);
            }
        }
        Some("tool") => {
            let Some(part_state) = part.get("state") else {
                return;
            };
            let status = part_state.get("status").and_then(Value::as_str);
            if !matches!(status, Some("completed") | Some("error")) {
                return;
            }
            if !state.counted_tools.insert(part_id.to_string()) {
                return;
            }
            let name = part
                .get("tool")
                .and_then(Value::as_str)
                .unwrap_or("tool")
                .to_string();
            *state.stats.tools.entry(name.clone()).or_insert(0) += 1;
            let ended = part_state
                .get("time")
                .and_then(|t| t.get("end"))
                .and_then(Value::as_f64)
                .map(|ms| ms / 1000.0)
                .unwrap_or_else(now_secs);
            state.stats.tool_timestamps.push(ended);

            if is_main {
                let title = part_state
                    .get("title")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| tool_input_preview(part_state));
                let output = part_state
                    .get("output")
                    .and_then(Value::as_str)
                    .unwrap_or_default();
                entries.push(TranscriptEntry::Tool {
                    name,
                    title,
                    output: truncate_chars(output, MAX_TOOL_OUTPUT_CHARS),
                });
            }
        }
        _ => {}
    }
}

/// Streaming text deltas keep the assistant buffer current even when full
/// part snapshots lag behind.
fn on_part_delta(state: &mut OpencodeState, props: &Value) {
    if props.get("field").and_then(Value::as_str) != Some("text") {
        return;
    }
    let (Some(part_id), Some(message_id), Some(sid), Some(delta)) = (
        props.get("partID").and_then(Value::as_str),
        props.get("messageID").and_then(Value::as_str),
        session_id(props),
        props.get("delta").and_then(Value::as_str),
    ) else {
        return;
    };
    if !state.is_main(sid) || state.written_parts.contains(part_id) {
        return;
    }
    state.text_slot(message_id, part_id).push_str(delta);
}

/// Replay a session's message history (from `GET /session/{id}/message`)
/// into stats and transcript entries. Used when continuing an existing
/// session, where the event stream starts mid-conversation.
pub fn backfill_messages(state: &mut OpencodeState, messages: &Value) -> Vec<TranscriptEntry> {
    let mut entries = Vec::new();
    let Some(list) = messages.as_array() else {
        return entries;
    };
    for message in list {
        // Parts must be seen before the message info so completed messages
        // flush fully populated text buffers.
        if let Some(parts) = message.get("parts").and_then(Value::as_array) {
            for part in parts {
                let props = serde_json::json!({ "part": part });
                on_part_updated(state, &props, &mut entries);
            }
        }
        if let Some(info) = message.get("info") {
            let props = serde_json::json!({ "info": info });
            on_message_updated(state, &props, &mut entries);
        }
    }
    // Replayed history must not leave the session looking active.
    state.stats.state = SessionState::Ready;
    state.stats.idle_since = None;
    state.stats.last_updated = Some(now_secs());
    entries
}

fn on_permission_asked(state: &mut OpencodeState, props: &Value) {
    if !state.is_main_event(props) {
        return;
    }
    let tool = props
        .get("permission")
        .or_else(|| props.get("type"))
        .and_then(Value::as_str)
        .unwrap_or("tool")
        .to_string();
    let input = props.get("metadata").cloned().unwrap_or(Value::Null);
    state.stats.permission = Some(PermissionDetails {
        tool: tool.clone(),
        input: input.clone(),
        suggestions: Vec::new(),
    });
    state.stats.active_prompt = Some(ActivePrompt::Permission {
        tool_name: tool,
        tool_input: Some(input),
    });
    state.set_state(SessionState::Permission);
    state.record_history("permission.asked");
}

fn agent_mode(agent: &str) -> ClaudeMode {
    if agent.eq_ignore_ascii_case("plan") {
        ClaudeMode::Plan
    } else {
        ClaudeMode::Normal
    }
}

/// Best-effort preview of a tool's input when the server gave no title.
fn tool_input_preview(part_state: &Value) -> String {
    let Some(input) = part_state.get("input").and_then(Value::as_object) else {
        return String::new();
    };
    for key in ["command", "filePath", "file_path", "path", "pattern", "url"] {
        if let Some(text) = input.get(key).and_then(Value::as_str) {
            return truncate_chars(text, 120);
        }
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn adopted_state() -> OpencodeState {
        let mut state = OpencodeState::default();
        apply_event(
            &mut state,
            &json!({
                "type": "session.created",
                "properties": {"info": {"id": "ses_1", "directory": "/work"}}
            }),
        );
        assert_eq!(state.main_session.as_deref(), Some("ses_1"));
        state
    }

    fn user_message(state: &mut OpencodeState, id: &str, text: &str) -> Vec<TranscriptEntry> {
        let mut entries = apply_event(
            state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {"id": id, "sessionID": "ses_1", "role": "user"}}
            }),
        );
        entries.extend(apply_event(
            state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": format!("{id}_p"), "messageID": id, "sessionID": "ses_1",
                    "type": "text", "text": text
                }}
            }),
        ));
        entries
    }

    #[test]
    fn user_message_follows_in_pane_session_switch() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "session.created",
                "properties": {"info": {"id": "ses_2"}}
            }),
        );
        // Creating a second session does not switch tracking by itself
        assert_eq!(state.main_session.as_deref(), Some("ses_1"));

        // A user message on the new session means the pane switched to it
        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {"id": "msg_s2", "sessionID": "ses_2", "role": "user"}}
            }),
        );
        assert_eq!(state.main_session.as_deref(), Some("ses_2"));
    }

    #[test]
    fn backfill_replays_history_and_resets_state() {
        let mut state = OpencodeState {
            main_session: Some("ses_1".to_string()),
            ..Default::default()
        };
        let messages = json!([
            {
                "info": {"id": "msg_u1", "sessionID": "ses_1", "role": "user"},
                "parts": [{"id": "prt_u1", "messageID": "msg_u1", "sessionID": "ses_1",
                           "type": "text", "text": "review the PR"}]
            },
            {
                "info": {"id": "msg_a1", "sessionID": "ses_1", "role": "assistant",
                         "time": {"created": 1, "completed": 2}, "finish": "stop"},
                "parts": [
                    {"id": "prt_t1", "messageID": "msg_a1", "sessionID": "ses_1",
                     "type": "tool", "tool": "bash",
                     "state": {"status": "completed", "title": "gh pr view 7", "output": "ok"}},
                    {"id": "prt_a1", "messageID": "msg_a1", "sessionID": "ses_1",
                     "type": "text", "text": "looks good"}
                ]
            }
        ]);
        let entries = backfill_messages(&mut state, &messages);
        assert_eq!(state.stats.prompts, 1);
        assert_eq!(state.stats.completions, 1);
        assert_eq!(state.stats.tools.get("bash"), Some(&1));
        assert_eq!(state.stats.state, SessionState::Ready);
        assert_eq!(entries.len(), 3);
        assert!(matches!(&entries[0], TranscriptEntry::User { text } if text == "review the PR"));
        assert!(matches!(&entries[2], TranscriptEntry::Assistant { text } if text == "looks good"));
    }

    #[test]
    fn deltas_extend_lagging_snapshots() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {"id": "msg_a1", "sessionID": "ses_1", "role": "assistant"}}
            }),
        );
        apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "text", "text": "hello"
                }}
            }),
        );
        apply_event(
            &mut state,
            &json!({
                "type": "message.part.delta",
                "properties": {"sessionID": "ses_1", "messageID": "msg_a1", "partID": "prt_1",
                               "field": "text", "delta": " world"}
            }),
        );
        // A stale (shorter) snapshot must not clobber the delta-extended text
        apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "text", "text": "hello"
                }}
            }),
        );
        let entries = apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_a1", "sessionID": "ses_1", "role": "assistant",
                    "time": {"created": 1, "completed": 2}
                }}
            }),
        );
        assert!(
            matches!(&entries[..], [TranscriptEntry::Assistant { text }] if text == "hello world")
        );
    }

    #[test]
    fn child_sessions_are_never_adopted_and_count_subagents() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "session.created",
                "properties": {"info": {"id": "ses_child", "parentID": "ses_1"}}
            }),
        );
        assert_eq!(state.main_session.as_deref(), Some("ses_1"));

        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_c1", "sessionID": "ses_child", "role": "assistant",
                    "time": {"created": 1, "completed": 2}
                }}
            }),
        );
        assert_eq!(state.stats.subagent_messages, 1);
        assert_eq!(state.stats.completions, 0);
    }

    #[test]
    fn prompt_starts_thinking_and_writes_user_entry() {
        let mut state = adopted_state();
        let entries = user_message(&mut state, "msg_u1", "hello there");
        assert_eq!(state.stats.prompts, 1);
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(matches!(&entries[..], [TranscriptEntry::User { text }] if text == "hello there"));

        // Repeated message.updated events do not double count
        user_message(&mut state, "msg_u1", "hello there");
        assert_eq!(state.stats.prompts, 1);
    }

    #[test]
    fn assistant_text_flushes_on_completion_only() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {"id": "msg_a1", "sessionID": "ses_1", "role": "assistant"}}
            }),
        );
        let entries = apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "text", "text": "partial"
                }}
            }),
        );
        assert!(entries.is_empty());

        // Snapshot grows, then the message completes
        apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "text", "text": "partial then full"
                }}
            }),
        );
        let entries = apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_a1", "sessionID": "ses_1", "role": "assistant",
                    "time": {"created": 1, "completed": 2}, "finish": "stop"
                }}
            }),
        );
        assert_eq!(state.stats.completions, 1);
        assert!(
            matches!(&entries[..], [TranscriptEntry::Assistant { text }] if text == "partial then full")
        );
    }

    #[test]
    fn tool_completion_counts_and_writes_entry() {
        let mut state = adopted_state();
        let entries = apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_t1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "tool", "tool": "bash",
                    "state": {
                        "status": "completed",
                        "title": "git log --oneline",
                        "input": {"command": "git log --oneline"},
                        "output": "b7cc90d init\n",
                        "time": {"start": 1000.0, "end": 2000.0}
                    }
                }}
            }),
        );
        assert_eq!(state.stats.tools.get("bash"), Some(&1));
        assert_eq!(state.stats.tool_timestamps, vec![2.0]);
        assert!(matches!(
            &entries[..],
            [TranscriptEntry::Tool { name, title, .. }]
                if name == "bash" && title == "git log --oneline"
        ));

        // A second update for the same part does not double count
        let entries = apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_t1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "tool", "tool": "bash",
                    "state": {"status": "completed", "output": ""}
                }}
            }),
        );
        assert!(entries.is_empty());
        assert_eq!(state.stats.tools.get("bash"), Some(&1));
    }

    #[test]
    fn permission_lifecycle() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "permission.asked",
                "properties": {
                    "id": "per_1", "sessionID": "ses_1", "permission": "bash",
                    "patterns": ["git push"], "metadata": {"command": "git push"}
                }
            }),
        );
        assert_eq!(state.stats.state, SessionState::Permission);
        assert_eq!(
            state.stats.permission.as_ref().map(|p| p.tool.as_str()),
            Some("bash")
        );

        // busy status noise must not clear the prompt
        apply_event(
            &mut state,
            &json!({
                "type": "session.status",
                "properties": {"sessionID": "ses_1", "status": {"type": "busy"}}
            }),
        );
        assert_eq!(state.stats.state, SessionState::Permission);

        apply_event(
            &mut state,
            &json!({
                "type": "permission.replied",
                "properties": {"sessionID": "ses_1", "permissionID": "per_1", "response": "once"}
            }),
        );
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.permission.is_none());
    }

    #[test]
    fn abort_error_marks_interrupted_and_flushes_partial_text() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {"id": "msg_a1", "sessionID": "ses_1", "role": "assistant"}}
            }),
        );
        apply_event(
            &mut state,
            &json!({
                "type": "message.part.updated",
                "properties": {"part": {
                    "id": "prt_1", "messageID": "msg_a1", "sessionID": "ses_1",
                    "type": "text", "text": "partial answer"
                }}
            }),
        );
        let entries = apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_a1", "sessionID": "ses_1", "role": "assistant",
                    "error": {"name": "MessageAbortedError", "data": {"message": "Aborted"}}
                }}
            }),
        );
        assert!(
            matches!(&entries[..], [TranscriptEntry::Assistant { text }] if text == "partial answer")
        );
        assert_eq!(state.stats.completions, 0);

        apply_event(
            &mut state,
            &json!({
                "type": "session.error",
                "properties": {"sessionID": "ses_1", "error": {"name": "MessageAbortedError"}}
            }),
        );
        assert_eq!(state.stats.state, SessionState::Interrupted);

        // The idle events that follow an abort must not hide the interrupt
        apply_event(
            &mut state,
            &json!({
                "type": "session.status",
                "properties": {"sessionID": "ses_1", "status": {"type": "idle"}}
            }),
        );
        apply_event(
            &mut state,
            &json!({"type": "session.idle", "properties": {"sessionID": "ses_1"}}),
        );
        assert_eq!(state.stats.state, SessionState::Interrupted);

        // The next prompt clears it
        user_message(&mut state, "msg_u2", "try again");
        assert_eq!(state.stats.state, SessionState::Thinking);
    }

    #[test]
    fn idle_completes_the_turn() {
        let mut state = adopted_state();
        user_message(&mut state, "msg_u1", "hi");
        apply_event(
            &mut state,
            &json!({"type": "session.idle", "properties": {"sessionID": "ses_1"}}),
        );
        assert_eq!(state.stats.state, SessionState::Complete);
        assert!(state.stats.idle_since.is_some());
    }

    #[test]
    fn model_uses_catalog_name_with_id_fallback() {
        let mut state = adopted_state();
        state
            .model_names
            .insert("stealth/ox-alpha".to_string(), "Ox Alpha".to_string());
        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_a1", "sessionID": "ses_1", "role": "assistant",
                    "modelID": "stealth/ox-alpha", "providerID": "openrouter"
                }}
            }),
        );
        assert_eq!(state.stats.model.as_deref(), Some("Ox Alpha"));

        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_a2", "sessionID": "ses_1", "role": "assistant",
                    "modelID": "anthropic/claude-sonnet-5"
                }}
            }),
        );
        assert_eq!(state.stats.model.as_deref(), Some("claude-sonnet-5"));
    }

    #[test]
    fn plan_agent_maps_to_plan_mode() {
        let mut state = adopted_state();
        apply_event(
            &mut state,
            &json!({
                "type": "message.updated",
                "properties": {"info": {
                    "id": "msg_a1", "sessionID": "ses_1", "role": "assistant", "agent": "plan"
                }}
            }),
        );
        assert_eq!(state.stats.mode, ClaudeMode::Plan);
    }
}
