//! Stats widget - displays session statistics
//!
//! Shows session state, duration, messages, tool calls, and compressions.

use std::io::{Stdout, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::terminal::escape::{self, color, fg, RESET};
use crate::hooks::ClaudeStats;
use crate::platforms::SessionState;
use super::utils::{format_number, strip_ansi_len};

/// Braille spinner frames for the thinking animation
const THROBBER: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

/// Get current throbber frame based on time (10 FPS)
fn throbber_frame() -> char {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    let frame = (millis / 100) as usize % THROBBER.len();
    THROBBER[frame]
}

/// Format the state indicator for the header row
fn format_state_indicator(state: SessionState) -> String {
    match state {
        SessionState::Ready => {
            format!("{}○ Ready{}", fg(color::GRAY), RESET)
        }
        SessionState::Thinking => {
            format!("{}{}{}", fg(color::GREEN), throbber_frame(), RESET)
        }
        SessionState::Question => {
            format!("{}? Question{}", fg(color::CYAN), RESET)
        }
        SessionState::Complete => {
            format!("{}✓ Complete{}", fg(color::PURPLE), RESET)
        }
    }
}

/// Draw the stats widget at the given position
pub fn draw_stats_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    stats: &ClaudeStats,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(pty_rows + 1 + row, col + 1))?;

    let content = match row {
        1 => {
            // Header with state indicator on the right
            let header = format!("{} Stats{}", fg(color::PURPLE), RESET);
            let state = format_state_indicator(stats.platform_stats.state);
            let header_len = strip_ansi_len(&header);
            let state_len = strip_ansi_len(&state);
            let gap = (width as usize).saturating_sub(header_len + state_len);
            format!("{}{:gap$}{}", header, "", state, gap = gap)
        }
        2 => {
            // Session/work time
            format!(
                "{}◆ Session{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::BLUE), stats.format_work(), RESET
            )
        }
        3 => {
            // Messages from platform stats
            format!(
                "{}✉ Messages{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.platform_stats.messages, RESET
            )
        }
        4 => {
            // Tool calls
            format!(
                "{}⚙ Tools{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::ORANGE), format_number(stats.platform_stats.total_tool_calls() as u64), RESET
            )
        }
        5 => {
            // Compressions (only show if > 0)
            let compressions = stats.platform_stats.compressions;
            if compressions > 0 {
                format!(
                    "{}⊜ Compact{} {}{}{}",
                    fg(color::GRAY), RESET,
                    fg(color::PINK), compressions, RESET
                )
            } else {
                String::new()
            }
        }
        _ => String::new(),
    };

    write!(stdout, "{}", content)?;
    let content_len = strip_ansi_len(&content);
    let pad = (width as usize).saturating_sub(content_len);
    write!(stdout, "{:pad$}", "", pad = pad)?;

    Ok(())
}
