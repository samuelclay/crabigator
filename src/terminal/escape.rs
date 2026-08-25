//! ANSI escape sequence constants and helpers
//!
//! This module centralizes all terminal escape sequences used throughout
//! the application, providing readable names for raw control codes.

// === Cursor Control ===

/// Move cursor to home position (top-left)
pub const CURSOR_HOME: &str = "\x1b[H";

/// Save current cursor position
pub const CURSOR_SAVE: &str = "\x1b[s";

/// Restore previously saved cursor position
pub const CURSOR_RESTORE: &str = "\x1b[u";

/// Hide the terminal cursor (DECTCEM reset)
pub const CURSOR_HIDE: &str = "\x1b[?25l";

/// Show the terminal cursor (DECTCEM set)
pub const CURSOR_SHOW: &str = "\x1b[?25h";

/// Move cursor to specific row and column (1-indexed)
#[inline]
pub fn cursor_to(row: u16, col: u16) -> String {
    format!("\x1b[{};{}H", row, col)
}

/// Move cursor to a column on the current row (CHA, 1-indexed)
#[inline]
pub fn cursor_col(col: u16) -> String {
    format!("\x1b[{}G", col)
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
#[allow(dead_code)]
pub fn scroll_up(n: u16) -> String {
    "\n".repeat(n as usize)
}

// === Text Styles ===

/// Bold text
pub const BOLD: &str = "\x1b[1m";

/// Back to normal intensity, leaving color and underline in place.
pub const RESET_BOLD: &str = "\x1b[22m";

/// Dim/faint text
pub const DIM: &str = "\x1b[2m";

/// Underline text
pub const UNDERLINE: &str = "\x1b[4m";

/// Stop underlining without changing other text attributes
pub const RESET_UNDERLINE: &str = "\x1b[24m";

/// Reset all text attributes
pub const RESET: &str = "\x1b[0m";

/// Reset foreground color only (keeps background)
pub const RESET_FG: &str = "\x1b[39m";

/// Reset background color only (keeps foreground)
pub const RESET_BG: &str = "\x1b[49m";

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

/// Set foreground color using 24-bit truecolor
#[inline]
pub fn fg_rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[38;2;{r};{g};{b}m")
}

/// Set background color using 24-bit truecolor
#[inline]
pub fn bg_rgb((r, g, b): (u8, u8, u8)) -> String {
    format!("\x1b[48;2;{r};{g};{b}m")
}

// === Precomputed Foreground Colors (256-color palette) ===

/// Cyan foreground (45)
pub const FG_CYAN: &str = "\x1b[38;5;45m";

/// Blue foreground (33)
#[allow(dead_code)]
pub const FG_BLUE: &str = "\x1b[38;5;33m";

/// Purple foreground (141)
pub const FG_PURPLE: &str = "\x1b[38;5;141m";

/// Orange foreground (179)
pub const FG_ORANGE: &str = "\x1b[38;5;179m";

/// Gray foreground (245)
pub const FG_GRAY: &str = "\x1b[38;5;245m";

/// Green foreground (83) - Session start
pub const FG_GREEN: &str = "\x1b[38;5;83m";

/// Red foreground (203) - Session end
pub const FG_RED: &str = "\x1b[38;5;203m";

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

    /// Selection band background (238) - one step lighter than BG_DARK so a
    /// highlighted row reads against both the default and dark backgrounds
    pub const BG_SELECTED: u8 = 238;

    /// Black (16) - For text on colored backgrounds
    #[allow(dead_code)]
    pub const BLACK: u8 = 16;

    /// White (231) - For text on dark backgrounds
    #[allow(dead_code)]
    pub const WHITE: u8 = 231;

    /// The one-notch-dimmer counterpart of a palette color, for muting a
    /// whole row (e.g. secondary PRs) while keeping each cell's hue.
    pub fn dimmed(color: u8) -> u8 {
        match color {
            GREEN => 71,
            LIGHT_GREEN => 65,
            YELLOW => 136,
            LIGHT_YELLOW => 143,
            ORANGE => 137,
            DARK_ORANGE => 130,
            RED => 131,
            CYAN => 37,
            BLUE => 31,
            LIGHT_BLUE => 67,
            PURPLE => 97,
            PINK => 133,
            GRAY => 242,
            DARK_GRAY => 238,
            FAINT => 246,
            WHITE => 251,
            other => other,
        }
    }
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

// === Bracketed Paste ===

/// Start-of-paste marker (DEC private mode 2004).
/// crossterm strips these when parsing `Event::Paste`, so we must re-emit them
/// when forwarding a paste to a child PTY whose app (e.g. Claude Code) relies
/// on them to distinguish pasted content from typed input.
pub const BRACKETED_PASTE_START: &[u8] = b"\x1b[200~";

/// End-of-paste marker (DEC private mode 2004).
pub const BRACKETED_PASTE_END: &[u8] = b"\x1b[201~";

// === Screen Control ===

/// Begin synchronized update (DEC private mode 2026)
/// Terminal batches all output until SYNC_END, preventing partial renders
pub const SYNC_BEGIN: &str = "\x1b[?2026h";

/// End synchronized update
/// Terminal renders all batched output atomically
pub const SYNC_END: &str = "\x1b[?2026l";

/// Disable auto-wrap (DECAWM reset): overlong lines clip at the right margin
/// instead of wrapping onto the next row.
pub const WRAP_OFF: &str = "\x1b[?7l";

/// Re-enable auto-wrap (DECAWM set)
pub const WRAP_ON: &str = "\x1b[?7h";

/// Clear from cursor to end of screen (ED mode 0)
pub const CLEAR_TO_END: &str = "\x1b[J";

/// Clear the entire current line (EL mode 2)
pub const CLEAR_LINE: &str = "\x1b[2K";

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
