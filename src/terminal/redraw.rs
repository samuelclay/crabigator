//! Full screen redraw detection
//!
//! Detects escape sequences that indicate the CLI has reset the scroll region,
//! requiring us to re-establish our scroll region constraints.
//!
//! Sequences detected:
//! - `\x1b[r` - Reset scroll region (DECSTBM with no params)
//!
//! Note: We intentionally do NOT trigger on `\x1b[2J` (clear screen) since that
//! operates within the existing scroll region and doesn't require re-establishment.

/// Parse state for redraw sequence detection
#[derive(Clone, Copy, Debug)]
enum RedrawParseState {
    Idle,
    Esc,
    EscBracket,
}

/// Result from scanning PTY output for redraw sequences
pub struct RedrawScanResult {
    /// Output bytes to pass through (sequences are NOT stripped)
    pub output: Vec<u8>,
    /// Whether a scroll region reset was detected
    pub needs_redraw: bool,
}

/// Scans PTY output for escape sequences that reset the scroll region
pub struct RedrawScanner {
    state: RedrawParseState,
}

impl RedrawScanner {
    pub fn new() -> Self {
        Self {
            state: RedrawParseState::Idle,
        }
    }

    /// Scan data for scroll region reset sequences.
    /// Passes through ALL bytes (including the sequences).
    /// Only signals whether a scroll region reset was detected.
    pub fn scan(&mut self, data: &[u8]) -> RedrawScanResult {
        let mut needs_redraw = false;

        for &byte in data {
            match self.state {
                RedrawParseState::Idle => {
                    if byte == 0x1b {
                        self.state = RedrawParseState::Esc;
                    }
                }
                RedrawParseState::Esc => {
                    if byte == b'[' {
                        self.state = RedrawParseState::EscBracket;
                    } else {
                        self.state = RedrawParseState::Idle;
                        // Check if this is another ESC starting a new sequence
                        if byte == 0x1b {
                            self.state = RedrawParseState::Esc;
                        }
                    }
                }
                RedrawParseState::EscBracket => {
                    if byte == b'r' {
                        // ESC [ r - Reset scroll region to full terminal
                        // This breaks our layout, so we need to re-establish
                        needs_redraw = true;
                    }
                    self.state = RedrawParseState::Idle;
                    // Check if this is another ESC
                    if byte == 0x1b {
                        self.state = RedrawParseState::Esc;
                    }
                }
            }
        }

        RedrawScanResult {
            output: data.to_vec(),
            needs_redraw,
        }
    }
}

impl Default for RedrawScanner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scroll_region_reset_detection() {
        let mut scanner = RedrawScanner::new();

        // Normal output - no redraw needed
        let result = scanner.scan(b"hello world");
        assert!(!result.needs_redraw);
        assert_eq!(result.output, b"hello world");

        // Reset scroll region - needs redraw
        let result = scanner.scan(b"\x1b[r");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"\x1b[r");
    }

    #[test]
    fn test_clear_screen_does_not_trigger() {
        let mut scanner = RedrawScanner::new();

        // Clear screen does NOT trigger redraw (operates within scroll region)
        let result = scanner.scan(b"\x1b[2J");
        assert!(!result.needs_redraw);
        assert_eq!(result.output, b"\x1b[2J");
    }

    #[test]
    fn test_embedded_sequences() {
        let mut scanner = RedrawScanner::new();

        // Scroll region reset embedded in output
        let result = scanner.scan(b"before\x1b[rafter");
        assert!(result.needs_redraw);
        assert_eq!(result.output, b"before\x1b[rafter");
    }

    #[test]
    fn test_partial_sequences() {
        let mut scanner = RedrawScanner::new();

        // Partial sequence (state carries across calls)
        let result = scanner.scan(b"\x1b[");
        assert!(!result.needs_redraw);

        let result = scanner.scan(b"r");
        assert!(result.needs_redraw);
    }

    #[test]
    fn test_non_matching_sequences() {
        let mut scanner = RedrawScanner::new();

        // Other CSI sequences should not trigger
        let result = scanner.scan(b"\x1b[1J"); // Clear to cursor
        assert!(!result.needs_redraw);

        let result = scanner.scan(b"\x1b[3;10r"); // Set scroll region WITH params (not reset)
        assert!(!result.needs_redraw);

        let result = scanner.scan(b"\x1b[H"); // Cursor home
        assert!(!result.needs_redraw);

        let result = scanner.scan(b"\x1b[2J"); // Clear screen
        assert!(!result.needs_redraw);
    }

    #[test]
    fn test_multiple_sequences() {
        let mut scanner = RedrawScanner::new();

        // Multiple sequences - only triggers if scroll region reset is present
        let result = scanner.scan(b"\x1b[H\x1b[2J\x1b[3J");
        assert!(!result.needs_redraw); // No \x1b[r present

        let result = scanner.scan(b"\x1b[H\x1b[r\x1b[3J");
        assert!(result.needs_redraw); // \x1b[r present
    }
}
