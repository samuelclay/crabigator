//! Handoff strip above the widget separator.
//!
//! This area is reserved for setup prompts, update notices, and the latest
//! automatic recap. It sits between the assistant PTY and the status widgets.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::recap::{RecapState, RecapStatus, RecapVariant, TurnLineDelta};
use crate::terminal::escape::{self, bg, color, fg, RESET, RESET_FG};

/// Rows reserved between PTY output and the widget separator.
pub const HANDOFF_RESERVED_ROWS: u16 = 3;

pub fn draw_recap_handoff(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    recap: &RecapState,
    available_rows: u16,
) -> Result<u16> {
    if available_rows == 0 || !recap.prefers_handoff() {
        return Ok(0);
    }

    if recap.latest.is_some() {
        return draw_latest_recap(stdout, row, width, available_rows, recap);
    }
    if let RecapStatus::Failed(error) = &recap.status {
        return draw_recap_failure(stdout, row, width, available_rows, error);
    }
    Ok(0)
}

fn draw_latest_recap(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    available_rows: u16,
    recap: &RecapState,
) -> Result<u16> {
    let Some(latest) = recap.latest.as_ref() else {
        return Ok(0);
    };

    let delta = recap
        .line_delta
        .or(Some(latest.line_delta))
        .map(format_line_delta);
    let right = delta;

    fill_row(stdout, row, width)?;
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    let prefix = format!(
        "{}{} ●{} {}Recap{} ",
        bg(color::BG_DARK),
        fg(color::CYAN),
        fg(color::WHITE),
        fg(color::WHITE),
        RESET_FG
    );
    let headline = latest.headline.trim();
    write_truncated(stdout, width, &prefix, headline, right.as_deref())?;

    let mut used = 1;
    if available_rows <= 1 {
        write!(stdout, "{}", RESET)?;
        return Ok(used);
    }

    let mut detail_lines = Vec::new();
    if latest.variant == RecapVariant::Bullets {
        detail_lines.extend(latest.bullets.iter().take(2).map(|b| format!("• {}", b)));
    }
    if detail_lines.is_empty() {
        detail_lines.extend(
            latest
                .next_prompt_notes
                .iter()
                .take(1)
                .map(|n| format!("Next: {}", n)),
        );
    } else if let Some(note) = latest.next_prompt_notes.first() {
        detail_lines.push(format!("Next: {}", note));
    }
    if let Some(artifact) = latest.artifacts.first() {
        detail_lines.push(format!("Artifact: {}", artifact));
    }

    for detail in detail_lines.into_iter().take((available_rows - 1) as usize) {
        let detail_row = row + used;
        fill_row(stdout, detail_row, width)?;
        write!(stdout, "{}", escape::cursor_to(detail_row, 1))?;
        let prefix = format!(
            "{}{}   {}",
            bg(color::BG_DARK),
            fg(color::DARK_GRAY),
            RESET_FG
        );
        write_truncated(stdout, width, &prefix, &detail, None)?;
        used += 1;
    }

    write!(stdout, "{}", RESET)?;
    Ok(used)
}

fn draw_recap_failure(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    available_rows: u16,
    error: &str,
) -> Result<u16> {
    fill_row(stdout, row, width)?;
    write!(stdout, "{}", escape::cursor_to(row, 1))?;

    // Warning glyph + label in amber, body in muted gray. Using YELLOW (220)
    // rather than RED so a transient API hiccup doesn't read as a hard error.
    let prefix = format!(
        "{}{} ⚠{} {}Recap unavailable{}{}: ",
        bg(color::BG_DARK),
        fg(color::YELLOW),
        RESET_FG,
        fg(color::YELLOW),
        RESET_FG,
        bg(color::BG_DARK),
    );
    let body = extract_friendly_error(error);
    write_failure_line(stdout, width, &prefix, &body)?;

    let mut used = 1;
    if available_rows > 1 {
        let hint_row = row + used;
        fill_row(stdout, hint_row, width)?;
        write!(stdout, "{}", escape::cursor_to(hint_row, 1))?;
        let hint_prefix = format!("{}{}   ", bg(color::BG_DARK), fg(color::DARK_GRAY));
        let hint = "(clears on next prompt — `crabigator recap status` for details)";
        write_failure_line(stdout, width, &hint_prefix, hint)?;
        used += 1;
    }

    write!(stdout, "{}", RESET)?;
    Ok(used)
}

/// Extract a human-friendly error string. When the raw error contains an
/// embedded JSON object with `error.message` (Anthropic's standard error
/// shape), surface just that message; otherwise return the cleaned raw text.
fn extract_friendly_error(error: &str) -> String {
    let cleaned = error.replace('\n', " ").trim().to_string();
    if let Some(start) = cleaned.find('{') {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&cleaned[start..]) {
            if let Some(msg) = value
                .get("error")
                .and_then(|e| e.get("message"))
                .and_then(|m| m.as_str())
            {
                return msg.to_string();
            }
            if let Some(msg) = value.get("message").and_then(|m| m.as_str()) {
                return msg.to_string();
            }
        }
    }
    cleaned
}

fn write_failure_line(stdout: &mut Stdout, width: u16, prefix: &str, body: &str) -> Result<()> {
    let prefix_width = crate::ui::utils::strip_ansi_len(prefix);
    let body_width = (width as usize).saturating_sub(prefix_width).max(1);
    let body = truncate_display(body, body_width);
    // Body text uses a slightly brighter gray than the dim hint so the
    // message itself is what catches the eye.
    write!(stdout, "{}{}{}", prefix, fg(color::GRAY), body)?;
    Ok(())
}

fn format_line_delta(delta: TurnLineDelta) -> String {
    if delta.additions == 0 && delta.deletions == 0 {
        return format!("{}Δ ·{}", fg(color::DARK_GRAY), RESET_FG);
    }
    if delta.additions >= 0 && delta.deletions >= 0 {
        return format!(
            "{}Δ {}+{} {}-{}{}",
            fg(color::DARK_GRAY),
            fg(color::GREEN),
            delta.additions,
            fg(color::RED),
            delta.deletions,
            RESET_FG
        );
    }
    let net = delta.additions - delta.deletions;
    let sign = if net >= 0 { "+" } else { "" };
    format!("{}Δ net {}{}{}", fg(color::DARK_GRAY), sign, net, RESET_FG)
}

fn fill_row(stdout: &mut Stdout, row: u16, width: u16) -> Result<()> {
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(stdout, "{}", bg(color::BG_DARK))?;
    for _ in 0..width {
        write!(stdout, " ")?;
    }
    write!(stdout, "{}", RESET)?;
    Ok(())
}

fn write_truncated(
    stdout: &mut Stdout,
    width: u16,
    prefix: &str,
    body: &str,
    right: Option<&str>,
) -> Result<()> {
    let prefix_width = crate::ui::utils::strip_ansi_len(prefix);
    let right_width = right.map(crate::ui::utils::strip_ansi_len).unwrap_or(0);
    let gap = usize::from(right.is_some());
    let body_width = (width as usize)
        .saturating_sub(prefix_width)
        .saturating_sub(right_width)
        .saturating_sub(gap)
        .max(1);
    let body = truncate_display(body, body_width);
    write!(stdout, "{}{}{}", prefix, fg(color::WHITE), body)?;

    if let Some(right) = right {
        let used = prefix_width + crate::ui::utils::strip_ansi_len(&body);
        let padding = (width as usize)
            .saturating_sub(used)
            .saturating_sub(right_width);
        write!(stdout, "{:padding$}{}", "", right, padding = padding)?;
    }
    Ok(())
}

fn truncate_display(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    let mut width = 0;
    let mut out = String::new();
    for ch in text.chars() {
        let ch_width = ch.width().unwrap_or(0);
        if width + ch_width > max_width.saturating_sub(1) {
            out.push('…');
            return out;
        }
        width += ch_width;
        out.push(ch);
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn anthropic_credit_error_surfaces_just_the_message() {
        let raw = r#"Anthropic returned 400 Bad Request: {"type":"error","error":{"type":"invalid_request_error","message":"Your credit balance is too low to access the Anthropic API."}}"#;
        let extracted = extract_friendly_error(raw);
        assert_eq!(
            extracted,
            "Your credit balance is too low to access the Anthropic API."
        );
    }

    #[test]
    fn flat_message_field_is_also_extracted() {
        let raw = r#"{"message":"timed out after 45s"}"#;
        assert_eq!(extract_friendly_error(raw), "timed out after 45s");
    }

    #[test]
    fn non_json_errors_pass_through_cleaned() {
        let raw = "  recap worker stopped\n  ";
        assert_eq!(extract_friendly_error(raw), "recap worker stopped");
    }
}
