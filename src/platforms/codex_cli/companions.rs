//! Direct child rollouts, read independently from the parent transcript.
//!
//! Codex's session_meta supplies the parent thread and stable child id. A
//! forked rollout can embed the parent's entire history with fresh outer
//! timestamps: subagent_history_start_ordinal is the number of records after
//! the first metadata record to skip. Replaying it would invent child work.
//! Only real child lifecycle/tool records feed the roster. No message text is
//! copied into the dashboard, and aggregate completion counts are not agents.
use super::log_parser::parse_timestamp;
use crate::platforms::{PlatformAgent, PlatformAgentState};
use chrono::{Datelike, Local};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

const SCAN_INTERVAL: Duration = Duration::from_secs(2);
const SCAN_DAYS: i64 = 30;
const MAX_HEADERS: usize = 64;
const MAX_CHILDREN: usize = 64;
const MAX_SEEN: usize = 8192;
const MAX_RETIRED: usize = 1024;
const MAX_HEADER_BYTES: u64 = 1024 * 1024;
const MAX_PARSE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_RECORD_BYTES: usize = 8 * 1024 * 1024;
const QUIET_SECONDS: f64 = 20.0;
const DONE_SECONDS: f64 = 6.0;

fn seconds(time: SystemTime) -> f64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}
fn timestamp(v: &Value) -> Option<f64> {
    v.get("timestamp")
        .and_then(Value::as_str)
        .and_then(parse_timestamp)
        .map(seconds)
}

struct Child {
    agent: PlatformAgent,
    path: PathBuf,
    offset: u64,
    skip_history: u64,
    created: f64,
    last_work: f64,
    done_at: Option<f64>,
    active: bool,
    caught_up: bool,
    tools: HashSet<String>,
    waits: HashSet<String>,
    pending_record: Vec<u8>,
    record_too_large: bool,
    async_waits: HashMap<String, HashSet<usize>>,
}

impl Child {
    fn from_header(path: PathBuf, parent: &str) -> Option<Self> {
        let file = File::open(&path).ok()?;
        let mut line = String::new();
        BufReader::new(file.take(MAX_HEADER_BYTES))
            .read_line(&mut line)
            .ok()?;
        if !line.ends_with('\n') {
            return None;
        }
        let record: Value = serde_json::from_str(&line).ok()?;
        if record.get("type").and_then(Value::as_str) != Some("session_meta") {
            return None;
        }
        let meta = record.get("payload")?;
        let actual_parent = meta
            .get("parent_thread_id")
            .and_then(Value::as_str)
            .or_else(|| {
                meta.pointer("/source/subagent/thread_spawn/parent_thread_id")
                    .and_then(Value::as_str)
            });
        if actual_parent != Some(parent) {
            return None;
        }
        let id = meta.get("id").and_then(Value::as_str)?.to_string();
        let agent_path = meta.get("agent_path").and_then(Value::as_str).or_else(|| {
            meta.pointer("/source/subagent/thread_spawn/agent_path")
                .and_then(Value::as_str)
        });
        let task = agent_path
            .and_then(|s| s.rsplit('/').next())
            .filter(|s| !s.is_empty())
            .or_else(|| meta.get("agent_nickname").and_then(Value::as_str))
            .unwrap_or("Codex agent")
            .to_string();
        let kind = meta
            .pointer("/source/subagent/thread_spawn/agent_role")
            .and_then(Value::as_str)
            .unwrap_or("codex")
            .to_string();
        let created = meta
            .get("timestamp")
            .and_then(Value::as_str)
            .and_then(parse_timestamp)
            .map(seconds)
            .or_else(|| timestamp(&record))
            .unwrap_or(0.0);
        Some(Self {
            agent: PlatformAgent {
                id,
                task,
                kind,
                state: PlatformAgentState::Idle,
                ok: true,
                ms: 0.0,
            },
            path,
            offset: line.len() as u64,
            skip_history: meta
                .get("subagent_history_start_ordinal")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            created,
            last_work: created,
            done_at: None,
            active: false,
            caught_up: false,
            tools: HashSet::new(),
            waits: HashSet::new(),
            pending_record: Vec::new(),
            record_too_large: false,
            async_waits: HashMap::new(),
        })
    }

    fn observe(&mut self, record: &Value) {
        let Some(payload) = record.get("payload") else {
            return;
        };
        let at = timestamp(record).unwrap_or(self.last_work);
        let kind = payload
            .get("type")
            .and_then(Value::as_str)
            .unwrap_or_default();
        match record.get("type").and_then(Value::as_str) {
            Some("event_msg") => match kind {
                "task_started" => {
                    self.active = true;
                    self.done_at = None;
                    self.last_work = at;
                    self.agent.ok = true;
                    self.tools.clear();
                    self.waits.clear();
                    self.async_waits.clear();
                }
                "task_complete" | "turn_aborted" => {
                    self.active = false;
                    self.done_at = Some(at);
                    self.agent.ok = kind == "task_complete";
                    self.tools.clear();
                    self.waits.clear();
                    self.async_waits.clear();
                }
                // Some older native rollouts stream this event instead of a
                // response_item. It is activity only, never a completion.
                "agent_message" if self.active => self.last_work = at,
                _ => {}
            },
            Some("response_item") if self.active => match kind {
                "message" if payload.get("role").and_then(Value::as_str) == Some("user") => {
                    self.question_replies(payload)
                }
                "message" if payload.get("role").and_then(Value::as_str) == Some("assistant") => {
                    self.last_work = at
                }
                "function_call" | "custom_tool_call" => {
                    self.last_work = at;
                    let Some(id) = payload.get("call_id").and_then(Value::as_str) else {
                        return;
                    };
                    self.tools.insert(id.to_string());
                    let name = payload
                        .get("name")
                        .and_then(Value::as_str)
                        .unwrap_or_default();
                    let short_name = name.rsplit('.').next().unwrap_or(name);
                    if short_name == "request_user_input_async" {
                        let input = payload
                            .get("arguments")
                            .or_else(|| payload.get("input"))
                            .cloned()
                            .unwrap_or(Value::Null);
                        let input = if let Some(text) = input.as_str() {
                            serde_json::from_str(text).unwrap_or(Value::Null)
                        } else {
                            input
                        };
                        let count = input
                            .get("questions")
                            .and_then(Value::as_array)
                            .map_or(1, Vec::len);
                        self.async_waits
                            .insert(id.into(), (0..count.max(1)).collect());
                    }
                    if matches!(
                        name.rsplit('.').next().unwrap_or(name),
                        "request_user_input"
                            | "request_user_input_async"
                            | "AskUserQuestion"
                            | "ask_user"
                    ) {
                        self.waits.insert(id.to_string());
                    }
                    // An escalation requested in arguments can auto-approve.
                    // Without an observed dialog event it is still working.
                }
                "function_call_output" | "custom_tool_call_output" => {
                    self.last_work = at;
                    if let Some(id) = payload.get("call_id").and_then(Value::as_str) {
                        self.tools.remove(id);
                        if self.async_waits.contains_key(id) {
                            let output = payload.get("output").cloned().unwrap_or(Value::Null);
                            let output = if let Some(text) = output.as_str() {
                                serde_json::from_str(text).unwrap_or(Value::Null)
                            } else {
                                output
                            };
                            if output.get("accepted").and_then(Value::as_bool) == Some(false) {
                                self.waits.remove(id);
                                self.async_waits.remove(id);
                            }
                        } else {
                            self.waits.remove(id);
                        }
                    }
                }
                _ => {}
            },
            _ => {}
        }
    }

    fn question_replies(&mut self, payload: &Value) {
        let Some(content) = payload.get("content").and_then(Value::as_array) else {
            return;
        };
        for part in content {
            let Some(text) = part.get("text").and_then(Value::as_str) else {
                continue;
            };
            let Some(start) = text.find("<send_user_message_question_reply>") else {
                continue;
            };
            let body = &text[start + "<send_user_message_question_reply>".len()..];
            let Some(end) = body.find("</send_user_message_question_reply>") else {
                continue;
            };
            let Ok(replies) = serde_json::from_str::<Vec<Value>>(&body[..end]) else {
                continue;
            };
            for reply in replies {
                let Some(item) = reply.get("questionItemId").and_then(Value::as_str) else {
                    continue;
                };
                let Ok(key) = serde_json::from_str::<Vec<Value>>(item) else {
                    continue;
                };
                if key.len() != 3 || key[0].as_str() != Some("request_user_input_async") {
                    continue;
                }
                let (Some(call), Some(index)) = (key[1].as_str(), key[2].as_u64()) else {
                    continue;
                };
                if let Some(left) = self.async_waits.get_mut(call) {
                    left.remove(&(index as usize));
                    if left.is_empty() {
                        self.async_waits.remove(call);
                        self.waits.remove(call);
                    }
                }
            }
        }
    }

    fn tail(&mut self, budget: &mut u64) {
        let Ok(mut file) = File::open(&self.path) else {
            return;
        };
        let len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if len < self.offset {
            return;
        } // discovery replaces truncated files
        if file.seek(SeekFrom::Start(self.offset)).is_err() {
            return;
        }
        let mut reader = BufReader::new(file);
        while *budget > 0 {
            let Ok(available) = reader.fill_buf() else {
                break;
            };
            if available.is_empty() {
                break;
            }
            let take = available.len().min(*budget as usize);
            let newline = available[..take].iter().position(|&b| b == b'\n');
            let used = newline.map_or(take, |i| i + 1);
            if self.skip_history == 0 && !self.record_too_large {
                if self.pending_record.len() + used <= MAX_RECORD_BYTES {
                    self.pending_record.extend_from_slice(&available[..used]);
                } else {
                    self.record_too_large = true;
                }
            }
            self.offset += used as u64;
            *budget -= used as u64;
            reader.consume(used);
            if newline.is_none() {
                continue;
            }
            if self.skip_history > 0 {
                self.skip_history -= 1;
            } else if !self.record_too_large {
                if let Ok(record) = serde_json::from_slice(&self.pending_record) {
                    self.observe(&record);
                }
            } else {
                // Native outputs put call_id before their possibly huge
                // output. Keep just that metadata so its running call ends.
                let marker = b",\"output\":";
                if let Some(end) = self
                    .pending_record
                    .windows(marker.len())
                    .position(|w| w == marker)
                {
                    // The retained output can end midway through a UTF-8
                    // character. Decode only the complete metadata prefix.
                    if let Ok(prefix) = std::str::from_utf8(&self.pending_record[..end]) {
                        if let Ok(record) = serde_json::from_str::<Value>(&format!("{prefix}}}}}"))
                        {
                            let kind = record.pointer("/payload/type").and_then(Value::as_str);
                            if matches!(
                                kind,
                                Some("function_call_output" | "custom_tool_call_output")
                            ) {
                                self.observe(&record);
                            }
                        }
                    }
                }
            }
            self.pending_record.clear();
            self.record_too_large = false;
        }
        if self.offset >= len
            && self.pending_record.is_empty()
            && !self.record_too_large
            && self.skip_history == 0
        {
            self.caught_up = true;
        }
    }

    fn snapshot(&self, now: f64) -> Option<PlatformAgent> {
        if !self.caught_up {
            return None;
        }
        if let Some(done) = self.done_at {
            if now - done > DONE_SECONDS {
                return None;
            }
        }
        let mut agent = self.agent.clone();
        agent.state = if self.done_at.is_some() {
            PlatformAgentState::Done
        } else if !self.waits.is_empty() {
            PlatformAgentState::Waiting
        } else if self.active && (!self.tools.is_empty() || now - self.last_work < QUIET_SECONDS) {
            PlatformAgentState::Working
        } else {
            PlatformAgentState::Idle
        };
        agent.ms = ((self.done_at.unwrap_or(now) - self.created) * 1000.0).max(0.0);
        Some(agent)
    }
}

#[derive(Default)]
pub(super) struct ChildRoster {
    parent: String,
    last_scan: Option<SystemTime>,
    inspected: HashSet<PathBuf>,
    children: HashMap<PathBuf, Child>,
    retired: HashMap<PathBuf, Child>,
}
impl ChildRoster {
    pub fn refresh(
        &mut self,
        root: &Path,
        parent_path: &Path,
        parent: &str,
        now: SystemTime,
    ) -> Vec<PlatformAgent> {
        if self.parent != parent {
            *self = Self {
                parent: parent.into(),
                ..Self::default()
            };
        }
        if parent.is_empty() {
            return Vec::new();
        }
        if self
            .last_scan
            .is_none_or(|at| now.duration_since(at).unwrap_or_default() >= SCAN_INTERVAL)
        {
            self.last_scan = Some(now);
            self.retire(seconds(now));
            self.discover(root, parent_path, parent);
        }
        let mut budget = MAX_PARSE_BYTES;
        // Newest work gets first use of a bounded catch-up budget. A large,
        // completed transcript cannot delay active companions indefinitely.
        let mut children: Vec<_> = self.children.values_mut().collect();
        children.sort_by(|a, b| b.created.total_cmp(&a.created));
        for child in children {
            if budget == 0 {
                break;
            }
            child.tail(&mut budget);
        }
        let mut ordered: Vec<_> = self.children.values().collect();
        ordered.sort_by(|a, b| {
            a.created
                .total_cmp(&b.created)
                .then(a.agent.id.cmp(&b.agent.id))
        });
        ordered
            .into_iter()
            .filter_map(|child| child.snapshot(seconds(now)))
            .collect()
    }

    fn retire(&mut self, now: f64) {
        let paths: Vec<_> = self
            .children
            .iter()
            .filter(|(_, c)| c.caught_up && c.done_at.is_some_and(|done| now - done > DONE_SECONDS))
            .map(|(p, _)| p.clone())
            .collect();
        for path in paths {
            if let Some(child) = self.children.remove(&path) {
                self.retired.insert(path, child);
            }
        }
        while self.retired.len() > MAX_RETIRED {
            let oldest = self
                .retired
                .iter()
                .min_by(|a, b| a.1.created.total_cmp(&b.1.created))
                .map(|(p, _)| p.clone())
                .unwrap();
            self.retired.remove(&oldest);
        }
    }

    fn discover(&mut self, root: &Path, parent_path: &Path, parent: &str) {
        let today = Local::now();
        let mut dirs = Vec::new();
        for day in 0..SCAN_DAYS {
            let date = today - chrono::Duration::days(day);
            dirs.push(
                root.join(format!("{:04}", date.year()))
                    .join(format!("{:02}", date.month()))
                    .join(format!("{:02}", date.day())),
            );
        }
        // A long-lived/resumed parent and children created on its original
        // day can live outside the recent scan window.
        if let Some(dir) = parent_path.parent() {
            if !dirs.iter().any(|d| d == dir) {
                dirs.push(dir.into());
            }
        }
        let mut files = Vec::new();
        for dir in dirs {
            let Ok(entries) = fs::read_dir(dir) else {
                continue;
            };
            for entry in entries.flatten() {
                let path = entry.path();
                if path == parent_path || path.extension().and_then(|s| s.to_str()) != Some("jsonl")
                {
                    continue;
                }
                if let Some(child) = self.children.get(&path) {
                    if entry
                        .metadata()
                        .map(|m| m.len() >= child.offset)
                        .unwrap_or(true)
                    {
                        continue;
                    }
                    self.children.remove(&path);
                    self.inspected.remove(&path);
                }
                if let Some(child) = self.retired.get(&path) {
                    let len = entry.metadata().map(|m| m.len()).unwrap_or(child.offset);
                    if len == child.offset {
                        continue;
                    }
                    if self.children.len() >= MAX_CHILDREN {
                        continue;
                    }
                    let child = self.retired.remove(&path).unwrap();
                    if len > child.offset {
                        self.children.insert(path, child);
                        continue;
                    }
                    self.inspected.remove(&path);
                }
                if self.inspected.contains(&path) {
                    continue;
                }
                let modified = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(UNIX_EPOCH);
                files.push((modified, path));
            }
        }
        files.sort_by_key(|entry| std::cmp::Reverse(entry.0));
        for (_, path) in files.into_iter().take(MAX_HEADERS) {
            if self.children.len() >= MAX_CHILDREN {
                break;
            }
            if let Some(child) = Child::from_header(path.clone(), parent) {
                self.children.insert(path, child);
                continue;
            }
            // Empty/incomplete headers are retried; a complete non-child
            // header cannot change identity and is inspected only once.
            let complete = File::open(&path)
                .ok()
                .and_then(|file| {
                    let mut line = String::new();
                    BufReader::new(file.take(MAX_HEADER_BYTES))
                        .read_line(&mut line)
                        .ok()?;
                    if !line.ends_with('\n') {
                        return None;
                    }
                    serde_json::from_str::<Value>(&line).ok()
                })
                .is_some();
            if complete && self.inspected.len() < MAX_SEEN {
                self.inspected.insert(path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;
    fn at(seconds: u64) -> String {
        chrono::DateTime::from_timestamp(seconds as i64, 0)
            .unwrap()
            .to_rfc3339()
    }
    fn header(id: &str, parent: &str, skip: u64) -> Value {
        json!({"type":"session_meta","payload":{"id":id,"parent_thread_id":parent,"agent_path":"/root/scene_port","timestamp":at(100),"subagent_history_start_ordinal":skip}})
    }
    fn event(kind: &str, t: u64) -> Value {
        json!({"type":"event_msg","timestamp":at(t),"payload":{"type":kind}})
    }
    fn tool(kind: &str, id: &str, name: &str, t: u64) -> Value {
        json!({"type":"response_item","timestamp":at(t),"payload":{"type":kind,"call_id":id,"name":name}})
    }
    fn write(path: &Path, records: &[Value]) {
        let mut file = File::create(path).unwrap();
        for r in records {
            writeln!(file, "{r}").unwrap();
        }
    }
    fn tail(child: &mut Child) {
        let mut budget = MAX_PARSE_BYTES;
        child.tail(&mut budget);
    }
    #[test]
    fn skips_fork_history_and_keeps_child_identity() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("child.jsonl");
        write(
            &path,
            &[
                header("child", "parent", 3),
                json!({"type":"session_meta","payload":{"id":"parent"}}),
                event("task_started", 101),
                event("task_complete", 102),
                event("task_started", 103),
            ],
        );
        let mut child = Child::from_header(path, "parent").unwrap();
        tail(&mut child);
        let a = child.snapshot(104.).unwrap();
        assert_eq!(a.id, "child");
        assert_eq!(a.state, PlatformAgentState::Working);
        assert_eq!(a.task, "scene_port");
    }
    #[test]
    fn separate_waits_survive_other_tools_and_a_quiet_running_tool() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("child.jsonl");
        write(
            &path,
            &[
                header("child", "parent", 0),
                event("task_started", 100),
                tool(
                    "function_call",
                    "question",
                    "functions.request_user_input",
                    101,
                ),
                tool("function_call", "shell", "exec_command", 102),
                tool("function_call_output", "shell", "", 103),
            ],
        );
        let mut child = Child::from_header(path, "parent").unwrap();
        tail(&mut child);
        assert_eq!(
            child.snapshot(140.).unwrap().state,
            PlatformAgentState::Waiting
        );
        child.observe(&tool("function_call_output", "question", "", 141));
        assert_eq!(
            child.snapshot(142.).unwrap().state,
            PlatformAgentState::Working
        );
        assert_eq!(
            child.snapshot(162.).unwrap().state,
            PlatformAgentState::Idle
        );
        child.observe(&tool("function_call", "long", "exec_command", 163));
        assert_eq!(
            child.snapshot(200.).unwrap().state,
            PlatformAgentState::Working
        );
    }
    #[test]
    fn done_failure_and_a_followup_reuse_the_same_id() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("child.jsonl");
        write(
            &path,
            &[
                header("child", "parent", 0),
                event("task_started", 100),
                event("turn_aborted", 110),
            ],
        );
        let mut child = Child::from_header(path, "parent").unwrap();
        tail(&mut child);
        let a = child.snapshot(112.).unwrap();
        assert_eq!(a.state, PlatformAgentState::Done);
        assert!(!a.ok);
        assert_eq!(a.ms, 10000.);
        assert!(child.snapshot(117.).is_none());
        child.observe(&event("task_started", 120));
        assert_eq!(child.snapshot(121.).unwrap().id, "child");
        child.observe(&event("task_complete", 130));
        assert!(child.snapshot(131.).unwrap().ok);
    }
    #[test]
    fn incomplete_records_are_retried_and_history_catchup_is_not_shown() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("child.jsonl");
        write(
            &path,
            &[header("child", "parent", 1), event("task_complete", 101)],
        );
        let mut child = Child::from_header(path.clone(), "parent").unwrap();
        let mut budget = 1;
        child.tail(&mut budget);
        assert!(child.snapshot(102.).is_none());
        tail(&mut child);
        let part = event("task_started", 103).to_string();
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        file.write_all(&part.as_bytes()[..10]).unwrap();
        let offset = child.offset;
        tail(&mut child);
        assert_eq!(child.offset, offset + 10);
        assert_eq!(
            child.snapshot(103.).unwrap().state,
            PlatformAgentState::Idle
        );
        file.write_all(&part.as_bytes()[10..]).unwrap();
        file.write_all(b"\n").unwrap();
        tail(&mut child);
        assert_eq!(
            child.snapshot(104.).unwrap().state,
            PlatformAgentState::Working
        );
    }
    #[test]
    fn discovers_only_direct_children_and_resets_on_parent_switch() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let parent = root.join("parent.jsonl");
        write(&parent, &[]);
        write(
            &root.join("ours.jsonl"),
            &[header("ours", "parent", 0), event("task_started", 100)],
        );
        write(
            &root.join("other.jsonl"),
            &[header("other", "elsewhere", 0), event("task_started", 100)],
        );
        write(
            &root.join("grandchild.jsonl"),
            &[header("grandchild", "ours", 0), event("task_started", 100)],
        );
        let mut roster = ChildRoster::default();
        let a = roster.refresh(
            root,
            &parent,
            "parent",
            UNIX_EPOCH + Duration::from_secs(105),
        );
        assert_eq!(a.len(), 1);
        assert_eq!(a[0].id, "ours");
        let b = roster.refresh(
            root,
            &parent,
            "elsewhere",
            UNIX_EPOCH + Duration::from_secs(106),
        );
        assert_eq!(b.len(), 1);
        assert_eq!(b[0].id, "other");
    }
    #[test]
    fn async_acknowledgment_waits_for_each_matching_answer() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("child.jsonl");
        write(
            &path,
            &[header("child", "parent", 0), event("task_started", 100)],
        );
        let mut child = Child::from_header(path, "parent").unwrap();
        tail(&mut child);
        let mut ask = tool("function_call", "ask", "request_user_input_async", 101);
        ask["payload"]["arguments"] = json!({"questions":[{},{}]}).to_string().into();
        child.observe(&ask);
        let mut ack = tool("function_call_output", "ask", "", 102);
        ack["payload"]["output"] = json!({"accepted":true}).to_string().into();
        child.observe(&ack);
        assert_eq!(
            child.snapshot(103.).unwrap().state,
            PlatformAgentState::Waiting
        );
        let reply = |call: &str, index: u64| json!({"type":"response_item","timestamp":at(104),"payload":{"type":"message","role":"user","content":[{"type":"input_text","text":format!("<send_user_message_question_reply>{}</send_user_message_question_reply>",json!([{"questionItemId":json!(["request_user_input_async",call,index]).to_string(),"answer":"yes"}]))}]}});
        child.observe(&reply("other", 0));
        assert_eq!(child.waits.len(), 1);
        child.observe(&reply("ask", 0));
        assert_eq!(child.waits.len(), 1);
        child.observe(&reply("ask", 1));
        assert!(child.waits.is_empty());
    }

    #[test]
    fn retired_children_leave_room_and_resume_from_their_saved_cursor() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        let parent = root.join("parent.jsonl");
        write(&parent, &[]);
        let child_path = root.join("child.jsonl");
        write(
            &child_path,
            &[
                header("child", "parent", 0),
                event("task_started", 100),
                event("task_complete", 101),
            ],
        );
        let mut roster = ChildRoster::default();
        let now = |s| UNIX_EPOCH + Duration::from_secs(s);
        assert_eq!(roster.refresh(root, &parent, "parent", now(102)).len(), 1);
        assert!(roster.refresh(root, &parent, "parent", now(110)).is_empty());
        assert!(roster.children.is_empty());
        let offset = roster.retired[&child_path].offset;
        let mut file = fs::OpenOptions::new()
            .append(true)
            .open(&child_path)
            .unwrap();
        writeln!(file, "{}", event("task_started", 112)).unwrap();
        let resumed = roster.refresh(root, &parent, "parent", now(113));
        assert_eq!(resumed.len(), 1);
        assert_eq!(resumed[0].id, "child");
        assert_eq!(resumed[0].state, PlatformAgentState::Working);
        assert!(roster.children[&child_path].offset > offset);
    }

    #[test]
    #[ignore = "Read-only local child roster probe; set CRABIGATOR_CODEX_PARENT_ROLLOUT"]
    fn verify_live_child_roster() {
        let parent_path = PathBuf::from(std::env::var("CRABIGATOR_CODEX_PARENT_ROLLOUT").unwrap());
        let root = parent_path.ancestors().nth(4).unwrap();
        let mut header = String::new();
        BufReader::new(File::open(&parent_path).unwrap())
            .read_line(&mut header)
            .unwrap();
        let header: Value = serde_json::from_str(&header).unwrap();
        let parent = header
            .pointer("/payload/id")
            .and_then(Value::as_str)
            .unwrap();
        let mut roster = ChildRoster::default();
        let mut agents = Vec::new();
        for _ in 0..1000 {
            agents = roster.refresh(root, &parent_path, parent, SystemTime::now());
            if !roster.children.is_empty() && roster.children.values().all(|child| child.caught_up)
            {
                break;
            }
        }
        assert!(
            !roster.children.is_empty(),
            "no direct child rollouts found"
        );
        assert!(
            roster.children.values().all(|child| child.caught_up),
            "catch-up budget exhausted"
        );
        for agent in &agents {
            // Stable metadata and state only; never print prompt/transcript text.
            eprintln!("child={} state={:?} ok={}", agent.id, agent.state, agent.ok);
        }
        eprintln!(
            "caught_up={} currently_visible={}",
            roster.children.len(),
            agents.len()
        );
    }

    #[test]
    fn large_outputs_obey_the_read_budget_and_release_their_call() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("child.jsonl");
        write(
            &path,
            &[
                header("child", "parent", 0),
                event("task_started", 100),
                tool("function_call", "shell", "exec_command", 101),
            ],
        );
        let mut child = Child::from_header(path.clone(), "parent").unwrap();
        tail(&mut child);
        let mut file = fs::OpenOptions::new().append(true).open(&path).unwrap();
        // Native struct order puts both record types and call_id before
        // output. Non-ASCII text also exercises a truncated UTF-8 prefix.
        let output = "🌍".repeat(MAX_RECORD_BYTES / 4 + 100);
        writeln!(file, "{{\"timestamp\":\"{}\",\"type\":\"response_item\",\"payload\":{{\"type\":\"function_call_output\",\"call_id\":\"shell\",\"output\":{}}}}}", at(102), serde_json::to_string(&output).unwrap()).unwrap();
        let before = child.offset;
        tail(&mut child);
        assert_eq!(child.offset - before, MAX_PARSE_BYTES);
        assert_eq!(child.tools.len(), 1);
        while child.offset < file.metadata().unwrap().len() {
            tail(&mut child);
        }
        assert!(child.tools.is_empty());
        assert!(child.pending_record.is_empty());
    }
}
