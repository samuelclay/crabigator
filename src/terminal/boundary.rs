//! Where the assistant's output stream stands, so the status bar can draw
//! between its writes instead of waiting for it to fall silent.
//!
//! Drawing writes cursor moves and colours into the same terminal stream.
//! That is safe between the assistant's writes as long as the stream is not
//! in the middle of an escape sequence or a UTF-8 character, and the
//! assistant is not inside a synchronized update (`ESC [ ? 2026 h` … `l`):
//! our own synchronized draw would end theirs early and tear their frame.
//! This tracks both from the bytes written to the terminal.

/// How far into an escape sequence (or a character) the stream is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Ground,
    /// After ESC.
    Escape,
    /// After ESC and an intermediate byte (`ESC ( B`).
    EscapeIntermediate,
    /// Inside `ESC [` … final byte.
    Csi,
    /// Inside an OSC, DCS, SOS, PM or APC string, until BEL or ST.
    String,
    /// ESC seen inside a string: ST if `\` follows.
    StringEscape,
    /// Continuation bytes still owed by a UTF-8 character.
    Utf8(u8),
}

/// Tracks the assistant's output stream as it is written to the terminal.
#[derive(Debug)]
pub struct OutputBoundary {
    state: State,
    /// The current CSI's parameter and intermediate bytes.
    csi: Vec<u8>,
    /// Inside the assistant's synchronized update.
    synchronized: bool,
}

impl Default for OutputBoundary {
    fn default() -> Self {
        Self {
            state: State::Ground,
            csi: Vec::with_capacity(16),
            synchronized: false,
        }
    }
}

impl OutputBoundary {
    /// Follow bytes as they go to the terminal.
    pub fn scan(&mut self, bytes: &[u8]) {
        for &b in bytes {
            self.state = match self.state {
                State::Ground => match b {
                    0x1b => State::Escape,
                    0xc0..=0xdf => State::Utf8(1),
                    0xe0..=0xef => State::Utf8(2),
                    0xf0..=0xf7 => State::Utf8(3),
                    _ => State::Ground,
                },
                State::Utf8(n) => match b {
                    0x80..=0xbf if n > 1 => State::Utf8(n - 1),
                    0x80..=0xbf => State::Ground,
                    // A broken character: start over from this byte.
                    0x1b => State::Escape,
                    _ => State::Ground,
                },
                State::Escape => match b {
                    b'[' => {
                        self.csi.clear();
                        State::Csi
                    }
                    b']' | b'P' | b'X' | b'^' | b'_' => State::String,
                    0x20..=0x2f => State::EscapeIntermediate,
                    0x1b => State::Escape,
                    _ => State::Ground,
                },
                State::EscapeIntermediate => match b {
                    0x20..=0x2f => State::EscapeIntermediate,
                    _ => State::Ground,
                },
                State::Csi => match b {
                    0x20..=0x3f => {
                        if self.csi.len() < 64 {
                            self.csi.push(b);
                        }
                        State::Csi
                    }
                    0x40..=0x7e => {
                        self.end_csi(b);
                        State::Ground
                    }
                    0x1b => State::Escape,
                    // C0 controls are executed mid-sequence; anything else aborts it.
                    0x00..=0x1f => State::Csi,
                    _ => State::Ground,
                },
                State::String => match b {
                    0x07 => State::Ground,
                    0x1b => State::StringEscape,
                    _ => State::String,
                },
                State::StringEscape => match b {
                    b'\\' => State::Ground,
                    0x1b => State::StringEscape,
                    _ => State::String,
                },
            };
        }
    }

    /// A CSI ended: note the assistant starting or ending a synchronized update.
    fn end_csi(&mut self, final_byte: u8) {
        if !matches!(final_byte, b'h' | b'l') || self.csi.first() != Some(&b'?') {
            return;
        }
        let modes = &self.csi[1..];
        if modes.split(|&c| c == b';').any(|mode| mode == b"2026") {
            self.synchronized = final_byte == b'h';
        }
    }

    /// Between whole sequences and characters, outside any synchronized update.
    pub fn at_rest(&self) -> bool {
        self.state == State::Ground && !self.synchronized
    }

    /// Inside the assistant's synchronized update.
    pub fn in_synchronized_update(&self) -> bool {
        self.synchronized
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn after(chunks: &[&[u8]]) -> OutputBoundary {
        let mut b = OutputBoundary::default();
        for chunk in chunks {
            b.scan(chunk);
        }
        b
    }

    #[test]
    fn whole_sequences_and_text_leave_it_at_rest() {
        assert!(after(&[b"hello \x1b[1;31mred\x1b[0m \x1b]0;title\x07 done"]).at_rest());
        assert!(after(&["héllo ⣿ 漢".as_bytes()]).at_rest());
        assert!(after(&[b"\x1b(B\x1b7\x1b8"]).at_rest());
    }

    #[test]
    fn a_split_sequence_or_character_is_not_at_rest_until_it_ends() {
        let mut b = after(&[b"text \x1b[38;2;10;"]);
        assert!(!b.at_rest());
        b.scan(b"20;30m more");
        assert!(b.at_rest());
        let mut b = after(&[b"osc \x1b]8;;https://exa"]);
        assert!(!b.at_rest());
        b.scan(b"mple.com\x1b\\");
        assert!(b.at_rest());
        let bytes = "⣿".as_bytes();
        let mut b = after(&[&bytes[..2]]);
        assert!(!b.at_rest());
        b.scan(&bytes[2..]);
        assert!(b.at_rest());
    }

    #[test]
    fn a_synchronized_update_holds_until_it_ends() {
        let mut b = after(&[b"\x1b[?2026h\x1b[Hframe"]);
        assert!(!b.at_rest());
        assert!(b.in_synchronized_update());
        b.scan(b" more\x1b[?2026l");
        assert!(b.at_rest());
        // Other private modes don't count.
        assert!(after(&[b"\x1b[?25l\x1b[?1049h"]).at_rest());
        assert!(!after(&[b"\x1b[?1;2026h"]).at_rest());
    }
}
