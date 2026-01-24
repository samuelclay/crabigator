//! Pairing widget - displays QR code and pairing code for mobile pairing
//!
//! Shows a QR code and human-readable pairing code when no mobile devices are linked.
//! Disappears automatically when pairing completes.

use std::io::{Stdout, Write};

use anyhow::Result;
use qrcode::{QrCode, EcLevel};

use crate::terminal::escape::{self, color, fg, RESET};
use super::utils::strip_ansi_len;
use super::WidgetArea;

/// Pairing state for the widget
#[derive(Clone, Debug)]
pub struct PairingState {
    /// Whether we have any linked devices
    pub has_linked_devices: bool,
    /// Current pairing token (if generated)
    pub pairing_token: Option<String>,
    /// Human-readable pairing code (e.g., "ABC-DEF-GHI")
    pub pairing_code: Option<String>,
    /// QR code data URL
    pub qr_data: Option<String>,
    /// Whether pairing just completed (for toast message)
    pub just_paired: Option<String>,
    /// Timestamp when just_paired was set (for auto-clear)
    pub just_paired_at: Option<std::time::Instant>,
}

impl Default for PairingState {
    fn default() -> Self {
        Self {
            has_linked_devices: false,
            pairing_token: None,
            pairing_code: None,
            qr_data: None,
            just_paired: None,
            just_paired_at: None,
        }
    }
}

impl PairingState {
    /// Check if we should show the pairing widget
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
        self.qr_data = None;
    }

    /// Set a new pairing token (for pairing additional devices)
    #[allow(dead_code)]
    pub fn set_new_token(&mut self, token: String, code: String, qr_data: String) {
        self.pairing_token = Some(token);
        self.pairing_code = Some(code);
        self.qr_data = Some(qr_data);
    }
}

/// Convert QR data to a compact terminal representation using Unicode block characters
/// Uses half-block characters to display 2 rows per line
fn render_qr_compact(qr_data: &str, max_width: usize, max_height: usize) -> Vec<String> {
    let code = match QrCode::with_error_correction_level(qr_data, EcLevel::L) {
        Ok(c) => c,
        Err(_) => return vec!["[QR Error]".to_string()],
    };

    let modules = code.to_colors();
    let size = (modules.len() as f64).sqrt() as usize;

    // Calculate scaling factor
    // Each character represents 2 QR modules vertically (using half blocks)
    // and 1 QR module horizontally (but we need some padding)
    let scale = 1usize;
    let qr_width = size / scale + 2; // +2 for quiet zone
    let qr_height = (size / scale + 1) / 2 + 1; // /2 for half blocks, +1 for quiet zone

    if qr_width > max_width || qr_height > max_height {
        return vec!["[QR too large]".to_string()];
    }

    let mut lines = Vec::new();

    // Unicode half blocks:
    // '▀' (U+2580) = upper half block
    // '▄' (U+2584) = lower half block
    // '█' (U+2588) = full block
    // ' ' = empty

    // Process 2 rows at a time
    for y in (0..size).step_by(2 * scale) {
        let mut line = String::new();
        for x in (0..size).step_by(scale) {
            let top_dark = modules.get(y * size + x).map(|c| *c == qrcode::Color::Dark).unwrap_or(false);
            let bottom_dark = if y + scale < size {
                modules.get((y + scale) * size + x).map(|c| *c == qrcode::Color::Dark).unwrap_or(false)
            } else {
                false
            };

            let ch = match (top_dark, bottom_dark) {
                (true, true) => '█',
                (true, false) => '▀',
                (false, true) => '▄',
                (false, false) => ' ',
            };
            line.push(ch);
        }
        lines.push(line);
    }

    lines
}

/// Draw the pairing widget at the given position
pub fn draw_pairing_widget(
    stdout: &mut Stdout,
    area: WidgetArea,
    state: &PairingState,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(area.pty_rows + 1 + area.row, area.col + 1))?;

    // Handle toast message
    if state.should_show_toast() {
        if let Some(ref device_name) = state.just_paired {
            let content = format!(
                "{}Paired with {}!{}",
                fg(color::GREEN), device_name, RESET
            );
            write!(stdout, "{}", content)?;
            let content_len = strip_ansi_len(&content);
            let pad = (area.width as usize).saturating_sub(content_len);
            write!(stdout, "{:pad$}", "", pad = pad)?;
            return Ok(());
        }
    }

    // Show pairing widget based on available height
    let content = draw_pairing_row(area.row, area.width, state);
    write!(stdout, "{}", content)?;
    let content_len = strip_ansi_len(&content);
    let pad = (area.width as usize).saturating_sub(content_len);
    write!(stdout, "{:pad$}", "", pad = pad)?;

    Ok(())
}

/// Draw a row of the pairing widget
fn draw_pairing_row(row: u16, width: u16, state: &PairingState) -> String {
    let Some(ref code) = state.pairing_code else {
        return String::new();
    };

    // Check if we have enough width for QR code
    let qr_data = state.qr_data.as_deref().unwrap_or("");
    let qr_lines = if !qr_data.is_empty() && width >= 40 {
        render_qr_compact(qr_data, 20, 10)
    } else {
        vec![]
    };

    match row {
        1 => {
            // Header
            format!(
                "{}Pair with mobile{} {}(scan QR){}",
                fg(color::PURPLE), RESET,
                fg(color::GRAY), RESET
            )
        }
        2 => {
            // QR code line 1 or empty
            if !qr_lines.is_empty() {
                qr_lines.get(0).cloned().unwrap_or_default()
            } else {
                format!(
                    "{}Visit{} drinkcrabigator.com",
                    fg(color::GRAY), RESET
                )
            }
        }
        3 => {
            // QR code line 2 or code label
            if qr_lines.len() > 1 {
                qr_lines.get(1).cloned().unwrap_or_default()
            } else {
                format!(
                    "{}Code:{} {}{}{}",
                    fg(color::GRAY), RESET,
                    fg(color::YELLOW), code, RESET
                )
            }
        }
        4 => {
            // QR code line 3 or empty
            if qr_lines.len() > 2 {
                qr_lines.get(2).cloned().unwrap_or_default()
            } else {
                String::new()
            }
        }
        5 => {
            // Code display if QR is showing
            if !qr_lines.is_empty() {
                format!(
                    "{}Code:{} {}{}{}",
                    fg(color::GRAY), RESET,
                    fg(color::YELLOW), code, RESET
                )
            } else {
                String::new()
            }
        }
        _ => {
            // Additional QR lines or empty
            let qr_row = (row as usize).saturating_sub(2);
            if qr_row < qr_lines.len() {
                qr_lines.get(qr_row).cloned().unwrap_or_default()
            } else {
                String::new()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pairing_state_default() {
        let state = PairingState::default();
        assert!(!state.has_linked_devices);
        assert!(!state.should_show_widget());
    }

    #[test]
    fn test_should_show_widget() {
        let mut state = PairingState::default();
        assert!(!state.should_show_widget());

        state.pairing_code = Some("ABC-DEF-GHI".to_string());
        assert!(state.should_show_widget());

        state.has_linked_devices = true;
        assert!(!state.should_show_widget());
    }

    #[test]
    fn test_just_paired_toast() {
        let mut state = PairingState::default();
        state.set_just_paired("iPhone".to_string());
        assert!(state.should_show_toast());
        assert!(state.has_linked_devices);
    }
}
