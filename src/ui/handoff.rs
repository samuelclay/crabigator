//! Handoff strip above the widget separator.
//!
//! This area is reserved for setup prompts, update notices, and the latest
//! automatic recap. It sits between the assistant PTY and the status widgets.

use std::io::{Stdout, Write};

use anyhow::Result;

use crate::pr::SessionPr;
use crate::recap::{RecapState, RecapStatus, RecapVariant, TurnLineDelta};
use crate::terminal::escape::{self, bg, color, fg, RESET, RESET_FG};

use super::cooldown::Cooldowns;
use super::pr_cells::*;
use super::time::format_elapsed_age;

/// Maximum rows the recap/toast/pairing content may use in the handoff strip.
pub const MAX_RECAP_ROWS: u16 = 3;

/// Maximum PR lines rendered in the handoff strip (one per PR). Bounds how far
/// the handoff can grow into the assistant's PTY area.
pub const MAX_PR_ROWS: u16 = 6;

/// Thin divider row shown only when the handoff contains PRs.
pub const PR_SEPARATOR_ROWS: u16 = 1;

const RECAP_HINT_MIN_WIDTH: u16 = 47;
const RECAP_TOAST_MIN_WIDTH: u16 = 18;
/// PR lines the handoff will render for this session (one per PR, capped).
pub fn pr_handoff_rows(prs: &[SessionPr]) -> u16 {
    (display_prs(prs).len() as u16).min(MAX_PR_ROWS)
}

/// Total handoff height = active recap/banner rows + optional divider + PR lines.
pub fn total_handoff_rows(recap_rows: u16, prs: &[SessionPr]) -> u16 {
    recap_rows + pr_separator_rows(prs) + pr_handoff_rows(prs)
}

/// Divider height between recap content and the PR table.
pub fn pr_separator_rows(prs: &[SessionPr]) -> u16 {
    if display_prs(prs).is_empty() {
        0
    } else {
        PR_SEPARATOR_ROWS
    }
}

/// Draw a subtle inset rule between recap content and the PR table.
pub fn draw_pr_separator(stdout: &mut Stdout, row: u16, width: u16) -> Result<()> {
    fill_row(stdout, row, width)?;
    write!(
        stdout,
        "{}{}{}",
        escape::cursor_to(row, 1),
        bg(color::BG_DARK),
        fg(color::DARK_GRAY)
    )?;
    let left_padding = usize::from(width > 0);
    write!(
        stdout,
        "{:left_padding$}{}",
        "",
        "─".repeat(pr_separator_rule_width(width)),
        left_padding = left_padding
    )?;
    write!(stdout, "{}", RESET)?;
    Ok(())
}

fn pr_separator_rule_width(width: u16) -> usize {
    (width as usize).saturating_sub(2)
}

/// Draw the session's PR list, one PR per row, on the terminal's own
/// background so the strip reads as part of the session, not a panel.
/// Each row uses shared columns:
/// `☆ repo  #num  +A -D  N files  ⎇  branch  state CI 💬N merge`.
/// The first five are left-aligned, with the branch column flexing across the
/// middle. Status columns are anchored at the right edge with a tighter
/// one-space rhythm. The table leaves one cell at the right window edge.
#[allow(clippy::too_many_arguments)]
pub fn draw_pr_handoff(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    prs: &[SessionPr],
    pr_scope: &str,
    available_rows: u16,
    cooldowns: &Cooldowns,
    now_ms: u64,
) -> Result<u16> {
    let display = display_prs(prs);
    if available_rows == 0 || display.is_empty() {
        return Ok(0);
    }
    let limit = display
        .len()
        .min(available_rows as usize)
        .min(MAX_PR_ROWS as usize);
    let widths = PrColumnWidths::from_pr_refs(&display[..limit], width as usize);
    for (i, pr) in display.iter().take(limit).enumerate() {
        write!(
            stdout,
            "{}{}",
            escape::cursor_to(row + i as u16, 1),
            pr_row_text_tinted(width, pr, &widths, cooldowns.pr_tints(pr, now_ms), pr_scope)
        )?;
    }
    write!(stdout, "{}", RESET)?;
    Ok(limit as u16)
}

pub fn draw_recap_handoff(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    recap: &RecapState,
    available_rows: u16,
    toast_visible: bool,
) -> Result<u16> {
    if available_rows == 0 {
        return Ok(0);
    }

    if recap.latest.is_some() {
        return draw_latest_recap(stdout, row, width, available_rows, recap);
    }
    if let RecapStatus::Failed(error) = &recap.status {
        return draw_recap_failure(stdout, row, width, available_rows, error);
    }
    if matches!(recap.status, RecapStatus::MissingKey) {
        return draw_recap_hint(stdout, row, width);
    }
    if toast_visible
        && matches!(
            recap.status,
            RecapStatus::Waiting | RecapStatus::Updating | RecapStatus::Ready
        )
    {
        return draw_recap_toast(stdout, row, width);
    }
    Ok(0)
}

/// Exact recap rows needed at this width, capped by `available_rows`.
///
/// This shares the same item planning and wrapping logic as the renderer so
/// layout reservations cannot drift from what is actually drawn.
pub fn recap_handoff_rows(
    width: u16,
    recap: &RecapState,
    available_rows: u16,
    toast_visible: bool,
) -> u16 {
    if available_rows == 0 {
        return 0;
    }

    if recap.latest.is_some() {
        return latest_recap_rows(width, available_rows, recap);
    }
    if matches!(recap.status, RecapStatus::Failed(_)) {
        return available_rows.min(2);
    }
    if matches!(recap.status, RecapStatus::MissingKey) {
        return u16::from(recap_hint_fits(width));
    }
    if toast_visible
        && matches!(
            recap.status,
            RecapStatus::Waiting | RecapStatus::Updating | RecapStatus::Ready
        )
    {
        return u16::from(recap_toast_fits(width));
    }
    0
}

fn recap_hint_fits(width: u16) -> bool {
    width >= RECAP_HINT_MIN_WIDTH
}

fn recap_toast_fits(width: u16) -> bool {
    width >= RECAP_TOAST_MIN_WIDTH
}

/// One-line "✦ Per-turn AI recaps off — crabigator key <key> · crabigator
/// recap disable" hint, painted on the dark recap background. Three width
/// tiers degrade gracefully:
/// - **Verbose** (≥78 cols): full label + both commands.
/// - **Full**    (≥64 cols): drops the "Per-turn AI" preface.
/// - **Compact** (≥47 cols): just the two commands.
///
/// Returns 1 row consumed when rendered, 0 if even the compact tier won't fit.
fn draw_recap_hint(stdout: &mut Stdout, row: u16, width: u16) -> Result<u16> {
    const VERBOSE_VISIBLE: usize = 74;
    const FULL_VISIBLE: usize = 62;
    let usable = width as usize;
    let dim = fg(color::DARK_GRAY);
    let sparkle = format!("{}✦{}", fg(color::YELLOW), RESET_FG);
    let dash = format!("{}—{}", fg(color::DARK_GRAY), RESET_FG);
    let dot = format!("{}·{}", fg(color::DARK_GRAY), RESET_FG);
    let key_cmd = format!("{}crabigator key{}", fg(color::CYAN), RESET_FG);
    let placeholder = format!("{}<key>{}", fg(color::GRAY), RESET_FG);
    let disable_cmd = format!(
        "{}crabigator recap disable{}",
        fg(color::DARK_GRAY),
        RESET_FG
    );

    let body = if usable >= VERBOSE_VISIBLE + 4 {
        let label = format!("{}Per-turn AI recaps off{}", fg(color::GRAY), RESET_FG);
        format!(
            "{} {} {} {} {} {} {}",
            sparkle, label, dash, key_cmd, placeholder, dot, disable_cmd
        )
    } else if usable >= FULL_VISIBLE + 2 {
        let label = format!("{}Recaps off{}", fg(color::GRAY), RESET_FG);
        format!(
            "{} {} {} {} {} {} {}",
            sparkle, label, dash, key_cmd, placeholder, dot, disable_cmd
        )
    } else if usable >= RECAP_HINT_MIN_WIDTH as usize {
        format!("{} {}  {}  {}", sparkle, key_cmd, dot, disable_cmd)
    } else {
        return Ok(0);
    };

    fill_row(stdout, row, width)?;
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    // The dim-grey background on the row (from fill_row) reads as muted, so
    // the colored sparkle/cmd accents pop without shouting.
    write!(stdout, "{}{} {}{}", bg(color::BG_DARK), dim, body, RESET)?;
    Ok(1)
}

/// One-line "✓ Per-turn AI recaps enabled" toast, painted on the dark recap
/// background. Two width tiers; returns 0 if even the compact tier won't fit.
fn draw_recap_toast(stdout: &mut Stdout, row: u16, width: u16) -> Result<u16> {
    const FULL_VISIBLE: usize = 28;
    let usable = width as usize;
    let check = format!("{}✓{}", fg(color::GREEN), RESET_FG);

    let body = if usable >= FULL_VISIBLE + 2 {
        let label = format!("{}Per-turn AI recaps enabled{}", fg(color::GRAY), RESET_FG);
        format!("{} {}", check, label)
    } else if usable >= RECAP_TOAST_MIN_WIDTH as usize {
        let label = format!("{}Recaps enabled{}", fg(color::GRAY), RESET_FG);
        format!("{} {}", check, label)
    } else {
        return Ok(0);
    };

    fill_row(stdout, row, width)?;
    write!(stdout, "{}", escape::cursor_to(row, 1))?;
    write!(
        stdout,
        "{}{} {}{}",
        bg(color::BG_DARK),
        fg(color::DARK_GRAY),
        body,
        RESET
    )?;
    Ok(1)
}

struct RecapItem {
    first_prefix: String,
    cont_prefix: String,
    body: String,
    right: Option<String>,
}

fn latest_recap_items(recap: &RecapState) -> Vec<RecapItem> {
    let Some(latest) = recap.latest.as_ref() else {
        return Vec::new();
    };

    let delta = recap
        .line_delta
        .or(Some(latest.line_delta))
        .map(format_line_delta);
    let meta = format_recap_meta(latest.generated_at, delta);

    // Headline first — wraps onto the next row only if it overflows the
    // available body width on row 1 (where the Δ delta lives).
    let mut items = vec![RecapItem {
        first_prefix: format!("{}{} ●{} ", bg(color::BG_DARK), fg(color::CYAN), RESET_FG),
        cont_prefix: format!(
            "{}{}   {}",
            bg(color::BG_DARK),
            fg(color::DARK_GRAY),
            RESET_FG
        ),
        body: latest.headline.trim().to_string(),
        right: meta,
    }];

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
        // Continuation rows hang under the label so the eye tracks the bullet.
        let pad = " ".repeat(3 + label.chars().count());
        items.push(RecapItem {
            first_prefix: format!(
                "{}{}   {}{}{}",
                bg(color::BG_DARK),
                fg(color::DARK_GRAY),
                RESET_FG,
                fg(color::WHITE),
                label,
            ),
            cont_prefix: format!(
                "{}{}{}{}",
                bg(color::BG_DARK),
                fg(color::DARK_GRAY),
                pad,
                RESET_FG
            ),
            body,
            right: None,
        });
    }

    items
}

fn latest_recap_rows(width: u16, available_rows: u16, recap: &RecapState) -> u16 {
    let mut used = 0;
    for item in latest_recap_items(recap) {
        let remaining = available_rows.saturating_sub(used);
        if remaining == 0 {
            break;
        }
        used += wrapped_lines(
            width,
            remaining,
            &item.first_prefix,
            &item.cont_prefix,
            &item.body,
            item.right.as_deref(),
        )
        .len() as u16;
    }
    used
}

fn draw_latest_recap(
    stdout: &mut Stdout,
    row: u16,
    width: u16,
    available_rows: u16,
    recap: &RecapState,
) -> Result<u16> {
    let mut used = 0;
    for item in latest_recap_items(recap) {
        let remaining = available_rows.saturating_sub(used);
        if remaining == 0 {
            break;
        }
        used += draw_wrapped(
            stdout,
            row + used,
            width,
            remaining,
            &item.first_prefix,
            &item.cont_prefix,
            &item.body,
            item.right.as_deref(),
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
    let lines = wrapped_lines(
        width,
        available_rows,
        first_prefix,
        cont_prefix,
        body,
        right,
    );
    let right_width = right.map(crate::ui::utils::strip_ansi_len).unwrap_or(0);

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

fn wrapped_lines(
    width: u16,
    available_rows: u16,
    first_prefix: &str,
    cont_prefix: &str,
    body: &str,
    right: Option<&str>,
) -> Vec<String> {
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

    wrap_to_widths(
        body,
        first_body_budget,
        cont_body_budget,
        available_rows as usize,
    )
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
    let body = truncate_to_width(body, body_width);
    // Body text uses a slightly brighter gray than the dim hint so the
    // message itself is what catches the eye.
    write!(stdout, "{}{}{}", prefix, fg(color::GRAY), body)?;
    Ok(())
}

/// `Δ +A -D` in green/red, or `Δ net +N` when a rebase makes either side
/// negative. Shared with the PR board's recap detail lines.
pub(crate) fn format_line_delta(delta: TurnLineDelta) -> String {
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

fn format_recap_meta(generated_at_ms: u64, delta: Option<String>) -> Option<String> {
    let age = format_elapsed_age(Some(generated_at_ms as f64 / 1000.0))
        .map(|age| format!("{}{}{}", fg(color::GRAY), age, RESET_FG));

    match (age, delta) {
        (Some(age), Some(delta)) => Some(format!("{}  {}", age, delta)),
        (Some(age), None) => Some(age),
        (None, delta) => delta,
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use unicode_width::UnicodeWidthStr;

    fn sample_pr(repo: &str, number: u64, branch: &str) -> SessionPr {
        SessionPr {
            number,
            owner: "Tavus-Engineering".to_string(),
            repo: repo.to_string(),
            url: format!("https://github.com/Tavus-Engineering/{repo}/pull/{number}"),
            author_login: String::new(),
            authored_by_viewer: None,
            branch: branch.to_string(),
            title: String::new(),
            state: "OPEN".to_string(),
            is_draft: false,
            additions: 1_869,
            deletions: 2,
            changed_files: 13,
            mergeable: "MERGEABLE".to_string(),
            merge_state_status: "BEHIND".to_string(),
            checks_passed: 38,
            checks_failed: 0,
            checks_pending: 2,
            checks_total: 40,
            ci_url: format!("https://github.com/Tavus-Engineering/{repo}/pull/{number}/checks"),
            unresolved_comments: 3,
            comments_url: format!(
                "https://github.com/Tavus-Engineering/{repo}/pull/{number}#discussion_r1"
            ),
            comments_refreshed_at: 0,
            created_here: false,
            mentions: 0,
            user_mentions: 0,
            first_mentioned_at: 0,
            last_mentioned_at: 0,
            last_mention_prompt: 0,
            branch_matched: false,
            review_decision: String::new(),
            review_dismissed: false,
            closed_at: 0,
            updated_at: 0,
            primary: false,
            primary_source: String::new(),
            dismissed: false,
            watched: false,
            slack_origin_url: String::new(),
            slack_comment_urls: Vec::new(),
            ai_note: String::new(),
            ai_confidence: String::new(),
            fetch_error: String::new(),
            refreshed_at: 0,
        }
    }

    /// The OSC 8 opening sequence a cell carries when it links to `url`.
    fn link_to(url: &str) -> String {
        format!("\x1b]8;;{url}\x07")
    }

    #[test]
    fn pr_separator_reserves_a_row_only_when_prs_exist() {
        assert_eq!(pr_separator_rows(&[]), 0);
        assert_eq!(total_handoff_rows(0, &[]), 0);
        assert_eq!(total_handoff_rows(2, &[]), 2);

        let prs = [sample_pr("request-handler", 2475, "feature/full")];
        assert_eq!(pr_separator_rows(&prs), PR_SEPARATOR_ROWS);
        assert_eq!(total_handoff_rows(2, &prs), 2 + PR_SEPARATOR_ROWS + 1);
    }

    #[test]
    fn primaries_sort_first_and_dismissed_disappear() {
        let mut primary = sample_pr("portal", 1079, "sam/fix");
        primary.primary = true;
        let secondary = sample_pr("portal", 1075, "sam/old");
        let mut dismissed = sample_pr("cvi", 4546, "sam/rejected");
        dismissed.dismissed = true;

        let prs = [secondary, dismissed, primary];
        let display = display_prs(&prs);
        assert_eq!(
            display.iter().map(|pr| pr.number).collect::<Vec<_>>(),
            vec![1079, 1075],
            "primary first, dismissed gone"
        );
        assert_eq!(pr_handoff_rows(&prs), 2);

        // A session whose every PR is dismissed reserves no rows at all.
        let mut all_dismissed = sample_pr("portal", 1, "x");
        all_dismissed.dismissed = true;
        assert_eq!(pr_separator_rows(std::slice::from_ref(&all_dismissed)), 0);
        assert_eq!(pr_handoff_rows(std::slice::from_ref(&all_dismissed)), 0);
    }

    #[test]
    fn primary_prs_wear_the_star() {
        let mut pr = sample_pr("portal", 1079, "sam/fix");
        assert!(pr_identity_text(&pr).starts_with("☆ "));
        pr.primary = true;
        assert!(pr_identity_text(&pr).starts_with("★ "));
        // A primary's star links to the demote action.
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);
        let cells = pr_left_cells(&pr, &widths);
        assert!(cells[0].0.contains(&format!(
            "\x1b]8;;{}\x07★",
            pr_action_url(&pr, "secondary", "")
        )));
    }

    #[test]
    fn unenriched_prs_show_fetch_progress_then_errors() {
        let mut pr = sample_pr("portal", 1079, "");
        pr.state = String::new();
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);
        let row = pr_row_text(160, &pr, &widths);
        assert!(row.contains("fetch…"), "never-enriched PR shows progress");

        pr.fetch_error = "HTTP 403: rate limited".to_string();
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);
        let row = pr_row_text(160, &pr, &widths);
        assert!(row.contains("error"), "failed fetch says so, not silence");
    }

    #[test]
    fn changed_status_cells_wear_the_fading_tint() {
        let mut cooldowns = Cooldowns::default();
        let mut pr = sample_pr("request-handler", 2475, "feature/full");
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);

        cooldowns.observe_pr(&pr, 1_000);
        let row = pr_row_text_tinted(160, &pr, &widths, cooldowns.pr_tints(&pr, 1_000), "");
        assert!(
            !row.contains("\x1b[48;2;"),
            "a PR seen for the first time stays cold"
        );

        pr.mergeable = "CONFLICTING".to_string();
        cooldowns.observe_pr(&pr, 5_000);
        let row = pr_row_text_tinted(160, &pr, &widths, cooldowns.pr_tints(&pr, 5_000), "");
        assert!(
            row.contains("\x1b[48;2;"),
            "the merge cell lights up when it flips to conflicts"
        );
    }

    #[test]
    fn secondary_prs_render_one_notch_dimmer() {
        let mut pr = sample_pr("portal", 1079, "sam/fix");
        pr.state = "MERGED".to_string();
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);

        let secondary_row = pr_row_text(160, &pr, &widths);
        assert!(
            secondary_row.contains(&fg(escape::color::dimmed(escape::color::PURPLE))),
            "secondary merged state uses the dim purple"
        );

        pr.primary = true;
        let primary_row = pr_row_text(160, &pr, &widths);
        assert!(
            primary_row.contains(&fg(escape::color::PURPLE)),
            "primary merged state keeps the full purple"
        );
    }

    #[test]
    fn pr_separator_rule_leaves_one_cell_at_each_edge() {
        assert_eq!(pr_separator_rule_width(0), 0);
        assert_eq!(pr_separator_rule_width(1), 0);
        assert_eq!(pr_separator_rule_width(2), 0);
        assert_eq!(pr_separator_rule_width(3), 1);
        assert_eq!(pr_separator_rule_width(120), 118);
    }

    #[test]
    fn pr_columns_use_widest_visible_value_with_caps() {
        let mut long = sample_pr(
            "request-handler-with-an-unreasonably-long-name",
            2475,
            "sam/modify-system-user-document-callback-with-more-detail",
        );
        long.changed_files = 48;
        long.checks_failed = 4;
        long.checks_pending = 0;
        long.mergeable = "CONFLICTING".to_string();

        let prs = [
            long,
            sample_pr("developer-portal", 989, "sam/pal-maker-fanout-local"),
        ];
        let widths = PrColumnWidths::from_prs(&prs, 220);
        let long_cells = pr_left_cells(&prs[0], &widths);

        assert_eq!(widths.identity, PR_IDENTITY_MAX);
        assert_eq!(widths.number, "#2475".width());
        assert!(widths.branch > PR_BRANCH_MAX);
        assert_eq!(long_cells[4].1, PR_BRANCH_MAX);
        assert_eq!(long_cells[4].2, widths.branch);
        assert_eq!(
            crate::parsers::strip_ansi_for_debug(&long_cells[4].0),
            "⎇  …callback-with-more-detail"
        );
        assert_eq!(widths.files, "48 files".width());
        assert_eq!(widths.state, "open".width());
        assert_eq!(widths.ci, "✗4 CI".width());
        assert_eq!(widths.merge, "conflicts".width());
        assert_eq!(widths.total_width(), 220);
    }

    #[test]
    fn pr_columns_shrink_branch_before_core_columns_at_narrow_widths() {
        let prs = [
            sample_pr(
                "request-handler",
                2475,
                "reev/rqh-ecs-preview-with-a-long-branch-name",
            ),
            sample_pr("developer-portal", 989, "sam/pal-maker-fanout-local"),
        ];
        let widths = PrColumnWidths::from_prs(&prs, 80);

        assert!(widths.branch < PR_BRANCH_MAX);
        assert!(widths.identity > 0);
        assert!(widths.diff > 0);
        assert!(widths.files > 0);
        assert!(widths.state > 0);
        assert_eq!(widths.total_width(), 80);
    }

    /// A session's rows carry the session's disposition scope in their action
    /// links, URL-encoded, so a dismissal lands on that session (or its
    /// worktree) instead of the whole group.
    #[test]
    fn action_links_carry_the_session_scope() {
        let pr = sample_pr("portal", 7, "sam/scoped");
        let url = pr_action_url(&pr, "dismissed", "session:abc-123");
        assert!(url.ends_with("&scope=session%3Aabc-123"), "{url}");
        let url = pr_action_url(&pr, "dismissed", "path:/Users/sam/work tree");
        assert!(
            url.ends_with("&scope=path%3A/Users/sam/work%20tree"),
            "{url}"
        );
        // No scope means the old group-wide link, byte for byte.
        assert!(!pr_action_url(&pr, "dismissed", "").contains("scope="));

        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);
        let row = pr_row_text_tinted(
            160,
            &pr,
            &widths,
            super::super::cooldown::StatusTints::default(),
            "session:abc-123",
        );
        assert!(
            row.contains(&link_to(&pr_action_url(
                &pr,
                "dismissed",
                "session:abc-123"
            ))),
            "the dismiss cell links with the scope: {row}"
        );
    }

    /// Every cell that has a target carries its own link, and linking never
    /// changes the width the column was laid out for.
    #[test]
    fn pr_cells_link_to_their_own_github_targets() {
        let pr = sample_pr("request-handler", 2412, "sam/pal-fanout-fable");
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);
        let left = pr_left_cells(&pr, &widths);
        let right = pr_right_cells(&pr, &widths);

        // Identity → the PR itself; the `☆` glyph is its own link that flips
        // the PR to primary via the web action page.
        assert!(left[0].0.contains(&link_to(&pr.url)));
        assert!(left[0]
            .0
            .contains(&link_to(&pr_action_url(&pr, "primary", ""))));
        // Number → the PR; diff and file count → the Files-changed tab.
        assert!(left[1].0.contains("#2412"));
        assert_eq!(left[1].1, "#2412".width());
        assert!(left[1].0.contains(&link_to(&pr.url)));
        let files = link_to(&format!("{}/files", pr.url));
        assert!(left[2].0.contains(&files));
        assert!(left[3].0.contains(&files));
        // CI and the thread count → wherever they pointed; state, review, and
        // merge stay plain.
        assert!(right[1].0.contains(&link_to(&pr.ci_url)));
        assert!(right[2].0.contains(&link_to(&pr.comments_url)));
        assert!(!right[0].0.contains("\x1b]8;;"));
        assert!(!right[3].0.contains("\x1b]8;;"));
        assert!(!right[4].0.contains("\x1b]8;;"));
        // The promote action (`↑` on a secondary PR) and the dismiss action
        // link to the same page with their own dispositions.
        assert!(right[5].0.contains('↑'));
        assert!(right[5]
            .0
            .contains(&link_to(&pr_action_url(&pr, "primary", ""))));
        assert!(right[6].0.contains('✕'));
        assert!(right[6]
            .0
            .contains(&link_to(&pr_action_url(&pr, "dismissed", ""))));

        for (styled, visible, _) in left.iter().chain(right.iter()) {
            assert_eq!(crate::ui::utils::strip_ansi_len(styled), *visible);
        }
    }

    /// The action beside `✕` always offers the opposite classification: `↓`
    /// demotes a primary, `↑` promotes a secondary — including a watched PR,
    /// whose identity glyph unwatches instead.
    #[test]
    fn promote_demote_action_offers_the_opposite_classification() {
        let mut primary = sample_pr("request-handler", 2412, "sam/pal-fanout-fable");
        primary.primary = true;
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&primary), 160);
        let right = pr_right_cells(&primary, &widths);
        assert!(right[5].0.contains('↓'));
        assert!(right[5]
            .0
            .contains(&link_to(&pr_action_url(&primary, "secondary", ""))));

        let mut watched = sample_pr("request-handler", 2413, "sam/watched");
        watched.watched = true;
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&watched), 160);
        let left = pr_left_cells(&watched, &widths);
        let right = pr_right_cells(&watched, &widths);
        assert!(left[0]
            .0
            .contains(&link_to(&pr_action_url(&watched, "unwatched", ""))));
        assert!(right[5].0.contains('↑'));
        assert!(right[5]
            .0
            .contains(&link_to(&pr_action_url(&watched, "primary", ""))));
    }

    /// A PR with nothing to point at (no checks, no diff yet) stays unlinked
    /// rather than rendering a clickable blank cell.
    #[test]
    fn pr_cells_without_targets_are_not_linked() {
        let mut pending = sample_pr("developer-portal", 989, "");
        pending.additions = 0;
        pending.deletions = 0;
        pending.changed_files = 0;
        pending.ci_url = String::new();
        pending.unresolved_comments = 0;
        pending.comments_url = String::new();

        let widths = PrColumnWidths::from_prs(&[sample_pr("request-handler", 2412, "x")], 160);
        let left = pr_left_cells(&pending, &widths);
        let right = pr_right_cells(&pending, &widths);

        assert!(!left[2].0.contains("\x1b]8;;"));
        assert!(!left[3].0.contains("\x1b]8;;"));
        assert!(!right[1].0.contains("\x1b]8;;"));
        assert!(!right[2].0.contains("\x1b]8;;"));
    }

    /// A settled conversation leaves the column blank; an unresolved one shows
    /// `💬N`, and the emoji's two cells are budgeted for.
    #[test]
    fn unresolved_comments_render_only_when_threads_are_open() {
        let mut pr = sample_pr("developer-portal", 952, "sam/pal-maker-fanout-fable");
        pr.unresolved_comments = 70;
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 200);
        // Two cells for the emoji plus one per digit.
        assert_eq!(widths.comments, "💬70".width());
        assert_eq!(widths.comments, 4);

        let (label, _) = pr_comments_label(&pr);
        assert_eq!(label, "💬70");

        pr.unresolved_comments = 0;
        assert_eq!(pr_comments_label(&pr).0, "");
        assert_eq!(pr_right_cells(&pr, &widths)[2].1, 0);
    }

    /// The review column distinguishes approved, changes requested, an
    /// approval dismissed by new commits, and still awaiting review — and
    /// disappears once the PR is no longer open.
    #[test]
    fn review_column_covers_every_approval_state() {
        let mut pr = sample_pr("request-handler", 2412, "sam/pal-fanout-fable");
        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 160);
        let glyph = |pr: &SessionPr| pr_right_cells(pr, &widths)[3].0.clone();

        assert!(glyph(&pr).contains('◌'), "open PR awaits review");
        pr.review_decision = "APPROVED".to_string();
        assert!(glyph(&pr).contains('✓'));
        pr.review_decision = "CHANGES_REQUESTED".to_string();
        assert!(glyph(&pr).contains('✗'));
        pr.review_decision = String::new();
        pr.review_dismissed = true;
        assert!(glyph(&pr).contains('⊘'), "dismissed approval");
        pr.state = "MERGED".to_string();
        assert_eq!(pr_right_cells(&pr, &widths)[3].1, 0, "merged PRs show none");
    }

    /// The assembled row reads left to right in column order and fills the
    /// window save for the right padding, with the status cluster anchored right.
    #[test]
    fn a_row_lays_its_columns_out_in_order() {
        let mut pr = sample_pr("request-handler", 2412, "sam/pal-fanout-fable");
        pr.unresolved_comments = 3;
        pr.checks_failed = 1;
        pr.mergeable = "CONFLICTING".to_string();

        let widths = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 120);
        let row = pr_row_text(120, &pr, &widths);
        assert_eq!(
            crate::ui::utils::strip_ansi_len(&row),
            120 - PR_RIGHT_PADDING
        );
        assert_eq!(
            widths.right_width(),
            widths.state
                + widths.ci
                + widths.comments
                + widths.review
                + widths.merge
                + widths.flip
                + widths.dismiss
                + 6 * PR_RIGHT_COLUMN_GAP,
            "the seven status columns use six one-cell gaps"
        );

        let at = |needle: &str| {
            row.find(needle)
                .unwrap_or_else(|| panic!("{needle} missing"))
        };
        let order = [
            at("request-handler"),
            at("#2412"),
            at("+1869"),
            at("13 files"),
            at("⎇  sam/pal-fanout-fable"),
            at("open"),
            at("✗1 CI"),
            at("💬3"),
            at("◌"),
            at("conflicts"),
        ];
        assert!(
            order.windows(2).all(|pair| pair[0] < pair[1]),
            "columns out of order: {order:?}"
        );
    }

    /// Narrow terminals shed the right cluster in priority order, so the thread
    /// count outlives merge cleanliness but yields to CI and state.
    #[test]
    fn narrow_rows_drop_merge_before_the_comment_count() {
        let mut pr = sample_pr("request-handler", 2501, "sam/guardrails");
        pr.unresolved_comments = 2;

        let roomy = PrColumnWidths::from_prs(std::slice::from_ref(&pr), 200);
        assert!(roomy.merge > 0 && roomy.comments > 0 && roomy.ci > 0);

        // The width at which the right cluster gets exactly `budget` columns.
        let total_for = |budget: usize| {
            PR_LEFT_PADDING
                + PR_IDENTITY_MIN
                + PR_COLUMN_GAP
                + roomy.number
                + PR_COLUMN_GAP
                + PR_RIGHT_PADDING
                + budget
        };
        // Enough for everything but merge cleanliness.
        let mut tight = roomy;
        tight.fit_right(total_for(
            roomy.right_width() - roomy.merge - PR_RIGHT_COLUMN_GAP,
        ));
        assert_eq!(tight.merge, 0);
        assert!(tight.comments > 0 && tight.ci > 0);

        // Tighter still: the thread count goes next, CI and state hold on.
        let mut tighter = roomy;
        tighter.fit_right(total_for(
            tight.right_width() - tight.comments - PR_RIGHT_COLUMN_GAP,
        ));
        assert_eq!((tighter.merge, tighter.comments), (0, 0));
        assert!(tighter.ci > 0 && tighter.state > 0);
    }

    #[test]
    fn empty_pr_values_leave_aligned_cells_blank() {
        let full = sample_pr("request-handler", 2475, "feature/full");
        let mut pending = sample_pr("developer-portal", 989, "");
        pending.additions = 0;
        pending.deletions = 0;
        pending.changed_files = 0;

        let widths = PrColumnWidths::from_prs(&[full, pending.clone()], 160);
        let cells = pr_left_cells(&pending, &widths);

        assert_eq!(cells[2].1, 0);
        assert_eq!(cells[3].1, 0);
        assert_eq!(cells[4].1, 0);
        assert_eq!(crate::ui::utils::strip_ansi_len(&cells[2].0), 0);
    }

    #[test]
    fn right_cells_truncate_styled_labels_to_their_columns() {
        let mut pr = sample_pr("request-handler", 2475, "feature/full");
        pr.state = "MERGED".to_string();
        pr.checks_failed = 123_456_789;
        pr.checks_pending = 0;
        pr.mergeable = "CONFLICTING".to_string();
        let widths = PrColumnWidths {
            state: 3,
            ci: 6,
            merge: 5,
            ..PrColumnWidths::default()
        };

        for (styled, visible, width) in pr_right_cells(&pr, &widths) {
            assert_eq!(visible, width);
            assert_eq!(crate::ui::utils::strip_ansi_len(&styled), width);
        }
    }

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

    fn sample_recap(headline: &str, artifact: Option<&str>) -> RecapState {
        RecapState {
            enabled: true,
            status: RecapStatus::Ready,
            latest: Some(crate::recap::TurnRecap {
                prompt_count: 1,
                generated_at: 0,
                title: None,
                variant: RecapVariant::Brief,
                headline: headline.to_string(),
                bullets: Vec::new(),
                next_prompt_notes: Vec::new(),
                artifacts: artifact.into_iter().map(str::to_string).collect(),
                line_delta: TurnLineDelta::default(),
                pr_notes: Vec::new(),
                slack_threads: Vec::new(),
            }),
            line_delta: None,
            model: String::new(),
        }
    }

    #[test]
    fn recap_rows_match_the_content_that_will_be_drawn() {
        let one_line = sample_recap("Finished the requested change", None);
        assert_eq!(recap_handoff_rows(200, &one_line, MAX_RECAP_ROWS, false), 1);

        let two_lines = sample_recap(
            "Listed four pull request URLs as requested",
            Some("request-handler/pull/2475"),
        );
        assert_eq!(
            recap_handoff_rows(200, &two_lines, MAX_RECAP_ROWS, false),
            2
        );
    }

    #[test]
    fn recap_rows_are_capped_and_disappear_with_no_content() {
        let long = sample_recap(
            "A deliberately long recap headline that must wrap across several narrow rows",
            Some("request-handler/pull/2475"),
        );
        assert_eq!(recap_handoff_rows(24, &long, 2, false), 2);
        assert_eq!(
            recap_handoff_rows(200, &RecapState::default(), MAX_RECAP_ROWS, false),
            0
        );
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
        let lines = wrap_to_widths("alpha beta gamma delta epsilon zeta eta theta", 30, 10, 5);
        assert!(lines[0].chars().count() <= 30);
        for line in &lines[1..] {
            assert!(line.chars().count() <= 10);
        }
    }

    #[test]
    fn wrap_ellipsizes_overflow_on_last_allowed_line() {
        let lines = wrap_to_widths(
            "one two three four five six seven eight nine ten",
            12,
            12,
            2,
        );
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
