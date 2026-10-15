//! What Claude Code's hooks say, as it happens. crabigator's hook appends a
//! line to the session's activity log for every hook event, and the column
//! reads the new lines each frame. That is how flow's plugin hears the work:
//! a tool as it starts (an edit by the lines it writes), a failed command, a
//! permission asked and answered, each model request, the effort, streamed
//! text, and which subagents are running.
//!
//! Until the log says anything, the stats stand in (`feed.rs`): other
//! assistants have no such log.

use std::collections::HashMap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::PathBuf;
use std::time::{Duration, Instant};

use serde::Deserialize;

use super::activity::Activity;
use super::agents::{AgentDial, AgentState};

/// A subagent or a tool call silent this long is gone (its stop was missed:
/// the session was interrupted, or the hook didn't run).
const SILENT: Duration = Duration::from_secs(600);
/// The most of the log read in one frame (the rest waits for the next).
const MAX_READ: u64 = 64 * 1024;

/// One line of the activity log (written by `stats_hook.py`'s `activity_entry`).
#[derive(Debug, Default, Deserialize)]
#[serde(default)]
struct Line {
    ev: String,
    /// The subagent the event came from (none: the main loop). For
    /// SubagentStart and SubagentStop, the subagent itself.
    agent: Option<String>,
    effort: Option<serde_json::Value>,
    tool: Option<String>,
    id: Option<String>,
    /// Lines a write-ish tool call writes.
    lines: Option<f64>,
    interrupt: bool,
    /// Characters of streamed text.
    chars: Option<f64>,
    source: Option<String>,
    fingerprint: Option<String>,
    agent_type: Option<String>,
    error: Option<serde_json::Value>,
    reason: Option<String>,
}

/// A tool call started and not yet finished.
struct Call {
    tool: String,
    agent: Option<String>,
    since: Instant,
    /// The person was asked to allow it: its finishing is a change of state too.
    asked: bool,
    fingerprint: Option<String>,
}

struct Agent {
    id: String,
    kind: String,
    started: Instant,
    seen: Instant,
    ended: Option<Instant>,
    ok: bool,
}

/// The activity log, read as it grows, and what it says is going on.
pub struct HookFeed {
    path: PathBuf,
    offset: u64,
    /// The start of a line still being written.
    partial: Vec<u8>,
    calls: HashMap<String, Call>,
    /// Running subagents, and when each last said anything.
    agents: Vec<Agent>,
    elicitations: HashMap<Option<String>, (u32, Instant)>,
    /// The log has said something: it, not the stats, tells the work.
    pub live: bool,
    /// The main loop's events have told the effort: the screen's banner is no
    /// longer read for it.
    pub knows_effort: bool,
    /// When text last streamed through Claude Code's display.
    pub streamed_at: Option<Instant>,
}

impl HookFeed {
    /// Follow the log at `path`. One already there has only history in it
    /// (a resumed session's history isn't news): reading starts at its end.
    pub fn open(path: PathBuf) -> Self {
        let offset = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
        Self {
            path,
            offset,
            partial: Vec::new(),
            calls: HashMap::new(),
            agents: Vec::new(),
            elicitations: HashMap::new(),
            live: false,
            knows_effort: false,
            streamed_at: None,
        }
    }

    /// Hear what the log says since the last read, and set the running
    /// subagents and tool calls on `activity`.
    pub fn poll(&mut self, activity: &mut Activity, now: Instant) {
        for line in self.read_lines() {
            if let Ok(line) = serde_json::from_slice::<Line>(&line) {
                self.live = true;
                self.hear(&line, activity, now);
            }
        }
        self.agents.retain(|a| {
            a.ended.map_or(now.duration_since(a.seen) < SILENT, |end| {
                now.duration_since(end) < Duration::from_secs(6)
            })
        });
        self.calls
            .retain(|_, call| now.duration_since(call.since) < SILENT);
        self.elicitations
            .retain(|_, (_, since)| now.duration_since(*since) < SILENT);
        activity.running_agents = self.agents.iter().filter(|a| a.ended.is_none()).count() as u32;
        activity.tools_in_flight = self.calls.len() as u32;
        let waiting = self.calls.values().any(|c| c.asked) || !self.elicitations.is_empty();
        if activity.hook_waiting != waiting {
            activity.changed();
        }
        activity.hook_waiting = waiting;
        activity.agents = self
            .agents
            .iter()
            .map(|a| {
                let (has_call, asked) = self
                    .calls
                    .values()
                    .filter(|c| c.agent.as_deref() == Some(&a.id))
                    .fold((false, false), |(_, asked), c| (true, asked || c.asked));
                let state = if a.ended.is_some() {
                    AgentState::Done
                } else if asked || self.elicitations.contains_key(&Some(a.id.clone())) {
                    AgentState::Waiting
                } else if has_call || now.duration_since(a.seen) < Duration::from_secs(20) {
                    AgentState::Working
                } else {
                    AgentState::Idle
                };
                AgentDial {
                    id: a.id.clone(),
                    state,
                    ok: a.ok,
                    task: String::new(),
                    kind: a.kind.clone(),
                    ms: a
                        .ended
                        .unwrap_or(now)
                        .duration_since(a.started)
                        .as_secs_f64()
                        * 1000.0,
                }
            })
            .collect();
    }

    /// Pass over what the log says until now, and forget the calls and
    /// subagents heard so far: the column was off, and that is old news.
    pub fn skip_ahead(&mut self) {
        self.offset = std::fs::metadata(&self.path).map_or(0, |m| m.len());
        self.partial.clear();
        self.calls.clear();
        self.agents.clear();
        self.elicitations.clear();
    }

    /// The main turn is over (however crabigator heard it): its calls are done.
    pub fn turn_ended(&mut self) {
        self.calls.retain(|_, call| call.agent.is_some());
        self.elicitations.remove(&None);
    }

    /// The whole lines appended since the last read.
    fn read_lines(&mut self) -> Vec<Vec<u8>> {
        let Ok(len) = std::fs::metadata(&self.path).map(|m| m.len()) else {
            return Vec::new();
        };
        if len < self.offset {
            // Replaced: start over.
            self.offset = 0;
            self.partial.clear();
            self.calls.clear();
            self.agents.clear();
            self.elicitations.clear();
        }
        if len == self.offset {
            return Vec::new();
        }
        let mut bytes = Vec::new();
        let read = File::open(&self.path).and_then(|mut file| {
            file.seek(SeekFrom::Start(self.offset))?;
            file.take(MAX_READ).read_to_end(&mut bytes)
        });
        let Ok(read) = read else {
            return Vec::new();
        };
        self.offset += read as u64;
        self.partial.extend_from_slice(&bytes);
        let Some(end) = self.partial.iter().rposition(|&b| b == b'\n') else {
            return Vec::new();
        };
        let rest = self.partial.split_off(end + 1);
        let whole = std::mem::replace(&mut self.partial, rest);
        whole
            .split(|&b| b == b'\n')
            .filter(|line| !line.is_empty())
            .map(<[u8]>::to_vec)
            .collect()
    }

    fn hear(&mut self, line: &Line, activity: &mut Activity, now: Instant) {
        let is_subagent = line.agent.is_some();
        if let Some(id) = &line.agent {
            let active_event = matches!(
                line.ev.as_str(),
                "SubagentStart"
                    | "PreToolUse"
                    | "PostToolUse"
                    | "PostToolUseFailure"
                    | "PermissionDenied"
                    | "PostToolBatch"
                    | "MessageDisplay"
                    | "PermissionRequest"
                    | "Elicitation"
                    | "ElicitationResult"
            );
            if active_event {
                if let Some(a) = self.agents.iter_mut().find(|a| &a.id == id) {
                    a.seen = now;
                    // Completion hooks and async display events can arrive after
                    // Stop. Only fresh work starts another run under this id.
                    if a.ended.is_some()
                        && matches!(
                            line.ev.as_str(),
                            "SubagentStart" | "PreToolUse" | "PostToolBatch"
                        )
                    {
                        a.ended = None;
                        a.started = now;
                        a.ok = true;
                    }
                    if let Some(kind) = &line.agent_type {
                        a.kind.clone_from(kind);
                    }
                } else {
                    self.agents.push(Agent {
                        id: id.clone(),
                        kind: line.agent_type.clone().unwrap_or_default(),
                        started: now,
                        seen: now,
                        ended: None,
                        ok: true,
                    });
                }
            }
        }
        // Only the main loop's effort sets the floor (flow's `modelStep`).
        if let Some(effort) = line.effort.as_ref().filter(|_| !is_subagent) {
            activity.floor = super::effort_floor(effort.as_str().unwrap_or_default());
            self.knows_effort = true;
        }
        let tool = line.tool.as_deref().unwrap_or_default();
        match line.ev.as_str() {
            "UserPromptSubmit" if !is_subagent => {
                self.turn_ended();
                activity.turn_started();
                activity.changed();
                activity.model_step(false);
            }
            "Stop" | "StopFailure" if !is_subagent => {
                self.turn_ended();
                activity.turn_ended();
                activity.changed();
                if line.ev == "StopFailure" {
                    activity.failed();
                }
            }
            "SessionStart" if !is_subagent && line.source.as_deref() != Some("compact") => {
                self.calls.clear();
                self.agents.clear();
                self.elicitations.clear();
            }
            // After each batch of tool calls, the next model request.
            "PostToolBatch" => activity.model_step(is_subagent),
            "PreToolUse" => {
                if let Some(lines) = line.lines {
                    activity.edited(lines, is_subagent);
                } else if tool == "Bash" {
                    activity.ran_command(is_subagent);
                } else if tool == "Agent" {
                    activity.spawned_agent();
                } else {
                    activity.read(is_subagent);
                }
                if let Some(id) = &line.id {
                    self.calls.insert(
                        id.clone(),
                        Call {
                            tool: tool.to_string(),
                            agent: line.agent.clone(),
                            since: now,
                            asked: matches!(tool, "AskUserQuestion" | "ExitPlanMode"),
                            fingerprint: line.fingerprint.clone(),
                        },
                    );
                }
            }
            "PostToolUse" | "PostToolUseFailure" | "PermissionDenied" => {
                let call = line.id.as_ref().and_then(|id| self.calls.remove(id));
                if call.is_some_and(|call| call.asked) {
                    activity.changed();
                }
                if line.ev == "PostToolUseFailure" && tool == "Bash" && !line.interrupt {
                    activity.failed();
                }
            }
            // Correlate a shown prompt with its call. Never guess between
            // concurrent calls of the same tool: an older producer may lack ids.
            "PermissionRequest" => {
                activity.changed();
                let matched = if let Some(id) = &line.id {
                    self.calls.contains_key(id).then(|| id.clone())
                } else {
                    let mut candidates = self.calls.iter().filter(|(_, c)| {
                        c.agent == line.agent
                            && c.tool == tool
                            && (line.fingerprint.is_none() || c.fingerprint == line.fingerprint)
                    });
                    let first = candidates.next().map(|(id, _)| id.clone());
                    if candidates.next().is_none() {
                        first
                    } else {
                        None
                    }
                };
                if let Some(id) = matched {
                    self.calls.get_mut(&id).unwrap().asked = true;
                }
            }
            "Elicitation" => {
                let entry = self
                    .elicitations
                    .entry(line.agent.clone())
                    .or_insert((0, now));
                entry.0 += 1;
                entry.1 = now;
                activity.changed();
            }
            "ElicitationResult" => {
                if let Some(entry) = self.elicitations.get_mut(&line.agent) {
                    entry.0 = entry.0.saturating_sub(1);
                    if entry.0 == 0 {
                        self.elicitations.remove(&line.agent);
                    }
                }
                activity.changed();
            }
            "SubagentStart" => activity.model_step(true),
            "SubagentStop" | "Stop" | "StopFailure" if is_subagent => {
                if let Some(id) = &line.agent {
                    if let Some(a) = self.agents.iter_mut().find(|a| &a.id == id) {
                        a.ended.get_or_insert(now);
                        a.ok = a.ok
                            && line.ev != "StopFailure"
                            && !line.interrupt
                            && line
                                .error
                                .as_ref()
                                .is_none_or(|e| e.is_null() || e == false || e == "")
                            && !matches!(
                                line.reason.as_deref(),
                                Some("error" | "interrupted" | "cancelled" | "stopped")
                            );
                    }
                    self.calls.retain(|_, c| c.agent.as_ref() != Some(id));
                    self.elicitations.remove(&line.agent);
                }
            }
            "PostCompact" => {
                activity.compacted();
                activity.changed();
            }
            "MessageDisplay" => {
                activity.streamed(line.chars.unwrap_or_default(), is_subagent);
                self.streamed_at = Some(now);
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::activity::SUBAGENT_WEIGHT;
    use crate::flow::scene::Tint;
    use std::io::Write;

    /// A feed with no log behind it, heard a line at a time.
    struct Ear {
        feed: HookFeed,
        activity: Activity,
        now: Instant,
    }

    impl Ear {
        fn new() -> Self {
            Self {
                feed: HookFeed::open(PathBuf::from("/nonexistent/activity.jsonl")),
                activity: Activity::default(),
                now: Instant::now(),
            }
        }

        fn hear(&mut self, json: &str) -> &Activity {
            let line: Line = serde_json::from_str(json).unwrap();
            self.feed.hear(&line, &mut self.activity, self.now);
            self.feed.poll(&mut self.activity, self.now);
            &self.activity
        }
    }

    #[test]
    fn the_log_is_read_as_it_grows_whole_lines_at_a_time() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.jsonl");
        let line = |json: &str| json.as_bytes().to_vec();
        std::fs::write(&path, "{\"ev\":\"Stop\"}\n").unwrap();
        // What was there before is history.
        let mut feed = HookFeed::open(path.clone());
        assert!(feed.read_lines().is_empty());
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        write!(file, "{{\"ev\":\"PreToolUse\"}}\n{{\"ev\":\"Post").unwrap();
        assert_eq!(feed.read_lines(), vec![line(r#"{"ev":"PreToolUse"}"#)]);
        writeln!(file, "ToolUse\"}}").unwrap();
        assert_eq!(feed.read_lines(), vec![line(r#"{"ev":"PostToolUse"}"#)]);
        assert!(feed.read_lines().is_empty());
        // Replaced by a shorter file: read it from the start.
        std::fs::write(&path, "{\"ev\":\"Stop\"}\n").unwrap();
        assert_eq!(feed.read_lines(), vec![line(r#"{"ev":"Stop"}"#)]);
    }

    #[test]
    fn a_log_that_appears_later_is_read_from_its_start() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.jsonl");
        let mut feed = HookFeed::open(path.clone());
        std::fs::write(&path, "{\"ev\":\"UserPromptSubmit\",\"effort\":\"max\"}\n").unwrap();
        let mut activity = Activity::default();
        feed.poll(&mut activity, Instant::now());
        assert!(feed.live && feed.knows_effort);
        assert!(activity.is_turn_active);
        assert_eq!(activity.floor, 6.0);
    }

    #[test]
    fn skipping_ahead_passes_over_what_was_said_and_forgets_calls_in_flight() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("activity.jsonl");
        let mut feed = HookFeed::open(path.clone());
        let mut activity = Activity::default();
        std::fs::write(
            &path,
            "{\"ev\":\"PreToolUse\",\"tool\":\"Bash\",\"id\":\"t1\"}\n",
        )
        .unwrap();
        feed.poll(&mut activity, Instant::now());
        assert_eq!(activity.tools_in_flight, 1);
        let mut file = std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .unwrap();
        writeln!(file, "{{\"ev\":\"UserPromptSubmit\"}}").unwrap();
        feed.skip_ahead();
        feed.poll(&mut activity, Instant::now());
        assert_eq!(activity.tools_in_flight, 0);
        assert!(!activity.is_turn_active);
        writeln!(file, "{{\"ev\":\"UserPromptSubmit\"}}").unwrap();
        feed.poll(&mut activity, Instant::now());
        assert!(activity.is_turn_active);
    }

    #[test]
    fn a_tool_is_heard_as_it_starts_and_held_in_flight_until_it_ends() {
        let mut ear = Ear::new();
        let before = ear.hear(r#"{"ev":"UserPromptSubmit"}"#).heat;
        let after = ear.hear(r#"{"ev":"PreToolUse","tool":"Write","id":"t1","lines":60}"#);
        // Sixty lines: flow's biggest edit flare.
        assert!((after.heat - before - 4.0).abs() < 1e-9);
        assert_eq!(after.tools_in_flight, 1);
        let done = ear.hear(r#"{"ev":"PostToolUse","tool":"Write","id":"t1"}"#);
        assert_eq!(done.tools_in_flight, 0);
        // Other tools add the same small spark as a read.
        let before = done.heat;
        let task = ear.hear(r#"{"ev":"PreToolUse","tool":"TaskCreate","id":"t2"}"#);
        assert_eq!(task.heat, (before + 0.5).min(5.0));
        // The turn ends: whatever it left in flight is done.
        let stopped = ear.hear(r#"{"ev":"Stop"}"#);
        assert_eq!(stopped.tools_in_flight, 0);
        assert!(!stopped.is_turn_active);
    }

    #[test]
    fn a_failed_command_smokes_but_an_interrupted_one_does_not() {
        let mut ear = Ear::new();
        let interrupted =
            ear.hear(r#"{"ev":"PostToolUseFailure","tool":"Bash","id":"t1","interrupt":true}"#);
        assert_eq!(interrupted.tint(), Tint::Normal);
        let failed = ear.hear(r#"{"ev":"PostToolUseFailure","tool":"Bash","id":"t2"}"#);
        assert_eq!(failed.tint(), Tint::Smoke);
    }

    #[test]
    fn a_permission_asked_and_answered_each_relight_the_scene() {
        let mut ear = Ear::new();
        ear.hear(r#"{"ev":"PreToolUse","tool":"Bash","id":"t1"}"#);
        ear.activity.since_change_ms = 60_000.0;
        let asked = ear.hear(r#"{"ev":"PermissionRequest","tool":"Bash"}"#);
        assert_eq!(asked.since_change_ms, 0.0);
        ear.activity.since_change_ms = 60_000.0;
        let answered = ear.hear(r#"{"ev":"PostToolUse","tool":"Bash","id":"t1"}"#);
        assert_eq!(answered.since_change_ms, 0.0);
        // A call nobody was asked about finishes quietly.
        ear.hear(r#"{"ev":"PreToolUse","tool":"Bash","id":"t2"}"#);
        ear.activity.since_change_ms = 60_000.0;
        let quiet = ear.hear(r#"{"ev":"PostToolUse","tool":"Bash","id":"t2"}"#);
        assert_eq!(quiet.since_change_ms, 60_000.0);
    }

    #[test]
    fn subagents_run_from_start_to_stop_and_a_silent_one_is_dropped() {
        let mut ear = Ear::new();
        ear.hear(r#"{"ev":"SubagentStart","agent":"a1"}"#);
        let two = ear.hear(r#"{"ev":"SubagentStart","agent":"a2"}"#);
        assert_eq!(two.running_agents, 2);
        assert!(two.is_working());
        // A subagent's work counts at half, and its effort isn't the session's.
        let before = two.heat;
        let ran =
            ear.hear(r#"{"ev":"PreToolUse","agent":"a1","tool":"Bash","id":"t1","effort":"low"}"#);
        assert!((ran.heat - before - SUBAGENT_WEIGHT).abs() < 1e-9);
        assert_eq!(ran.floor, 3.0);
        let one = ear.hear(r#"{"ev":"SubagentStop","agent":"a1"}"#);
        assert_eq!((one.running_agents, one.tools_in_flight), (1, 0));
        // a2 never said it stopped: ten minutes of silence and it's gone.
        ear.feed.poll(&mut ear.activity, ear.now + SILENT);
        assert_eq!(ear.activity.running_agents, 0);
    }

    #[test]
    fn streamed_text_and_compactions_are_heard() {
        let mut ear = Ear::new();
        ear.hear(r#"{"ev":"MessageDisplay","chars":80}"#);
        assert_eq!(ear.feed.streamed_at, Some(ear.now));
        ear.activity.tick(1.0);
        assert!(ear.activity.heat > 0.0);
        let compacted = ear.hear(r#"{"ev":"PostCompact","trigger":"auto"}"#);
        assert_eq!(compacted.heat, 0.0);
        assert_eq!(compacted.tint(), Tint::Smoke);
    }

    #[test]
    fn companions_keep_identity_wait_for_their_own_calls_and_leave_after_completion() {
        let mut ear = Ear::new();
        ear.hear(r#"{"ev":"UserPromptSubmit"}"#);
        ear.hear(r#"{"ev":"SubagentStart","agent":"a","agent_type":"Explore"}"#);
        ear.hear(r#"{"ev":"SubagentStart","agent":"b"}"#);
        ear.hear(r#"{"ev":"PreToolUse","tool":"Bash","id":"x","agent":"a","fingerprint":"one"}"#);
        ear.hear(r#"{"ev":"PreToolUse","tool":"Bash","id":"y","agent":"a","fingerprint":"two"}"#);
        ear.hear(r#"{"ev":"PermissionRequest","tool":"Bash","agent":"a","fingerprint":"one"}"#);
        assert!(ear.activity.waiting());
        assert_eq!(ear.activity.agents[0].state, AgentState::Waiting);
        assert_eq!(ear.activity.agents[1].state, AgentState::Working);
        ear.hear(r#"{"ev":"PostToolUse","tool":"Bash","id":"y","agent":"a"}"#);
        assert!(
            ear.activity.waiting(),
            "another call cannot answer this prompt"
        );
        ear.hear(r#"{"ev":"PostToolUse","tool":"Bash","id":"x","agent":"a"}"#);
        assert!(!ear.activity.waiting());
        ear.now += Duration::from_secs(21);
        ear.feed.poll(&mut ear.activity, ear.now);
        assert_eq!(ear.activity.agents[0].state, AgentState::Idle);
        ear.hear(r#"{"ev":"StopFailure","agent":"a","error":"failed"}"#);
        assert!(
            ear.activity.is_turn_active,
            "a subagent cannot end the main turn"
        );
        assert_eq!(ear.activity.agents[0].state, AgentState::Done);
        assert!(!ear.activity.agents[0].ok);
        ear.hear(r#"{"ev":"SubagentStop","agent":"a"}"#);
        ear.hear(r#"{"ev":"PostToolUseFailure","tool":"Bash","id":"x","agent":"a"}"#);
        assert_eq!(ear.activity.agents[0].state, AgentState::Done);
        assert!(
            !ear.activity.agents[0].ok,
            "late completion must not revive or erase failure"
        );
        assert_eq!(ear.activity.agents[0].kind, "Explore");
        assert_eq!(ear.activity.running_agents, 1);
        ear.now += Duration::from_secs(6);
        ear.feed.poll(&mut ear.activity, ear.now);
        assert_eq!(ear.activity.agents.len(), 1);
        assert_eq!(ear.activity.agents[0].id, "b");
    }

    #[test]
    fn questions_and_elicitations_remain_waiting_until_all_are_answered() {
        let mut ear = Ear::new();
        ear.hear(r#"{"ev":"PreToolUse","tool":"AskUserQuestion","id":"q"}"#);
        ear.hear(r#"{"ev":"Elicitation","agent":"a"}"#);
        ear.hear(r#"{"ev":"Elicitation","agent":"a"}"#);
        ear.hear(r#"{"ev":"ElicitationResult","agent":"a"}"#);
        ear.hear(r#"{"ev":"PostToolUse","tool":"AskUserQuestion","id":"q"}"#);
        assert!(ear.activity.waiting());
        ear.hear(r#"{"ev":"ElicitationResult","agent":"a"}"#);
        assert!(!ear.activity.waiting());
    }
}
