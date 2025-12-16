//! Stats widget - displays session statistics
//!
//! Shows session state, duration, messages, tool calls, and compressions.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::terminal::escape::{self, color, bg, fg, RESET};
use crate::hooks::ClaudeStats;
use crate::platforms::SessionState;
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
            // Session state indicator
            match stats.platform_stats.state {
                SessionState::Thinking => {
                    format!(
                        "{}{}  Thinking  {}",
                        bg(color::GREEN), fg(color::BLACK), RESET
                    )
                }
                SessionState::Question => {
                    format!(
                        "{}{}  Question  {}",
                        bg(color::CYAN), fg(color::BLACK), RESET
                    )
                }
                SessionState::Complete => {
                    format!(
                        "{}{}  Complete  {}",
                        bg(color::PURPLE), fg(color::WHITE), RESET
                    )
                }
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
            // Messages from platform stats
            format!(
                "{}✉ Messages{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.platform_stats.messages, RESET
            )
        }
        5 => {
            // Tool calls
            format!(
                "{}⚙ Tools{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::ORANGE), format_number(stats.platform_stats.total_tool_calls() as u64), RESET
            )
        }
        6 => {
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
