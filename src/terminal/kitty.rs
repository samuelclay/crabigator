//! Kitty keyboard protocol tracking
//!
//! Crabigator forwards the child's `CSI > flags u` push and `CSI < u` pop to
//! the host terminal untouched, so the host starts reporting modified keys as
//! `CSI code ; modifiers u`. crossterm decodes those into key events, and the
//! input encoder needs to know which keyboard mode the child expects when it
//! turns the events back into bytes: Alt+Enter is `ESC CR` on a legacy
//! terminal but `CSI 13;3u` once the kitty protocol is on.

/// Longest push/pop/set sequence worth buffering before giving up.
const MAX_PENDING: usize = 16;

/// Deepest flag stack we keep; the protocol lets terminals cap it too.
const MAX_STACK: usize = 32;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Idle,
    Esc,
    Csi,
}

#[derive(Debug)]
pub struct KittyKeyboardTracker {
    state: State,
    pending: Vec<u8>,
    /// Flag stack pushed by the child. The base entry is the legacy mode
    /// (no flags) and is never popped.
    stack: Vec<u16>,
}

impl Default for KittyKeyboardTracker {
    fn default() -> Self {
        Self::new()
    }
}

impl KittyKeyboardTracker {
    pub fn new() -> Self {
        Self {
            state: State::Idle,
            pending: Vec::with_capacity(MAX_PENDING),
            stack: vec![0],
        }
    }

    /// True while the child has any kitty keyboard flags enabled.
    pub fn active(&self) -> bool {
        self.stack.last().copied().unwrap_or(0) != 0
    }

    /// Scan one chunk of PTY output for push, pop, and set sequences.
    pub fn scan(&mut self, data: &[u8]) {
        for &byte in data {
            match self.state {
                State::Idle => {
                    if byte == 0x1b {
                        self.restart();
                    }
                }
                State::Esc => {
                    if byte == b'[' {
                        self.pending.push(byte);
                        self.state = State::Csi;
                    } else {
                        self.reset_or_restart(byte);
                    }
                }
                State::Csi => {
                    if self.pending.len() == 2 {
                        // First byte after `CSI`: only the kitty prefixes matter.
                        if matches!(byte, b'>' | b'<' | b'=') {
                            self.pending.push(byte);
                        } else {
                            self.reset_or_restart(byte);
                        }
                    } else if byte == b'u' {
                        let prefix = self.pending[2];
                        let params = self.pending[3..].to_vec();
                        self.apply(prefix, &params);
                        self.state = State::Idle;
                        self.pending.clear();
                    } else if (byte.is_ascii_digit() || byte == b';')
                        && self.pending.len() < MAX_PENDING
                    {
                        self.pending.push(byte);
                    } else {
                        self.reset_or_restart(byte);
                    }
                }
            }
        }
    }

    fn restart(&mut self) {
        self.pending.clear();
        self.pending.push(0x1b);
        self.state = State::Esc;
    }

    fn reset_or_restart(&mut self, byte: u8) {
        if byte == 0x1b {
            self.restart();
        } else {
            self.pending.clear();
            self.state = State::Idle;
        }
    }

    fn apply(&mut self, prefix: u8, params: &[u8]) {
        let mut fields = params.split(|&b| b == b';').map(|field| {
            std::str::from_utf8(field)
                .ok()
                .and_then(|s| s.parse::<u16>().ok())
        });
        let first = fields.next().flatten();
        let second = fields.next().flatten();
        match prefix {
            b'>' => {
                if self.stack.len() == MAX_STACK {
                    self.stack.remove(1);
                }
                self.stack.push(first.unwrap_or(1));
            }
            b'<' => {
                let count = usize::from(first.unwrap_or(1));
                let keep = (self.stack.len().saturating_sub(count)).max(1);
                self.stack.truncate(keep);
            }
            b'=' => {
                let flags = first.unwrap_or(1);
                if let Some(current) = self.stack.last_mut() {
                    *current = match second.unwrap_or(1) {
                        2 => *current | flags,
                        3 => *current & !flags,
                        _ => flags,
                    };
                }
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn push_turns_the_protocol_on_and_pop_turns_it_off() {
        let mut tracker = KittyKeyboardTracker::new();
        assert!(!tracker.active());
        tracker.scan(b"\x1b[?u\x1b[>5u\x1b[>4;2m");
        assert!(tracker.active());
        tracker.scan(b"\x1b[<u");
        assert!(!tracker.active());
    }

    #[test]
    fn pop_never_removes_the_legacy_base() {
        let mut tracker = KittyKeyboardTracker::new();
        tracker.scan(b"\x1b[<u\x1b[<3u");
        assert!(!tracker.active());
        tracker.scan(b"\x1b[>1u\x1b[>1u\x1b[<5u");
        assert!(!tracker.active());
    }

    #[test]
    fn set_replaces_ors_and_clears_flags() {
        let mut tracker = KittyKeyboardTracker::new();
        tracker.scan(b"\x1b[=1;1u");
        assert!(tracker.active());
        tracker.scan(b"\x1b[=1;3u");
        assert!(!tracker.active());
        tracker.scan(b"\x1b[=4;2u");
        assert!(tracker.active());
        tracker.scan(b"\x1b[=0u");
        assert!(!tracker.active());
    }

    #[test]
    fn sequences_split_across_chunks_still_count() {
        let mut tracker = KittyKeyboardTracker::new();
        tracker.scan(b"text\x1b[>");
        tracker.scan(b"1");
        tracker.scan(b"u more");
        assert!(tracker.active());
    }

    #[test]
    fn unrelated_sequences_are_ignored() {
        let mut tracker = KittyKeyboardTracker::new();
        tracker.scan(b"\x1b[1;2H\x1b[?2004h\x1b[>0q\x1b[38;2;1;2;3mu\x1b[u\x1b]0;title\x07");
        assert!(!tracker.active());
    }
}
