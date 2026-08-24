//! opencode transcript log: normalized JSONL written by the event listener,
//! read back for scrollback.log, recaps, and PR tracking.
//!
//! opencode keeps its history in SQLite rather than a JSONL file, so the
//! event listener writes this log itself. Each line is one
//! [`TranscriptEntry`](super::events::TranscriptEntry).

use std::fs::OpenOptions;
use std::io::{BufRead, BufReader, Seek, SeekFrom, Write};
use std::path::Path;

use super::events::TranscriptEntry;

mod colors {
    pub const RESET: &str = "\x1b[0m";
    pub const BOLD: &str = "\x1b[1m";
    pub const CYAN: &str = "\x1b[36m";
    pub const GREEN: &str = "\x1b[32m";
}

/// Append entries to the transcript log as JSON lines.
pub fn append_entries(path: &Path, entries: &[TranscriptEntry]) -> std::io::Result<()> {
    if entries.is_empty() {
        return Ok(());
    }
    let mut file = OpenOptions::new().create(true).append(true).open(path)?;
    let mut buf = String::new();
    for entry in entries {
        if let Ok(line) = serde_json::to_string(entry) {
            buf.push_str(&line);
            buf.push('\n');
        }
    }
    file.write_all(buf.as_bytes())
}

/// Read and format new transcript entries for scrollback, mirroring the
/// Claude/Codex scrollback style.
pub fn read_transcript(path: &Path, offset: u64) -> std::io::Result<(String, u64)> {
    let file = std::fs::File::open(path)?;
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
        let parsed = serde_json::from_str::<TranscriptEntry>(line.trim_end());
        line.clear();
        let Ok(entry) = parsed else {
            continue;
        };
        output.push_str(&format_entry(&entry));
    }

    Ok((output, current_pos))
}

fn format_entry(entry: &TranscriptEntry) -> String {
    match entry {
        TranscriptEntry::User { text } => {
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
        TranscriptEntry::Assistant { text } => {
            let trimmed = text.trim();
            if trimmed.is_empty() {
                return String::new();
            }
            format!("\n● {}\n", trimmed)
        }
        TranscriptEntry::Tool {
            name,
            title,
            output,
        } => {
            let suffix = if title.is_empty() {
                String::new()
            } else {
                format!("({})", truncate(title, 80))
            };
            let mut formatted = format!(
                "\n{}{}● {}{}{}\n",
                colors::BOLD,
                colors::CYAN,
                name,
                colors::RESET,
                suffix
            );
            let trimmed = output.trim();
            if !trimmed.is_empty() {
                let line_count = trimmed.lines().count();
                if line_count == 1 && trimmed.chars().count() <= 80 {
                    formatted.push_str(&format!("  ⎿  {}\n", trimmed));
                } else {
                    formatted.push_str(&format!("  ⎿  {} returned {} lines\n", name, line_count));
                }
            }
            formatted
        }
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

    #[test]
    fn round_trips_entries_incrementally() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("transcript.jsonl");

        append_entries(
            &path,
            &[
                TranscriptEntry::User {
                    text: "check the PR".to_string(),
                },
                TranscriptEntry::Tool {
                    name: "bash".to_string(),
                    title: "gh pr view 42".to_string(),
                    output: "https://github.com/o/r/pull/42\n".to_string(),
                },
            ],
        )
        .unwrap();

        let (text, offset) = read_transcript(&path, 0).unwrap();
        assert!(text.contains("> check the PR"));
        assert!(text.contains("● bash"));
        assert!(text.contains("gh pr view 42"));
        assert!(text.contains("https://github.com/o/r/pull/42"));

        // Incremental read from the saved offset picks up only new entries
        append_entries(
            &path,
            &[TranscriptEntry::Assistant {
                text: "done".to_string(),
            }],
        )
        .unwrap();
        let (text, _) = read_transcript(&path, offset).unwrap();
        assert!(!text.contains("check the PR"));
        assert!(text.contains("● done"));
    }

    #[test]
    fn multiline_tool_output_is_summarized() {
        let entry = TranscriptEntry::Tool {
            name: "bash".to_string(),
            title: "ls".to_string(),
            output: "a\nb\nc\n".to_string(),
        };
        let formatted = format_entry(&entry);
        assert!(formatted.contains("bash returned 3 lines"));
    }
}
