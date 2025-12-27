//! Stats widget - displays session statistics
//!
//! Shows session state, duration, messages, tool calls, and compressions.

use std::io::{Stdout, Write};
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use crate::terminal::escape::{self, color, fg, RESET};
use crate::hooks::SessionStats;
use crate::platforms::SessionState;
use super::sparkline::render_sparkline;
use super::utils::strip_ansi_len;

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

/// Calculate idle seconds from idle_since timestamp
fn idle_seconds(idle_since: Option<f64>) -> Option<u64> {
    let since = idle_since?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    let secs = (now - since) as u64;
    if secs >= 60 {
        Some(secs)
    } else {
        None
    }
}

/// Format duration as compact string (e.g., "1m", "2h3m")
fn format_duration_compact(secs: u64) -> String {
    if secs >= 3600 {
        let h = secs / 3600;
        let m = (secs % 3600) / 60;
        if m > 0 {
            format!("{}h{}m", h, m)
        } else {
            format!("{}h", h)
        }
    } else {
        let m = secs / 60;
        format!("{}m", m)
    }
}

/// Calculate elapsed seconds since a timestamp
fn elapsed_since(timestamp: Option<f64>) -> Option<u64> {
    let since = timestamp?;
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64();
    Some((now - since) as u64)
}

/// Format elapsed time since a timestamp
fn format_elapsed(timestamp: Option<f64>) -> String {
    if let Some(secs) = elapsed_since(timestamp) {
        if secs >= 60 {
            format!(" {} ago", format_duration_compact(secs))
        } else {
            " just now".to_string()
        }
    } else {
        String::new()
    }
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
        SessionState::Permission => {
            format!("{}{} ?{}", fg(color::YELLOW), throbber_frame(), RESET)
        }
        SessionState::Question => {
            format!("{}» ? «{}", fg(color::ORANGE), RESET)
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
    height: u16,
    stats: &SessionStats,
) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(pty_rows + 1 + row, col + 1))?;

    // Use compact mode when we have 4 or fewer content rows (status_rows <= 5)
    // Compact mode: header + 2 rows with abbreviated two-column layout
    let compact = height <= 5;

    let content = if compact {
        draw_compact_row(row, width, stats)
    } else {
        draw_normal_row(row, width, stats)
    };

    write!(stdout, "{}", content)?;
    let content_len = strip_ansi_len(&content);
    let pad = (width as usize).saturating_sub(content_len);
    write!(stdout, "{:pad$}", "", pad = pad)?;

    Ok(())
}

/// Draw a row in compact mode (two-column layout with separator)
fn draw_compact_row(row: u16, width: u16, stats: &SessionStats) -> String {
    // Split width into two columns with a separator
    let half = (width as usize) / 2;

    match row {
        1 => {
            // Header with state indicator (same as normal)
            let header = format!("{} Stats{}", fg(color::PURPLE), RESET);
            let state = format_state_indicator(stats.platform_stats.state);
            let header_len = strip_ansi_len(&header);
            let state_len = strip_ansi_len(&state);
            let gap = (width as usize).saturating_sub(header_len + state_len);
            format!("{}{:gap$}{}", header, "", state, gap = gap)
        }
        2 => {
            // Row 2: Left column = Session + Thinking, Right column = Prompts + Completions
            let sess = format!(
                "{}◆ Sess{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::BLUE), stats.format_work(), RESET
            );
            let thinking_val = stats.format_thinking().unwrap_or_else(|| "—".to_string());
            let think = format!(
                "{}◇ Thnk{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::GREEN), thinking_val, RESET
            );

            let prm = format!(
                "{}▸ Pmt{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.platform_stats.prompts, RESET
            );
            let cmp = format!(
                "{}◂ Fin{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.platform_stats.completions, RESET
            );

            // Left side: Session and Thinking with gap between
            let sess_len = strip_ansi_len(&sess);
            let think_len = strip_ansi_len(&think);
            let left_gap = half.saturating_sub(sess_len + think_len + 1); // -1 for separator
            let left = format!("{}{:gap$}{}", sess, "", think, gap = left_gap.max(1));

            // Right side: Prompts and Completions with gap between
            let prm_len = strip_ansi_len(&prm);
            let cmp_len = strip_ansi_len(&cmp);
            let right_gap = half.saturating_sub(prm_len + cmp_len);
            let right = format!("{}{:gap$}{}", prm, "", cmp, gap = right_gap.max(1));

            // Combine with separator
            format!(
                "{}{}│{}{}",
                left,
                fg(color::DARK_GRAY),
                RESET,
                right
            )
        }
        3 => {
            // Row 3: Tools sparkline on left, compressions on right if any
            let compressions = stats.platform_stats.compressions;

            let label = format!("{}⚙{} ", fg(color::GRAY), RESET);
            let label_len = strip_ansi_len(&label);

            if compressions > 0 {
                // Sparkline takes left half, compressions on right
                let sparkline_width = half.saturating_sub(label_len + 1); // -1 for separator
                let bins = stats.tool_usage_bins(sparkline_width);
                let sparkline = render_sparkline(&bins, sparkline_width);

                let comp_label = format!(
                    "{}⊜ Cmp{} {}{}{}",
                    fg(color::GRAY), RESET,
                    fg(color::PINK), compressions, RESET
                );

                format!(
                    "{}{}{}│{}{}",
                    label, sparkline,
                    fg(color::DARK_GRAY),
                    RESET,
                    comp_label
                )
            } else {
                // No compressions - sparkline spans full width
                let sparkline_width = (width as usize).saturating_sub(label_len);
                let bins = stats.tool_usage_bins(sparkline_width);
                let sparkline = render_sparkline(&bins, sparkline_width);
                format!("{}{}", label, sparkline)
            }
        }
        _ => String::new(),
    }
}

/// Draw a row in normal mode (full labels, single column)
fn draw_normal_row(row: u16, width: u16, stats: &SessionStats) -> String {
    match row {
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
            // Session/work time (right-aligned)
            let label = format!("{}◆ Session{}", fg(color::GRAY), RESET);
            let value = format!("{}{}{}", fg(color::BLUE), stats.format_work(), RESET);
            let label_len = strip_ansi_len(&label);
            let value_len = strip_ansi_len(&value);
            let gap = (width as usize).saturating_sub(label_len + value_len);
            format!("{}{:gap$}{}", label, "", value, gap = gap)
        }
        3 => {
            // Thinking time (always show, with dash when no thinking yet)
            let label = format!("{}◇ Thinking{}", fg(color::GRAY), RESET);
            let thinking_value = stats.format_thinking().unwrap_or_else(|| "—".to_string());
            let value = format!("{}{}{}", fg(color::GREEN), thinking_value, RESET);
            let label_len = strip_ansi_len(&label);
            let value_len = strip_ansi_len(&value);
            let gap = (width as usize).saturating_sub(label_len + value_len);
            format!("{}{:gap$}{}", label, "", value, gap = gap)
        }
        4 => {
            // Prompts: count left-aligned after label, timer right-aligned
            let label = format!(
                "{}▸ Prompts{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.platform_stats.prompts, RESET
            );
            let elapsed = format_elapsed(stats.prompts_changed_at);
            let timer = format!("{}{}{}", fg(color::GRAY), elapsed, RESET);
            let label_len = strip_ansi_len(&label);
            let timer_len = strip_ansi_len(&timer);
            let gap = (width as usize).saturating_sub(label_len + timer_len);
            format!("{}{:gap$}{}", label, "", timer, gap = gap)
        }
        5 => {
            // Completions: count left-aligned after label, timer right-aligned
            let label = format!(
                "{}◂ Completions{} {}{}{}",
                fg(color::GRAY), RESET,
                fg(color::LIGHT_BLUE), stats.platform_stats.completions, RESET
            );
            let elapsed = format_elapsed(stats.completions_changed_at);
            let timer = format!("{}{}{}", fg(color::GRAY), elapsed, RESET);
            let label_len = strip_ansi_len(&label);
            let timer_len = strip_ansi_len(&timer);
            let gap = (width as usize).saturating_sub(label_len + timer_len);
            format!("{}{:gap$}{}", label, "", timer, gap = gap)
        }
        6 => {
            // Tool usage sparkline (spans from after label to right edge)
            let label = format!("{}⚙ Tools{} ", fg(color::GRAY), RESET);
            let label_len = strip_ansi_len(&label);
            let sparkline_width = (width as usize).saturating_sub(label_len);
            let bins = stats.tool_usage_bins(sparkline_width);
            let sparkline = render_sparkline(&bins, sparkline_width);
            format!("{}{}", label, sparkline)
        }
        7 => {
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
        8 => {
            // Idle time (only show when complete/question state and idle > 60s)
            let is_idle_state = matches!(
                stats.platform_stats.state,
                SessionState::Complete | SessionState::Question
            );
            if is_idle_state {
                if let Some(secs) = idle_seconds(stats.platform_stats.idle_since) {
                    format!(
                        "{}◇ Idle{} {}{}{}",
                        fg(color::GRAY), RESET,
                        fg(color::GRAY), format_duration_compact(secs), RESET
                    )
                } else {
                    String::new()
                }
            } else {
                String::new()
            }
        }
        _ => String::new(),
    }
}
