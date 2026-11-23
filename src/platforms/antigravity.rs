//! Google Antigravity CLI reports its active conversation and agent state to a
//! status-line command. Each wrapped CLI gets a private snapshot, including
//! when several panes run in the same checkout or switch conversations.
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};

use anyhow::Result;
use serde_json::{json, Value};
use tempfile::TempDir;

use super::{ClaudeMode, ContextUsage, Platform, PlatformKind, PlatformStats, SessionState};

#[derive(Default)]
pub struct Antigravity {
    directory: OnceLock<TempDir>,
    history: Mutex<History>,
}

#[derive(Default)]
struct History {
    stats: PlatformStats,
    offset: u64,
    awaiting_reply: bool,
}

fn quote(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

fn read_json(path: &Path) -> Option<Value> {
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

fn status_settings(settings: &mut Value, dir: &Path, saved: Option<&Value>) {
    let mut line = settings
        .get("statusLine")
        .or(saved)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let file = quote(&dir.join("status.json").to_string_lossy());
    let mut command = format!("umask 077; cat > {file}.tmp && mv {file}.tmp {file}");
    if let Some(previous) = line
        .get("command")
        .and_then(Value::as_str)
        .filter(|s| !s.is_empty())
    {
        // agy expands a leading tilde before handing the command to sh.
        let previous = previous
            .strip_prefix("~/")
            .map(|tail| format!("\"$HOME\"/{tail}"))
            .unwrap_or_else(|| previous.to_owned());
        command.push_str(&format!(" && sh -c {} < {file}", quote(&previous)));
    } else {
        line.insert("stack_with_default".into(), json!(true));
    }
    line.insert("command".into(), json!(command));
    settings["statusLine"] = Value::Object(line);
}

impl Platform for Antigravity {
    fn kind(&self) -> PlatformKind {
        PlatformKind::Antigravity
    }
    fn command(&self) -> &'static str {
        self.kind().command()
    }
    fn ensure_hooks_installed(&self) -> Result<()> {
        Ok(())
    }
    fn uses_alt_screen(&self) -> bool {
        true
    }

    fn spawn_args(&self, user_args: Vec<String>) -> Vec<String> {
        let mut args = Vec::new();
        let mut iter = user_args.into_iter().peekable();
        while let Some(arg) = iter.next() {
            match arg.as_str() {
                "--resume" if iter.peek().is_some_and(|v| !v.starts_with('-')) => {
                    args.push("--conversation".into())
                }
                "--resume" => args.push("--continue".into()),
                _ => args.push(arg),
            }
        }
        let supplied = args.iter().enumerate().rev().find_map(|(i, arg)| {
            if arg == "--settings" {
                Some((i, 2, args.get(i + 1)?.as_str()))
            } else {
                arg.strip_prefix("--settings=").map(|s| (i, 1, s))
            }
        });
        let mut settings = match supplied {
            Some((_, _, raw)) => match serde_json::from_str::<Value>(raw)
                .ok()
                .or_else(|| read_json(Path::new(raw)))
            {
                Some(v) if v.is_object() => v,
                _ => return args, // Let agy diagnose invalid settings.
            },
            None => json!({}),
        };
        let mut builder = tempfile::Builder::new();
        builder.prefix("crabigator-agy-");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            builder.permissions(fs::Permissions::from_mode(0o700));
        }
        let Ok(dir) = builder.tempdir() else {
            return args;
        };
        let saved = dirs::home_dir()
            .and_then(|home| read_json(&home.join(".gemini/antigravity-cli/settings.json")));
        status_settings(
            &mut settings,
            dir.path(),
            saved.as_ref().and_then(|v| v.get("statusLine")),
        );
        let config = dir.path().join("settings.json");
        if fs::write(&config, settings.to_string()).is_err() {
            return args;
        }
        let replacement = [
            "--settings".to_owned(),
            config.to_string_lossy().into_owned(),
        ];
        if let Some((i, len, _)) = supplied {
            args.splice(i..i + len, replacement);
        } else {
            args.splice(0..0, replacement);
        }
        let _ = self.directory.set(dir);
        args
    }

    fn load_stats(&self, _cwd: &str) -> Result<PlatformStats> {
        let mut history = self.history.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(snapshot) = self
            .directory
            .get()
            .and_then(|d| read_json(&d.path().join("status.json")))
        {
            history.update(&snapshot)?;
        }
        Ok(history.stats.clone())
    }
}

// In 1.5.14 statusLine can report the app's brain directory even for CLI sessions.
fn transcript_path(snapshot: &Value, id: &str, home: Option<&Path>) -> Option<PathBuf> {
    let reported = snapshot["transcript_path"]
        .as_str()
        .filter(|s| !s.is_empty())
        .map(PathBuf::from);
    if reported.as_ref().is_some_and(|p| p.is_file()) {
        return reported;
    }
    home.map(|p| {
        p.join(".gemini/antigravity-cli/brain")
            .join(id)
            .join(".system_generated/logs/transcript.jsonl")
    })
    .or(reported)
}

impl History {
    fn update(&mut self, snapshot: &Value) -> io::Result<()> {
        let id = snapshot["conversation_id"]
            .as_str()
            .filter(|s| uuid::Uuid::parse_str(s).is_ok());
        let path = id.and_then(|id| transcript_path(snapshot, id, dirs::home_dir().as_deref()));
        if self.stats.native_session_id.as_deref() != id {
            *self = History::default();
            self.stats.native_session_id = id.map(str::to_owned);
            if let Some(id) = self
                .stats
                .native_session_id
                .as_deref()
                .filter(|s| uuid::Uuid::parse_str(s).is_ok())
            {
                super::link_session_dir(id);
            }
        }
        self.stats.transcript_path = path.as_ref().map(|p| p.to_string_lossy().into_owned());
        if let Some(file) = path.and_then(|p| fs::File::open(p).ok()) {
            if file.metadata()?.len() < self.offset {
                self.offset = 0;
                self.stats.prompts = 0;
                self.stats.completions = 0;
                self.stats.tools.clear();
                self.stats.tool_timestamps.clear();
                self.awaiting_reply = false;
            }
            let (lines, offset) = crate::jsonl::read_pass(file, self.offset)?;
            for entry in lines
                .iter()
                .filter_map(|s| serde_json::from_str::<Value>(s).ok())
            {
                self.count(&entry);
            }
            self.offset = offset;
        }
        let stats = &mut self.stats;
        stats.model = snapshot["model"]["display_name"]
            .as_str()
            .map(str::to_owned);
        let context = &snapshot["context_window"];
        let used = context["current_usage"]["input_tokens"].as_u64().map(|n| {
            n + context["current_usage"]["cache_read_input_tokens"]
                .as_u64()
                .unwrap_or(0)
                + context["current_usage"]["cache_creation_input_tokens"]
                    .as_u64()
                    .unwrap_or(0)
        });
        stats.context =
            used.and_then(|n| ContextUsage::new(n, context["context_window_size"].as_u64()?));
        stats.mode = match snapshot["cycle_mode"].as_str() {
            Some("plan") => ClaudeMode::Plan,
            Some("auto" | "accept-edits") => ClaudeMode::AutoAccept,
            _ => ClaudeMode::Normal,
        };
        let state = if snapshot["tool_confirmation_pending"] == true {
            SessionState::Permission
        } else {
            match snapshot["agent_state"].as_str() {
                Some("working" | "tool_use" | "reviewing") => SessionState::Thinking,
                Some("idle" | "error") if stats.prompts > 0 => SessionState::Complete,
                _ => SessionState::Ready,
            }
        };
        if stats.state != state {
            let now = chrono::Utc::now().timestamp_millis() as f64 / 1000.0;
            stats.last_updated = Some(now);
            stats.idle_since = (state == SessionState::Complete).then_some(now);
            stats.state = state;
        }
        Ok(())
    }

    fn count(&mut self, entry: &Value) {
        let time = entry["created_at"]
            .as_str()
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|d| d.timestamp_millis() as f64 / 1000.0);
        if user_prompt(entry).is_some() {
            self.stats.prompts += 1;
            self.stats.prompts_changed_at = time;
            self.awaiting_reply = true;
        }
        if entry["type"] != "PLANNER_RESPONSE" {
            return;
        }
        let calls = entry["tool_calls"]
            .as_array()
            .map(Vec::as_slice)
            .unwrap_or_default();
        for call in calls {
            if let Some(name) = call["name"].as_str() {
                *self.stats.tools.entry(name.to_owned()).or_default() += 1;
                self.stats.tool_timestamps.extend(time);
            }
        }
        if self.awaiting_reply
            && calls.is_empty()
            && entry["content"]
                .as_str()
                .is_some_and(|s| !s.trim().is_empty())
        {
            self.stats.completions += 1;
            self.stats.completions_changed_at = time;
            self.awaiting_reply = false;
        }
    }
}

/// Discard the environment and settings that agy appends to a user's request.
pub(crate) fn user_prompt(entry: &Value) -> Option<&str> {
    if entry["type"] != "USER_INPUT" {
        return None;
    }
    let text = entry["content"].as_str()?.trim();
    let prompt = match text.strip_prefix("<USER_REQUEST>") {
        Some(body) => body
            .rsplit_once("</USER_REQUEST>")
            .map_or(body, |(request, _)| request),
        None if !text.starts_with('<') => text,
        None => return None,
    }
    .trim();
    (!prompt.is_empty()).then_some(prompt)
}

pub(crate) fn visit_activity(entry: &Value, mut emit: impl FnMut(&str, &str)) {
    let content = entry["content"].as_str().unwrap_or("").trim();
    match entry["type"].as_str() {
        Some("PLANNER_RESPONSE") => {
            if !content.is_empty() {
                emit("assistant", content);
            }
            for call in entry["tool_calls"].as_array().into_iter().flatten() {
                if let Some(name) = call["name"].as_str() {
                    emit("tool", &format!("{} {}", name, call["args"]));
                }
            }
        }
        Some("GENERIC") if !content.is_empty() => emit("tool_result", content),
        Some("ERROR_MESSAGE") if !content.is_empty() => emit("error", content),
        _ => {}
    }
}

pub fn read_transcript(path: &Path, offset: u64) -> io::Result<(String, u64)> {
    use std::fmt::Write;
    let (lines, offset) = crate::jsonl::read_pass(fs::File::open(path)?, offset)?;
    let mut out = String::new();
    for entry in lines
        .iter()
        .filter_map(|s| serde_json::from_str::<Value>(s).ok())
    {
        if let Some(prompt) = user_prompt(&entry) {
            let _ = writeln!(out, "\n> {prompt}");
        } else {
            visit_activity(&entry, |kind, text| {
                if kind == "tool_result" && text.lines().count() > 1 {
                    let _ = writeln!(out, "  ⎿  {} lines", text.lines().count());
                } else {
                    let _ = writeln!(out, "\n● {text}");
                }
            });
        }
    }
    Ok((out, offset))
}
