//! Codex CLI platform implementation
//!
//! Reads Codex session logs under ~/.codex/sessions and derives session stats.

mod log_parser;

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
    parse_timestamp, reset_state, set_last_updated, update_from_log,
    CodexState, SessionCandidate, SessionMetaInfo,
};

pub struct CodexPlatform {
    sessions_dir: PathBuf,
    state: Mutex<CodexState>,
}

impl CodexPlatform {
    pub fn new() -> Self {
        let home = dirs::home_dir().expect("Could not find home directory");
        let sessions_dir = home.join(".codex").join("sessions");

        Self {
            sessions_dir,
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

        if let (Some(path), Some(session_start)) =
            (state.session_path.as_ref(), state.session_started_at)
        {
            if path.exists() && session_start >= threshold {
                return Ok(Some((path.clone(), Some(session_start))));
            }
        }

        if !Self::should_rescan(state) {
            return Ok(state.session_path.clone().map(|path| (path, state.session_started_at)));
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
                let session_start =
                    payload.get("timestamp").and_then(|v| v.as_str()).and_then(parse_timestamp);
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
        candidates
            .iter()
            .max_by_key(|candidate| candidate.modified)
            .map(|candidate| {
                (
                    candidate.path.clone(),
                    candidate.session_start.unwrap_or(candidate.modified),
                )
            })
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
        while reader.read_line(&mut line)? > 0 {
            update_from_log(&mut state, line.trim_end());
            saw_update = true;
            line.clear();
        }

        if saw_update {
            state.session_offset = reader.stream_position()?;
            set_last_updated(&mut state);
        }

        Ok(state.stats.clone())
    }
}
