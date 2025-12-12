//! Stats widget - displays session statistics
//!
//! Shows idle time, session duration, token usage, and message count.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::hooks::ClaudeStats;
use crate::utils::{format_number, strip_ansi_len};

/// Draw the stats widget at the given position
pub fn draw_stats_widget(
    stdout: &mut Stdout,
    pty_rows: u16,
    col: u16,
    row: u16,
    width: u16,
    stats: &ClaudeStats,
) -> Result<()> {
    write!(stdout, "\x1b[{};{}H", pty_rows + 1 + row, col + 1)?;

    let content = match row {
        1 => {
            // Header
            "\x1b[38;5;141m Stats\x1b[0m".to_string()
        }
        2 => {
            // Idle time with color based on duration
            let idle_color = if stats.idle_seconds < 5 {
                "38;5;83" // Bright green
            } else if stats.idle_seconds < 60 {
                "38;5;228" // Yellow
            } else {
                "38;5;203" // Red
            };
            format!(
                "\x1b[38;5;245m⏱ Idle\x1b[0m \x1b[{}m{}\x1b[0m",
                idle_color,
                stats.format_idle()
            )
        }
        3 => {
            // Session/work time
            format!(
                "\x1b[38;5;245m⚡ Session\x1b[0m \x1b[38;5;39m{}\x1b[0m",
                stats.format_work()
            )
        }
        4 => {
            // Tokens
            format!(
                "\x1b[38;5;245m◈ Tokens\x1b[0m \x1b[38;5;213m{}\x1b[0m",
                format_number(stats.tokens_used)
            )
        }
        5 => {
            // Messages
            format!(
                "\x1b[38;5;245m✉ Msgs\x1b[0m \x1b[38;5;75m{}\x1b[0m",
                stats.messages_count
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
