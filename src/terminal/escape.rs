//! ANSI escape sequence constants and helpers
//!
//! This module centralizes all terminal escape sequences used throughout
//! the application, providing readable names for raw control codes.

// === Cursor Control ===

/// Move cursor to home position (top-left)
#[allow(dead_code)]
pub const CURSOR_HOME: &str = "\x1b[H";

/// Save current cursor position
pub const CURSOR_SAVE: &str = "\x1b[s";

/// Restore previously saved cursor position
pub const CURSOR_RESTORE: &str = "\x1b[u";

/// Move cursor to specific row and column (1-indexed)
#[inline]
pub fn cursor_to(row: u16, col: u16) -> String {
    format!("\x1b[{};{}H", row, col)
}

// === Scroll Region ===

/// Reset scroll region to full screen
pub const SCROLL_REGION_RESET: &str = "\x1b[r";

/// Set scroll region (DECSTBM) from top to bottom row (1-indexed)
#[inline]
pub fn scroll_region(top: u16, bottom: u16) -> String {
    format!("\x1b[{};{}r", top, bottom)
}

/// Scroll display up by n lines (content moves up, blank lines appear at bottom)
/// Uses Index (IND) repeated n times - works regardless of scroll region
#[inline]
pub fn scroll_up(n: u16) -> String {
    "\n".repeat(n as usize)
}

// === Text Styles ===

/// Bold text
pub const BOLD: &str = "\x1b[1m";

/// Dim/faint text
pub const DIM: &str = "\x1b[2m";

/// Reset all text attributes
pub const RESET: &str = "\x1b[0m";

// === Color Helpers ===

/// Set foreground color using 256-color palette
#[inline]
pub fn fg(color: u8) -> String {
    format!("\x1b[38;5;{}m", color)
}

/// Set background color using 256-color palette
#[inline]
pub fn bg(color: u8) -> String {
    format!("\x1b[48;5;{}m", color)
}

// === Precomputed Foreground Colors (256-color palette) ===

/// Cyan foreground (45)
pub const FG_CYAN: &str = "\x1b[38;5;45m";

/// Blue foreground (33)
pub const FG_BLUE: &str = "\x1b[38;5;33m";

/// Purple foreground (141)
pub const FG_PURPLE: &str = "\x1b[38;5;141m";

/// Orange foreground (179)
pub const FG_ORANGE: &str = "\x1b[38;5;179m";

/// Gray foreground (245)
pub const FG_GRAY: &str = "\x1b[38;5;245m";

// === Named Colors (256-color palette) ===

pub mod color {
    /// Green (83) - Success, clean status, additions, active
    pub const GREEN: u8 = 83;

    /// Light green (114) - Branch names, impl blocks
    pub const LIGHT_GREEN: u8 = 114;

    /// Yellow (220) - File counts, modified status, enum
    pub const YELLOW: u8 = 220;

    /// Light yellow (228)
    #[allow(dead_code)]
    pub const LIGHT_YELLOW: u8 = 228;

    /// Orange (179) - Changes header, struct
    pub const ORANGE: u8 = 179;

    /// Dark orange (208) - Const
    #[allow(dead_code)]
    pub const DARK_ORANGE: u8 = 208;

    /// Red (203) - Deletions, errors, idle 5+ minutes
    pub const RED: u8 = 203;

    /// Cyan (45) - Untracked files, folder bars
    pub const CYAN: u8 = 45;

    /// Blue (39) - Session time, function
    pub const BLUE: u8 = 39;

    /// Light blue (75) - Message count, method
    pub const LIGHT_BLUE: u8 = 75;

    /// Purple (141) - Stats header, class
    pub const PURPLE: u8 = 141;

    /// Pink (213) - Tokens, trait
    pub const PINK: u8 = 213;

    /// Gray (245) - Muted text, loading, diamond icons
    pub const GRAY: u8 = 245;

    /// Dark gray (240) - Separators, dots
    pub const DARK_GRAY: u8 = 240;

    /// Faint (250) - Default icon fallback
    pub const FAINT: u8 = 250;

    /// Dark background (236)
    pub const BG_DARK: u8 = 236;

    /// Black (16) - For text on colored backgrounds
    #[allow(dead_code)]
    pub const BLACK: u8 = 16;

    /// White (231) - For text on dark backgrounds
    #[allow(dead_code)]
    pub const WHITE: u8 = 231;
}

// === Standard ANSI Colors (16-color) ===

/// Basic ANSI foreground colors for simple output
pub mod ansi {
    /// Green foreground (32)
    pub const GREEN: &str = "\x1b[32m";

    /// Yellow foreground (33)
    pub const YELLOW: &str = "\x1b[33m";
}

// === OSC 8 Hyperlinks ===

/// Wrap text in an OSC 8 hyperlink (clickable in supporting terminals)
///
/// Modern terminals (iTerm2, Kitty, WezTerm, Windows Terminal) support
/// clickable hyperlinks. Terminals without support show plain text.
#[inline]
pub fn hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{}\x07{}\x1b]8;;\x07", url, text)
}

// === Screen Control ===

/// Clear entire screen
#[allow(dead_code)]
pub const CLEAR_SCREEN: &str = "\x1b[2J";

/// Clear entire screen and move cursor to home position
pub const CLEAR_SCREEN_HOME: &str = "\x1b[2J\x1b[H";

/// Format cursor position report response (CPR)
/// This is the terminal's response to a cursor position query
#[inline]
pub fn cursor_position_report(row: u16, col: u16) -> String {
    format!("\x1b[{};{}R", row, col)
}

// === Key Encoding Bytes ===

pub mod key {
    /// Escape byte (0x1b / 27)
    pub const ESC: u8 = 0x1b;

    /// Delete/backspace byte (0x7f / 127)
    pub const DEL: u8 = 0x7f;

    /// Tab byte
    pub const TAB: u8 = b'\t';

    /// Carriage return byte
    pub const CR: u8 = b'\r';

    /// Null byte
    pub const NUL: u8 = 0x00;

    // === CSI Sequences for Special Keys ===

    /// Shift+Tab / Back Tab (CSI Z)
    pub const BACK_TAB: [u8; 3] = [ESC, b'[', b'Z'];

    /// Delete key without modifiers (CSI 3 ~)
    pub const DELETE: [u8; 4] = [ESC, b'[', b'3', b'~'];

    /// Insert key without modifiers (CSI 2 ~)
    pub const INSERT: [u8; 4] = [ESC, b'[', b'2', b'~'];

    /// Option+Delete - delete word forward (ESC d)
    pub const ALT_DELETE: [u8; 2] = [ESC, b'd'];

    // === Arrow Keys ===

    /// Arrow key without modifiers (CSI direction)
    /// Direction: b'A' = Up, b'B' = Down, b'C' = Right, b'D' = Left
    #[inline]
    pub fn arrow(direction: u8) -> Vec<u8> {
        vec![ESC, b'[', direction]
    }

    /// Arrow key with modifiers (CSI 1 ; modifier direction)
    #[inline]
    pub fn arrow_modified(direction: u8, modifier: u8) -> Vec<u8> {
        format!("\x1b[1;{}{}", modifier, direction as char).into_bytes()
    }

    // === Home/End Keys ===

    /// Home or End key without modifiers (CSI H or CSI F)
    /// Key: b'H' = Home, b'F' = End
    #[inline]
    pub fn home_end(key: u8) -> Vec<u8> {
        vec![ESC, b'[', key]
    }

    /// Home or End key with modifiers (CSI 1 ; modifier key)
    #[inline]
    pub fn home_end_modified(key: u8, modifier: u8) -> Vec<u8> {
        format!("\x1b[1;{}{}", modifier, key as char).into_bytes()
    }

    // === Page Up/Down ===

    /// Page Up or Page Down without modifiers (CSI code ~)
    /// Code: 5 = Page Up, 6 = Page Down
    #[inline]
    pub fn page(code: u8) -> Vec<u8> {
        vec![ESC, b'[', b'0' + code, b'~']
    }

    /// Page Up or Page Down with modifiers (CSI code ; modifier ~)
    #[inline]
    pub fn page_modified(code: u8, modifier: u8) -> Vec<u8> {
        format!("\x1b[{};{}~", code, modifier).into_bytes()
    }

    // === Delete/Insert with Modifiers ===

    /// Delete key with modifiers (CSI 3 ; modifier ~)
    #[inline]
    pub fn delete_modified(modifier: u8) -> Vec<u8> {
        format!("\x1b[3;{}~", modifier).into_bytes()
    }

    /// Insert key with modifiers (CSI 2 ; modifier ~)
    #[inline]
    pub fn insert_modified(modifier: u8) -> Vec<u8> {
        format!("\x1b[2;{}~", modifier).into_bytes()
    }

    // === Function Keys ===

    /// F1-F4 without modifiers (SS3 format: ESC O P/Q/R/S)
    #[inline]
    pub fn f1_f4(base: &str) -> Vec<u8> {
        format!("\x1bO{}", base).into_bytes()
    }

    /// F1-F4 with modifiers (CSI 1 ; modifier P/Q/R/S)
    #[inline]
    pub fn f1_f4_modified(base: &str, modifier: u8) -> Vec<u8> {
        format!("\x1b[1;{}{}", modifier, base).into_bytes()
    }

    /// F5-F12 without modifiers (CSI code ~)
    #[inline]
    pub fn f5_f12(base: &str) -> Vec<u8> {
        format!("\x1b[{}", base).into_bytes()
    }

    /// F5-F12 with modifiers (CSI num ; modifier ~)
    #[inline]
    pub fn f5_f12_modified(num: u8, modifier: u8) -> Vec<u8> {
        format!("\x1b[{};{}~", num, modifier).into_bytes()
    }

    // === Alt/Meta Key Encoding ===

    /// Alt/Option + character (ESC prefix before character bytes)
    #[inline]
    pub fn alt_char(bytes: &[u8]) -> Vec<u8> {
        let mut result = vec![ESC];
        result.extend_from_slice(bytes);
        result
    }

    /// Ctrl+Tab (CSI 9 ; modifier ~)
    #[inline]
    pub fn ctrl_tab(modifier: u8) -> Vec<u8> {
        format!("\x1b[9;{}~", modifier).into_bytes()
    }
}
