//! OSC (Operating System Command) scanner
//!
//! Scans PTY output for OSC title sequences (ESC ] 0 ; title BEL or ESC ] 2 ; title BEL)
//! and extracts the title while passing through all bytes unchanged.

/// State machine for parsing OSC title sequences
#[derive(Clone, Copy, Debug)]
enum OscParseState {
    Idle,
    Esc,        // Saw ESC
    OscStart,   // Saw ESC ]
    TitleType,  // Saw ESC ] 0 or ESC ] 2
    Collecting, // Saw ESC ] N ; - collecting title bytes
    MaybeST,    // Saw ESC while collecting (might be ST)
}

/// Scans PTY output for OSC title sequences
///
/// Extracts window/icon titles from terminal escape sequences while
/// passing through all bytes unchanged to the output.
pub struct OscScanner {
    state: OscParseState,
    pending: Vec<u8>,
    title_buf: Vec<u8>,
}

impl OscScanner {
    pub fn new() -> Self {
        Self {
            state: OscParseState::Idle,
            pending: Vec::with_capacity(64),
            title_buf: Vec::with_capacity(128),
        }
    }

    fn reset(&mut self) {
        self.state = OscParseState::Idle;
        self.pending.clear();
        self.title_buf.clear();
    }

    /// Scan data for OSC title sequences.
    /// Returns (passthrough_bytes, Option<extracted_title>).
    /// All input bytes are included in passthrough (no suppression).
    pub fn scan(&mut self, data: &[u8]) -> (Vec<u8>, Option<String>) {
        let mut output = Vec::with_capacity(data.len() + self.pending.len());
        let mut extracted_title: Option<String> = None;

        for &byte in data {
            match self.state {
                OscParseState::Idle => {
                    if byte == 0x1b {
                        // Flush any pending bytes first
                        output.extend_from_slice(&self.pending);
                        self.pending.clear();
                        self.pending.push(byte);
                        self.state = OscParseState::Esc;
                    } else {
                        output.push(byte);
                    }
                }
                OscParseState::Esc => {
                    self.pending.push(byte);
                    if byte == b']' {
                        self.state = OscParseState::OscStart;
                    } else {
                        // Not an OSC sequence, flush pending
                        output.extend_from_slice(&self.pending);
                        self.reset();
                    }
                }
                OscParseState::OscStart => {
                    self.pending.push(byte);
                    if byte == b'0' || byte == b'2' {
                        self.state = OscParseState::TitleType;
                    } else {
                        // Not a title sequence
                        output.extend_from_slice(&self.pending);
                        self.reset();
                    }
                }
                OscParseState::TitleType => {
                    self.pending.push(byte);
                    if byte == b';' {
                        self.state = OscParseState::Collecting;
                        self.title_buf.clear();
                    } else {
                        // Invalid sequence
                        output.extend_from_slice(&self.pending);
                        self.reset();
                    }
                }
                OscParseState::Collecting => {
                    self.pending.push(byte);
                    if byte == 0x07 {
                        // BEL terminator - extract title
                        let title = String::from_utf8_lossy(&self.title_buf).to_string();
                        extracted_title = Some(title);
                        output.extend_from_slice(&self.pending);
                        self.reset();
                    } else if byte == 0x1b {
                        // Might be ST terminator
                        self.state = OscParseState::MaybeST;
                    } else if self.title_buf.len() < 256 {
                        // Accumulate title (with limit)
                        self.title_buf.push(byte);
                    }
                    // If title_buf is full, keep collecting but don't add more
                }
                OscParseState::MaybeST => {
                    self.pending.push(byte);
                    if byte == b'\\' {
                        // ST terminator (ESC \) - extract title
                        let title = String::from_utf8_lossy(&self.title_buf).to_string();
                        extracted_title = Some(title);
                        output.extend_from_slice(&self.pending);
                        self.reset();
                    } else {
                        // Not ST, the ESC might start a new sequence
                        // Flush all pending up to the ESC, then restart
                        let esc_pos = self.pending.len() - 2; // Position of ESC
                        output.extend_from_slice(&self.pending[..esc_pos]);
                        self.pending.drain(..esc_pos);
                        // Now pending has [ESC, byte]
                        if byte == b']' {
                            self.state = OscParseState::OscStart;
                        } else {
                            output.extend_from_slice(&self.pending);
                            self.reset();
                        }
                    }
                }
            }
        }

        (output, extracted_title)
    }
}

impl Default for OscScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_osc_title_parsing() {
        let mut scanner = OscScanner::new();
        // Exact bytes from Claude Code: ESC ] 0 ; ✳ CLAUDE.md Refactoring BEL
        let input: &[u8] = b"\x1b]0;\xe2\x9c\xb3 CLAUDE.md Refactoring\x07";
        let (passthrough, title) = scanner.scan(input);
        assert_eq!(title, Some("✳ CLAUDE.md Refactoring".to_string()));
        assert_eq!(passthrough, input.to_vec());
    }

    #[test]
    fn test_osc_title_in_stream() {
        let mut scanner = OscScanner::new();
        // OSC sequence embedded in other output
        let input: &[u8] = b"some text\x1b]0;My Title\x07more text";
        let (passthrough, title) = scanner.scan(input);
        assert_eq!(title, Some("My Title".to_string()));
        assert_eq!(passthrough, input.to_vec());
    }
}
