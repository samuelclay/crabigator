//! Scroll region filter
//!
//! Detects DECSTBM "full-screen reset" sequences and injects our scroll region
//! immediately after to maintain constraints. All bytes pass through immediately
//! (no buffering) to avoid display glitches.
//!
//! Reset forms detected (followed by scroll region injection):
//! - `\x1b[r` - No params (most common)
//! - `\x1b[0r` - Single zero
//! - `\x1b[;r` - Empty params (semicolon only)
//! - `\x1b[0;0r` - Both zeros
//!
//! Parameterized sequences like `\x1b[3;15r` (sub-regions) pass through unchanged.

/// Parse state for scroll region sequence detection
#[derive(Clone, Copy, Debug)]
enum ParseState {
    Idle,
    Esc,                              // Saw ESC
    EscBracket,                       // Saw ESC [
    Param1(u16),                      // Parsing first numeric param
    AfterSemicolon(u16),              // Saw semicolon after first param
    Param2(u16, u16),                 // Parsing second numeric param
}

/// Result from filtering PTY output
pub struct ScrollRegionFilterResult {
    /// Output bytes (all input passed through, with scroll region injected after resets)
    pub output: Vec<u8>,
    /// Whether a scroll region reset was detected
    pub needs_redraw: bool,
}

/// Filters PTY output to inject scroll region after reset sequences
pub struct ScrollRegionFilter {
    state: ParseState,
    pty_rows: u16,
}

impl ScrollRegionFilter {
    pub fn new(pty_rows: u16) -> Self {
        Self {
            state: ParseState::Idle,
            pty_rows,
        }
    }

    /// Update the PTY rows (call on terminal resize)
    pub fn set_pty_rows(&mut self, pty_rows: u16) {
        self.pty_rows = pty_rows;
    }

    /// Check if params indicate a "full reset" (needs scroll region injection)
    fn is_full_reset(param1: u16, param2: u16) -> bool {
        param1 == 0 && param2 == 0
    }

    /// Generate our scroll region sequence
    fn scroll_region_sequence(&self) -> Vec<u8> {
        format!("\x1b[1;{}r", self.pty_rows).into_bytes()
    }

    /// Filter data, injecting scroll region after full-screen resets.
    /// All input bytes pass through immediately (no buffering).
    pub fn scan(&mut self, data: &[u8]) -> ScrollRegionFilterResult {
        // Estimate output size: input + possible scroll region injections
        let mut output = Vec::with_capacity(data.len() + 16);
        let mut needs_redraw = false;

        for &byte in data {
            // Always pass through the byte immediately
            output.push(byte);

            match self.state {
                ParseState::Idle => {
                    if byte == 0x1b {
                        self.state = ParseState::Esc;
                    }
                }
                ParseState::Esc => {
                    if byte == b'[' {
                        self.state = ParseState::EscBracket;
                    } else {
                        self.state = ParseState::Idle;
                        if byte == 0x1b {
                            self.state = ParseState::Esc;
                        }
                    }
                }
                ParseState::EscBracket => {
                    if byte == b'r' {
                        // ESC [ r - Full reset with no params, inject our scroll region
                        needs_redraw = true;
                        output.extend_from_slice(&self.scroll_region_sequence());
                        self.state = ParseState::Idle;
                    } else if byte == b';' {
                        self.state = ParseState::AfterSemicolon(0);
                    } else if byte.is_ascii_digit() {
                        self.state = ParseState::Param1((byte - b'0') as u16);
                    } else {
                        // Not a DECSTBM sequence
                        self.state = ParseState::Idle;
                        if byte == 0x1b {
                            self.state = ParseState::Esc;
                        }
                    }
                }
                ParseState::Param1(p1) => {
                    if byte == b'r' {
                        if p1 == 0 {
                            // ESC [ 0 r - Full reset, inject scroll region
                            needs_redraw = true;
                            output.extend_from_slice(&self.scroll_region_sequence());
                        }
                        self.state = ParseState::Idle;
                    } else if byte == b';' {
                        self.state = ParseState::AfterSemicolon(p1);
                    } else if byte.is_ascii_digit() {
                        let new_p1 = p1.saturating_mul(10).saturating_add((byte - b'0') as u16);
                        self.state = ParseState::Param1(new_p1);
                    } else {
                        self.state = ParseState::Idle;
                        if byte == 0x1b {
                            self.state = ParseState::Esc;
                        }
                    }
                }
                ParseState::AfterSemicolon(p1) => {
                    if byte == b'r' {
                        if Self::is_full_reset(p1, 0) {
                            // ESC [ ; r or ESC [ 0 ; r - Full reset
                            needs_redraw = true;
                            output.extend_from_slice(&self.scroll_region_sequence());
                        }
                        self.state = ParseState::Idle;
                    } else if byte.is_ascii_digit() {
                        self.state = ParseState::Param2(p1, (byte - b'0') as u16);
                    } else {
                        self.state = ParseState::Idle;
                        if byte == 0x1b {
                            self.state = ParseState::Esc;
                        }
                    }
                }
                ParseState::Param2(p1, p2) => {
                    if byte == b'r' {
                        if Self::is_full_reset(p1, p2) {
                            // ESC [ 0 ; 0 r - Full reset
                            needs_redraw = true;
                            output.extend_from_slice(&self.scroll_region_sequence());
                        }
                        self.state = ParseState::Idle;
                    } else if byte.is_ascii_digit() {
                        let new_p2 = p2.saturating_mul(10).saturating_add((byte - b'0') as u16);
                        self.state = ParseState::Param2(p1, new_p2);
                    } else {
                        self.state = ParseState::Idle;
                        if byte == 0x1b {
                            self.state = ParseState::Esc;
                        }
                    }
                }
            }
        }

        ScrollRegionFilterResult { output, needs_redraw }
    }
}

// Keep old name as alias for backward compatibility
pub type RedrawScanner = ScrollRegionFilter;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_params_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[r");
        assert!(result.needs_redraw);
        // Original reset passes through, then our scroll region is injected
        assert_eq!(result.output, b"\x1b[r\x1b[1;20r");
    }

    #[test]
    fn test_zero_param_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[0r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[0r\x1b[1;20r");
    }

    #[test]
    fn test_semicolon_only_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[;r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[;r\x1b[1;20r");
    }

    #[test]
    fn test_zero_zero_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[0;0r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[0;0r\x1b[1;20r");
    }

    #[test]
    fn test_sub_region_passthrough() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[3;15r");
        assert!(!result.needs_redraw);
        // Sub-region passes through unchanged, no injection
        assert_eq!(result.output, b"\x1b[3;15r");
    }

    #[test]
    fn test_normal_output_passthrough() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"hello world");
        assert!(!result.needs_redraw);
        assert_eq!(result.output, b"hello world");
    }

    #[test]
    fn test_other_csi_passthrough() {
        let mut filter = ScrollRegionFilter::new(20);

        let result = filter.scan(b"\x1b[2J");
        assert!(!result.needs_redraw);
        assert_eq!(result.output, b"\x1b[2J");

        let result = filter.scan(b"\x1b[H");
        assert!(!result.needs_redraw);
        assert_eq!(result.output, b"\x1b[H");
    }

    #[test]
    fn test_embedded_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"before\x1b[rafter");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"before\x1b[r\x1b[1;20rafter");
    }

    #[test]
    fn test_split_sequence() {
        let mut filter = ScrollRegionFilter::new(20);

        // First chunk - state carries over
        let result1 = filter.scan(b"text\x1b[");
        assert!(!result1.needs_redraw);
        assert_eq!(result1.output, b"text\x1b[");

        // Second chunk completes the sequence
        let result2 = filter.scan(b"r");
        assert!(result2.needs_redraw);
        assert_eq!(result2.output, b"r\x1b[1;20r");
    }

    #[test]
    fn test_resize_updates_sequence() {
        let mut filter = ScrollRegionFilter::new(20);

        let result = filter.scan(b"\x1b[r");
        assert_eq!(result.output, b"\x1b[r\x1b[1;20r");

        filter.set_pty_rows(30);

        let result = filter.scan(b"\x1b[r");
        assert_eq!(result.output, b"\x1b[r\x1b[1;30r");
    }

    #[test]
    fn test_multiple_resets() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[H\x1b[r\x1b[2J\x1b[0;0r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[H\x1b[r\x1b[1;20r\x1b[2J\x1b[0;0r\x1b[1;20r");
    }

    #[test]
    fn test_backward_compat_names() {
        let mut scanner = RedrawScanner::new(20);
        let result = scanner.scan(b"\x1b[r");
        assert!(result.needs_redraw);
    }
}
