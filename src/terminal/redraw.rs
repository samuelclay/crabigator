//! Scroll region filter
//!
//! Virtualizes DECSTBM sequences from the child PTY so omitted/default bottoms
//! resolve to the child's viewport instead of the real terminal's full height.
//!
//! Forms translated:
//! - `\x1b[r` - No params (most common)
//! - `\x1b[0r` - Single zero
//! - `\x1b[;r` - Empty params (semicolon only)
//! - `\x1b[0;0r` - Both zeros
//! - `\x1b[3r`, `\x1b[3;r` - Omitted bottom
//!
//! Parameterized sequences like `\x1b[3;15r` (sub-regions) pass through unchanged
//! unless their bottom would reach into Crabigator's widget area.

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
    /// Output bytes with child full-screen resets translated to the PTY viewport.
    pub output: Vec<u8>,
    /// Whether a scroll region reset was detected
    pub needs_redraw: bool,
}

/// Filters PTY output to translate full-screen scroll-region resets.
pub struct ScrollRegionFilter {
    state: ParseState,
    pty_rows: u16,
    pending: Vec<u8>,
}

impl ScrollRegionFilter {
    pub fn new(pty_rows: u16) -> Self {
        Self {
            state: ParseState::Idle,
            pty_rows,
            pending: Vec::with_capacity(16),
        }
    }

    /// Update the PTY rows (call on terminal resize)
    pub fn set_pty_rows(&mut self, pty_rows: u16) {
        self.pty_rows = pty_rows;
    }

    /// Generate the child's full-height scroll region in host-terminal coordinates.
    fn scroll_region_sequence(&self, top: u16, bottom: u16) -> Vec<u8> {
        let top = top.max(1).min(self.pty_rows);
        let bottom = bottom.max(top).min(self.pty_rows);
        format!("\x1b[{};{}r", top, bottom).into_bytes()
    }

    fn flush_pending(&mut self, output: &mut Vec<u8>) {
        output.extend_from_slice(&self.pending);
        self.pending.clear();
        self.state = ParseState::Idle;
    }

    fn flush_or_restart_on_current(&mut self, output: &mut Vec<u8>, byte: u8) {
        if byte == 0x1b {
            let restart_at = self.pending.len().saturating_sub(1);
            output.extend_from_slice(&self.pending[..restart_at]);
            self.pending.clear();
            self.pending.push(0x1b);
            self.state = ParseState::Esc;
        } else {
            self.flush_pending(output);
        }
    }

    fn translate_region(&mut self, output: &mut Vec<u8>, top: u16, bottom: u16) {
        output.extend_from_slice(&self.scroll_region_sequence(top, bottom));
        self.pending.clear();
        self.state = ParseState::Idle;
    }

    /// Filter data, replacing full-screen resets with our PTY-height reset.
    /// Partial CSI sequences are held only until they can be classified.
    pub fn scan(&mut self, data: &[u8]) -> ScrollRegionFilterResult {
        let mut output = Vec::with_capacity(data.len());
        let mut needs_redraw = false;

        for &byte in data {
            match self.state {
                ParseState::Idle => {
                    if byte == 0x1b {
                        self.pending.clear();
                        self.pending.push(byte);
                        self.state = ParseState::Esc;
                    } else {
                        output.push(byte);
                    }
                }
                ParseState::Esc => {
                    self.pending.push(byte);
                    if byte == b'[' {
                        self.state = ParseState::EscBracket;
                    } else {
                        self.flush_or_restart_on_current(&mut output, byte);
                    }
                }
                ParseState::EscBracket => {
                    self.pending.push(byte);
                    if byte == b'r' {
                        // ESC [ r - Full reset with no params
                        needs_redraw = true;
                        self.translate_region(&mut output, 1, self.pty_rows);
                    } else if byte == b';' {
                        self.state = ParseState::AfterSemicolon(0);
                    } else if byte.is_ascii_digit() {
                        self.state = ParseState::Param1((byte - b'0') as u16);
                    } else {
                        self.flush_or_restart_on_current(&mut output, byte);
                    }
                }
                ParseState::Param1(p1) => {
                    self.pending.push(byte);
                    if byte == b'r' {
                        // ESC [ N r - bottom omitted, so the child means its
                        // own terminal bottom, not the host terminal bottom.
                        needs_redraw = true;
                        self.translate_region(&mut output, p1, self.pty_rows);
                    } else if byte == b';' {
                        self.state = ParseState::AfterSemicolon(p1);
                    } else if byte.is_ascii_digit() {
                        let new_p1 = p1.saturating_mul(10).saturating_add((byte - b'0') as u16);
                        self.state = ParseState::Param1(new_p1);
                    } else {
                        self.flush_or_restart_on_current(&mut output, byte);
                    }
                }
                ParseState::AfterSemicolon(p1) => {
                    self.pending.push(byte);
                    if byte == b'r' {
                        // ESC [ N ; r - bottom omitted, same as above.
                        needs_redraw = true;
                        self.translate_region(&mut output, p1, self.pty_rows);
                    } else if byte.is_ascii_digit() {
                        self.state = ParseState::Param2(p1, (byte - b'0') as u16);
                    } else {
                        self.flush_or_restart_on_current(&mut output, byte);
                    }
                }
                ParseState::Param2(p1, p2) => {
                    self.pending.push(byte);
                    if byte == b'r' {
                        if p2 == 0 {
                            // ESC [ N ; 0 r - bottom default/full height.
                            needs_redraw = true;
                            self.translate_region(&mut output, p1, self.pty_rows);
                        } else if p2 > self.pty_rows {
                            needs_redraw = true;
                            self.translate_region(&mut output, p1, p2);
                        } else {
                            self.flush_pending(&mut output);
                        }
                    } else if byte.is_ascii_digit() {
                        let new_p2 = p2.saturating_mul(10).saturating_add((byte - b'0') as u16);
                        self.state = ParseState::Param2(p1, new_p2);
                    } else {
                        self.flush_or_restart_on_current(&mut output, byte);
                    }
                }
            }
        }

        ScrollRegionFilterResult { output, needs_redraw }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_params_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[1;20r");
    }

    #[test]
    fn test_zero_param_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[0r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[1;20r");
    }

    #[test]
    fn test_semicolon_only_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[;r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[1;20r");
    }

    #[test]
    fn test_zero_zero_reset() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[0;0r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[1;20r");
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
    fn test_omitted_or_oversized_bottom_is_constrained() {
        let mut filter = ScrollRegionFilter::new(20);

        let result = filter.scan(b"\x1b[3r\x1b[4;r\x1b[2;99r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[3;20r\x1b[4;20r\x1b[2;20r");
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
        assert_eq!(result.output, b"before\x1b[1;20rafter");
    }

    #[test]
    fn test_split_sequence() {
        let mut filter = ScrollRegionFilter::new(20);

        // First chunk - state carries over
        let result1 = filter.scan(b"text\x1b[");
        assert!(!result1.needs_redraw);
        assert_eq!(result1.output, b"text");

        // Second chunk completes the sequence
        let result2 = filter.scan(b"r");
        assert!(result2.needs_redraw);
        assert_eq!(result2.output, b"\x1b[1;20r");
    }

    #[test]
    fn test_resize_updates_sequence() {
        let mut filter = ScrollRegionFilter::new(20);

        let result = filter.scan(b"\x1b[r");
        assert_eq!(result.output, b"\x1b[1;20r");

        filter.set_pty_rows(30);

        let result = filter.scan(b"\x1b[r");
        assert_eq!(result.output, b"\x1b[1;30r");
    }

    #[test]
    fn test_multiple_resets() {
        let mut filter = ScrollRegionFilter::new(20);
        let result = filter.scan(b"\x1b[H\x1b[r\x1b[2J\x1b[0;0r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[H\x1b[1;20r\x1b[2J\x1b[1;20r");
    }

    #[test]
    fn test_codex_full_reset_between_subregions_is_translated() {
        let mut filter = ScrollRegionFilter::new(16);
        let result = filter.scan(b"\x1b[1;16r\x1b[1;1H\x1bM\x1bM\x1b[r\x1b[1;10r");

        assert!(result.needs_redraw);
        assert_eq!(
            result.output,
            b"\x1b[1;16r\x1b[1;1H\x1bM\x1bM\x1b[1;16r\x1b[1;10r"
        );
        assert!(!result.output.windows(3).any(|w| w == b"\x1b[r"));
    }
}
