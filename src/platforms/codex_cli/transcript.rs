//! Codex CLI transcript (JSONL) reader for dashboard scrollback.

use std::collections::HashMap;
use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use serde_json::{Map, Value};

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

/// Read and format new Codex response items from a JSONL transcript.
///
/// Codex also emits `event_msg` copies of user and assistant messages. Reading
/// only `response_item` entries avoids duplicate scrollback while retaining tool
/// calls, which are not consistently represented as events.
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
        if entry.get("type").and_then(Value::as_str) != Some("response_item") {
            continue;
        }
        let Some(payload) = entry.get("payload").and_then(Value::as_object) else {
            continue;
        };

        match payload.get("type").and_then(Value::as_str) {
            Some("message") => output.push_str(&format_message(payload)),
            Some("function_call") | Some("custom_tool_call") => {
                output.push_str(&format_tool_call(payload));
                if let (Some(call_id), Some(name)) = (
                    payload.get("call_id").and_then(Value::as_str),
                    payload.get("name").and_then(Value::as_str),
                ) {
                    pending_tools.insert(
                        call_id.to_string(),
                        PendingToolUse {
                            name: name.to_string(),
                        },
                    );
                }
            }
            Some("function_call_output") | Some("custom_tool_call_output") => {
                output.push_str(&format_tool_output(payload, pending_tools));
            }
            Some("local_shell_call") => output.push_str(&format_local_shell_call(payload)),
            _ => {}
        }
    }

    Ok((output, current_pos))
}

fn format_message(payload: &Map<String, Value>) -> String {
    let role = payload.get("role").and_then(Value::as_str);
    let text = message_text(payload);
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return String::new();
    }

    match role {
        Some("user") if !super::is_injected_user_message(trimmed) => format!(
            "\n{}{}> {}{}\n",
            colors::BOLD,
            colors::GREEN,
            trimmed,
            colors::RESET
        ),
        Some("assistant") => format!("\n● {}\n", trimmed),
        _ => String::new(),
    }
}

fn message_text(payload: &Map<String, Value>) -> String {
    payload
        .get("content")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|part| part.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>()
        .join("\n")
}

fn format_tool_call(payload: &Map<String, Value>) -> String {
    let name = payload
        .get("name")
        .and_then(Value::as_str)
        .unwrap_or("tool");
    let preview = tool_input_preview(name, payload);
    let suffix = if preview.is_empty() {
        String::new()
    } else {
        format!("({preview})")
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

fn tool_input_preview(name: &str, payload: &Map<String, Value>) -> String {
    let input = payload.get("arguments").or_else(|| payload.get("input"));
    let Some(input) = input else {
        return String::new();
    };

    let parsed;
    let value = if let Some(raw) = input.as_str() {
        parsed = serde_json::from_str::<Value>(raw).ok();
        parsed.as_ref().unwrap_or(input)
    } else {
        input
    };

    if let Some(object) = value.as_object() {
        for key in [
            "cmd",
            "command",
            "path",
            "file_path",
            "pattern",
            "query",
            "url",
            "target",
        ] {
            if let Some(text) = object.get(key).and_then(Value::as_str) {
                return truncate(text, 80);
            }
        }
    }

    // Custom tool inputs can contain large programs or encoded connector data.
    // The tool name is sufficient when no small, recognized field is available.
    if payload.get("type").and_then(Value::as_str) == Some("custom_tool_call") {
        return String::new();
    }

    if name == "request_user_input" {
        return "question".to_string();
    }
    String::new()
}

fn format_tool_output(
    payload: &Map<String, Value>,
    pending_tools: &mut HashMap<String, PendingToolUse>,
) -> String {
    let call_id = payload.get("call_id").and_then(Value::as_str);
    let name = call_id
        .and_then(|id| pending_tools.remove(id))
        .map(|tool| tool.name)
        .unwrap_or_else(|| "tool".to_string());
    let text = payload.get("output").map(value_text).unwrap_or_default();
    let trimmed = text.trim();
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

fn value_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Array(parts) => parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("\n"),
        _ => String::new(),
    }
}

fn format_local_shell_call(payload: &Map<String, Value>) -> String {
    let command = payload
        .get("action")
        .and_then(|action| action.get("command"))
        .and_then(Value::as_str)
        .map(|command| truncate(command, 80))
        .unwrap_or_default();
    let suffix = if command.is_empty() {
        String::new()
    } else {
        format!("({command})")
    };
    format!(
        "\n{}{}● shell{}{}\n",
        colors::BOLD,
        colors::CYAN,
        colors::RESET,
        suffix
    )
}

fn truncate(text: &str, max_chars: usize) -> String {
    let single_line = text.replace('\n', " ");
    if single_line.chars().count() <= max_chars {
        return single_line;
    }
    let prefix = single_line
        .chars()
        .take(max_chars.saturating_sub(3))
        .collect::<String>();
    format!("{prefix}...")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn reads_codex_messages_and_tools_incrementally_without_bootstrap() {
        let mut transcript = tempfile::NamedTempFile::new().expect("temp transcript");
        writeln!(
            transcript,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"<environment_context>hidden</environment_context>"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            transcript,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"show the status"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            transcript,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"user","content":[{{"type":"input_text","text":"<skill>\n<name>status</name>\n<path>/skills/status/SKILL.md</path>"}}]}}}}"#
        )
        .unwrap();
        writeln!(
            transcript,
            r#"{{"type":"response_item","payload":{{"type":"message","role":"assistant","content":[{{"type":"output_text","text":"Checking now."}}]}}}}"#
        )
        .unwrap();
        writeln!(
            transcript,
            r#"{{"type":"response_item","payload":{{"type":"function_call","name":"exec_command","call_id":"call-1","arguments":"{{\"cmd\":\"git status --short\"}}"}}}}"#
        )
        .unwrap();
        transcript.flush().unwrap();

        let mut pending = HashMap::new();
        let (first, offset) = read_transcript(transcript.path(), 0, &mut pending).unwrap();
        assert!(!first.contains("environment_context"));
        assert!(
            !first.contains("<skill>"),
            "skill bodies are Codex's, not the user's"
        );
        assert!(first.contains("> show the status"));
        assert!(first.contains("● Checking now."));
        assert!(first.contains("● exec_command"));
        assert!(first.contains("git status --short"));

        writeln!(
            transcript,
            r#"{{"type":"response_item","payload":{{"type":"function_call_output","call_id":"call-1","output":" M src/main.rs\n M src/app.rs"}}}}"#
        )
        .unwrap();
        transcript.flush().unwrap();

        let (second, next_offset) =
            read_transcript(transcript.path(), offset, &mut pending).unwrap();
        assert_eq!(second, "  ⎿  exec_command returned 2 lines\n");
        assert!(next_offset > offset);

        let (empty, final_offset) =
            read_transcript(transcript.path(), next_offset, &mut pending).unwrap();
        assert!(empty.is_empty());
        assert_eq!(final_offset, next_offset);
    }

    #[test]
    fn handles_unicode_tool_preview_boundaries() {
        assert_eq!(truncate("🦀🦀🦀🦀", 4), "🦀🦀🦀🦀");
        assert_eq!(truncate("🦀🦀🦀🦀🦀", 4), "🦀...");
    }
}
