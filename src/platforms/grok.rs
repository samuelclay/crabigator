//! Grok Build TUI platform implementation.
//!
//! Grok is a full-screen TUI. Crabigator strips its alternate-screen switch
//! (see `uses_alt_screen`) so it paints inside the scroll region. Session
//! state is read from `~/.grok/sessions/<encoded-cwd>/<id>/` — `events.jsonl`
//! for the live state machine and `updates.jsonl` for the transcript.

pub(crate) mod acp;
mod events;
pub mod transcript;

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use anyhow::Result;
use serde_json::Value;

use super::{Platform, PlatformKind, PlatformStats};
use events::EventState;

/// How long the tracked session must be quiet before an in-pane `/new` or
/// `/resume` is allowed to steal the follow.
const RESUME_QUIET: Duration = Duration::from_secs(120);
const RESUME_FRESH: Duration = Duration::from_secs(300);

pub struct GrokPlatform {
    sessions_dir: PathBuf,
    mirror_dir: PathBuf,
    state: Mutex<GrokState>,
    continue_requested: AtomicBool,
}

struct GrokState {
    inner: EventState,
    session_dir: Option<PathBuf>,
    native_session_dir: Option<PathBuf>,
    native_mtime_at_switch: Option<SystemTime>,
    resume_followed: bool,
    last_user_input: Option<SystemTime>,
    last_scan: Option<SystemTime>,
    app_start: SystemTime,
    events_offset: u64,
    updates_offset: u64,
    linked_session: Option<String>,
}

impl Default for GrokState {
    fn default() -> Self {
        Self {
            inner: EventState::default(),
            session_dir: None,
            native_session_dir: None,
            native_mtime_at_switch: None,
            resume_followed: false,
            last_user_input: None,
            last_scan: None,
            app_start: SystemTime::now(),
            events_offset: 0,
            updates_offset: 0,
            linked_session: None,
        }
    }
}

impl GrokState {
    fn switch_to(&mut self, session_dir: PathBuf) {
        if self.session_dir.as_ref() == Some(&session_dir) {
            return;
        }
        self.inner = EventState::default();
        self.events_offset = 0;
        self.updates_offset = 0;
        self.session_dir = Some(session_dir);
        self.linked_session = None;
    }
}

impl GrokPlatform {
    pub fn new() -> Self {
        Self::with_dirs(grok_home().join("sessions"), PathBuf::from("/tmp"))
    }

    fn with_dirs(sessions_dir: PathBuf, mirror_dir: PathBuf) -> Self {
        Self {
            sessions_dir,
            mirror_dir,
            state: Mutex::new(GrokState::default()),
            continue_requested: AtomicBool::new(false),
        }
    }

    fn resolve_session_dir(
        &self,
        cwd: &str,
        state: &mut GrokState,
        continue_requested: bool,
    ) -> Option<PathBuf> {
        let group = find_group_dir(&self.sessions_dir, cwd);
        if !group.is_dir() {
            return state.session_dir.clone().filter(|path| path.exists());
        }

        if let Some(path) = state.session_dir.clone().filter(|path| path.exists()) {
            if let Some(switched) = self.follow_resumed_session(&group, state, &path) {
                return Some(switched);
            }
            return Some(path);
        }

        if !should_rescan(state) {
            return state.session_dir.clone();
        }
        state.last_scan = Some(SystemTime::now());

        let threshold = state
            .app_start
            .checked_sub(Duration::from_secs(2))
            .unwrap_or(state.app_start);
        choose_session(&group, threshold, state.app_start, continue_requested)
            .or_else(|| state.session_dir.clone())
    }

    fn follow_resumed_session(
        &self,
        group: &Path,
        state: &mut GrokState,
        tracked: &Path,
    ) -> Option<PathBuf> {
        let now = SystemTime::now();

        if state.resume_followed {
            if let (Some(native), Some(mtime_at_switch)) = (
                state.native_session_dir.as_ref(),
                state.native_mtime_at_switch,
            ) {
                if session_mtime(native).is_some_and(|mtime| mtime > mtime_at_switch) {
                    state.resume_followed = false;
                    return state.native_session_dir.clone();
                }
            }
        }

        let tracked_mtime = session_mtime(tracked)?;
        if now.duration_since(tracked_mtime).unwrap_or_default() < RESUME_QUIET {
            return None;
        }
        let input_since_tracked = state
            .last_user_input
            .is_some_and(|input| input > tracked_mtime);
        if !input_since_tracked {
            return None;
        }
        if !should_rescan(state) {
            return None;
        }
        state.last_scan = Some(now);

        let claimed = self.sessions_claimed_by_other();
        let best = list_sessions(group)
            .into_iter()
            .filter(|candidate| {
                candidate.path != *tracked
                    && !claimed.contains(&candidate.path)
                    && candidate.modified > tracked_mtime
                    && candidate.modified >= state.app_start
                    && now.duration_since(candidate.modified).unwrap_or_default() <= RESUME_FRESH
            })
            .max_by_key(|candidate| candidate.modified)?;

        if !state.resume_followed {
            state.native_session_dir = Some(tracked.to_path_buf());
            state.native_mtime_at_switch = Some(tracked_mtime);
        }
        state.resume_followed = true;
        Some(best.path)
    }

    fn sessions_claimed_by_other(&self) -> HashSet<PathBuf> {
        let own_id = std::env::var("CRABIGATOR_SESSION_ID").unwrap_or_default();
        let now = SystemTime::now();
        let mut claimed = HashSet::new();
        let Ok(entries) = fs::read_dir(&self.mirror_dir) else {
            return claimed;
        };
        for entry in entries.flatten() {
            if !entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.starts_with("crabigator-"))
            {
                continue;
            }
            let mirror = entry.path().join("inspect.json");
            let fresh = fs::metadata(&mirror)
                .and_then(|m| m.modified())
                .is_ok_and(|mtime| now.duration_since(mtime).unwrap_or_default() <= RESUME_FRESH);
            if !fresh {
                continue;
            }
            let Ok(content) = fs::read_to_string(&mirror) else {
                continue;
            };
            let Ok(value) = serde_json::from_str::<Value>(&content) else {
                continue;
            };
            let session_id = value.get("session_id").and_then(|v| v.as_str());
            if !own_id.is_empty() && session_id == Some(own_id.as_str()) {
                continue;
            }
            if let Some(transcript) = value.get("transcript_path").and_then(|v| v.as_str()) {
                if let Some(parent) = Path::new(transcript).parent() {
                    claimed.insert(parent.to_path_buf());
                }
            }
        }
        claimed
    }
}

impl Default for GrokPlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for GrokPlatform {
    fn kind(&self) -> PlatformKind {
        PlatformKind::Grok
    }

    fn command(&self) -> &'static str {
        PlatformKind::Grok.command()
    }

    fn spawn_args(&self, user_args: Vec<String>) -> Vec<String> {
        if user_args
            .iter()
            .any(|arg| matches!(arg.as_str(), "--resume" | "--continue" | "-c" | "-r"))
        {
            self.continue_requested.store(true, Ordering::SeqCst);
        }
        user_args
    }

    fn uses_alt_screen(&self) -> bool {
        true
    }

    fn ensure_hooks_installed(&self) -> Result<()> {
        Ok(())
    }

    fn note_user_input(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.last_user_input = Some(SystemTime::now());
    }

    fn load_stats(&self, cwd: &str) -> Result<PlatformStats> {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        let continue_requested = self.continue_requested.load(Ordering::SeqCst);
        let Some(session_dir) = self.resolve_session_dir(cwd, &mut state, continue_requested)
        else {
            return Ok(PlatformStats::default());
        };

        state.switch_to(session_dir.clone());

        state.events_offset = consume_jsonl(
            &session_dir.join("events.jsonl"),
            state.events_offset,
            &mut state.inner,
        );
        state.updates_offset = consume_jsonl(
            &session_dir.join("updates.jsonl"),
            state.updates_offset,
            &mut state.inner,
        );

        if let Some(summary) = read_summary(&session_dir) {
            if state.inner.stats.model.is_none() {
                if let Some(model) = summary
                    .get("current_model_id")
                    .and_then(Value::as_str)
                    .filter(|s| !s.is_empty())
                {
                    state.inner.stats.model = Some(model.to_string());
                }
            }
            let session_id = summary
                .pointer("/info/id")
                .and_then(Value::as_str)
                .map(str::to_string)
                .or_else(|| {
                    session_dir
                        .file_name()
                        .and_then(|name| name.to_str())
                        .map(str::to_string)
                });
            if let Some(id) = session_id {
                if state.linked_session.as_ref() != Some(&id) {
                    link_session_dir(&id);
                    state.linked_session = Some(id);
                }
            }
        }

        let mut stats = state.inner.stats.clone();
        stats.transcript_path = Some(
            session_dir
                .join("updates.jsonl")
                .to_string_lossy()
                .into_owned(),
        );
        stats.native_session_id = state.linked_session.clone();
        Ok(stats)
    }
}

fn grok_home() -> PathBuf {
    if let Ok(home) = std::env::var("GROK_HOME") {
        if !home.is_empty() {
            return PathBuf::from(home);
        }
    }
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("."))
        .join(".grok")
}

/// Percent-encode a cwd the way Grok names session group directories.
fn encode_cwd(cwd: &str) -> String {
    let mut out = String::new();
    for &b in cwd.as_bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                out.push(b as char);
            }
            _ => out.push_str(&format!("%{b:02X}")),
        }
    }
    out
}

fn find_group_dir(sessions_dir: &Path, cwd: &str) -> PathBuf {
    let encoded = encode_cwd(cwd);
    let direct = sessions_dir.join(&encoded);
    if direct.is_dir() {
        return direct;
    }
    if let Ok(entries) = fs::read_dir(sessions_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let cwd_file = path.join(".cwd");
            if let Ok(saved) = fs::read_to_string(&cwd_file) {
                if saved.trim() == cwd {
                    return path;
                }
            }
        }
    }
    direct
}

struct SessionCandidate {
    path: PathBuf,
    modified: SystemTime,
    created: Option<SystemTime>,
}

fn list_sessions(group: &Path) -> Vec<SessionCandidate> {
    let Ok(entries) = fs::read_dir(group) else {
        return Vec::new();
    };
    let mut candidates = Vec::new();
    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let Some(modified) = session_mtime(&path) else {
            continue;
        };
        candidates.push(SessionCandidate {
            created: session_created_at(&path),
            path,
            modified,
        });
    }
    candidates
}

fn session_mtime(dir: &Path) -> Option<SystemTime> {
    for name in ["updates.jsonl", "events.jsonl", "summary.json"] {
        if let Ok(mtime) = fs::metadata(dir.join(name)).and_then(|m| m.modified()) {
            return Some(mtime);
        }
    }
    fs::metadata(dir).and_then(|m| m.modified()).ok()
}

fn session_created_at(dir: &Path) -> Option<SystemTime> {
    let summary = read_summary(dir)?;
    let created = summary.get("created_at").and_then(Value::as_str)?;
    let parsed = chrono::DateTime::parse_from_rfc3339(created).ok()?;
    Some(SystemTime::UNIX_EPOCH + Duration::from_secs(parsed.timestamp().max(0) as u64))
}

fn read_summary(dir: &Path) -> Option<Value> {
    let content = fs::read_to_string(dir.join("summary.json")).ok()?;
    serde_json::from_str(&content).ok()
}

fn choose_session(
    group: &Path,
    threshold: SystemTime,
    app_start: SystemTime,
    continue_requested: bool,
) -> Option<PathBuf> {
    let candidates = list_sessions(group);
    if continue_requested {
        return candidates
            .iter()
            .filter(|candidate| candidate.modified >= threshold)
            .max_by_key(|candidate| candidate.modified)
            .or_else(|| candidates.iter().max_by_key(|candidate| candidate.modified))
            .map(|candidate| candidate.path.clone());
    }

    candidates
        .into_iter()
        .filter(|candidate| candidate.created.unwrap_or(candidate.modified) >= threshold)
        .min_by_key(|candidate| {
            abs_duration(candidate.created.unwrap_or(candidate.modified), app_start)
        })
        .map(|candidate| candidate.path)
}

fn abs_duration(left: SystemTime, right: SystemTime) -> Duration {
    left.duration_since(right)
        .unwrap_or_else(|_| right.duration_since(left).unwrap_or_default())
}

fn should_rescan(state: &GrokState) -> bool {
    let Some(last_scan) = state.last_scan else {
        return true;
    };
    last_scan.elapsed().unwrap_or(Duration::from_secs(0)) >= Duration::from_secs(2)
}

fn consume_jsonl(path: &Path, offset: u64, state: &mut EventState) -> u64 {
    let Ok(file) = File::open(path) else {
        return offset;
    };
    let Ok(file_len) = file.metadata().map(|m| m.len()) else {
        return offset;
    };
    if offset >= file_len {
        return offset;
    }
    let mut reader = BufReader::new(file);
    if reader.seek(SeekFrom::Start(offset)).is_err() {
        return offset;
    }
    let mut current = offset;
    let mut line = String::new();
    loop {
        line.clear();
        let Ok(n) = reader.read_line(&mut line) else {
            break;
        };
        if n == 0 {
            break;
        }
        // A live append can hit EOF mid-line. Leave the offset at the start
        // of that line so the next poll retries once it is complete.
        if !line.ends_with('\n') {
            break;
        }
        current += n as u64;
        if let Ok(value) = serde_json::from_str::<Value>(line.trim_end()) {
            state.apply_line(&value);
        }
    }
    current
}

fn link_session_dir(grok_session: &str) {
    #[cfg(not(unix))]
    let _ = grok_session;
    #[cfg(unix)]
    {
        let Ok(session_id) = std::env::var("CRABIGATOR_SESSION_ID") else {
            return;
        };
        if session_id.is_empty() || grok_session.is_empty() {
            return;
        }
        let target = PathBuf::from(format!("/tmp/crabigator-{session_id}"));
        let link = PathBuf::from(format!("/tmp/crabigator-{grok_session}"));
        if !target.is_dir() || link.exists() {
            return;
        }
        let _ = std::os::unix::fs::symlink(&target, &link);
    }
}

#[cfg(test)]
mod tests {
    use std::io::Write;

    use super::*;

    #[test]
    fn encodes_cwd_like_grok_group_dirs() {
        assert_eq!(
            encode_cwd("/Users/sclay/projects/crabigator"),
            "%2FUsers%2Fsclay%2Fprojects%2Fcrabigator"
        );
        assert_eq!(encode_cwd("abc-._~XYZ012"), "abc-._~XYZ012");
    }

    #[test]
    fn spawn_args_mark_continue_and_pass_through() {
        let platform = GrokPlatform::new();
        let args = platform.spawn_args(vec![
            "--yolo".to_string(),
            "-m".to_string(),
            "grok-4.6".to_string(),
        ]);
        assert_eq!(args, vec!["--yolo", "-m", "grok-4.6"]);
        assert!(!platform.continue_requested.load(Ordering::SeqCst));

        let args = platform.spawn_args(vec!["--resume".to_string(), "abc".to_string()]);
        assert_eq!(args, vec!["--resume", "abc"]);
        assert!(platform.continue_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn finds_group_via_cwd_file() {
        let dir = tempfile::tempdir().unwrap();
        let group = dir.path().join("slug-hash");
        fs::create_dir(&group).unwrap();
        fs::write(group.join(".cwd"), "/very/long/path\n").unwrap();
        let found = find_group_dir(dir.path(), "/very/long/path");
        assert_eq!(found, group);
    }

    #[test]
    fn load_stats_reads_a_fresh_session() {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions");
        let cwd = "/tmp/demo";
        let group = sessions.join(encode_cwd(cwd));
        let session = group.join("01abc-session");
        fs::create_dir_all(&session).unwrap();
        let created = chrono::Utc::now().to_rfc3339();
        fs::write(
            session.join("summary.json"),
            format!(
                r#"{{"info":{{"id":"01abc-session","cwd":"{cwd}"}},"created_at":"{created}","current_model_id":"grok-4.6"}}"#
            ),
        )
        .unwrap();
        let mut events = File::create(session.join("events.jsonl")).unwrap();
        writeln!(
            events,
            r#"{{"type":"turn_started","session_id":"01abc-session","turn_number":0,"model_id":"grok-4.6","session_relationship":"primary"}}"#
        )
        .unwrap();
        writeln!(events, r#"{{"type":"turn_ended","outcome":"completed"}}"#).unwrap();

        let platform = GrokPlatform::with_dirs(sessions, root.path().join("mirrors"));
        fs::create_dir_all(root.path().join("mirrors")).unwrap();
        let stats = platform.load_stats(cwd).unwrap();
        assert_eq!(stats.prompts, 1);
        assert_eq!(stats.completions, 1);
        assert_eq!(stats.model.as_deref(), Some("grok-4.6"));
        assert!(stats
            .transcript_path
            .as_deref()
            .is_some_and(|path| path.ends_with("updates.jsonl")));
    }

    #[test]
    fn incomplete_jsonl_line_is_retried_on_the_next_read() {
        let root = tempfile::tempdir().unwrap();
        let sessions = root.path().join("sessions");
        let cwd = "/tmp/partial";
        let group = sessions.join(encode_cwd(cwd));
        let session = group.join("01partial");
        fs::create_dir_all(&session).unwrap();
        let created = chrono::Utc::now().to_rfc3339();
        fs::write(
            session.join("summary.json"),
            format!(r#"{{"info":{{"id":"01partial","cwd":"{cwd}"}},"created_at":"{created}"}}"#),
        )
        .unwrap();
        let events_path = session.join("events.jsonl");
        let mut events = File::create(&events_path).unwrap();
        write!(
            events,
            r#"{{"type":"turn_started","session_id":"01partial","turn_number":0,"session_relationship":"primary","model_id":"grok-4.6"}}"#
        )
        .unwrap();
        events.flush().unwrap();

        let platform = GrokPlatform::with_dirs(sessions, root.path().join("mirrors"));
        fs::create_dir_all(root.path().join("mirrors")).unwrap();
        let stats = platform.load_stats(cwd).unwrap();
        assert_eq!(stats.prompts, 0);

        writeln!(events).unwrap();
        events.flush().unwrap();
        let stats = platform.load_stats(cwd).unwrap();
        assert_eq!(stats.prompts, 1);
        assert_eq!(stats.model.as_deref(), Some("grok-4.6"));
    }
}
