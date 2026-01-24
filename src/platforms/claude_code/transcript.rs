//! Claude Code transcript (JSONL) reader.
//!
//! Reads conversation history from Claude Code's transcript files,
//! which are stored as JSONL in ~/.claude/projects/{project}/{session}.jsonl

use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::Path;

use serde::Deserialize;
use serde_json::Value;

/// Content block types in assistant messages
#[derive(Debug, Deserialize)]
#[serde(tag = "type")]
enum ContentBlock {
    #[serde(rename = "text")]
    Text { text: String },
    #[serde(rename = "thinking")]
    Thinking {
        #[allow(dead_code)]
        thinking: String,
    },
    #[serde(rename = "tool_use")]
    ToolUse {
        name: String,
        input: Value,
        #[allow(dead_code)]
        #[serde(default)]
        id: String,
    },
    #[serde(rename = "tool_result")]
    ToolResult {
        #[allow(dead_code)]
        #[serde(default)]
        tool_use_id: String,
        #[serde(default)]
        content: Value,
    },
    #[serde(other)]
    Other,
}

/// ANSI color codes for terminal output
mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const DIM: &str = "\x1b[2m";
    pub const CYAN: &str = "\x1b[36m";
    pub const GREEN: &str = "\x1b[32m";
}

/// Read and format transcript messages from a JSONL file.
///
/// Returns formatted text suitable for display in a terminal scrollback,
/// with ANSI color codes for syntax highlighting.
pub fn read_transcript(path: &Path, offset: u64) -> std::io::Result<(String, u64)> {
    let file = File::open(path)?;
    let file_len = file.metadata()?.len();

    // If we've already read everything, return empty
    if offset >= file_len {
        return Ok((String::new(), offset));
    }

    let mut reader = BufReader::new(file);
    reader.seek(SeekFrom::Start(offset))?;

    let mut output = String::new();
    let mut current_pos = offset;

    for line in reader.lines() {
        let line = line?;
        current_pos += line.len() as u64 + 1; // +1 for newline

        if line.is_empty() {
            continue;
        }

        let Ok(entry) = serde_json::from_str::<Value>(&line) else {
            continue;
        };

        let Some(msg_type) = entry.get("type").and_then(|t| t.as_str()) else {
            continue;
        };

        match msg_type {
            "user" => {
                if let Some(content) = entry
                    .get("message")
                    .and_then(|m| m.get("content"))
                    .and_then(|c| c.as_str())
                {
                    output.push_str(&format_user_message(content));
                }
            }
            "assistant" => {
                if let Some(content) = entry.get("message").and_then(|m| m.get("content")) {
                    output.push_str(&format_assistant_message(content));
                }
            }
            "progress" => {
                // Progress messages contain tool results in data.message.message.content
                if let Some(content) = entry
                    .get("data")
                    .and_then(|d| d.get("message"))
                    .and_then(|m| m.get("message"))
                    .and_then(|m| m.get("content"))
                {
                    output.push_str(&format_assistant_message(content));
                }
            }
            _ => {
                // Skip other message types (file-history-snapshot, etc.)
            }
        }
    }

    Ok((output, current_pos))
}

/// Format a user message with prompt indicator
fn format_user_message(content: &str) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "\n{}{}> {}{}",
        colors::BOLD,
        colors::GREEN,
        content.trim(),
        colors::RESET
    ));
    out.push('\n');
    out
}

/// Format an assistant message, handling different content block types
fn format_assistant_message(content: &Value) -> String {
    let mut out = String::new();

    let blocks: Vec<ContentBlock> = match content {
        Value::Array(arr) => arr
            .iter()
            .filter_map(|v| serde_json::from_value(v.clone()).ok())
            .collect(),
        Value::String(s) => {
            // Simple string content (older format)
            out.push_str("\n");
            out.push_str(s.trim());
            out.push('\n');
            return out;
        }
        _ => return out,
    };

    for block in blocks {
        match block {
            ContentBlock::Text { text } => {
                out.push('\n');
                out.push_str(text.trim());
                out.push('\n');
            }
            ContentBlock::ToolUse { name, input, .. } => {
                out.push_str(&format_tool_use(&name, &input));
            }
            ContentBlock::ToolResult { content, .. } => {
                out.push_str(&format_tool_result(&content));
            }
            ContentBlock::Thinking { .. } | ContentBlock::Other => {
                // Skip thinking blocks and unknown types
            }
        }
    }

    out
}

/// Format a tool use block
fn format_tool_use(name: &str, input: &Value) -> String {
    let mut out = String::new();

    // Tool header
    out.push_str(&format!(
        "\n{}{}● {}{}",
        colors::BOLD,
        colors::CYAN,
        name,
        colors::RESET
    ));

    // Format input based on tool type
    match name {
        "Bash" => {
            if let Some(cmd) = input.get("command").and_then(|c| c.as_str()) {
                out.push_str(&format!("({})", truncate(cmd, 60)));
            }
        }
        "Read" | "Write" => {
            if let Some(path) = input.get("file_path").and_then(|p| p.as_str()) {
                out.push_str(&format!("({})", shorten_path(path)));
            }
        }
        "Edit" => {
            if let Some(path) = input.get("file_path").and_then(|p| p.as_str()) {
                out.push_str(&format!("({})", shorten_path(path)));
            }
            out.push('\n');
            // Generate diff from old_string and new_string
            if let (Some(old), Some(new)) = (
                input.get("old_string").and_then(|s| s.as_str()),
                input.get("new_string").and_then(|s| s.as_str()),
            ) {
                out.push_str(&format_edit_diff(old, new));
            }
            return out;
        }
        "Grep" => {
            if let Some(pattern) = input.get("pattern").and_then(|p| p.as_str()) {
                out.push_str(&format!("({})", truncate(pattern, 40)));
            }
        }
        "Glob" => {
            if let Some(pattern) = input.get("pattern").and_then(|p| p.as_str()) {
                out.push_str(&format!("({})", pattern));
            }
        }
        "Task" => {
            if let Some(desc) = input.get("description").and_then(|d| d.as_str()) {
                out.push_str(&format!("({})", desc));
            }
        }
        "WebFetch" | "WebSearch" => {
            if let Some(url) = input.get("url").and_then(|u| u.as_str()) {
                out.push_str(&format!("({})", truncate(url, 50)));
            } else if let Some(query) = input.get("query").and_then(|q| q.as_str()) {
                out.push_str(&format!("({})", truncate(query, 50)));
            }
        }
        _ => {
            // For other tools, show first string field if short enough
            if let Some(obj) = input.as_object() {
                for (_, v) in obj.iter().take(1) {
                    if let Some(s) = v.as_str() {
                        if s.len() < 60 {
                            out.push_str(&format!("({})", s));
                            break;
                        }
                    }
                }
            }
        }
    }

    out.push('\n');
    out
}

/// Format an edit diff showing removed and added lines
fn format_edit_diff(old: &str, new: &str) -> String {
    let mut out = String::new();

    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    // Simple diff: show removed lines then added lines
    // Red for removed, green for added
    const RED: &str = "\x1b[31m";
    const GREEN: &str = "\x1b[32m";

    // Show removed lines (from old but not in new)
    for line in &old_lines {
        if !new_lines.contains(line) {
            out.push_str(&format!("{}  - {}{}\n", RED, line, colors::RESET));
        }
    }

    // Show added lines (in new but not in old)
    for line in &new_lines {
        if !old_lines.contains(line) {
            out.push_str(&format!("{}  + {}{}\n", GREEN, line, colors::RESET));
        }
    }

    out
}

/// Format a tool result block
fn format_tool_result(content: &Value) -> String {
    let mut out = String::new();

    // Extract text from tool result
    let text = match content {
        Value::String(s) => s.clone(),
        Value::Array(arr) => {
            // Tool results can be array of content blocks
            arr.iter()
                .filter_map(|v| {
                    if v.get("type").and_then(|t| t.as_str()) == Some("text") {
                        v.get("text").and_then(|t| t.as_str()).map(String::from)
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => return out,
    };

    if text.is_empty() {
        return out;
    }

    // Show full result with dim white dot prefix to match terminal style
    let lines: Vec<&str> = text.lines().collect();

    out.push_str(&format!("{}  ● ", colors::DIM));
    out.push_str(&lines.join(&format!("{}\n{}  ● ", colors::RESET, colors::DIM)));

    out.push_str(colors::RESET);
    out.push('\n');
    out
}

/// Truncate a string with ellipsis
fn truncate(s: &str, max_len: usize) -> String {
    let s = s.replace('\n', " ");
    if s.len() <= max_len {
        s
    } else {
        format!("{}...", &s[..max_len - 3])
    }
}

/// Shorten a file path by replacing home dir and common prefixes
fn shorten_path(path: &str) -> String {
    let home = dirs::home_dir()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();

    let shortened = if !home.is_empty() && path.starts_with(&home) {
        format!("~{}", &path[home.len()..])
    } else {
        path.to_string()
    };

    // Further shorten if still long
    if shortened.len() > 50 {
        let parts: Vec<&str> = shortened.split('/').collect();
        if parts.len() > 3 {
            format!(".../{}", parts[parts.len() - 2..].join("/"))
        } else {
            shortened
        }
    } else {
        shortened
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("short", 10), "short");
        assert_eq!(truncate("this is a longer string", 10), "this is...");
    }

    #[test]
    fn test_shorten_path() {
        // Just verify it doesn't panic
        let _ = shorten_path("/some/very/long/path/to/file.rs");
    }
}
