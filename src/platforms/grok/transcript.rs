//! Grok `updates.jsonl` reader for dashboard scrollback.
//!
//! The ACP session-update stream is the conversation log. Thought chunks are
//! skipped; user text, assistant text, and tool calls are formatted like the
//! Claude/Codex/opencode scrollback.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use serde_json::Value;

use super::acp;

mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const CYAN: &str = "\x1b[36m";
    pub const GREEN: &str = "\x1b[32m";
}

/// Tool call metadata retained until the matching result arrives.
pub struct PendingToolUse {
    name: String,
}

/// Read and format new ACP session updates for scrollback.
pub fn read_transcript(
    path: &Path,
    offset: u64,
    pending_tools: &mut HashMap<String, PendingToolUse>,
) -> std::io::Result<(String, u64)> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();
    if offset >= file_len {
        return Ok((String::new(), offset));
    }

    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(offset))?;

    let mut output = String::new();
    let mut current_pos = offset;
    let mut line = String::new();

    while reader.read_line(&mut line)? > 0 {
        current_pos = reader.stream_position()?;
        let parsed = serde_json::from_str::<Value>(line.trim_end());
        line.clear();
        let Ok(entry) = parsed else {
            continue;
        };
        let Some(update) = entry.pointer("/params/update") else {
            continue;
        };
        output.push_str(&format_update(update, pending_tools));
    }

    Ok((output, current_pos))
}

fn format_update(update: &Value, pending_tools: &mut HashMap<String, PendingToolUse>) -> String {
    let Some(kind) = update.get("sessionUpdate").and_then(Value::as_str) else {
        return String::new();
    };
    match kind {
        "user_message_chunk" => format_user(acp::text_content(update)),
        "agent_message_chunk" => format_assistant(acp::text_content(update)),
        "tool_call" => {
            let name = acp::tool_name_or_tool(update);
            let title = update.get("title").and_then(Value::as_str).unwrap_or("");
            if let Some(call_id) = update.get("toolCallId").and_then(Value::as_str) {
                pending_tools.insert(
                    call_id.to_string(),
                    PendingToolUse {
                        name: name.to_string(),
                    },
                );
            }
            format_tool_start(name, title)
        }
        "tool_call_update" => {
            let status = update.get("status").and_then(Value::as_str).unwrap_or("");
            if !matches!(status, "completed" | "failed" | "error") {
                return String::new();
            }
            let call_id = update.get("toolCallId").and_then(Value::as_str);
            let name = call_id
                .and_then(|id| pending_tools.remove(id))
                .map(|pending| pending.name)
                .unwrap_or_else(|| acp::tool_name_or_tool(update).to_string());
            format_tool_result(&name, &acp::tool_output_text(update))
        }
        _ => String::new(),
    }
}

fn format_user(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!(
        "\n{}{}> {}{}\n",
        colors::BOLD,
        colors::GREEN,
        trimmed,
        colors::RESET
    )
}

fn format_assistant(text: &str) -> String {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    format!("\n● {}\n", trimmed)
}

fn format_tool_start(name: &str, title: &str) -> String {
    let suffix = if title.is_empty() || title == name {
        String::new()
    } else {
        format!("({})", truncate(title, 80))
    };
    format!(
        "\n{}{}● {}{}{}\n",
        colors::BOLD,
        colors::CYAN,
        name,
        colors::RESET,
        suffix
    )
}

fn format_tool_result(name: &str, output: &str) -> String {
    let trimmed = output.trim();
    if trimmed.is_empty() {
        return String::new();
    }
    let line_count = trimmed.lines().count();
    if line_count == 1 && trimmed.chars().count() <= 80 {
        format!("  ⎿  {}\n", trimmed)
    } else {
        format!("  ⎿  {} returned {} lines\n", name, line_count)
    }
}

fn truncate(text: &str, max_chars: usize) -> String {
    let single_line = text.replace('\n', " ");
    if single_line.chars().count() <= max_chars {
        return single_line;
    }
    let prefix: String = single_line
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::io::Write;

    fn envelope(update: Value) -> String {
        json!({
            "method": "session/update",
            "params": { "sessionId": "s", "update": update }
        })
        .to_string()
    }

    #[test]
    fn incremental_user_assistant_and_tool() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("updates.jsonl");
        let mut file = std::fs::File::create(&path).unwrap();
        writeln!(
            file,
            "{}",
            envelope(json!({
                "sessionUpdate": "user_message_chunk",
                "content": {"type": "text", "text": "list the files"}
            }))
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            envelope(json!({
                "sessionUpdate": "agent_thought_chunk",
                "content": {"type": "text", "text": "I should list them"}
            }))
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            envelope(json!({
                "sessionUpdate": "tool_call",
                "toolCallId": "call-1",
                "title": "list_dir",
                "_meta": {"x.ai/tool": {"name": "list_dir"}}
            }))
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            envelope(json!({
                "sessionUpdate": "tool_call_update",
                "toolCallId": "call-1",
                "status": "completed",
                "content": [{"type": "content", "content": {"type": "text", "text": "a.rs"}}]
            }))
        )
        .unwrap();
        writeln!(
            file,
            "{}",
            envelope(json!({
                "sessionUpdate": "agent_message_chunk",
                "content": {"type": "text", "text": "There is one file."}
            }))
        )
        .unwrap();

        let mut pending = HashMap::new();
        let (text, offset) = read_transcript(&path, 0, &mut pending).unwrap();
        assert!(text.contains("> list the files"));
        assert!(text.contains("● list_dir"));
        assert!(text.contains("a.rs"));
        assert!(text.contains("● There is one file."));
        assert!(!text.contains("I should list them"));

        writeln!(
            file,
            "{}",
            envelope(json!({
                "sessionUpdate": "agent_message_chunk",
                "content": {"type": "text", "text": "done"}
            }))
        )
        .unwrap();
        let (text, _) = read_transcript(&path, offset, &mut pending).unwrap();
        assert!(!text.contains("list the files"));
        assert!(text.contains("● done"));
    }

    #[test]
    fn multiline_tool_output_is_summarized() {
        let formatted = format_tool_result("grep", "a\nb\nc\n");
        assert!(formatted.contains("grep returned 3 lines"));
    }
}
