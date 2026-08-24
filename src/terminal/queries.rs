//! Terminal capability query responder for full-screen TUI platforms.
//!
//! Crabigator reads host input through crossterm's event stream, which only
//! surfaces key/mouse/paste events — responses the real terminal sends to a
//! child's capability queries never reach the child's PTY. opencode's TUI
//! blocks on that negotiation (primary device attributes, kitty keyboard,
//! DECRQM, colors) before its first paint, so crabigator answers those
//! queries itself and strips them from the host-bound stream so the host
//! terminal never produces a second, orphaned answer.

/// Longest query sequence worth buffering before giving up and passing
/// bytes through unchanged.
const MAX_PENDING: usize = 24;

#[derive(Clone, Copy, Debug)]
enum State {
    Idle,
    Esc,
    /// Accumulating CSI parameter/intermediate bytes
    Csi,
    /// Accumulating an OSC payload that is still a prefix of a color query
    Osc,
    /// Saw ESC inside an OSC payload (possible ST terminator)
    OscEsc,
}

/// Result of scanning one chunk of PTY output.
pub struct QueryScanResult {
    /// Bytes to forward to the host terminal (queries removed)
    pub output: Vec<u8>,
    /// Responses to write back to the child PTY
    pub responses: Vec<u8>,
}

pub struct QueryResponder {
    enabled: bool,
    state: State,
    pending: Vec<u8>,
}

impl QueryResponder {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            state: State::Idle,
            pending: Vec::with_capacity(MAX_PENDING),
        }
    }

    pub fn scan(&mut self, data: &[u8]) -> QueryScanResult {
        if !self.enabled {
            return QueryScanResult {
                output: data.to_vec(),
                responses: Vec::new(),
            };
        }

        let mut output = Vec::with_capacity(data.len());
        let mut responses = Vec::new();

        for &byte in data {
            match self.state {
                State::Idle => {
                    if byte == 0x1b {
                        self.pending.clear();
                        self.pending.push(byte);
                        self.state = State::Esc;
                    } else {
                        output.push(byte);
                    }
                }
                State::Esc => {
                    self.pending.push(byte);
                    match byte {
                        b'[' => self.state = State::Csi,
                        b']' => self.state = State::Osc,
                        _ => self.flush_or_restart(&mut output, byte),
                    }
                }
                State::Csi => {
                    self.pending.push(byte);
                    if (0x40..=0x7e).contains(&byte) {
                        // Final byte: the sequence is complete
                        let answer = classify_csi(&self.pending[2..self.pending.len() - 1], byte);
                        self.answer_or_flush(answer, &mut output, &mut responses);
                    } else if self.pending.len() > MAX_PENDING {
                        self.flush(&mut output);
                    }
                }
                State::Osc => {
                    self.pending.push(byte);
                    if byte == 0x07 {
                        let answer =
                            classify_osc(&self.pending[2..self.pending.len() - 1], b"\x07");
                        self.answer_or_flush(answer, &mut output, &mut responses);
                    } else if byte == 0x1b {
                        self.state = State::OscEsc;
                    } else if !is_color_query_prefix(&self.pending[2..]) {
                        // Not a color query (e.g. a title) - stop buffering;
                        // the rest of the sequence streams through Idle.
                        self.flush(&mut output);
                    }
                }
                State::OscEsc => {
                    self.pending.push(byte);
                    if byte == b'\\' {
                        let answer =
                            classify_osc(&self.pending[2..self.pending.len() - 2], b"\x1b\\");
                        self.answer_or_flush(answer, &mut output, &mut responses);
                    } else {
                        self.flush_or_restart(&mut output, byte);
                    }
                }
            }
        }

        QueryScanResult { output, responses }
    }

    /// Finish a complete sequence: either answer it (and strip it from the
    /// host-bound stream) or pass it through untouched.
    fn answer_or_flush(
        &mut self,
        answer: Option<Vec<u8>>,
        output: &mut Vec<u8>,
        responses: &mut Vec<u8>,
    ) {
        match answer {
            Some(response) => {
                responses.extend_from_slice(&response);
                self.pending.clear();
                self.state = State::Idle;
            }
            None => self.flush(output),
        }
    }

    fn flush(&mut self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.pending);
        self.pending.clear();
        self.state = State::Idle;
    }

    /// Flush pending bytes; if the current byte is ESC, keep it as the start
    /// of a new sequence.
    fn flush_or_restart(&mut self, output: &mut Vec<u8>, byte: u8) {
        if byte == 0x1b {
            let keep_from = self.pending.len() - 1;
            output.extend_from_slice(&self.pending[..keep_from]);
            self.pending.clear();
            self.pending.push(0x1b);
            self.state = State::Esc;
        } else {
            self.flush(output);
        }
    }
}

/// Whether an in-progress OSC payload could still become a color query.
fn is_color_query_prefix(payload: &[u8]) -> bool {
    const QUERIES: [&[u8]; 2] = [b"10;?", b"11;?"];
    QUERIES
        .iter()
        .any(|query| query.starts_with(payload) || payload.starts_with(query))
}

/// Classify a complete CSI sequence; Some(response) means crabigator
/// answers it and strips it, None passes it through.
fn classify_csi(params: &[u8], final_byte: u8) -> Option<Vec<u8>> {
    match final_byte {
        // Primary Device Attributes - the negotiation barrier opencode
        // blocks on. Answer as a VT102.
        b'c' if params.is_empty() || params == b"0" => Some(b"\x1b[?6c".to_vec()),
        // Kitty keyboard protocol query: no kitty flags supported, so the
        // child sticks to legacy key encoding (which crossterm forwards).
        b'u' if params == b"?" => Some(b"\x1b[?0u".to_vec()),
        // DECRQM private mode request: report "not recognized"
        b'p' if params.first() == Some(&b'?') && params.last() == Some(&b'$') => {
            let mode = &params[1..params.len() - 1];
            if !mode.is_empty() && mode.iter().all(u8::is_ascii_digit) {
                let mut response = b"\x1b[?".to_vec();
                response.extend_from_slice(mode);
                response.extend_from_slice(b";0$y");
                Some(response)
            } else {
                None
            }
        }
        // XTVERSION
        b'q' if params == b">0" => Some(b"\x1bP>|crabigator\x1b\\".to_vec()),
        // Text area size in pixels: unknown
        b't' if params == b"14" => Some(b"\x1b[4;0;0t".to_vec()),
        _ => None,
    }
}

/// Classify a complete OSC payload; color queries get dark-theme defaults.
fn classify_osc(payload: &[u8], terminator: &[u8]) -> Option<Vec<u8>> {
    let body: &[u8] = match payload {
        b"10;?" => b"\x1b]10;rgb:ffff/ffff/ffff",
        b"11;?" => b"\x1b]11;rgb:0000/0000/0000",
        _ => return None,
    };
    let mut response = body.to_vec();
    response.extend_from_slice(terminator);
    Some(response)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn scan_all(responder: &mut QueryResponder, data: &[u8]) -> (Vec<u8>, Vec<u8>) {
        let result = responder.scan(data);
        (result.output, result.responses)
    }

    #[test]
    fn answers_and_strips_da1() {
        let mut responder = QueryResponder::new(true);
        let (output, responses) = scan_all(&mut responder, b"before\x1b[cafter");
        assert_eq!(output, b"beforeafter");
        assert_eq!(responses, b"\x1b[?6c");
    }

    #[test]
    fn answers_kitty_decrqm_version_winsize() {
        let mut responder = QueryResponder::new(true);
        let (output, responses) = scan_all(&mut responder, b"\x1b[?u\x1b[?2004$p\x1b[>0q\x1b[14t");
        assert_eq!(output, b"");
        assert_eq!(
            responses,
            b"\x1b[?0u\x1b[?2004;0$y\x1bP>|crabigator\x1b\\\x1b[4;0;0t"
        );
    }

    #[test]
    fn answers_color_queries_with_matching_terminator() {
        let mut responder = QueryResponder::new(true);
        let (output, responses) = scan_all(&mut responder, b"\x1b]11;?\x07\x1b]10;?\x1b\\");
        assert_eq!(output, b"");
        assert_eq!(
            responses,
            b"\x1b]11;rgb:0000/0000/0000\x07\x1b]10;rgb:ffff/ffff/ffff\x1b\\"
        );
    }

    #[test]
    fn passes_through_everything_else() {
        let mut responder = QueryResponder::new(true);
        let input: &[u8] = b"\x1b[1;2H\x1b[38;2;1;2;3mtext\x1b[?1049h\x1b]0;title\x07\x1b[5c";
        let (output, responses) = scan_all(&mut responder, input);
        assert_eq!(output, input);
        assert_eq!(responses, b"");
    }

    #[test]
    fn handles_queries_split_across_chunks() {
        let mut responder = QueryResponder::new(true);
        let (out1, resp1) = scan_all(&mut responder, b"x\x1b[?2004");
        let (out2, resp2) = scan_all(&mut responder, b"$py");
        assert_eq!([out1, out2].concat(), b"xy");
        assert_eq!([resp1, resp2].concat(), b"\x1b[?2004;0$y");
    }

    #[test]
    fn disabled_responder_is_transparent() {
        let mut responder = QueryResponder::new(false);
        let (output, responses) = scan_all(&mut responder, b"\x1b[c\x1b[?u");
        assert_eq!(output, b"\x1b[c\x1b[?u");
        assert_eq!(responses, b"");
    }
}
