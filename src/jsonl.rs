//! Bounded JSONL reads.
//!
//! Session logs keep growing, and a single tool result can be many megabytes.
//! Callers that redraw on a timer must be able to move past those lines
//! without parsing them or holding them in memory.

use std::io::{BufRead, BufReader, Read, Seek, SeekFrom};

/// A line longer than this is skipped. Prompts and the fields a redraw needs
/// are far smaller; the rest is tool output.
pub const MAX_LINE_BYTES: usize = 256 * 1024;

/// One JSONL record, counted in file bytes so the caller can resume after it.
pub struct JsonlRecord {
    /// The record without its trailing newline. `None` when the line was too
    /// long to keep, or was not valid UTF-8.
    pub text: Option<String>,
    pub bytes: u64,
    /// False when the file ended before the closing newline.
    pub complete: bool,
}

/// Read the next JSONL record. `None` means there is nothing left to read.
pub fn read_jsonl_record<R: BufRead>(
    reader: &mut R,
    max_len: usize,
) -> std::io::Result<Option<JsonlRecord>> {
    let mut stored: Vec<u8> = Vec::new();
    let mut total = 0u64;
    let mut skipped = false;
    let mut saw_byte = false;
    loop {
        let buf = reader.fill_buf()?;
        if buf.is_empty() {
            if !saw_byte {
                return Ok(None);
            }
            return Ok(Some(JsonlRecord {
                text: record_text(&stored, skipped),
                bytes: total,
                complete: false,
            }));
        }
        saw_byte = true;
        if let Some(pos) = buf.iter().position(|byte| *byte == b'\n') {
            let take = pos + 1;
            if !skipped && stored.len().saturating_add(pos) <= max_len {
                stored.extend_from_slice(&buf[..pos]);
            } else {
                skipped = true;
                stored.clear();
            }
            reader.consume(take);
            total += take as u64;
            return Ok(Some(JsonlRecord {
                text: record_text(&stored, skipped),
                bytes: total,
                complete: true,
            }));
        }
        let n = buf.len();
        if !skipped && stored.len().saturating_add(n) <= max_len {
            stored.extend_from_slice(buf);
        } else {
            skipped = true;
            stored.clear();
        }
        reader.consume(n);
        total += n as u64;
    }
}

fn record_text(stored: &[u8], skipped: bool) -> Option<String> {
    if skipped {
        None
    } else {
        Some(String::from_utf8_lossy(stored).into_owned())
    }
}

/// Read complete JSONL records starting at `start`, stopping after `max_bytes`.
///
/// `align` skips the first fragment when `start` may fall in the middle of a
/// line. A saved offset is already on a record boundary, so pass false.
/// A line longer than [`MAX_LINE_BYTES`] is skipped too, and its bytes still
/// count so the next read begins after it.
pub fn read_jsonl_window(
    path: &std::path::Path,
    start: u64,
    max_bytes: u64,
    align: bool,
) -> std::io::Result<JsonlWindow> {
    let mut file = std::fs::File::open(path)?;
    let file_len = file.metadata()?.len();
    if start >= file_len {
        return Ok(JsonlWindow {
            text: String::new(),
            end: file_len,
            reached_end: true,
        });
    }
    file.seek(SeekFrom::Start(start))?;
    let mut reader = BufReader::new(file);
    let mut consumed = 0u64;
    if align {
        if let Some(partial) = read_jsonl_record(&mut reader, MAX_LINE_BYTES)? {
            if !partial.complete {
                return Ok(JsonlWindow {
                    text: String::new(),
                    end: start,
                    reached_end: true,
                });
            }
            consumed += partial.bytes;
        }
    }
    let mut text = String::new();
    let mut reached_end = false;
    while consumed < max_bytes {
        let Some(record) = read_jsonl_record(&mut reader, MAX_LINE_BYTES)? else {
            reached_end = true;
            break;
        };
        if !record.complete {
            // Leave the unfinished line for the next read.
            break;
        }
        consumed += record.bytes;
        if let Some(line) = record.text {
            text.push_str(&line);
            text.push('\n');
        }
    }
    let end = start + consumed;
    if end >= file_len {
        reached_end = true;
    }
    Ok(JsonlWindow {
        text,
        end: end.min(file_len),
        reached_end,
    })
}

pub struct JsonlWindow {
    pub text: String,
    /// Absolute offset just past the last complete record that was consumed.
    pub end: u64,
    pub reached_end: bool,
}

/// How many bytes a redraw may pull from a transcript in one pass.
pub const MAX_PASS_BYTES: u64 = 512 * 1024;

/// Read at most [`MAX_PASS_BYTES`] of new transcript, parsing only lines that
/// fit in [`MAX_LINE_BYTES`]. Returns the formatted-handler input and the
/// offset to resume from.
pub fn read_pass<R: Read + Seek>(mut file: R, offset: u64) -> std::io::Result<(Vec<String>, u64)> {
    let file_len = file.seek(SeekFrom::End(0))?;
    if offset >= file_len {
        return Ok((Vec::new(), offset));
    }
    file.seek(SeekFrom::Start(offset))?;
    let mut reader = BufReader::new(file);
    let mut lines = Vec::new();
    let mut consumed = 0u64;
    while consumed < MAX_PASS_BYTES {
        let Some(record) = read_jsonl_record(&mut reader, MAX_LINE_BYTES)? else {
            break;
        };
        if !record.complete {
            break;
        }
        consumed += record.bytes;
        if let Some(text) = record.text {
            lines.push(text);
        }
    }
    Ok((lines, offset + consumed))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn a_huge_line_is_skipped_and_the_next_record_is_kept() {
        let big = "x".repeat(MAX_LINE_BYTES + 10);
        let raw = format!("{big}\n{{\"ok\":true}}\n");
        let mut reader = Cursor::new(raw.as_bytes());
        let skipped = read_jsonl_record(&mut reader, MAX_LINE_BYTES)
            .unwrap()
            .unwrap();
        assert!(skipped.text.is_none());
        assert!(skipped.complete);
        assert_eq!(skipped.bytes, big.len() as u64 + 1);
        let kept = read_jsonl_record(&mut reader, MAX_LINE_BYTES)
            .unwrap()
            .unwrap();
        assert_eq!(kept.text.as_deref(), Some("{\"ok\":true}"));
    }

    #[test]
    fn an_unfinished_line_is_not_treated_as_a_record_boundary() {
        let mut reader = Cursor::new(b"{\"partial\":true".as_slice());
        let record = read_jsonl_record(&mut reader, MAX_LINE_BYTES)
            .unwrap()
            .unwrap();
        assert!(!record.complete);
        assert_eq!(record.text.as_deref(), Some("{\"partial\":true"));
    }
}
