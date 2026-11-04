//! Crabigator's statusline for Claude Code (`statusline.py`).
//!
//! Claude Code tells only its statusline how large the context window is.
//! Crabigator starts Claude with `--settings` naming this statusline, which
//! saves the window's numbers to the session's `context.json` and then runs
//! the user's own statusline unchanged. It is a file of its own, next to the
//! hook script, so an older crabigator reinstalling its hook script never
//! takes it away from a session that uses it.

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_json::{json, Map, Value};

use crate::platforms::ContextUsage;

const SCRIPT: &str = include_str!("statusline.py");

/// Write the script into `dir` unless it is already there, and return its path.
pub fn install(dir: &Path) -> Result<PathBuf> {
    let path = dir.join("statusline.py");
    if fs::read_to_string(&path).ok().as_deref() != Some(SCRIPT) {
        fs::create_dir_all(dir).context("Failed to create crabigator directory")?;
        let tmp = dir.join("statusline.py.tmp");
        fs::write(&tmp, SCRIPT).context("Failed to write statusline script")?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&tmp, fs::Permissions::from_mode(0o755))?;
        }
        fs::rename(&tmp, &path).context("Failed to install statusline script")?;
    }
    Ok(path)
}

/// The `--settings` value that makes `script` Claude's statusline. It keeps
/// the rest of the user's statusline settings (such as its padding), so the
/// statusline sits where it always does.
pub fn settings_arg(script: &Path, project: &Path, claude_dir: &Path) -> String {
    let mut line = user_statusline(project, claude_dir).unwrap_or_default();
    line.insert("type".into(), json!("command"));
    line.insert(
        "command".into(),
        json!(shell_quote(&script.to_string_lossy())),
    );
    json!({ "statusLine": line }).to_string()
}

/// The statusline the user set up, in Claude Code's order of precedence.
fn user_statusline(project: &Path, claude_dir: &Path) -> Option<Map<String, Value>> {
    [
        project.join(".claude/settings.local.json"),
        project.join(".claude/settings.json"),
        claude_dir.join("settings.json"),
    ]
    .iter()
    .filter_map(|path| fs::read_to_string(path).ok())
    .filter_map(|text| serde_json::from_str::<Value>(&text).ok())
    .find_map(|settings| settings.get("statusLine")?.as_object().cloned())
}

fn shell_quote(text: &str) -> String {
    format!("'{}'", text.replace('\'', "'\\''"))
}

/// Where the statusline saves the context window for a crabigator session.
pub fn context_path(session_id: &str) -> PathBuf {
    PathBuf::from(format!("/tmp/crabigator-{session_id}/context.json"))
}

/// The context window as the statusline last saw it.
pub fn read_context(path: &Path) -> Option<ContextUsage> {
    let usage: ContextUsage = serde_json::from_str(&fs::read_to_string(path).ok()?).ok()?;
    ContextUsage::new(usage.used, usage.limit)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use std::process::{Command, Stdio};

    #[test]
    fn settings_keep_the_users_padding_but_name_our_script() {
        let root = tempfile::tempdir().unwrap();
        let claude_dir = root.path().join("claude");
        fs::create_dir_all(&claude_dir).unwrap();
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"statusLine":{"type":"command","command":"my-line","padding":2}}"#,
        )
        .unwrap();
        let script = Path::new("/home/o'neil/.claude/crabigator/statusline.py");
        let arg: Value =
            serde_json::from_str(&settings_arg(script, root.path(), &claude_dir)).unwrap();
        assert_eq!(
            arg,
            json!({"statusLine": {
                "type": "command",
                "command": "'/home/o'\\''neil/.claude/crabigator/statusline.py'",
                "padding": 2
            }})
        );

        // A project's own statusline wins over the user's.
        fs::create_dir_all(root.path().join(".claude")).unwrap();
        fs::write(
            root.path().join(".claude/settings.json"),
            r#"{"statusLine":{"type":"command","command":"project-line"}}"#,
        )
        .unwrap();
        let arg: Value =
            serde_json::from_str(&settings_arg(script, root.path(), &claude_dir)).unwrap();
        assert_eq!(arg["statusLine"].get("padding"), None);
    }

    /// Run the installed script as Claude Code would: the statusline JSON on
    /// stdin, in the project directory.
    fn run_script(script: &Path, env: &[(&str, &Path)], session: &str, input: &Value) -> String {
        let mut command = Command::new(script);
        command
            .env("CRABIGATOR_SESSION_ID", session)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped());
        for (key, value) in env {
            command.env(key, value);
        }
        let mut child = command.spawn().expect("python3 runs the statusline");
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.to_string().as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(output.status.success());
        String::from_utf8(output.stdout).unwrap()
    }

    #[test]
    fn script_saves_the_window_and_runs_the_users_statusline() {
        let root = tempfile::tempdir().unwrap();
        let claude_dir = root.path().join("claude");
        fs::create_dir_all(&claude_dir).unwrap();
        // The user's statusline reads the same input Claude Code gave ours.
        fs::write(
            claude_dir.join("settings.json"),
            r#"{"statusLine":{"type":"command","command":"python3 -c \"import json,sys; print('line', json.load(sys.stdin)['model']['id'])\""}}"#,
        )
        .unwrap();
        let script = install(&root.path().join("crabigator")).unwrap();
        assert_eq!(install(&root.path().join("crabigator")).unwrap(), script);

        let session = format!("statusline-test-{}", std::process::id());
        let session_dir = PathBuf::from(format!("/tmp/crabigator-{session}"));
        fs::create_dir_all(&session_dir).unwrap();
        let input = json!({
            "model": {"id": "claude-opus-5-5"},
            "workspace": {"project_dir": root.path()},
            "context_window": {
                "total_input_tokens": 186002,
                "context_window_size": 1000000,
                "current_usage": {"input_tokens": 2, "cache_creation_input_tokens": 3024, "cache_read_input_tokens": 182976, "output_tokens": 518},
                "used_percentage": 19
            }
        });
        let shown = run_script(
            &script,
            &[("CLAUDE_CONFIG_DIR", &claude_dir)],
            &session,
            &input,
        );
        assert_eq!(shown.trim(), "line claude-opus-5-5");
        assert_eq!(
            read_context(&context_path(&session)),
            ContextUsage::new(186_002, 1_000_000)
        );

        // With no statusline of the user's, it shows nothing but still saves.
        fs::remove_file(claude_dir.join("settings.json")).unwrap();
        let mut input = input;
        input["context_window"]["total_input_tokens"] = json!(250000);
        let shown = run_script(
            &script,
            &[("CLAUDE_CONFIG_DIR", &claude_dir)],
            &session,
            &input,
        );
        assert_eq!(shown, "");
        assert_eq!(
            read_context(&context_path(&session)),
            ContextUsage::new(250_000, 1_000_000)
        );
        fs::remove_dir_all(&session_dir).unwrap();
    }
}
