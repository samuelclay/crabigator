//! Pairing banner - full-width display for mobile pairing
//!
//! Shows a prominent, clickable URL banner when no mobile devices are linked.
//! Renders as a single line above the widget row for maximum clarity.
//! Disappears automatically when pairing completes.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::terminal::escape::{self, color, bg, fg, hyperlink, RESET, RESET_FG};

/// Pairing state for the banner
#[derive(Clone, Debug, Default)]
pub struct PairingState {
    /// Whether we have any linked devices
    pub has_linked_devices: bool,
    /// Current pairing token (if generated)
    pub pairing_token: Option<String>,
    /// Human-readable pairing code (e.g., "ABC-DEF-GHI")
    pub pairing_code: Option<String>,
    /// Whether pairing just completed (for toast message)
    pub just_paired: Option<String>,
    /// Timestamp when just_paired was set (for auto-clear)
    pub just_paired_at: Option<std::time::Instant>,
}

impl PairingState {
    /// Check if we should show the pairing banner
    pub fn should_show_widget(&self) -> bool {
        // Show if not paired and we have a pairing code
        !self.has_linked_devices && self.pairing_code.is_some()
    }

    /// Check if we should show the "just paired" toast
    pub fn should_show_toast(&self) -> bool {
        if let (Some(_), Some(at)) = (&self.just_paired, self.just_paired_at) {
            // Show toast for 3 seconds
            at.elapsed().as_secs() < 3
        } else {
            false
        }
    }

    /// Clear the toast if it's expired
    pub fn maybe_clear_toast(&mut self) {
        if let Some(at) = self.just_paired_at {
            if at.elapsed().as_secs() >= 3 {
                self.just_paired = None;
                self.just_paired_at = None;
            }
        }
    }

    /// Set that pairing just completed
    /// Note: Does NOT clear pairing token - caller should regenerate a new one
    /// so additional devices can pair
    pub fn set_just_paired(&mut self, device_name: String) {
        self.just_paired = Some(device_name);
        self.just_paired_at = Some(std::time::Instant::now());
        self.has_linked_devices = true;
        // Mark that we need a new token (the old one is consumed)
        self.pairing_token = None;
        self.pairing_code = None;
    }

    /// Set a new pairing token (for pairing additional devices)
    #[allow(dead_code)]
    pub fn set_new_token(&mut self, token: String, code: String) {
        self.pairing_token = Some(token);
        self.pairing_code = Some(code);
    }

    /// Get the number of rows needed for the pairing banner (0 if not shown)
    pub fn banner_rows(&self) -> u16 {
        if self.should_show_widget() || self.should_show_toast() {
            2 // Top border + content (bottom border is the main separator)
        } else {
            0
        }
    }
}

/// Draw the full-width pairing banner
/// Returns the number of rows consumed (0, 1, or 2)
pub fn draw_pairing_banner(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    state: &PairingState,
) -> Result<u16> {
    // Nothing to show
    if !state.should_show_widget() && !state.should_show_toast() {
        return Ok(0);
    }

    // Handle toast message (success notification) - single row
    if state.should_show_toast() {
        if let Some(ref device_name) = state.just_paired {
            // Draw top border
            write!(stdout, "{}", escape::cursor_to(row, 1))?;
            write!(stdout, "{}{}", bg(color::BG_DARK), fg(color::DARK_GRAY))?;
            for _ in 0..width {
                write!(stdout, "━")?;
            }
            write!(stdout, "{}", RESET)?;

            // Draw toast content
            write!(stdout, "{}", escape::cursor_to(row + 1, 1))?;
            write!(stdout, "{}", bg(color::BG_DARK))?;
            write!(stdout, " {}✓{} ", fg(color::GREEN), RESET_FG)?;
            write!(stdout, "{}{}Paired with {}{}", bg(color::BG_DARK), fg(color::GREEN), device_name, RESET)?;

            // Fill remaining width
            let used = 4 + 12 + device_name.len();
            let remaining = (width as usize).saturating_sub(used);
            write!(stdout, "{}{:remaining$}{}", bg(color::BG_DARK), "", RESET)?;
            return Ok(2);
        }
    }

    // Show pairing banner with clickable URL
    if let Some(ref code) = state.pairing_code {
        let url = format!("https://drinkcrabigator.com/dashboard?setup={}", code);

        // Row 1: Top border - gray line on dark background
        write!(stdout, "{}", escape::cursor_to(row, 1))?;
        write!(stdout, "{}{}", bg(color::BG_DARK), fg(color::DARK_GRAY))?;
        for _ in 0..width {
            write!(stdout, "━")?;
        }
        write!(stdout, "{}", RESET)?;

        // Row 2: Content row
        write!(stdout, "{}", escape::cursor_to(row + 1, 1))?;

        // Fill entire row with dark background first
        write!(stdout, "{}", bg(color::BG_DARK))?;
        for _ in 0..width {
            write!(stdout, " ")?;
        }
        write!(stdout, "{}", RESET)?;

        // Reposition and draw content
        write!(stdout, "{}", escape::cursor_to(row + 1, 1))?;
        write!(stdout, "{}", bg(color::BG_DARK))?;

        // Phone icon in cyan
        write!(stdout, " {}📱{}", fg(color::CYAN), fg(color::WHITE))?;

        // Explanatory text in white
        write!(stdout, " Access on phone or web  ")?;

        // Arrow separator in gray
        write!(stdout, "{}▸  {}", fg(color::DARK_GRAY), fg(color::WHITE))?;

        // Clickable link pill with dark teal background
        // URL in white, code highlighted in bright yellow
        let link_text = format!(
            "{}{} drinkcrabigator.com/dashboard?setup={}{}{} {}",
            bg(30),              // Dark teal background (color 30)
            fg(color::WHITE),    // White text for URL
            fg(color::YELLOW),   // Bright yellow for the code
            code,
            fg(color::WHITE),    // Back to white after code
            RESET,
        );
        let clickable = hyperlink(&url, &link_text);
        write!(stdout, "{}", clickable)?;
        write!(stdout, "{}", RESET)?;

        return Ok(2);
    }

    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_state_default() {
        let state = PairingState::default();
        assert!(!state.has_linked_devices);
        assert!(!state.should_show_widget());
        assert_eq!(state.banner_rows(), 0);
    }

    #[test]
    fn test_should_show_widget() {
        let mut state = PairingState::default();
        assert!(!state.should_show_widget());
        assert_eq!(state.banner_rows(), 0);

        state.pairing_code = Some("ABC-DEF-GHI".to_string());
        assert!(state.should_show_widget());
        assert_eq!(state.banner_rows(), 2); // Top border + content

        state.has_linked_devices = true;
        assert!(!state.should_show_widget());
        assert_eq!(state.banner_rows(), 0);
    }

    #[test]
    fn test_just_paired_toast() {
        let mut state = PairingState::default();
        state.set_just_paired("iPhone".to_string());
        assert!(state.should_show_toast());
        assert!(state.has_linked_devices);
        assert_eq!(state.banner_rows(), 2); // Top border + content
    }
}
