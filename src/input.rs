//! Input handling module
//!
//! Handles keyboard input encoding and forwarding to the PTY.
//! Implements proper xterm escape sequences for all key combinations.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::pty::ClaudePty;

/// Result of handling a key event
pub enum KeyAction {
    /// Key was handled internally, no further action needed
    Handled,
    /// Request to quit the application
    Quit,
}

/// Handle application-level key commands (Ctrl+A prefix)
///
/// Returns `Some(KeyAction)` if the key was handled, `None` if it should be forwarded to PTY
pub fn handle_app_command(
    key: KeyEvent,
    ctrl_a_pressed: &mut bool,
    pty: &mut ClaudePty,
) -> Result<Option<KeyAction>> {
    // Check for Ctrl+A prefix for app commands
    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('a') {
        *ctrl_a_pressed = true;
        return Ok(Some(KeyAction::Handled));
    }

    if *ctrl_a_pressed {
        *ctrl_a_pressed = false;
        match key.code {
            KeyCode::Char('q') | KeyCode::Char('Q') => {
                return Ok(Some(KeyAction::Quit));
            }
            KeyCode::Char('a') => {
                // Send literal Ctrl+A
                pty.write(&[0x01])?;
                return Ok(Some(KeyAction::Handled));
            }
            _ => {
                // Unknown command, ignore
                return Ok(Some(KeyAction::Handled));
            }
        }
    }

    Ok(None)
}

/// Forward a key event to the PTY with proper encoding
pub fn forward_key_to_pty(key: KeyEvent, pty: &mut ClaudePty) -> Result<()> {
    let bytes = encode_key(key);
    if !bytes.is_empty() {
        pty.write(&bytes)?;
    }
    Ok(())
}

/// Encode a key event into bytes for the PTY
fn encode_key(key: KeyEvent) -> Vec<u8> {
    let has_shift = key.modifiers.contains(KeyModifiers::SHIFT);
    let has_alt = key.modifiers.contains(KeyModifiers::ALT);
    let has_ctrl = key.modifiers.contains(KeyModifiers::CONTROL);

    // Calculate xterm modifier code: 1 + (shift ? 1 : 0) + (alt ? 2 : 0) + (ctrl ? 4 : 0)
    // Only used when we have modifiers on special keys
    let modifier_code = 1
        + (if has_shift { 1 } else { 0 })
        + (if has_alt { 2 } else { 0 })
        + (if has_ctrl { 4 } else { 0 });
    let has_modifiers = modifier_code > 1;

    match key.code {
        KeyCode::Char(c) => encode_char(c, has_ctrl, has_alt, has_shift),
        KeyCode::Enter => vec![b'\r'],
        KeyCode::Backspace => encode_backspace(has_alt, has_ctrl),
        KeyCode::Tab => encode_tab(has_shift, has_ctrl, modifier_code),
        KeyCode::BackTab => vec![0x1b, b'[', b'Z'],
        KeyCode::Esc => vec![0x1b],
        KeyCode::Up => encode_arrow(b'A', has_modifiers, modifier_code),
        KeyCode::Down => encode_arrow(b'B', has_modifiers, modifier_code),
        KeyCode::Right => encode_arrow(b'C', has_modifiers, modifier_code),
        KeyCode::Left => encode_arrow(b'D', has_modifiers, modifier_code),
        KeyCode::Home => encode_home_end(b'H', has_modifiers, modifier_code),
        KeyCode::End => encode_home_end(b'F', has_modifiers, modifier_code),
        KeyCode::PageUp => encode_page(5, has_modifiers, modifier_code),
        KeyCode::PageDown => encode_page(6, has_modifiers, modifier_code),
        KeyCode::Delete => encode_delete(has_alt, has_ctrl, has_shift, has_modifiers, modifier_code),
        KeyCode::Insert => encode_insert(has_modifiers, modifier_code),
        KeyCode::F(n) => encode_function_key(n, has_modifiers, modifier_code),
        KeyCode::Null => vec![0x00],
        _ => vec![],
    }
}

fn encode_char(c: char, has_ctrl: bool, has_alt: bool, has_shift: bool) -> Vec<u8> {
    if has_ctrl && !has_alt && !has_shift {
        // Ctrl+char: send control character
        vec![(c.to_ascii_lowercase() as u8) & 0x1f]
    } else if has_alt && !has_ctrl {
        // Alt/Option+char: send ESC prefix (meta key encoding)
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        let mut bytes = vec![0x1b]; // ESC prefix
        bytes.extend_from_slice(s.as_bytes());
        bytes
    } else if has_ctrl && has_alt {
        // Ctrl+Alt+char: ESC prefix + control character
        let mut bytes = vec![0x1b];
        bytes.push((c.to_ascii_lowercase() as u8) & 0x1f);
        bytes
    } else {
        let mut buf = [0u8; 4];
        let s = c.encode_utf8(&mut buf);
        s.as_bytes().to_vec()
    }
}

fn encode_backspace(has_alt: bool, has_ctrl: bool) -> Vec<u8> {
    if has_alt || has_ctrl {
        // Option+Backspace or Ctrl+Backspace: delete word backwards (ESC + DEL)
        vec![0x1b, 0x7f]
    } else {
        vec![0x7f]
    }
}

fn encode_tab(has_shift: bool, has_ctrl: bool, modifier_code: u8) -> Vec<u8> {
    if has_shift {
        // Shift+Tab: back tab (CSI Z)
        vec![0x1b, b'[', b'Z']
    } else if has_ctrl {
        // Ctrl+Tab: some terminals send this as CSI 9 ; modifier ~
        format!("\x1b[9;{}~", modifier_code).into_bytes()
    } else {
        vec![b'\t']
    }
}

fn encode_arrow(direction: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        format!("\x1b[1;{}{}", modifier_code, direction as char).into_bytes()
    } else {
        vec![0x1b, b'[', direction]
    }
}

fn encode_home_end(key: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        format!("\x1b[1;{}{}", modifier_code, key as char).into_bytes()
    } else {
        vec![0x1b, b'[', key]
    }
}

fn encode_page(code: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        format!("\x1b[{};{}~", code, modifier_code).into_bytes()
    } else {
        vec![0x1b, b'[', b'0' + code, b'~']
    }
}

fn encode_delete(
    has_alt: bool,
    has_ctrl: bool,
    has_shift: bool,
    has_modifiers: bool,
    modifier_code: u8,
) -> Vec<u8> {
    if has_alt && !has_ctrl && !has_shift {
        // Option+Delete: delete word forward (ESC + d)
        vec![0x1b, b'd']
    } else if has_modifiers {
        format!("\x1b[3;{}~", modifier_code).into_bytes()
    } else {
        vec![0x1b, b'[', b'3', b'~']
    }
}

fn encode_insert(has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        format!("\x1b[2;{}~", modifier_code).into_bytes()
    } else {
        vec![0x1b, b'[', b'2', b'~']
    }
}

fn encode_function_key(n: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    // Function key encoding varies, using xterm-style
    let base_code = match n {
        1 => "P",
        2 => "Q",
        3 => "R",
        4 => "S",
        5 => "15~",
        6 => "17~",
        7 => "18~",
        8 => "19~",
        9 => "20~",
        10 => "21~",
        11 => "23~",
        12 => "24~",
        _ => return vec![],
    };

    if has_modifiers && n >= 5 {
        // F5-F12 use tilde format with modifiers
        let num = match n {
            5 => 15,
            6 => 17,
            7 => 18,
            8 => 19,
            9 => 20,
            10 => 21,
            11 => 23,
            12 => 24,
            _ => return vec![],
        };
        format!("\x1b[{};{}~", num, modifier_code).into_bytes()
    } else if has_modifiers && n <= 4 {
        // F1-F4 use SS3 format, with modifiers use CSI 1 ; mod P/Q/R/S
        format!("\x1b[1;{}{}", modifier_code, base_code).into_bytes()
    } else if n <= 4 {
        // F1-F4 without modifiers: SS3 P/Q/R/S
        format!("\x1bO{}", base_code).into_bytes()
    } else {
        // F5-F12 without modifiers
        format!("\x1b[{}", base_code).into_bytes()
    }
}
