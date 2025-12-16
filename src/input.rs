//! Input handling module
//!
//! Handles keyboard input encoding and forwarding to the PTY.
//! Implements proper xterm escape sequences for all key combinations.

use anyhow::Result;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::escape::key;
use crate::pty::ClaudePty;

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
        KeyCode::Enter => vec![key::CR],
        KeyCode::Backspace => encode_backspace(has_alt, has_ctrl),
        KeyCode::Tab => encode_tab(has_shift, has_ctrl, modifier_code),
        KeyCode::BackTab => key::BACK_TAB.to_vec(),
        KeyCode::Esc => vec![key::ESC],
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
        KeyCode::Null => vec![key::NUL],
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
        key::alt_char(s.as_bytes())
    } else if has_ctrl && has_alt {
        // Ctrl+Alt+char: ESC prefix + control character
        let mut bytes = vec![key::ESC];
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
        vec![key::ESC, key::DEL]
    } else {
        vec![key::DEL]
    }
}

fn encode_tab(has_shift: bool, has_ctrl: bool, modifier_code: u8) -> Vec<u8> {
    if has_shift {
        // Shift+Tab: back tab (CSI Z)
        key::BACK_TAB.to_vec()
    } else if has_ctrl {
        // Ctrl+Tab: some terminals send this as CSI 9 ; modifier ~
        key::ctrl_tab(modifier_code)
    } else {
        vec![key::TAB]
    }
}

fn encode_arrow(direction: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        key::arrow_modified(direction, modifier_code)
    } else {
        key::arrow(direction)
    }
}

fn encode_home_end(key_byte: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        key::home_end_modified(key_byte, modifier_code)
    } else {
        key::home_end(key_byte)
    }
}

fn encode_page(code: u8, has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        key::page_modified(code, modifier_code)
    } else {
        key::page(code)
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
        key::ALT_DELETE.to_vec()
    } else if has_modifiers {
        key::delete_modified(modifier_code)
    } else {
        key::DELETE.to_vec()
    }
}

fn encode_insert(has_modifiers: bool, modifier_code: u8) -> Vec<u8> {
    if has_modifiers {
        key::insert_modified(modifier_code)
    } else {
        key::INSERT.to_vec()
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
        key::f5_f12_modified(num, modifier_code)
    } else if has_modifiers && n <= 4 {
        // F1-F4 use SS3 format, with modifiers use CSI 1 ; mod P/Q/R/S
        key::f1_f4_modified(base_code, modifier_code)
    } else if n <= 4 {
        // F1-F4 without modifiers: SS3 P/Q/R/S
        key::f1_f4(base_code)
    } else {
        // F5-F12 without modifiers
        key::f5_f12(base_code)
    }
}
