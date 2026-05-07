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

    // Headline first — wraps onto the next row only if it overflows the
    // available body width on row 1 (where the Δ delta lives).
    let headline = latest.headline.trim();
    let headline_prefix = format!(
        "{}{} ●{} ",
        bg(color::BG_DARK),
        fg(color::CYAN),
        RESET_FG
    );
    let headline_indent = format!(
        "{}{}   {}",
        bg(color::BG_DARK),
        fg(color::DARK_GRAY),
        RESET_FG
    );

    let mut used = 0u16;
    used += draw_wrapped(
        stdout,
        row,
        width,
        available_rows,
        &headline_prefix,
        &headline_indent,
        headline,
        delta.as_deref(),
        &fg(color::WHITE),
    )?;

    if used >= available_rows {
        write!(stdout, "{}", RESET)?;
        return Ok(used);
    }

    // Build the detail items (bullets first when present, then next-prompt
    // note, then artifact). Each item gets its own wrap pass so we never
    // mix two ideas onto one visually crowded row.
    let mut detail_items: Vec<(&'static str, String)> = Vec::new();
    if latest.variant == RecapVariant::Bullets {
        for bullet in latest.bullets.iter().take(2) {
            detail_items.push(("• ", bullet.trim().to_string()));
        }
    }
    if detail_items.is_empty() {
        if let Some(note) = latest.next_prompt_notes.first() {
            detail_items.push(("Next: ", note.trim().to_string()));
        }
    } else if let Some(note) = latest.next_prompt_notes.first() {
        detail_items.push(("Next: ", note.trim().to_string()));
    }
    if let Some(artifact) = latest.artifacts.first() {
        detail_items.push(("Artifact: ", artifact.trim().to_string()));
    }

    for (label, body) in detail_items {
        if used >= available_rows {
            break;
        }
        let detail_prefix = format!(
            "{}{}   {}{}{}",
            bg(color::BG_DARK),
            fg(color::DARK_GRAY),
            RESET_FG,
            fg(color::WHITE),
            label,
        );
        // Continuation rows hang under the label so the eye tracks the bullet.
        let pad = " ".repeat(3 + label.chars().count());
        let detail_indent = format!(
            "{}{}{}{}",
            bg(color::BG_DARK),
            fg(color::DARK_GRAY),
            pad,
            RESET_FG
        );
        used += draw_wrapped(
            stdout,
            row + used,
            width,
            available_rows.saturating_sub(used),
            &detail_prefix,
            &detail_indent,
            &body,
            None,
            &fg(color::WHITE),
        )?;
    }

    write!(stdout, "{}", RESET)?;
    Ok(used)
}

/// Render `body` starting at `row`, wrapping on word boundaries. The first
/// row gets `first_prefix`; continuation rows get `cont_prefix` (use it for
/// hanging indents). When `right` is supplied it paints right-aligned on
/// the first row only and the wrap budget reserves space for it. Returns
/// the number of rows actually consumed (capped at `available_rows`).
#[allow(clippy::too_many_arguments)]
fn draw_wrapped(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    available_rows: u16,
    first_prefix: &str,
    cont_prefix: &str,
    body: &str,
    right: Option<&str>,
    body_fg: &str,
) -> Result<u16> {
    if available_rows == 0 {
        return Ok(0);
    }

    let first_prefix_width = crate::ui::utils::strip_ansi_len(first_prefix);
    let cont_prefix_width = crate::ui::utils::strip_ansi_len(cont_prefix);
    let right_width = right.map(crate::ui::utils::strip_ansi_len).unwrap_or(0);
    let right_gap = usize::from(right.is_some());

    let first_body_budget = (width as usize)
        .saturating_sub(first_prefix_width)
        .saturating_sub(right_width)
        .saturating_sub(right_gap)
        .max(1);
    let cont_body_budget = (width as usize).saturating_sub(cont_prefix_width).max(1);

    let lines = wrap_to_widths(body, first_body_budget, cont_body_budget, available_rows as usize);

    for (i, line) in lines.iter().enumerate() {
        let line_row = row + i as u16;
        fill_row(stdout, line_row, width)?;
        write!(stdout, "{}", escape::cursor_to(line_row, 1))?;
        let (prefix, used_prefix_width) = if i == 0 {
            (first_prefix, first_prefix_width)
        } else {
            (cont_prefix, cont_prefix_width)
        };
        write!(stdout, "{}{}{}", prefix, body_fg, line)?;

        if i == 0 {
            if let Some(right) = right {
                let used = used_prefix_width + crate::ui::utils::strip_ansi_len(line);
                let padding = (width as usize)
                    .saturating_sub(used)
                    .saturating_sub(right_width);
                write!(stdout, "{:padding$}{}", "", right, padding = padding)?;
            }
        }
    }

    Ok(lines.len() as u16)
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

/// Word-wrap `text` into at most `max_lines` lines, splitting on whitespace.
/// The first line uses `first_budget` columns; continuations use `cont_budget`.
/// A word longer than the budget is hard-broken with a trailing ellipsis so a
/// single pathological URL or token can't blow up the layout.
fn wrap_to_widths(
    text: &str,
    first_budget: usize,
    cont_budget: usize,
    max_lines: usize,
) -> Vec<String> {
    use unicode_width::UnicodeWidthStr;

    if max_lines == 0 {
        return Vec::new();
    }

    let budget_for = |idx: usize| if idx == 0 { first_budget } else { cont_budget };
    let mut lines: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_width = 0usize;
    let mut budget = budget_for(0);

    let words: Vec<&str> = text.split_whitespace().collect();
    let mut i = 0;
    while i < words.len() {
        // Last allowed line: dump everything remaining and ellipsize if needed.
        if lines.len() + 1 == max_lines {
            let mut tail = if current.is_empty() {
                String::new()
            } else {
                current.clone()
            };
            for word in &words[i..] {
                if tail.is_empty() {
                    tail.push_str(word);
                } else {
                    tail.push(' ');
                    tail.push_str(word);
                }
            }
            lines.push(truncate_to_width(&tail, budget));
            return lines;
        }

        let word = words[i];
        let word_width = word.width();
        let separator_width = if current.is_empty() { 0 } else { 1 };

        if word_width > budget {
            // Hard-break: flush current, then truncate the long word.
            if !current.is_empty() {
                lines.push(std::mem::take(&mut current));
                current_width = 0;
                budget = budget_for(lines.len());
                if lines.len() == max_lines {
                    return lines;
                }
                continue; // re-evaluate the same word against the new budget
            }
            lines.push(truncate_to_width(word, budget));
            i += 1;
            if lines.len() == max_lines {
                return lines;
            }
            budget = budget_for(lines.len());
            continue;
        }

        if current_width + separator_width + word_width <= budget {
            if !current.is_empty() {
                current.push(' ');
                current_width += 1;
            }
            current.push_str(word);
            current_width += word_width;
            i += 1;
        } else {
            lines.push(std::mem::take(&mut current));
            current_width = 0;
            budget = budget_for(lines.len());
            if lines.len() == max_lines {
                return lines;
            }
            // Don't increment i — re-place the word on the new line.
        }
    }

    if !current.is_empty() && lines.len() < max_lines {
        lines.push(current);
    }
    lines
}

fn truncate_to_width(text: &str, max_width: usize) -> String {
    use unicode_width::UnicodeWidthChar;

    if max_width == 0 {
        return String::new();
    }
    let mut width = 0;
    let mut out = String::new();
    let chars: Vec<char> = text.chars().collect();
    let total = chars.len();
    for (i, ch) in chars.iter().enumerate() {
        let ch_width = ch.width().unwrap_or(0);
        let last = i + 1 == total;
        if width + ch_width > max_width {
            if !out.is_empty() {
                out.pop();
            }
            out.push('…');
            return out;
        }
        if !last && width + ch_width == max_width {
            // Reserve the final cell for the ellipsis when there's more text.
            out.push('…');
            return out;
        }
        width += ch_width;
        out.push(*ch);
    }
    out
}

fn truncate_display(text: &str, max_width: usize) -> String {
    truncate_to_width(text, max_width)
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

    #[test]
    fn wrap_keeps_short_text_on_one_line() {
        let lines = wrap_to_widths("hello world", 40, 30, 3);
        assert_eq!(lines, vec!["hello world"]);
    }

    #[test]
    fn wrap_breaks_at_word_boundary_when_overflowing() {
        let lines = wrap_to_widths("the quick brown fox jumps over the lazy dog", 20, 18, 4);
        assert_eq!(lines.len(), 3);
        assert!(lines[0].chars().count() <= 20);
        assert!(lines[1].chars().count() <= 18);
        // No word should be split across lines.
        assert!(!lines.iter().any(|line| line.ends_with(' ')));
    }

    #[test]
    fn wrap_uses_narrower_continuation_budget() {
        // First line gets 30 cols, continuations only 10.
        let lines = wrap_to_widths(
            "alpha beta gamma delta epsilon zeta eta theta",
            30,
            10,
            5,
        );
        assert!(lines[0].chars().count() <= 30);
        for line in &lines[1..] {
            assert!(line.chars().count() <= 10);
        }
    }

    #[test]
    fn wrap_ellipsizes_overflow_on_last_allowed_line() {
        let lines = wrap_to_widths("one two three four five six seven eight nine ten", 12, 12, 2);
        assert_eq!(lines.len(), 2);
        assert!(lines[1].ends_with('…'));
    }

    #[test]
    fn wrap_hard_breaks_a_pathologically_long_word() {
        let lines = wrap_to_widths("verylongwordwithoutspaces", 10, 10, 2);
        assert_eq!(lines.len(), 1);
        assert!(lines[0].ends_with('…'));
        assert!(lines[0].chars().count() <= 10);
    }

    #[test]
    fn truncate_to_width_keeps_room_for_ellipsis() {
        assert_eq!(truncate_to_width("hello world", 11), "hello world");
        assert_eq!(truncate_to_width("hello world!", 11), "hello worl…");
        assert_eq!(truncate_to_width("hello", 0), "");
    }
}
