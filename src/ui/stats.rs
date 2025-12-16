//! Stats widget - displays session statistics
//!
//! Shows idle time, session duration, token usage, and message count.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::terminal::escape::{self, color, fg, RESET};
use crate::hooks::ClaudeStats;
use super::utils::{format_number, strip_ansi_len};

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
            // Header
            format!("{} Stats{}", fg(color::PURPLE), RESET)
        }
        2 => {
            // Active/Idle status - "Active" when < 60s, "Idle" when >= 60s
            if stats.idle_seconds < 60 {
                format!(
                    "{}◇ Active{} {}{}{}",
                    fg(color::GRAY), RESET,
                    fg(color::GREEN), stats.format_idle(), RESET
                )
            } else {
                let idle_color = if stats.idle_seconds < 300 {
                    color::LIGHT_YELLOW // Yellow (1-5 min)
                } else {
                    color::RED // Red (5+ min)
                };
                format!(
                    "{}◇ Idle{} {}{}{}",
                    fg(color::GRAY), RESET,
                    fg(idle_color), stats.format_idle(), RESET
                )
            }
        }
        3 => {
            // Session/work time
            format!(
                "{}◆ Session{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::BLUE), stats.format_work(), RESET
            )
        }
        4 => {
            // Tokens
            format!(
                "{}◈ Tokens{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::PINK), format_number(stats.tokens_used), RESET
            )
        }
        5 => {
            // Messages
            format!(
                "{}✉ Msgs{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.messages_count, RESET
            )
        }
        _ => String::new(),
    };

    write!(stdout, "{}", content)?;
    let content_len = strip_ansi_len(&content);
    let pad = (width as usize).saturating_sub(content_len);
    write!(stdout, "{:pad$}", "", pad = pad)?;

    Ok(())
}
