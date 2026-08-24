//! opencode platform implementation
//!
//! opencode keeps session history in SQLite and has no hook system, but its
//! TUI always runs a local HTTP server. Crabigator passes `--port` when
//! spawning the CLI, follows the server's SSE `/event` stream to derive
//! session state, stats, permissions, and the model, and writes a normalized
//! transcript log that feeds scrollback, recaps, and PR tracking.

mod events;
mod sse;
pub mod transcript;

use std::collections::HashMap;
use std::net::TcpListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use serde_json::Value;

use super::{Platform, PlatformKind, PlatformStats};
use events::OpencodeState;

pub struct OpencodePlatform {
    /// Port the spawned opencode server listens on
    port: u16,
    shared: Arc<Mutex<OpencodeState>>,
    shutdown: Arc<AtomicBool>,
    listener_started: AtomicBool,
    /// Whether the user asked to continue an existing session, which allows
    /// adopting (and backfilling) the newest session from the server list
    continue_requested: Arc<AtomicBool>,
    transcript_path: PathBuf,
}

impl OpencodePlatform {
    pub fn new() -> Self {
        let port = TcpListener::bind("127.0.0.1:0")
            .and_then(|listener| listener.local_addr())
            .map(|addr| addr.port())
            .unwrap_or_else(|_| 20000 + (std::process::id() % 20000) as u16);

        let session_id = std::env::var("CRABIGATOR_SESSION_ID")
            .ok()
            .filter(|id| !id.is_empty())
            .unwrap_or_else(|| std::process::id().to_string());

        Self {
            port,
            shared: Arc::new(Mutex::new(OpencodeState::default())),
            shutdown: Arc::new(AtomicBool::new(false)),
            listener_started: AtomicBool::new(false),
            continue_requested: Arc::new(AtomicBool::new(false)),
            transcript_path: PathBuf::from(format!("/tmp/crabigator-opencode-{session_id}.jsonl")),
        }
    }

    fn start_listener(&self) {
        if self.listener_started.swap(true, Ordering::SeqCst) {
            return;
        }
        let port = self.port;
        let shared = Arc::clone(&self.shared);
        let shutdown = Arc::clone(&self.shutdown);
        let continue_requested = Arc::clone(&self.continue_requested);
        let transcript_path = self.transcript_path.clone();
        thread::spawn(move || {
            listener_loop(port, shared, shutdown, continue_requested, transcript_path);
        });
    }
}

impl Default for OpencodePlatform {
    fn default() -> Self {
        Self::new()
    }
}

impl Platform for OpencodePlatform {
    fn kind(&self) -> PlatformKind {
        PlatformKind::Opencode
    }

    fn command(&self) -> &'static str {
        PlatformKind::Opencode.command()
    }

    fn spawn_args(&self, user_args: Vec<String>) -> Vec<String> {
        let mut args = vec!["--port".to_string(), self.port.to_string()];
        for arg in user_args {
            match arg.as_str() {
                // opencode has no --resume flag; continuing the last session
                // is the closest behavior (ctrl+x l picks sessions in-app).
                "--resume" | "--continue" | "-c" => {
                    self.continue_requested.store(true, Ordering::SeqCst);
                    args.push("--continue".to_string());
                }
                "--session" | "-s" => {
                    self.continue_requested.store(true, Ordering::SeqCst);
                    args.push(arg);
                }
                _ => args.push(arg),
            }
        }
        args
    }

    fn uses_alt_screen(&self) -> bool {
        true
    }

    fn ensure_hooks_installed(&self) -> Result<()> {
        // opencode needs no hooks; state flows through its HTTP server.
        Ok(())
    }

    fn load_stats(&self, _cwd: &str) -> Result<PlatformStats> {
        self.start_listener();
        let state = self.shared.lock().unwrap_or_else(|p| p.into_inner());
        let mut stats = state.stats.clone();
        stats.transcript_path = Some(self.transcript_path.to_string_lossy().to_string());
        Ok(stats)
    }

    fn cleanup_stats(&self, _cwd: &str) {
        self.shutdown.store(true, Ordering::SeqCst);
        let _ = std::fs::remove_file(&self.transcript_path);
    }
}

/// Follow the opencode server's event stream until shutdown, reconnecting
/// while the server boots or after connection drops.
fn listener_loop(
    port: u16,
    shared: Arc<Mutex<OpencodeState>>,
    shutdown: Arc<AtomicBool>,
    continue_requested: Arc<AtomicBool>,
    transcript_path: PathBuf,
) {
    let mut catalog_loaded = false;
    let mut backfill_done = false;
    let mut linked_session: Option<String> = None;

    while !shutdown.load(Ordering::SeqCst) {
        // The model catalog maps ids to display names (Ox Alpha). Loading it
        // also doubles as the "server is up" probe.
        if !catalog_loaded {
            match sse::get_json(port, "/config/providers") {
                Ok(catalog) => {
                    let names = extract_model_names(&catalog);
                    let mut state = shared.lock().unwrap_or_else(|p| p.into_inner());
                    state.model_names = names;
                    catalog_loaded = true;
                }
                Err(_) => {
                    thread::sleep(Duration::from_millis(500));
                    continue;
                }
            }
        }

        // Continuing an existing session emits no session.created event, so
        // adopt the newest top-level session and backfill its history once.
        if !backfill_done {
            backfill_done = true;
            if continue_requested.load(Ordering::SeqCst) {
                backfill_continued_session(port, &shared, &transcript_path);
            }
        }

        let _ = sse::stream_events(port, "/event", &shutdown, &mut |data| {
            let Ok(event) = serde_json::from_str::<Value>(data) else {
                return;
            };
            let (entries, main_session) = {
                let mut state = shared.lock().unwrap_or_else(|p| p.into_inner());
                let entries = events::apply_event(&mut state, &event);
                (entries, state.main_session.clone())
            };
            if !entries.is_empty() {
                let _ = transcript::append_entries(&transcript_path, &entries);
            }
            if main_session != linked_session {
                if let Some(ref session) = main_session {
                    link_session_dir(session);
                }
                linked_session = main_session;
            }
        });

        if shutdown.load(Ordering::SeqCst) {
            break;
        }
        // Server not up yet, stream dropped, or read timed out — retry.
        thread::sleep(Duration::from_millis(1000));
    }
}

/// Adopt the newest top-level session from the server and replay its
/// message history into stats and the transcript log.
fn backfill_continued_session(
    port: u16,
    shared: &Arc<Mutex<OpencodeState>>,
    transcript_path: &Path,
) {
    {
        let state = shared.lock().unwrap_or_else(|p| p.into_inner());
        if state.main_session.is_some() {
            return;
        }
    }
    let Ok(sessions) = sse::get_json(port, "/session") else {
        return;
    };
    let Some(list) = sessions.as_array() else {
        return;
    };
    let newest = list
        .iter()
        .filter(|s| s.get("parentID").and_then(Value::as_str).is_none())
        .max_by_key(|s| {
            s.get("time")
                .and_then(|t| t.get("updated"))
                .and_then(Value::as_i64)
                .unwrap_or(0)
        });
    let Some(session_id) = newest
        .and_then(|s| s.get("id"))
        .and_then(Value::as_str)
        .map(str::to_string)
    else {
        return;
    };

    let messages = sse::get_json(port, &format!("/session/{session_id}/message")).ok();
    let entries = {
        let mut state = shared.lock().unwrap_or_else(|p| p.into_inner());
        state.main_session = Some(session_id);
        match messages {
            Some(ref messages) => events::backfill_messages(&mut state, messages),
            None => Vec::new(),
        }
    };
    let _ = transcript::append_entries(transcript_path, &entries);
}

/// Pull `model id -> display name` pairs out of the provider catalog.
fn extract_model_names(catalog: &Value) -> HashMap<String, String> {
    let mut names = HashMap::new();
    let Some(providers) = catalog.get("providers").and_then(Value::as_array) else {
        return names;
    };
    for provider in providers {
        let Some(models) = provider.get("models").and_then(Value::as_object) else {
            continue;
        };
        for (model_id, model) in models {
            if let Some(name) = model.get("name").and_then(Value::as_str) {
                names.insert(model_id.clone(), name.to_string());
            }
        }
    }
    names
}

/// Symlink `/tmp/crabigator-{opencode_session}` to the crabigator session
/// directory, mirroring the Claude Code conversation-UUID symlink so any
/// session id resolves to the session directory.
fn link_session_dir(opencode_session: &str) {
    #[cfg(unix)]
    {
        let Ok(session_id) = std::env::var("CRABIGATOR_SESSION_ID") else {
            return;
        };
        if session_id.is_empty() || opencode_session.is_empty() {
            return;
        }
        let target = PathBuf::from(format!("/tmp/crabigator-{session_id}"));
        let link = PathBuf::from(format!("/tmp/crabigator-{opencode_session}"));
        if !target.is_dir() || link.exists() {
            return;
        }
        let _ = std::os::unix::fs::symlink(&target, &link);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn spawn_args_injects_port_and_translates_resume() {
        let platform = OpencodePlatform::new();
        let args = platform.spawn_args(vec!["--resume".to_string()]);
        assert_eq!(args[0], "--port");
        assert_eq!(args[1], platform.port.to_string());
        assert_eq!(args[2], "--continue");
        assert!(platform.continue_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn spawn_args_passes_other_args_through() {
        let platform = OpencodePlatform::new();
        let args = platform.spawn_args(vec!["--model".to_string(), "opencode/foo".to_string()]);
        assert_eq!(
            args[2..],
            ["--model".to_string(), "opencode/foo".to_string()]
        );
        assert!(!platform.continue_requested.load(Ordering::SeqCst));
    }

    #[test]
    fn extracts_model_display_names() {
        let catalog = json!({
            "providers": [{
                "id": "openrouter",
                "models": {
                    "stealth/ox-alpha": {"id": "stealth/ox-alpha", "name": "Ox Alpha"}
                }
            }]
        });
        let names = extract_model_names(&catalog);
        assert_eq!(
            names.get("stealth/ox-alpha").map(String::as_str),
            Some("Ox Alpha")
        );
    }
}
