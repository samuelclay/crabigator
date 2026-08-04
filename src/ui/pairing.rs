//! Pairing and Update banners - full-width displays
//!
//! Shows prominent banners for:
//! - Mobile pairing: clickable URL when no mobile devices are linked
//! - Updates: reminder when a new version is available
//!
//! Banners render above the widget row and disappear when no longer needed.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::terminal::escape::{self, bg, color, fg, hyperlink, RESET, RESET_FG};
use crate::update::UpdateState;

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

    /// Get the number of rows needed in compact mode (0 if not shown)
    pub fn banner_rows_compact(&self) -> u16 {
        if self.should_show_widget() || self.should_show_toast() {
            1
        } else {
            0
        }
    }
}

fn draw_pairing_top_border(stdout: &mut Stdout, row: u16, width: u16) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(stdout, "{}{}", bg(color::BG_DARK), fg(color::DARK_GRAY))?;
    for _ in 0..width {
        write!(stdout, "━")?;
    }
    write!(stdout, "{}", RESET)?;
    Ok(())
}

fn draw_pairing_toast_row(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    device_name: &str,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(stdout, "{}", bg(color::BG_DARK))?;
    write!(stdout, " {}✓{} ", fg(color::GREEN), RESET_FG)?;
    write!(
        stdout,
        "{}{}Paired with {}{}",
        bg(color::BG_DARK),
        fg(color::GREEN),
        device_name,
        RESET
    )?;

    let used = 4 + 12 + device_name.len();
    let remaining = (width as usize).saturating_sub(used);
    write!(stdout, "{}{:remaining$}{}", bg(color::BG_DARK), "", RESET)?;
    Ok(())
}

fn draw_pairing_content_row(stdout: &mut Stdout, row: u16, width: u16, code: &str) -> Result<()> {
    let url = format!("https://drinkcrabigator.com/dashboard?setup={}", code);

    // Fill entire row with dark background first
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(stdout, "{}", bg(color::BG_DARK))?;
    for _ in 0..width {
        write!(stdout, " ")?;
    }
    write!(stdout, "{}", RESET)?;

    // Reposition and draw content
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
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
        bg(30),            // Dark teal background (color 30)
        fg(color::WHITE),  // White text for URL
        fg(color::YELLOW), // Bright yellow for the code
        code,
        fg(color::WHITE), // Back to white after code
        RESET,
    );
    let clickable = hyperlink(&url, &link_text);
    write!(stdout, "{}", clickable)?;
    write!(stdout, "{}", RESET)?;

    Ok(())
}

/// Draw the full-width pairing banner
/// Returns the number of rows consumed (0, 1, or 2)
pub fn draw_pairing_banner(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    state: &PairingState,
    compact: bool,
) -> Result<u16> {
    // Nothing to show
    if !state.should_show_widget() && !state.should_show_toast() {
        return Ok(0);
    }

    // Handle toast message (success notification) - single row
    if state.should_show_toast() {
        if let Some(ref device_name) = state.just_paired {
            let content_row = if compact { row } else { row + 1 };
            if !compact {
                draw_pairing_top_border(stdout, row, width)?;
            }
            draw_pairing_toast_row(stdout, content_row, width, device_name)?;
            return Ok(if compact { 1 } else { 2 });
        }
    }

    // Show pairing banner with clickable URL
    if let Some(ref code) = state.pairing_code {
        let content_row = if compact { row } else { row + 1 };
        if !compact {
            draw_pairing_top_border(stdout, row, width)?;
        }
        draw_pairing_content_row(stdout, content_row, width, code)?;
        return Ok(if compact { 1 } else { 2 });
    }

    Ok(0)
}

/// Draw the full-width update banner
/// Returns the number of rows consumed (0 or 1)
pub fn draw_update_banner(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    state: &UpdateState,
) -> Result<u16> {
    // Nothing to show
    if !state.should_show_banner() {
        return Ok(0);
    }

    let new_version = state.new_version.as_deref().unwrap_or("unknown");
    let current_version = crate::update::CURRENT_VERSION;
    let update_cmd = state.install_method.banner_command();

    // Draw single row banner with dark background
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(stdout, "{}", bg(color::BG_DARK))?;

    // Fill entire row with dark background first
    for _ in 0..width {
        write!(stdout, " ")?;
    }
    write!(stdout, "{}", RESET)?;

    // Reposition and draw content
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(stdout, "{}", bg(color::BG_DARK))?;

    // New version indicator in cyan
    write!(stdout, " {}\u{1F195}{}", fg(color::CYAN), RESET_FG)?; // 🆕 emoji

    // Version text: "Update available v0.3.0 → v99.0.0"
    write!(
        stdout,
        "{}{} Update available v{} {}→{} v{} {}",
        bg(color::BG_DARK),
        fg(231), // White
        current_version,
        fg(color::DARK_GRAY), // Arrow in dark gray
        fg(231),              // New version in white
        new_version,
        RESET_FG
    )?;

    // Arrow separator in gray
    write!(stdout, " {}▸  {}", fg(color::DARK_GRAY), RESET_FG)?;

    // Update command in green
    write!(
        stdout,
        "{}{}{}{}",
        bg(color::BG_DARK),
        fg(color::GREEN),
        update_cmd,
        RESET
    )?;

    Ok(1)
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
        assert_eq!(state.banner_rows_compact(), 0);

        state.pairing_code = Some("ABC-DEF-GHI".to_string());
        assert!(state.should_show_widget());
        assert_eq!(state.banner_rows(), 2); // Top border + content
        assert_eq!(state.banner_rows_compact(), 1);

        state.has_linked_devices = true;
        assert!(!state.should_show_widget());
        assert_eq!(state.banner_rows(), 0);
        assert_eq!(state.banner_rows_compact(), 0);
    }

    #[test]
    fn test_just_paired_toast() {
        let mut state = PairingState::default();
        state.set_just_paired("iPhone".to_string());
        assert!(state.should_show_toast());
        assert!(state.has_linked_devices);
        assert_eq!(state.banner_rows(), 2); // Top border + content
    }

    #[test]
    fn test_update_state_default() {
        let state = UpdateState::default();
        assert!(!state.should_show_banner());
        assert_eq!(state.banner_rows(), 0);
    }

    #[test]
    fn test_update_state_with_update() {
        let state = UpdateState {
            update_available: true,
            new_version: Some("0.4.0".to_string()),
            prompt_dismissed: true,
            ..Default::default()
        };

        assert!(state.should_show_banner());
        assert_eq!(state.banner_rows(), 1);
    }
}
