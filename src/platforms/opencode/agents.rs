//! Native child-session activity, kept separate from the main session's UI state.
use std::collections::HashMap;

use serde_json::Value;

use crate::platforms::{PlatformAgent, PlatformAgentState};

#[derive(Default)]
struct Child {
    parent: String,
    kind: String,
    task: String,
    completed: bool,
}

struct Run {
    id: String,
    parent: String,
    kind: String,
    task: String,
    started: f64,
    heard: f64,
    ended: Option<f64>,
    ok: bool,
    tools: HashMap<String, String>,
    waits: HashMap<String, String>,
}

#[derive(Default)]
pub(super) struct AgentRoster {
    root: Option<String>,
    children: HashMap<String, Child>,
    runs: Vec<Run>,
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

impl AgentRoster {
    fn show(&mut self, root: Option<&str>) {
        if self.root.as_deref() != root {
            self.root = root.map(str::to_owned);
            self.runs.clear();
        }
    }

    /// Nested agents contribute to their direct ancestor's companion.
    fn companion(&self, session: &str) -> Option<String> {
        let root = self.root.as_deref()?;
        let mut id = session;
        for _ in 0..8 {
            let child = self.children.get(id)?;
            if child.parent == root {
                return Some(id.to_owned());
            }
            id = &child.parent;
        }
        None
    }

    fn hear(&mut self, id: &str, now: f64, restart: bool) -> Option<&mut Run> {
        let child = self.children.get_mut(id)?;
        if restart {
            child.completed = false;
        }
        if child.completed {
            return None;
        }
        let index = if let Some(index) = self.runs.iter().position(|run| run.id == id) {
            if restart && self.runs[index].ended.is_some() {
                self.runs.remove(index);
                self.runs.len()
            } else {
                index
            }
        } else {
            self.runs.len()
        };
        if index == self.runs.len() {
            self.runs.push(Run {
                id: id.to_owned(),
                parent: child.parent.clone(),
                kind: child.kind.clone(),
                task: child.task.clone(),
                started: now,
                heard: now,
                ended: None,
                ok: true,
                tools: HashMap::new(),
                waits: HashMap::new(),
            });
        }
        let run = &mut self.runs[index];
        run.heard = now;
        Some(run)
    }

    fn finish(&mut self, id: &str, now: f64, ok: bool) {
        if let Some(child) = self.children.get_mut(id) {
            child.completed = true;
        }
        if let Some(run) = self.runs.iter_mut().find(|run| run.id == id) {
            // Repeated idle/completion messages must not extend the departure.
            if run.ended.is_none() {
                run.ended = Some(now);
                run.ok = ok;
                run.tools.clear();
                run.waits.clear();
            }
        }
    }

    fn end_loop(&mut self, session: &str) {
        for run in &mut self.runs {
            run.tools.retain(|_, origin| origin != session);
            run.waits.retain(|_, origin| origin != session);
        }
    }

    pub(super) fn owns_reply(&self, props: &Value) -> bool {
        let id = string(props, "requestID")
            .or_else(|| string(props, "permissionID"))
            .or_else(|| string(props, "id"));
        id.is_some_and(|id| self.runs.iter().any(|run| run.waits.contains_key(id)))
    }

    pub(super) fn event(&mut self, root: Option<&str>, event: &str, props: &Value, now: f64) {
        self.show(root);
        let data = props
            .get("info")
            .or_else(|| props.get("part"))
            .or_else(|| props.get("form"))
            .unwrap_or(props);
        let sid = string(data, "sessionID").or_else(|| string(props, "sessionID"));
        if matches!(event, "session.created" | "session.updated") {
            if let (Some(id), Some(parent)) = (string(data, "id").or(sid), string(data, "parentID"))
            {
                let child = self.children.entry(id.to_owned()).or_default();
                child.parent = parent.to_owned();
                if let Some(kind) = string(data, "agent") {
                    child.kind = kind.to_owned();
                }
                if let Some(task) = string(data, "title") {
                    child.task = task.to_owned();
                }
                if event == "session.created" && root == Some(parent) {
                    self.hear(id, now, true);
                }
            }
            return;
        }
        if event == "session.renamed" {
            if let (Some(id), Some(title)) = (sid, string(data, "title")) {
                if let Some(child) = self.children.get_mut(id) {
                    child.task = title.to_owned();
                }
            }
            return;
        }
        if event == "session.deleted" {
            if let Some(id) = sid.or_else(|| string(data, "id")) {
                self.end_loop(id);
                self.finish(id, now, false);
                self.children.remove(id);
            }
            return;
        }
        // Replies may omit sessionID. Only their own matching request is cleared.
        if matches!(
            event,
            "permission.replied"
                | "question.replied"
                | "question.rejected"
                | "form.replied"
                | "form.cancelled"
        ) {
            if let Some(id) = string(data, "requestID")
                .or_else(|| string(data, "permissionID"))
                .or_else(|| string(data, "id"))
            {
                for run in &mut self.runs {
                    if run.waits.remove(id).is_some() {
                        run.heard = now;
                    }
                }
            }
            return;
        }
        let Some(sid) = sid else {
            return;
        };
        let Some(id) = self.companion(sid) else {
            return;
        };
        let direct = id == sid;
        let status = data.get("status").and_then(|status| string(status, "type"));
        let finish = match event {
            "session.execution.succeeded" | "session.idle" => Some(true),
            "session.execution.failed" | "session.execution.interrupted" | "session.error" => {
                Some(false)
            }
            "session.status" if status == Some("idle") => Some(true),
            "message.updated" if string(data, "role") == Some("assistant") => {
                let error = data.get("error").is_some_and(|error| !error.is_null());
                let complete = data
                    .get("time")
                    .and_then(|time| time.get("completed"))
                    .is_some_and(|v| !v.is_null());
                if error {
                    Some(false)
                } else if complete && string(data, "finish") != Some("tool-calls") {
                    Some(true)
                } else {
                    None
                }
            }
            _ => None,
        };
        if let Some(ok) = finish {
            self.end_loop(sid);
            if direct {
                self.finish(&id, now, ok);
            }
            return;
        }
        let restart = direct
            && (event == "session.execution.started"
                || event == "session.status" && matches!(status, Some("busy" | "retry"))
                || event == "message.updated" && string(data, "role") == Some("user"));
        let active = restart
            || matches!(
                event,
                "message.updated"
                    | "message.part.updated"
                    | "message.part.delta"
                    | "session.step.started"
                    | "session.text.delta"
                    | "session.reasoning.delta"
                    | "session.tool.input.started"
                    | "session.tool.called"
                    | "session.tool.success"
                    | "session.tool.failed"
                    | "permission.asked"
                    | "permission.updated"
                    | "question.asked"
                    | "form.created"
            );
        if !active {
            return;
        }
        let Some(run) = self.hear(&id, now, restart) else {
            return;
        };
        match event {
            "permission.asked" | "permission.updated" | "question.asked" | "form.created" => {
                if let Some(id) = string(data, "id") {
                    run.waits.insert(id.to_owned(), sid.to_owned());
                }
            }
            "session.tool.called" => {
                if let Some(id) = string(data, "id") {
                    run.tools.insert(id.to_owned(), sid.to_owned());
                }
            }
            "session.tool.success" | "session.tool.failed" => {
                if let Some(id) = string(data, "id") {
                    run.tools.remove(id);
                }
            }
            "message.part.updated" if string(data, "type") == Some("tool") => {
                if let (Some(id), Some(status)) = (
                    string(data, "id"),
                    data.get("state").and_then(|s| string(s, "status")),
                ) {
                    match status {
                        "pending" | "running" => {
                            run.tools.insert(id.to_owned(), sid.to_owned());
                        }
                        "completed" | "error" => {
                            run.tools.remove(id);
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }

    pub(super) fn snapshot(&mut self, root: Option<&str>, now: f64) -> Vec<PlatformAgent> {
        self.show(root);
        self.runs
            .retain(|run| run.ended.is_none_or(|end| now - end < 6.0));
        self.runs
            .iter()
            .filter(|run| Some(run.parent.as_str()) == root)
            .map(|run| {
                let child = self.children.get(&run.id);
                let state = if run.ended.is_some() {
                    PlatformAgentState::Done
                } else if !run.waits.is_empty() {
                    PlatformAgentState::Waiting
                } else if !run.tools.is_empty() || now - run.heard < 20.0 {
                    PlatformAgentState::Working
                } else {
                    PlatformAgentState::Idle
                };
                PlatformAgent {
                    id: run.id.clone(),
                    state,
                    ok: run.ok,
                    kind: child.map_or_else(|| run.kind.clone(), |c| c.kind.clone()),
                    task: child.map_or_else(|| run.task.clone(), |c| c.task.clone()),
                    ms: ((run.ended.unwrap_or(now) - run.started) * 1000.0).max(0.0),
                }
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn child(roster: &mut AgentRoster, id: &str, parent: &str) {
        roster.event(
            Some("main"),
            "session.created",
            &json!({"info": {
                "id": id, "parentID": parent, "agent": "explore", "title": "Check the renderer"
            }}),
            0.0,
        );
    }

    #[test]
    fn legacy_child_tools_waits_and_completion_follow_their_own_ids() {
        let mut roster = AgentRoster::default();
        child(&mut roster, "child", "main");
        child(&mut roster, "other", "another-session");
        assert_eq!(
            roster.snapshot(Some("main"), 21.0)[0].state,
            PlatformAgentState::Idle
        );
        roster.event(
            Some("main"),
            "message.part.updated",
            &json!({"part": {
                "id":"tool", "sessionID":"child", "type":"tool", "state":{"status":"running"}
            }}),
            22.0,
        );
        let current = roster.snapshot(Some("main"), 50.0);
        assert_eq!(current.len(), 1);
        assert_eq!(current[0].state, PlatformAgentState::Working);
        assert_eq!(current[0].kind, "explore");
        assert_eq!(current[0].task, "Check the renderer");
        for id in ["one", "two"] {
            roster.event(
                Some("main"),
                "permission.asked",
                &json!({"sessionID":"child", "id":id}),
                51.0,
            );
        }
        roster.event(
            Some("main"),
            "permission.replied",
            &json!({"permissionID":"one"}),
            52.0,
        );
        assert_eq!(
            roster.snapshot(Some("main"), 80.0)[0].state,
            PlatformAgentState::Waiting
        );
        roster.event(
            Some("main"),
            "permission.replied",
            &json!({"permissionID":"two"}),
            81.0,
        );
        roster.event(
            Some("main"),
            "message.part.updated",
            &json!({"part": {
                "id":"tool", "sessionID":"child", "type":"tool", "state":{"status":"error"}
            }}),
            82.0,
        );
        assert_eq!(
            roster.snapshot(Some("main"), 83.0)[0].state,
            PlatformAgentState::Working
        );
        roster.event(
            Some("main"),
            "session.error",
            &json!({"sessionID":"child"}),
            84.0,
        );
        roster.event(
            Some("main"),
            "session.idle",
            &json!({"sessionID":"child"}),
            85.0,
        );
        let done = roster.snapshot(Some("main"), 89.0);
        assert_eq!(done[0].state, PlatformAgentState::Done);
        assert!(!done[0].ok);
        assert_eq!(done[0].ms, 84_000.0);
        assert!(roster.snapshot(Some("main"), 90.0).is_empty());
    }

    #[test]
    fn nested_work_and_forms_belong_to_one_companion_and_can_resume() {
        let mut roster = AgentRoster::default();
        child(&mut roster, "child", "main");
        child(&mut roster, "nested", "child");
        roster.event(
            Some("main"),
            "session.tool.called",
            &json!({"sessionID":"nested", "id":"tool"}),
            1.0,
        );
        roster.event(
            Some("main"),
            "form.created",
            &json!({"form":{"sessionID":"nested", "id":"form"}}),
            2.0,
        );
        assert_eq!(roster.snapshot(Some("main"), 3.0).len(), 1);
        assert_eq!(
            roster.snapshot(Some("main"), 3.0)[0].state,
            PlatformAgentState::Waiting
        );
        roster.event(Some("main"), "form.cancelled", &json!({"id":"form"}), 4.0);
        roster.event(
            Some("main"),
            "session.tool.success",
            &json!({"sessionID":"nested", "id":"tool"}),
            5.0,
        );
        roster.event(
            Some("main"),
            "session.execution.succeeded",
            &json!({"sessionID":"nested"}),
            6.0,
        );
        assert_eq!(
            roster.snapshot(Some("main"), 7.0)[0].state,
            PlatformAgentState::Working
        );
        roster.event(
            Some("main"),
            "session.execution.succeeded",
            &json!({"sessionID":"child"}),
            8.0,
        );
        assert!(roster.snapshot(Some("main"), 15.0).is_empty());
        roster.event(
            Some("main"),
            "session.text.delta",
            &json!({"sessionID":"child", "delta":"late"}),
            16.0,
        );
        assert!(roster.snapshot(Some("main"), 16.0).is_empty());
        roster.event(
            Some("main"),
            "session.execution.started",
            &json!({"sessionID":"child"}),
            20.0,
        );
        assert_eq!(roster.snapshot(Some("main"), 21.0)[0].ms, 1000.0);
        roster.event(
            Some("main"),
            "session.deleted",
            &json!({"sessionID":"child"}),
            22.0,
        );
        assert!(!roster.snapshot(Some("main"), 23.0)[0].ok);
        assert!(roster.snapshot(Some("other-main"), 24.0).is_empty());
    }

    #[test]
    fn nested_termination_clears_only_its_own_tools_and_questions() {
        let mut roster = AgentRoster::default();
        child(&mut roster, "child", "main");
        child(&mut roster, "nested", "child");
        child(&mut roster, "sibling", "child");
        for sid in ["nested", "sibling"] {
            roster.event(
                Some("main"),
                "session.tool.called",
                &json!({"sessionID":sid, "id":sid}),
                1.0,
            );
            roster.event(
                Some("main"),
                "permission.asked",
                &json!({"sessionID":sid, "id":sid}),
                2.0,
            );
        }
        roster.event(
            Some("main"),
            "session.execution.interrupted",
            &json!({"sessionID":"nested"}),
            3.0,
        );
        assert_eq!(
            roster.snapshot(Some("main"), 30.0)[0].state,
            PlatformAgentState::Waiting
        );
        roster.event(
            Some("main"),
            "permission.replied",
            &json!({"requestID":"sibling"}),
            31.0,
        );
        assert_eq!(
            roster.snapshot(Some("main"), 60.0)[0].state,
            PlatformAgentState::Working
        );
        roster.event(
            Some("main"),
            "session.deleted",
            &json!({"sessionID":"sibling"}),
            61.0,
        );
        assert_eq!(
            roster.snapshot(Some("main"), 62.0)[0].state,
            PlatformAgentState::Idle
        );
    }
}
