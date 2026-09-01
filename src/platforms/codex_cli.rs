//! Codex CLI platform implementation
//!
//! Reads Codex session logs under ~/.codex/sessions and derives session stats.

mod log_parser;
pub mod transcript;

use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use chrono::{Datelike, Local};
use serde_json::Value;

use super::{Platform, PlatformKind, PlatformStats};
use log_parser::{
    parse_timestamp, reset_state, set_last_updated, update_from_log, CodexState, SessionCandidate,
    SessionMetaInfo,
};

/// How long the tracked rollout must be quiet before crabigator considers
/// that the pane may have switched to a resumed conversation.
const RESUME_QUIET: Duration = Duration::from_secs(120);

/// A switch candidate counts only while it is being actively written. The
/// follow scan runs every couple of seconds, so any real resume is observed
/// well inside this window; it exists to reject long-dead rollouts.
const RESUME_FRESH: Duration = Duration::from_secs(300);

/// Resumed conversations keep appending to the rollout in the date directory
/// where they were created, so the follow scan looks further back than the
/// two days the launch scan covers.
const RESUME_SCAN_DAYS: i64 = 30;

/// The filters a rollout must pass to count as an in-pane conversation
/// switch, gathered once per follow scan so every candidate is judged
/// against the same clock reading.
struct ResumeScan<'a> {
    cwd: &'a str,
    tracked: &'a Path,
    tracked_mtime: SystemTime,
    app_start: SystemTime,
    now: SystemTime,
}

pub struct CodexPlatform {
    sessions_dir: PathBuf,
    /// Where other live sessions publish their mirrors (`/tmp` outside tests).
    /// The resume-follow scan reads them to skip rollouts another pane owns.
    mirror_dir: PathBuf,
    state: Mutex<CodexState>,
}

impl CodexPlatform {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        let sessions_dir = home.join(".codex").join("sessions");

        Self {
            sessions_dir,
            mirror_dir: PathBuf::from("/tmp"),
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

        if let Some(path) = state.session_path.clone().filter(|path| path.exists()) {
            // A conversation adopted after an in-pane resume predates app
            // start, so the start-time lock would reject it. Hold it
            // explicitly; otherwise hold the launch rollout while its start
            // time is still current.
            let keep_tracked = state.resume_followed
                || matches!(state.session_started_at, Some(start) if start >= threshold);
            if keep_tracked {
                // The tracked rollout going quiet while another same-cwd
                // rollout is written usually means the user resumed an older
                // conversation inside the pane. Follow the switch so stats,
                // recaps, and PR tracking stay live.
                if let Some(switched) = self.follow_resumed_session(cwd, state, &path) {
                    return Ok(Some(switched));
                }
                return Ok(Some((path, state.session_started_at)));
            }
        } else {
            state.resume_followed = false;
        }

        if !Self::should_rescan(state) {
            return Ok(state
                .session_path
                .clone()
                .map(|path| (path, state.session_started_at)));
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
                let session_start = payload
                    .get("timestamp")
                    .and_then(|v| v.as_str())
                    .and_then(parse_timestamp);
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
        None
    }

    /// Detect an in-pane `/resume`: Codex switches to appending the resumed
    /// conversation's original rollout file and the launch rollout goes
    /// silent, so a session locked to the launch rollout would stop seeing
    /// prompts, recaps, and PR mentions. Once the tracked rollout has been
    /// quiet for [`RESUME_QUIET`], adopt the same-cwd rollout most recently
    /// written during this session. The launch rollout keeps priority: if it
    /// is ever written again (the pane resumed back), re-adopt it immediately.
    fn follow_resumed_session(
        &self,
        cwd: &str,
        state: &mut CodexState,
        tracked: &Path,
    ) -> Option<(PathBuf, Option<SystemTime>)> {
        let now = SystemTime::now();

        if state.resume_followed {
            if let (Some(native), Some(mtime_at_switch)) = (
                state.native_session_path.as_ref(),
                state.native_mtime_at_switch,
            ) {
                let woke = fs::metadata(native)
                    .and_then(|m| m.modified())
                    .is_ok_and(|mtime| mtime > mtime_at_switch);
                if woke {
                    let native = native.clone();
                    state.resume_followed = false;
                    return Some((native, state.native_session_started_at));
                }
            }
        }

        let tracked_mtime = fs::metadata(tracked).and_then(|m| m.modified()).ok()?;
        if now.duration_since(tracked_mtime).unwrap_or_default() < RESUME_QUIET {
            return None;
        }

        // An in-pane resume starts with the user typing in this pane. With
        // no input since the tracked rollout last advanced, a same-cwd
        // rollout being written now belongs to a session in another pane.
        let input_since_tracked = state
            .last_user_input
            .is_some_and(|input| input > tracked_mtime);
        if !input_since_tracked {
            return None;
        }

        if !Self::should_rescan(state) {
            return None;
        }
        state.last_scan = Some(now);

        let scan = ResumeScan {
            cwd,
            tracked,
            tracked_mtime,
            app_start: state.app_start,
            now,
        };
        let today = Local::now();
        let mut candidates = Vec::new();
        for offset in 0..RESUME_SCAN_DAYS {
            let dir = self.sessions_dir_for_date(today - chrono::Duration::days(offset));
            self.collect_switch_candidates(&dir, &scan, &mut candidates);
        }
        if candidates.is_empty() {
            return None;
        }
        let claimed = self.rollouts_claimed_by_other_sessions();
        let best = candidates
            .into_iter()
            .filter(|candidate| !claimed.contains(&candidate.path))
            .max_by_key(|candidate| candidate.modified)?;

        if !state.resume_followed {
            state.native_session_path = Some(tracked.to_path_buf());
            state.native_session_started_at = state.session_started_at;
            state.native_mtime_at_switch = Some(tracked_mtime);
        }
        state.resume_followed = true;
        Some((best.path, best.session_start))
    }

    /// Collect rollouts in `dir` that an in-pane conversation switch could
    /// have moved the pane to: same cwd, written during this session, more
    /// recently than the tracked rollout, and still actively written. A date
    /// directory that does not exist reads as empty. The mtime filters run
    /// before the file is opened so the wide scan stays cheap. A concurrent
    /// same-cwd session in another pane can slip through these filters while
    /// this pane idles; the claim check in [`Self::follow_resumed_session`]
    /// drops rollouts another live session tracks, and the native-rollout
    /// priority there recovers from any remaining misadoption as soon as
    /// this pane's own conversation is written again.
    fn collect_switch_candidates(
        &self,
        dir: &Path,
        scan: &ResumeScan,
        candidates: &mut Vec<SessionCandidate>,
    ) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|ext| ext.to_str()) != Some("jsonl")
                || path == scan.tracked
            {
                continue;
            }
            let Ok(modified) = entry.metadata().and_then(|m| m.modified()) else {
                continue;
            };
            if modified <= scan.tracked_mtime
                || modified < scan.app_start
                || scan.now.duration_since(modified).unwrap_or_default() > RESUME_FRESH
            {
                continue;
            }
            let Ok(meta) = Self::session_meta_info(&path, scan.cwd) else {
                continue;
            };
            if meta.matches {
                candidates.push(SessionCandidate {
                    path,
                    modified,
                    session_start: meta.session_start,
                });
            }
        }
    }

    /// Rollouts that other live crabigator sessions already track, read from
    /// the mirrors they publish. A claimed rollout belongs to that pane's
    /// conversation, so this pane must not adopt it as a resume target. Only
    /// fresh mirrors count: the mirror republishes whenever its rollout
    /// advances, so a live claim on an actively written rollout is always
    /// recent, while mirrors of exited sessions go stale and drop out.
    fn rollouts_claimed_by_other_sessions(&self) -> HashSet<PathBuf> {
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
                claimed.insert(PathBuf::from(transcript));
            }
        }
        claimed
    }
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

    fn note_user_input(&self) {
        let mut state = self.state.lock().unwrap_or_else(|p| p.into_inner());
        state.last_user_input = Some(SystemTime::now());
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
            reset_state(&mut state, session_path.clone(), session_started_at);
        } else if state.session_started_at.is_none() && session_started_at.is_some() {
            state.session_started_at = session_started_at;
        }

        let mut file = File::open(&session_path)
            .with_context(|| format!("open {}", session_path.display()))?;
        let file_len = file.metadata().map(|m| m.len()).unwrap_or(0);
        if file_len < state.session_offset {
            let session_started_at = state.session_started_at;
            reset_state(&mut state, session_path.clone(), session_started_at);
        }

        file.seek(SeekFrom::Start(state.session_offset))?;
        let mut reader = BufReader::new(file);
        let mut line = String::new();
        let mut saw_update = false;
        // Adopting a resumed conversation can point this at a rollout with
        // hours of history. Parse at most a few MB per call so catch-up
        // spreads across ticks instead of stalling the UI; the saved offset
        // resumes where this call stopped.
        const MAX_PARSE_BYTES_PER_CALL: u64 = 4 * 1024 * 1024;
        let mut parsed_bytes: u64 = 0;
        while parsed_bytes < MAX_PARSE_BYTES_PER_CALL {
            let read = reader.read_line(&mut line)?;
            if read == 0 {
                break;
            }
            parsed_bytes += read as u64;
            update_from_log(&mut state, line.trim_end());
            saw_update = true;
            line.clear();
        }

        if saw_update {
            state.session_offset = reader.stream_position()?;
            set_last_updated(&mut state);
        }
        state.stats.transcript_path = Some(session_path.to_string_lossy().to_string());
        if state.stats.native_session_id.is_none() {
            if let Some(stem) = session_path.file_stem().and_then(|s| s.to_str()) {
                if let Some(id) = stem.rsplit('-').next().filter(|part| part.len() >= 8) {
                    state.stats.native_session_id = Some(id.to_string());
                }
            }
        }

        Ok(state.stats.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ignores_stale_same_cwd_sessions_when_start_time_is_known() {
        let app_start = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let stale_started = app_start - Duration::from_secs(60);
        let threshold = app_start - Duration::from_secs(2);
        let candidates = vec![SessionCandidate {
            path: PathBuf::from("/tmp/outer-codex.jsonl"),
            modified: app_start + Duration::from_secs(5),
            session_start: Some(stale_started),
        }];

        assert!(CodexPlatform::choose_candidate(&candidates, threshold, app_start).is_none());
    }

    #[test]
    fn accepts_current_session_candidate() {
        let app_start = SystemTime::UNIX_EPOCH + Duration::from_secs(10_000);
        let threshold = app_start - Duration::from_secs(2);
        let path = PathBuf::from("/tmp/current-codex.jsonl");
        let candidates = vec![SessionCandidate {
            path: path.clone(),
            modified: app_start + Duration::from_secs(1),
            session_start: Some(app_start),
        }];

        let selected = CodexPlatform::choose_candidate(&candidates, threshold, app_start)
            .expect("current session should be selected");
        assert_eq!(selected.0, path);
        assert_eq!(selected.1, app_start);
    }

    const CWD: &str = "/Users/example/project";

    fn platform_in(dir: &Path) -> CodexPlatform {
        CodexPlatform {
            sessions_dir: dir.to_path_buf(),
            mirror_dir: dir.join("mirrors"),
            state: Mutex::new(CodexState::default()),
        }
    }

    /// Publish a fake live mirror in the platform's mirror dir claiming
    /// `transcript` for another session.
    fn write_mirror_claim(platform: &CodexPlatform, session_id: &str, transcript: &Path) {
        let dir = platform.mirror_dir.join(format!("crabigator-{session_id}"));
        fs::create_dir_all(&dir).unwrap();
        fs::write(
            dir.join("inspect.json"),
            format!(
                "{{\"session_id\":\"{session_id}\",\"transcript_path\":\"{}\"}}",
                transcript.display()
            ),
        )
        .unwrap();
    }

    fn write_rollout(
        platform: &CodexPlatform,
        days_ago: i64,
        name: &str,
        started: SystemTime,
    ) -> PathBuf {
        let dir = platform.sessions_dir_for_date(Local::now() - chrono::Duration::days(days_ago));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join(name);
        let stamp = chrono::DateTime::<chrono::Utc>::from(started).to_rfc3339();
        fs::write(
            &path,
            format!(
                "{{\"type\":\"session_meta\",\"payload\":{{\"cwd\":\"{CWD}\",\"timestamp\":\"{stamp}\"}}}}\n"
            ),
        )
        .unwrap();
        path
    }

    fn set_mtime(path: &Path, mtime: SystemTime) {
        File::options()
            .write(true)
            .open(path)
            .unwrap()
            .set_modified(mtime)
            .unwrap();
    }

    /// State locked to a launch rollout that has been quiet past RESUME_QUIET,
    /// with user input recent enough for the resume-follow scan to run.
    fn quiet_locked_state(native: &Path, app_start: SystemTime) -> CodexState {
        set_mtime(
            native,
            SystemTime::now() - RESUME_QUIET - Duration::from_secs(60),
        );
        let mut state = CodexState::default();
        state.session_path = Some(native.to_path_buf());
        state.session_started_at = Some(app_start);
        state.app_start = app_start;
        state.last_user_input = Some(SystemTime::now());
        state
    }

    #[test]
    fn follows_in_pane_resume_to_older_rollout() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = platform_in(tmp.path());
        let app_start = SystemTime::now() - Duration::from_secs(3600);

        let native = write_rollout(&platform, 0, "native.jsonl", app_start);
        // Whole seconds: the rollout header stores millisecond precision.
        let resumed_start_secs = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs()
            - 3 * 86_400;
        let resumed_start = SystemTime::UNIX_EPOCH + Duration::from_secs(resumed_start_secs);
        let resumed = write_rollout(&platform, 3, "resumed.jsonl", resumed_start);
        let mut state = quiet_locked_state(&native, app_start);
        set_mtime(&resumed, SystemTime::now() - Duration::from_secs(30));

        let (path, started) = platform
            .resolve_session_path(CWD, &mut state)
            .unwrap()
            .expect("should resolve a session");
        assert_eq!(path, resumed);
        assert_eq!(started, Some(resumed_start));
        assert!(state.resume_followed);
        assert_eq!(state.native_session_path.as_deref(), Some(native.as_path()));
    }

    #[test]
    fn stays_locked_without_user_input_since_rollout_went_quiet() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = platform_in(tmp.path());
        let app_start = SystemTime::now() - Duration::from_secs(3600);

        let native = write_rollout(&platform, 0, "native.jsonl", app_start);
        let resumed = write_rollout(&platform, 3, "resumed.jsonl", app_start);
        let mut state = quiet_locked_state(&native, app_start);
        // No keystrokes reached this pane: nothing could have resumed it.
        state.last_user_input = None;
        set_mtime(&resumed, SystemTime::now() - Duration::from_secs(30));

        let (path, _) = platform
            .resolve_session_path(CWD, &mut state)
            .unwrap()
            .expect("should resolve a session");
        assert_eq!(path, native);
        assert!(!state.resume_followed);
    }

    #[test]
    fn ignores_rollouts_claimed_by_other_live_sessions() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = platform_in(tmp.path());
        let app_start = SystemTime::now() - Duration::from_secs(3600);

        let native = write_rollout(&platform, 0, "native.jsonl", app_start);
        let resumed = write_rollout(&platform, 3, "resumed.jsonl", app_start);
        let mut state = quiet_locked_state(&native, app_start);
        set_mtime(&resumed, SystemTime::now() - Duration::from_secs(30));
        // Another live session's mirror already tracks that rollout.
        write_mirror_claim(&platform, "other-session", &resumed);

        let (path, _) = platform
            .resolve_session_path(CWD, &mut state)
            .unwrap()
            .expect("should resolve a session");
        assert_eq!(path, native);
        assert!(!state.resume_followed);
    }

    #[test]
    fn resume_follow_returns_to_native_rollout_when_it_wakes() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = platform_in(tmp.path());
        let app_start = SystemTime::now() - Duration::from_secs(3600);

        let native = write_rollout(&platform, 0, "native.jsonl", app_start);
        let resumed = write_rollout(&platform, 3, "resumed.jsonl", app_start);
        let mut state = quiet_locked_state(&native, app_start);
        set_mtime(&resumed, SystemTime::now() - Duration::from_secs(30));

        platform.resolve_session_path(CWD, &mut state).unwrap();
        assert!(state.resume_followed);
        state.session_path = Some(resumed);

        // The pane resumed back: the native rollout is written again.
        set_mtime(&native, SystemTime::now());
        let (path, started) = platform
            .resolve_session_path(CWD, &mut state)
            .unwrap()
            .expect("should resolve a session");
        assert_eq!(path, native);
        assert_eq!(started, Some(app_start));
        assert!(!state.resume_followed);
    }

    #[test]
    fn quiet_rollout_without_switch_stays_locked() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = platform_in(tmp.path());
        let app_start = SystemTime::now() - Duration::from_secs(3600);

        let native = write_rollout(&platform, 0, "native.jsonl", app_start);
        let mut state = quiet_locked_state(&native, app_start);

        let (path, _) = platform
            .resolve_session_path(CWD, &mut state)
            .unwrap()
            .expect("should resolve a session");
        assert_eq!(path, native);
        assert!(!state.resume_followed);
    }

    #[test]
    fn ignores_rollouts_not_actively_written() {
        let tmp = tempfile::tempdir().unwrap();
        let platform = platform_in(tmp.path());
        let app_start = SystemTime::now() - Duration::from_secs(3600);

        let native = write_rollout(&platform, 0, "native.jsonl", app_start);
        let mut state = quiet_locked_state(&native, app_start);
        set_mtime(&native, SystemTime::now() - Duration::from_secs(1000));

        // Newer than the tracked rollout, but no longer actively written.
        let stale = write_rollout(&platform, 3, "stale.jsonl", app_start);
        set_mtime(
            &stale,
            SystemTime::now() - RESUME_FRESH - Duration::from_secs(100),
        );

        let (path, _) = platform
            .resolve_session_path(CWD, &mut state)
            .unwrap()
            .expect("should resolve a session");
        assert_eq!(path, native);
        assert!(!state.resume_followed);
    }
}
