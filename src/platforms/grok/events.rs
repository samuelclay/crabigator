//! Grok session-log processing.
//!
//! Translates `events.jsonl` and ACP `updates.jsonl` lines into crabigator's
//! [`PlatformStats`]. Parsing is pure so it can be unit-tested without a live
//! TUI.

use std::collections::HashSet;
use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;

use super::acp;
use crate::platforms::{
    ActivePrompt, ClaudeMode, HookEvent, PermissionDetails, PlatformStats, Question,
    QuestionOption, SessionState,
};

const EVENT_HISTORY_LIMIT: usize = 50;

/// State derived from a Grok session's log files.
#[derive(Default)]
pub struct EventState {
    pub stats: PlatformStats,
    counted_prompts: HashSet<String>,
    counted_completions: HashSet<String>,
    counted_tools: HashSet<String>,
    pending_permission: Option<String>,
    pending_call_id: Option<String>,
    current_turn_key: Option<String>,
    current_is_primary: bool,
    last_mode_ts: f64,
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn event_ts(value: &Value) -> f64 {
    if let Some(ts) = value.get("ts").and_then(Value::as_str) {
        if let Ok(parsed) = chrono::DateTime::parse_from_rfc3339(ts) {
            return parsed.timestamp() as f64
                + f64::from(parsed.timestamp_subsec_millis()) / 1000.0;
        }
    }
    if let Some(ts) = value.get("timestamp").and_then(Value::as_f64) {
        return ts;
    }
    now_secs()
}

impl EventState {
    fn set_state(&mut self, state: SessionState) {
        if self.stats.state == state {
            return;
        }
        self.stats.state = state;
        if matches!(state, SessionState::Complete) {
            self.stats.idle_since = Some(now_secs());
        } else {
            self.stats.idle_since = None;
        }
    }

    fn clear_permission(&mut self) {
        self.pending_permission = None;
        if matches!(
            self.stats.active_prompt,
            Some(ActivePrompt::Permission { .. })
        ) {
            self.stats.active_prompt = None;
        }
        self.stats.permission = None;
    }

    fn clear_interactive(&mut self) {
        self.pending_call_id = None;
        self.stats.active_prompt = None;
        self.clear_permission();
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

    /// Apply one JSON object from either log. `events.jsonl` rows have a
    /// top-level `type`; ACP rows carry `params.update.sessionUpdate`.
    pub fn apply_line(&mut self, value: &Value) {
        if value.get("sessionUpdate").is_some() {
            self.apply_update(value, event_ts(value));
        } else if let Some(update) = value.pointer("/params/update") {
            self.apply_update(update, event_ts(value));
        } else if value.get("type").and_then(Value::as_str).is_some() {
            self.apply_event(value);
        }
    }

    fn apply_event(&mut self, event: &Value) {
        let Some(event_type) = event.get("type").and_then(Value::as_str) else {
            return;
        };
        match event_type {
            "turn_started" => self.on_turn_started(event),
            "turn_ended" => self.on_turn_ended(event),
            "phase_changed" => self.on_phase_changed(event),
            "permission_requested" => self.on_permission_requested(event),
            "permission_resolved" => self.on_permission_resolved(event),
            "tool_completed" => self.on_tool_completed(event),
            "yolo_toggled" => self.on_yolo_toggled(event),
            "interjected" => {
                self.clear_interactive();
                self.set_state(SessionState::Interrupted);
                self.record_history("interjected");
            }
            _ => {}
        }
        self.stats.last_updated = Some(event_ts(event));
    }

    fn on_turn_started(&mut self, event: &Value) {
        let session_id = event
            .get("session_id")
            .and_then(Value::as_str)
            .unwrap_or("");
        let turn_number = event
            .get("turn_number")
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let key = format!("{session_id}:{turn_number}");
        let primary = event
            .get("session_relationship")
            .and_then(Value::as_str)
            .unwrap_or("primary")
            == "primary";
        self.current_turn_key = Some(key.clone());
        self.current_is_primary = primary;

        if let Some(model) = event.get("model_id").and_then(Value::as_str) {
            self.stats.model = Some(model.to_string());
        }
        if event.get("yolo_mode").and_then(Value::as_bool) == Some(true) {
            self.stats.mode = ClaudeMode::AutoAccept;
        }

        self.clear_interactive();
        if primary {
            if self.counted_prompts.insert(key) {
                self.stats.prompts += 1;
            }
            self.set_state(SessionState::Thinking);
            self.record_history("turn_started");
        }
    }

    fn on_turn_ended(&mut self, event: &Value) {
        let outcome = event.get("outcome").and_then(Value::as_str).unwrap_or("");
        let cancelled = outcome == "cancelled" || event.get("cancellation_category").is_some();
        self.clear_interactive();

        if cancelled {
            self.set_state(SessionState::Interrupted);
            self.record_history("turn_ended:cancelled");
            return;
        }

        if let Some(key) = self.current_turn_key.clone() {
            if self.current_is_primary {
                if self.counted_completions.insert(key) {
                    self.stats.completions += 1;
                }
                self.set_state(SessionState::Complete);
            } else if self.counted_completions.insert(format!("subagent:{key}")) {
                self.stats.subagent_messages += 1;
            }
        } else if self.current_is_primary {
            self.set_state(SessionState::Complete);
        }
        self.record_history("turn_ended");
    }

    fn on_phase_changed(&mut self, event: &Value) {
        let Some(phase) = event.get("phase").and_then(Value::as_str) else {
            return;
        };
        match phase {
            "permission_prompt" if self.pending_permission.is_some() => {
                self.set_permission_state();
            }
            "waiting_for_model" | "streaming_reasoning" | "streaming_text" | "tool_execution"
                if self.pending_permission.is_none()
                    && self.pending_call_id.is_none()
                    && self.stats.state != SessionState::Interrupted =>
            {
                self.set_state(SessionState::Thinking);
            }
            _ => {}
        }
    }

    fn on_permission_requested(&mut self, event: &Value) {
        let tool = event
            .get("tool_name")
            .and_then(Value::as_str)
            .unwrap_or("tool")
            .to_string();
        self.pending_permission = Some(tool);
        self.set_permission_state();
        self.record_history("permission_requested");
    }

    fn set_permission_state(&mut self) {
        let tool = self
            .pending_permission
            .as_deref()
            .unwrap_or("tool")
            .to_string();
        self.stats.permission = Some(PermissionDetails {
            tool: tool.clone(),
            input: Value::Null,
            suggestions: Vec::new(),
        });
        self.stats.active_prompt = Some(ActivePrompt::Permission {
            tool_name: tool,
            tool_input: None,
        });
        self.set_state(SessionState::Permission);
    }

    fn on_permission_resolved(&mut self, event: &Value) {
        let had_prompt = self.pending_permission.is_some()
            || matches!(self.stats.state, SessionState::Permission);
        self.clear_permission();
        if had_prompt && self.stats.state == SessionState::Permission {
            self.set_state(SessionState::Thinking);
        }
        let wait_ms = event.get("wait_ms").and_then(Value::as_u64).unwrap_or(0);
        self.record_history(&format!("permission_resolved:{wait_ms}"));
    }

    fn on_tool_completed(&mut self, event: &Value) {
        let tool = event
            .get("tool_name")
            .and_then(Value::as_str)
            .unwrap_or("tool");
        let call_id = event
            .get("tool_call_id")
            .and_then(Value::as_str)
            .unwrap_or(tool);
        if !self.counted_tools.insert(call_id.to_string()) {
            return;
        }
        *self.stats.tools.entry(tool.to_string()).or_insert(0) += 1;
        self.stats.tool_timestamps.push(event_ts(event));
    }

    fn on_yolo_toggled(&mut self, event: &Value) {
        let enabled = event
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let mode = if enabled {
            ClaudeMode::AutoAccept
        } else if self.stats.mode == ClaudeMode::Plan {
            ClaudeMode::Plan
        } else {
            ClaudeMode::Normal
        };
        self.set_mode(mode, event_ts(event));
        self.record_history(if enabled {
            "yolo_toggled:on"
        } else {
            "yolo_toggled:off"
        });
    }

    fn set_mode(&mut self, mode: ClaudeMode, ts: f64) {
        if ts + 0.001 < self.last_mode_ts {
            return;
        }
        self.last_mode_ts = ts;
        self.stats.mode = mode;
    }

    fn apply_update(&mut self, update: &Value, ts: f64) {
        let Some(kind) = update.get("sessionUpdate").and_then(Value::as_str) else {
            return;
        };
        match kind {
            "current_mode_update" => self.on_mode_update(update, ts),
            "tool_call" => self.on_tool_call(update),
            "tool_call_update" => self.on_tool_call_update(update),
            "subagent_finished" => {
                self.stats.subagent_messages += 1;
                self.record_history("subagent_finished");
            }
            "auto_compact_completed" | "compaction_checkpoint" => {
                self.stats.compressions += 1;
                self.record_history("compacted");
            }
            "turn_completed" => {
                let stop = update
                    .get("stop_reason")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if stop == "cancelled" {
                    self.clear_interactive();
                    self.set_state(SessionState::Interrupted);
                } else if self.stats.state != SessionState::Interrupted
                    && self.pending_permission.is_none()
                    && self.pending_call_id.is_none()
                {
                    if self.current_is_primary {
                        if let Some(key) = self.current_turn_key.clone() {
                            if self.counted_completions.insert(key) {
                                self.stats.completions += 1;
                            }
                        }
                    }
                    self.set_state(SessionState::Complete);
                }
            }
            _ => {}
        }
        self.stats.last_updated = Some(now_secs());
    }

    fn on_mode_update(&mut self, update: &Value, ts: f64) {
        let Some(mode_id) = update.get("currentModeId").and_then(Value::as_str) else {
            return;
        };
        let mode = match mode_id {
            "plan" => ClaudeMode::Plan,
            "acceptEdits" | "auto" | "bypassPermissions" | "always-approve" | "yolo" => {
                ClaudeMode::AutoAccept
            }
            _ => ClaudeMode::Normal,
        };
        self.set_mode(mode, ts);
    }

    fn on_tool_call(&mut self, update: &Value) {
        let name = acp::tool_name(update);
        let call_id = update
            .get("toolCallId")
            .and_then(Value::as_str)
            .map(str::to_string);
        let lower = name.to_ascii_lowercase();
        if is_ask_user_question(&lower) {
            if let Some(questions) = parse_questions(update) {
                self.set_question(
                    call_id,
                    ActivePrompt::Question { questions },
                    "ask_user_question",
                );
            }
            return;
        }
        if is_exit_plan_mode(&lower) {
            self.set_question(call_id, ActivePrompt::ExitPlan, "exit_plan_mode");
            return;
        }
        if is_elicit(&lower) {
            let question = Question {
                question: name.to_string(),
                header: Some("MCP".to_string()),
                options: vec![
                    QuestionOption {
                        label: "Accept".to_string(),
                        description: None,
                    },
                    QuestionOption {
                        label: "Decline".to_string(),
                        description: None,
                    },
                ],
                multi_select: false,
            };
            self.set_question(
                call_id,
                ActivePrompt::Question {
                    questions: vec![question],
                },
                "mcp_elicit",
            );
        }
    }

    fn set_question(&mut self, call_id: Option<String>, prompt: ActivePrompt, event: &str) {
        self.pending_call_id = call_id;
        self.stats.active_prompt = Some(prompt);
        self.set_state(SessionState::Question);
        self.record_history(event);
    }

    fn on_tool_call_update(&mut self, update: &Value) {
        let status = update.get("status").and_then(Value::as_str).unwrap_or("");
        if !matches!(status, "completed" | "failed" | "cancelled" | "error") {
            return;
        }
        let Some(call_id) = update.get("toolCallId").and_then(Value::as_str) else {
            return;
        };
        if self.pending_call_id.as_deref() == Some(call_id) {
            self.pending_call_id = None;
            self.stats.active_prompt = None;
            if self.stats.state == SessionState::Question {
                self.set_state(SessionState::Thinking);
            }
        }
    }
}

fn is_ask_user_question(lower: &str) -> bool {
    lower == "ask_user_question" || lower == "askuserquestion"
}

fn is_exit_plan_mode(lower: &str) -> bool {
    lower == "exit_plan_mode" || lower == "exitplanmode"
}

fn is_elicit(lower: &str) -> bool {
    lower.contains("elicit")
}

fn parse_questions(update: &Value) -> Option<Vec<Question>> {
    let questions = update.pointer("/rawInput/questions")?.as_array()?;
    let parsed: Vec<Question> = questions
        .iter()
        .filter_map(|q| {
            let question = q
                .get("question")
                .and_then(Value::as_str)
                .filter(|s| !s.is_empty())?
                .to_string();
            let header = q.get("header").and_then(Value::as_str).map(str::to_string);
            let options = q
                .get("options")
                .and_then(Value::as_array)
                .map(|opts| {
                    opts.iter()
                        .filter_map(|opt| {
                            let label = opt
                                .get("label")
                                .and_then(Value::as_str)
                                .filter(|s| !s.is_empty())?
                                .to_string();
                            Some(QuestionOption {
                                label,
                                description: opt
                                    .get("description")
                                    .and_then(Value::as_str)
                                    .map(str::to_string),
                            })
                        })
                        .collect()
                })
                .unwrap_or_default();
            Some(Question {
                question,
                header,
                options,
                multi_select: q
                    .get("multiSelect")
                    .or_else(|| q.get("multi_select"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false),
            })
        })
        .collect();
    (!parsed.is_empty()).then_some(parsed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn apply_all(state: &mut EventState, lines: &[Value]) {
        for line in lines {
            state.apply_line(line);
        }
    }

    #[test]
    fn prompt_thinking_completion() {
        let mut state = EventState::default();
        apply_all(
            &mut state,
            &[
                json!({
                    "type": "turn_started",
                    "session_id": "abc",
                    "turn_number": 0,
                    "model_id": "grok-4.6",
                    "session_relationship": "primary"
                }),
                json!({"type": "phase_changed", "phase": "waiting_for_model"}),
                json!({"type": "phase_changed", "phase": "streaming_text"}),
                json!({"type": "turn_ended", "outcome": "completed"}),
            ],
        );
        assert_eq!(state.stats.prompts, 1);
        assert_eq!(state.stats.completions, 1);
        assert_eq!(state.stats.state, SessionState::Complete);
        assert_eq!(state.stats.model.as_deref(), Some("grok-4.6"));
    }

    #[test]
    fn yolo_permission_does_not_stick() {
        let mut state = EventState::default();
        apply_all(
            &mut state,
            &[
                json!({
                    "type": "turn_started",
                    "session_id": "abc",
                    "turn_number": 0,
                    "session_relationship": "primary",
                    "yolo_mode": true
                }),
                json!({"type": "permission_requested", "tool_name": "todo_write"}),
                json!({
                    "type": "permission_resolved",
                    "tool_name": "todo_write",
                    "decision": "allow",
                    "wait_ms": 0
                }),
                json!({"type": "phase_changed", "phase": "permission_prompt"}),
                json!({"type": "phase_changed", "phase": "tool_execution"}),
            ],
        );
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.permission.is_none());
        assert_eq!(state.stats.mode, ClaudeMode::AutoAccept);
    }

    #[test]
    fn real_permission_prompt_stays_until_resolved() {
        let mut state = EventState::default();
        apply_all(
            &mut state,
            &[
                json!({
                    "type": "turn_started",
                    "session_id": "abc",
                    "turn_number": 0,
                    "session_relationship": "primary"
                }),
                json!({"type": "permission_requested", "tool_name": "run_terminal_command"}),
                json!({"type": "phase_changed", "phase": "permission_prompt"}),
            ],
        );
        assert_eq!(state.stats.state, SessionState::Permission);
        match &state.stats.active_prompt {
            Some(ActivePrompt::Permission { tool_name, .. }) => {
                assert_eq!(tool_name, "run_terminal_command");
            }
            other => panic!("expected permission, got {other:?}"),
        }

        state.apply_line(&json!({
            "type": "permission_resolved",
            "tool_name": "run_terminal_command",
            "decision": "allow",
            "wait_ms": 8029
        }));
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.active_prompt.is_none());
    }

    #[test]
    fn ask_user_question_becomes_active_prompt() {
        let mut state = EventState::default();
        state.apply_line(&json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "call-1",
            "title": "ask_user_question",
            "rawInput": {
                "questions": [{
                    "question": "Wrap how?",
                    "options": [
                        {"label": "Fullscreen", "description": "Like opencode"},
                        {"label": "Minimal"}
                    ]
                }]
            },
            "_meta": {"x.ai/tool": {"name": "ask_user_question"}}
        }));
        assert_eq!(state.stats.state, SessionState::Question);
        match &state.stats.active_prompt {
            Some(ActivePrompt::Question { questions }) => {
                assert_eq!(questions.len(), 1);
                assert_eq!(questions[0].question, "Wrap how?");
                assert_eq!(questions[0].options[0].label, "Fullscreen");
            }
            other => panic!("expected question, got {other:?}"),
        }

        state.apply_line(&json!({
            "sessionUpdate": "tool_call_update",
            "toolCallId": "call-1",
            "status": "completed"
        }));
        assert_eq!(state.stats.state, SessionState::Thinking);
        assert!(state.stats.active_prompt.is_none());
    }

    #[test]
    fn cancelled_turn_is_interrupted() {
        let mut state = EventState::default();
        apply_all(
            &mut state,
            &[
                json!({
                    "type": "turn_started",
                    "session_id": "abc",
                    "turn_number": 0,
                    "session_relationship": "primary"
                }),
                json!({
                    "type": "turn_ended",
                    "outcome": "cancelled",
                    "cancellation_category": "mid_turn_abort"
                }),
            ],
        );
        assert_eq!(state.stats.state, SessionState::Interrupted);
        assert_eq!(state.stats.completions, 0);
    }

    #[test]
    fn mode_updates_and_tools_and_compaction() {
        let mut state = EventState::default();
        apply_all(
            &mut state,
            &[
                json!({"sessionUpdate": "current_mode_update", "currentModeId": "plan"}),
                json!({
                    "type": "tool_completed",
                    "tool_name": "read_file",
                    "tool_call_id": "call-1",
                    "ts": "2026-08-31T22:56:53.022Z"
                }),
                json!({"sessionUpdate": "auto_compact_completed", "tokens_before": 10, "tokens_after": 2}),
                json!({"sessionUpdate": "current_mode_update", "currentModeId": "default"}),
            ],
        );
        assert_eq!(state.stats.mode, ClaudeMode::Normal);
        assert_eq!(state.stats.tools.get("read_file").copied(), Some(1));
        assert_eq!(state.stats.tool_timestamps.len(), 1);
        assert_eq!(state.stats.compressions, 1);
    }

    #[test]
    fn newer_yolo_toggle_wins_over_older_plan_mode() {
        let mut state = EventState::default();
        state.apply_line(&json!({
            "timestamp": 100.0,
            "params": {
                "update": {
                    "sessionUpdate": "current_mode_update",
                    "currentModeId": "plan"
                }
            }
        }));
        assert_eq!(state.stats.mode, ClaudeMode::Plan);
        state.apply_line(&json!({
            "type": "yolo_toggled",
            "enabled": true,
            "ts": "2026-08-31T23:39:50.107Z"
        }));
        assert_eq!(state.stats.mode, ClaudeMode::AutoAccept);
        state.apply_line(&json!({
            "timestamp": 50.0,
            "params": {
                "update": {
                    "sessionUpdate": "current_mode_update",
                    "currentModeId": "plan"
                }
            }
        }));
        assert_eq!(state.stats.mode, ClaudeMode::AutoAccept);
    }

    #[test]
    fn exit_plan_mode_is_exit_plan_prompt() {
        let mut state = EventState::default();
        state.apply_line(&json!({
            "sessionUpdate": "tool_call",
            "toolCallId": "call-plan",
            "title": "exit_plan_mode",
            "_meta": {"x.ai/tool": {"name": "exit_plan_mode"}}
        }));
        assert!(matches!(
            state.stats.active_prompt,
            Some(ActivePrompt::ExitPlan)
        ));
        assert_eq!(state.stats.state, SessionState::Question);
    }

    #[test]
    fn acp_envelope_is_unwrapped() {
        let mut state = EventState::default();
        state.apply_line(&json!({
            "method": "session/update",
            "params": {
                "update": {
                    "sessionUpdate": "current_mode_update",
                    "currentModeId": "plan"
                }
            }
        }));
        assert_eq!(state.stats.mode, ClaudeMode::Plan);
    }

    #[test]
    fn duplicate_tool_ids_are_not_double_counted() {
        let mut state = EventState::default();
        let event = json!({
            "type": "tool_completed",
            "tool_name": "grep",
            "tool_call_id": "call-dup"
        });
        state.apply_line(&event);
        state.apply_line(&event);
        assert_eq!(state.stats.tools.get("grep").copied(), Some(1));
    }
}
