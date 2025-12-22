//! DSR (Device Status Report) handler
//!
//! Handles terminal DSR responses for CLIs that request cursor position.
//! Parses ESC [ 6 n and ESC [ ? 6 n sequences.

/// Parse state for DSR sequence detection
#[derive(Clone, Copy, Debug)]
enum DsrParseState {
    Idle,
    Esc,
    EscBracket,
    EscBracketQuestion,
    EscBracket6,
    EscBracketQuestion6,
}

/// Chunk of PTY output after DSR scanning
pub enum DsrChunk {
    /// Regular output to pass through
    Output(Vec<u8>),
    /// DSR request detected - caller should respond with cursor position
    Request,
}

/// Handles terminal DSR (Device Status Report) sequences
///
/// Some CLIs send DSR requests (ESC [ 6 n) to query cursor position.
/// This handler detects those requests and separates them from normal output.
pub struct DsrHandler {
    state: DsrParseState,
    pending: Vec<u8>,
}

impl DsrHandler {
    pub fn new() -> Self {
        Self {
            state: DsrParseState::Idle,
            pending: Vec::new(),
        }
    }

    fn reset_with_byte(&mut self, current: &mut Vec<u8>, byte: u8) {
        if !self.pending.is_empty() {
            current.extend_from_slice(&self.pending);
            self.pending.clear();
        }
        self.state = DsrParseState::Idle;
        if byte == 0x1b {
            self.pending.push(byte);
            self.state = DsrParseState::Esc;
        } else {
            current.push(byte);
        }
    }

    /// Scan data for DSR sequences.
    /// Returns chunks of output and DSR requests.
    pub fn scan(&mut self, data: &[u8]) -> Vec<DsrChunk> {
        let mut chunks = Vec::new();
        let mut current = Vec::new();
        for &byte in data {
            match self.state {
                DsrParseState::Idle => {
                    if byte == 0x1b {
                        self.pending.clear();
                        self.pending.push(byte);
                        self.state = DsrParseState::Esc;
                    } else {
                        current.push(byte);
                    }
                }
                DsrParseState::Esc => {
                    if byte == b'[' {
                        self.pending.push(byte);
                        self.state = DsrParseState::EscBracket;
                    } else {
                        self.reset_with_byte(&mut current, byte);
                    }
                }
                DsrParseState::EscBracket => {
                    if byte == b'6' {
                        self.pending.push(byte);
                        self.state = DsrParseState::EscBracket6;
                    } else if byte == b'?' {
                        self.pending.push(byte);
                        self.state = DsrParseState::EscBracketQuestion;
                    } else {
                        self.reset_with_byte(&mut current, byte);
                    }
                }
                DsrParseState::EscBracketQuestion => {
                    if byte == b'6' {
                        self.pending.push(byte);
                        self.state = DsrParseState::EscBracketQuestion6;
                    } else {
                        self.reset_with_byte(&mut current, byte);
                    }
                }
                DsrParseState::EscBracket6 => {
                    if byte == b'n' {
                        self.pending.clear();
                        self.state = DsrParseState::Idle;
                        if !current.is_empty() {
                            chunks.push(DsrChunk::Output(current));
                            current = Vec::new();
                        }
                        chunks.push(DsrChunk::Request);
                    } else {
                        self.reset_with_byte(&mut current, byte);
                    }
                }
                DsrParseState::EscBracketQuestion6 => {
                    if byte == b'n' {
                        self.pending.clear();
                        self.state = DsrParseState::Idle;
                        if !current.is_empty() {
                            chunks.push(DsrChunk::Output(current));
                            current = Vec::new();
                        }
                        chunks.push(DsrChunk::Request);
                    } else {
                        self.reset_with_byte(&mut current, byte);
                    }
                }
            }
        }

        if !current.is_empty() {
            chunks.push(DsrChunk::Output(current));
        }

        chunks
    }
}

impl Default for DsrHandler {
    fn default() -> Self {
        Self::new()
    }
}
