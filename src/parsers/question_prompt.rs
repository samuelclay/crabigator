//! AskUserQuestion screen parser
//!
//! Claude Code's AskUserQuestion dialog changes pages, ticks checkboxes and
//! shows a final "Review your answers" page without sending any hook event.
//! This parser reads the dialog from the terminal screen so the dashboard can
//! mirror what the terminal shows: which question is on screen, which options
//! are ticked on a multi-select page, any custom text typed into the "Type
//! something" row, where the cursor is, and the review page.
//!
//! The dialog looks like this on a multi-select page:
//!
//! ```text
//! ←  ☒ Crust  ☐ Toppings  ✔ Submit  →
//! Which toppings?
//! ❯ 1. [✔] Cheese
//!   Classic
//!   2. [ ] Pepperoni
//!   Spicy
//!   5. [ ] Type something
//!      Submit
//! ────────────────────────────────
//!   6. Chat about this
//! Enter to select · ↑/↓ to navigate · Esc to cancel
//! ```
//!
//! and like this once every question has an answer:
//!
//! ```text
//! ←  ☒ Crust  ☒ Toppings  ✔ Submit  →
//! Review your answers
//!  ● Which crust?
//!    → Thick
//! Ready to submit your answers?
//! ❯ 1. Submit answers
//!   2. Cancel
//! ```
//!
//! A call with one single-select question has no tab strip, only its
//! header under the divider. Picking a row answers it, so it has no review
//! page either:
//!
//! ```text
//! ────────────────────────────────
//!  ☐ Crust
//! Which crust?
//!   1. Thin
//!      Crispy
//! ❯ 2. Thick
//!      Doughy
//!   3. Type something.
//! ────────────────────────────────
//!   4. Chat about this
//! Enter to select · ↑/↓ to navigate · Esc to cancel
//! ```
//!
//! Grok's question card is parsed from the same `parse` entry. It uses radio
//! `(○)`/`(●)`, checkboxes `[ ]`/`[x]`, a `z` free-text row, and a `[N/M]`
//! footer instead of Claude's tab strip.
//!
//! So are Codex's two question views. `request_user_input` shows a page
//! that ends in a "None of the above" row, which takes notes (Tab) but no
//! typed answer:
//!
//! ```text
//!   Question 1/2 (2 unanswered)
//!   Which crust?
//!
//!   › 1. Thin               Crispy
//!     2. Thick              Doughy
//!     3. None of the above  Optionally, add details in notes (tab)
//!
//!   tab to add notes | enter to submit answer | ←/→ to navigate questions | esc to interrupt
//! ```
//!
//! `request_user_input_async` questions wait under the composer behind
//! "⇧← to answer" while Codex keeps working. ⇧← opens them in place of the
//! composer, with an "Other" row for a typed answer, or only a text field
//! when the question has no options:
//!
//! ```text
//! • Queued follow-up inputs
//!
//!   1 of 2
//!   Which pizza size?
//!
//!   › 1. Small
//!     2. Medium
//!     3. Other
//!
//!   enter submit   ⌃] skip   ⇧→ main prompt   ⇧← next question
//! ```

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use super::permission_prompt::strip_ansi_codes;
use crate::platforms::{Question, QuestionOption};

/// Placeholder shown on the free-text row before the user types anything.
const TYPE_SOMETHING: &str = "Type something";
/// Grok's free-text row placeholder.
const TYPE_YOUR_ANSWER: &str = "Type your answer here";
/// Codex's last rows before anything is typed: the async view's "Other"
/// row and text field, and the sync view's notes-only row.
const CODEX_PLACEHOLDERS: &[&str] = &["Other", "Type your answer", "None of the above"];

/// One numbered row of a question page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ScreenRow {
    /// The row number Claude Code shows (1-indexed).
    pub number: u32,
    /// `Some(ticked)` on a multi-select page, `None` on a single-select page.
    pub checked: Option<bool>,
    /// The row label as shown; custom text once the user types on the free-text row.
    pub label: String,
    /// Whether the terminal cursor (❯) is on this row.
    pub cursor: bool,
}

impl ScreenRow {
    /// Whether this row still shows the "Type something" placeholder.
    pub fn is_placeholder(&self) -> bool {
        let label = self.label.trim_end_matches('.').trim();
        label == TYPE_SOMETHING
            || label.eq_ignore_ascii_case(TYPE_YOUR_ANSWER)
            || CODEX_PLACEHOLDERS.contains(&label)
    }
}

/// One answered question on the review page.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReviewAnswer {
    pub question: String,
    pub answer: String,
}

/// What the AskUserQuestion dialog is showing right now.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum QuestionScreen {
    /// One question page.
    Page {
        /// The question text as shown (wrapped lines joined with spaces).
        question: String,
        /// The numbered rows, including the free-text row.
        rows: Vec<ScreenRow>,
        /// `Some(cursor_on_it)` when the page has a Submit (or Next) row,
        /// which multi-select pages show below "Type something".
        submit_row: Option<bool>,
        /// 0-based page from Grok's `[N/M]` footer and Codex's "Question
        /// N/M" header, or 0 on Claude's one-question dialog. Claude's
        /// tab strip has no page numbers.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        page_index: Option<usize>,
    },
    /// The "Review your answers" page shown before the answers are sent.
    Review { answers: Vec<ReviewAnswer> },
}

fn row_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"^\s*(❯)?\s*(\d+)\.\s+(?:\[([ ✔✓x])\]\s*)?(.*)$").unwrap())
}

/// The tab strip at the top of the dialog: `←  ☐ Crust  ☐ Toppings  ✔ Submit  →`.
fn is_tab_header(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with('←') && trimmed.contains("✔ Submit")
}

/// The one-question dialog's header: a single `☐ Crust` chip right under
/// the divider. A todo list also draws `☐` lines, so the divider matters.
fn is_single_question_header(lines: &[&str], idx: usize) -> bool {
    let trimmed = lines[idx].trim();
    let is_chip = trimmed
        .strip_prefix(['☐', '☒'])
        .is_some_and(|label| label.starts_with(' ') && !label.trim().is_empty());
    is_chip && !trimmed.contains('←') && idx > 0 && is_divider(lines[idx - 1])
}

fn is_divider(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '─' || c == '━')
}

/// The dialog's key hints, its last line.
fn is_footer(line: &str) -> bool {
    line.trim().starts_with("Enter to select")
}

impl QuestionScreen {
    /// Parse the dialog from the terminal screen. Returns `None` when no
    /// AskUserQuestion dialog is visible.
    pub fn parse(screen_content: &str) -> Option<Self> {
        parse_claude(screen_content)
            .or_else(|| parse_grok(screen_content))
            .or_else(|| parse_codex(screen_content))
    }

    /// The question on a Codex page, for when Codex's log has not named it:
    /// every row but the last, which is the "None of the above" or "Other"
    /// row, or the text field of a question without options.
    pub fn codex_question(&self, view: CodexQuestionView) -> Option<Question> {
        let QuestionScreen::Page { question, rows, .. } = self else {
            return None;
        };
        let options = rows[..rows.len().saturating_sub(1)]
            .iter()
            .map(|row| QuestionOption {
                label: row.label.clone(),
                description: None,
            })
            .collect();
        Some(Question {
            question: question.clone(),
            options,
            allows_other: (view == CodexQuestionView::Sync).then_some(false),
            ..Default::default()
        })
    }
}

/// Which of Codex's question views is on screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CodexQuestionView {
    /// `request_user_input`'s page, headed "Question 1/2 (2 unanswered)".
    Sync,
    /// `request_user_input_async`'s view, opened from the composer with ⇧←.
    Async,
}

/// Which Codex question view is on screen. Its footer must be the last
/// line Codex drew: an answered view is replaced by a summary, so a footer
/// higher up is not a live question.
pub fn codex_question_view(screen_content: &str) -> Option<CodexQuestionView> {
    let stripped = strip_ansi_codes(screen_content);
    let lines: Vec<&str> = stripped.lines().collect();
    codex_view_footer(&lines).map(|(view, _)| view)
}

fn codex_view_footer(lines: &[&str]) -> Option<(CodexQuestionView, usize)> {
    let footer = lines.iter().rposition(|line| !line.trim().is_empty())?;
    let text = lines[footer].trim();
    if text.starts_with("enter submit") {
        return Some((CodexQuestionView::Async, footer));
    }
    if !text.contains("enter to submit answer") && !text.contains("enter to submit all") {
        return None;
    }
    let has_header = lines[..footer]
        .iter()
        .rev()
        .take(40)
        .any(|line| codex_sync_page(line).is_some());
    has_header.then_some((CodexQuestionView::Sync, footer))
}

/// Whether Codex shows async questions waiting under the composer:
///
/// ```text
/// • Queued follow-up inputs
///   ? 2 questions · 12s
///     ⇧← to answer
/// ```
pub fn codex_questions_waiting(screen_content: &str) -> bool {
    let stripped = strip_ansi_codes(screen_content);
    let recent: Vec<&str> = stripped
        .lines()
        .rev()
        .filter(|line| !line.trim().is_empty())
        .take(12)
        .collect();
    recent.windows(2).any(|pair| {
        let (hint, count) = (pair[0].trim(), pair[1].trim());
        hint == "⇧← to answer" && count.starts_with("? ") && count.contains("question")
    })
}

/// 0-based page from the sync view's "Question 2/3 (1 unanswered)" header.
fn codex_sync_page(line: &str) -> Option<usize> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^Question (\d+)/\d+\b").unwrap());
    let n: usize = re.captures(line.trim())?[1].parse().ok()?;
    n.checked_sub(1)
}

/// A numbered Codex row: `› 1. Thin               Crispy`. The sync view
/// puts a description after a run of spaces; keep only the label.
fn parse_codex_row(line: &str) -> Option<ScreenRow> {
    static RE: OnceLock<Regex> = OnceLock::new();
    let re = RE.get_or_init(|| Regex::new(r"^\s*(›)?\s*(\d+)\.\s+(.*)$").unwrap());
    let caps = re.captures(line)?;
    let text = caps[3].trim();
    let label = text.split_once("  ").map_or(text, |(label, _)| label);
    Some(ScreenRow {
        number: caps[2].parse().ok()?,
        checked: None,
        label: label.trim().to_string(),
        cursor: caps.get(1).is_some(),
    })
}

/// Parse whichever Codex question view is on screen.
fn parse_codex(screen_content: &str) -> Option<QuestionScreen> {
    let stripped = strip_ansi_codes(screen_content);
    let lines: Vec<&str> = stripped.lines().collect();
    let (view, footer) = codex_view_footer(&lines)?;
    match view {
        CodexQuestionView::Sync => parse_codex_sync(&lines[..footer]),
        CodexQuestionView::Async => parse_codex_async(&lines[..footer]),
    }
}

/// The sync page: header, question, then rows. A notes line (`› text`
/// without a number) may follow the rows.
fn parse_codex_sync(lines: &[&str]) -> Option<QuestionScreen> {
    let header = lines
        .iter()
        .rposition(|line| codex_sync_page(line).is_some())?;
    let page_index = codex_sync_page(lines[header]);
    let mut question_lines = Vec::new();
    let mut rows = Vec::new();
    for line in &lines[header + 1..] {
        if let Some(row) = parse_codex_row(line) {
            rows.push(row);
        } else if rows.is_empty() && !line.trim().is_empty() {
            question_lines.push(line.trim());
        }
    }
    if rows.is_empty() {
        return None;
    }
    Some(QuestionScreen::Page {
        question: question_lines.join(" "),
        rows,
        submit_row: None,
        page_index,
    })
}

/// The async view, read upward from its footer: the rows (or the text
/// field of a question without options), then the question, which may sit
/// under an "N of M" counter.
fn parse_codex_async(lines: &[&str]) -> Option<QuestionScreen> {
    fn block_above<'a>(lines: &[&'a str], end: &mut usize) -> Vec<&'a str> {
        while *end > 0 && lines[*end - 1].trim().is_empty() {
            *end -= 1;
        }
        let mut block = Vec::new();
        while *end > 0 && !lines[*end - 1].trim().is_empty() {
            *end -= 1;
            block.push(lines[*end].trim());
        }
        block.reverse();
        block
    }

    let mut end = lines.len();
    let answer = block_above(lines, &mut end);
    let mut rows: Vec<ScreenRow> = answer.iter().filter_map(|l| parse_codex_row(l)).collect();
    if rows.is_empty() {
        // A question without options: the whole block is its text field.
        rows.push(ScreenRow {
            number: 1,
            checked: None,
            label: answer.join(" "),
            cursor: true,
        });
    }

    // The question is the end of the block above, after any "N of M"
    // counter or "• Queued follow-up inputs" heading.
    let is_counter = |line: &str| {
        line.split_once(" of ")
            .is_some_and(|(n, m)| n.parse::<u32>().is_ok() && m.parse::<u32>().is_ok())
    };
    let block = block_above(lines, &mut end);
    let start = block
        .iter()
        .rposition(|line| is_counter(line) || line.starts_with('•'))
        .map_or(0, |heading| heading + 1);
    let question = &block[start..];
    if question.is_empty() {
        return None;
    }
    Some(QuestionScreen::Page {
        question: question.join(" "),
        rows,
        submit_row: None,
        // Answered questions leave the view, so its counter counts only
        // the ones left. Match the question by its text instead.
        page_index: None,
    })
}

fn parse_claude(screen_content: &str) -> Option<QuestionScreen> {
    let stripped = strip_ansi_codes(screen_content);
    let lines: Vec<&str> = stripped.lines().collect();

    // The dialog is the last header on screen; earlier ones are scrollback.
    let (header_idx, single_question) = (0..lines.len()).rev().find_map(|i| {
        if is_tab_header(lines[i]) {
            Some((i, false))
        } else {
            is_single_question_header(&lines, i).then_some((i, true))
        }
    })?;
    let body: Vec<&str> = lines[header_idx + 1..]
        .iter()
        .take_while(|line| !line.trim().starts_with('━'))
        .copied()
        .collect();

    if single_question {
        // Without the tab strip, only the key hints below set the dialog
        // apart from other `☐` lines.
        if !body.iter().any(|line| is_footer(line)) {
            return None;
        }
        return parse_page(&body, Some(0));
    }

    if body.iter().any(|line| line.trim() == "Review your answers") {
        return Some(QuestionScreen::Review {
            answers: parse_review_answers(&body),
        });
    }

    parse_page(&body, None)
}

/// Read the review page: `● question` lines, each followed by `→ answer`.
fn parse_review_answers(body: &[&str]) -> Vec<ReviewAnswer> {
    let mut answers: Vec<ReviewAnswer> = Vec::new();
    // A wrapped line continues whichever half of the entry came last.
    let mut wrapping_answer = false;

    for line in body {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed == "Review your answers" {
            continue;
        }
        if trimmed.starts_with("Ready to submit") {
            break;
        }
        if let Some(rest) = trimmed.strip_prefix('●') {
            answers.push(ReviewAnswer {
                question: rest.trim().to_string(),
                answer: String::new(),
            });
            wrapping_answer = false;
        } else if let Some(rest) = trimmed.strip_prefix('→') {
            if let Some(entry) = answers.last_mut() {
                entry.answer = rest.trim().to_string();
                wrapping_answer = true;
            }
        } else if let Some(entry) = answers.last_mut() {
            let wrapped = if wrapping_answer {
                &mut entry.answer
            } else {
                &mut entry.question
            };
            wrapped.push(' ');
            wrapped.push_str(trimmed);
        }
    }
    answers
}

fn parse_page(body: &[&str], page_index: Option<usize>) -> Option<QuestionScreen> {
    let mut question_lines: Vec<&str> = Vec::new();
    let mut rows: Vec<ScreenRow> = Vec::new();
    let mut submit_row: Option<bool> = None;

    for line in body {
        let trimmed = line.trim();
        // The rows end at the divider above "Chat about this" and the footer.
        if is_divider(line) || is_footer(line) {
            break;
        }
        if let Some(caps) = row_regex().captures(line) {
            let Ok(number) = caps[2].parse::<u32>() else {
                continue;
            };
            rows.push(ScreenRow {
                number,
                checked: caps.get(3).map(|m| m.as_str() != " "),
                label: caps[4].trim().to_string(),
                cursor: caps.get(1).is_some(),
            });
            continue;
        }
        // The last row reads "Next" until the final question, then "Submit".
        let without_cursor = trimmed.trim_start_matches('❯').trim();
        if (without_cursor == "Submit" || without_cursor == "Next") && !rows.is_empty() {
            submit_row = Some(trimmed.starts_with('❯'));
            continue;
        }
        if rows.is_empty() && !trimmed.is_empty() {
            question_lines.push(trimmed);
        }
        // Anything else is an option description or a wrapped label; skip it.
    }

    if rows.is_empty() {
        return None;
    }

    Some(QuestionScreen::Page {
        question: question_lines.join(" "),
        rows,
        submit_row,
        page_index,
    })
}

fn grok_option_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| {
        Regex::new(r"^([1-9a-fA-FzZ])\s+(\((?:○|●)\)|\[(?:[xX ])\])\s+(.*)$").unwrap()
    })
}

fn grok_footer_regex() -> &'static Regex {
    static RE: OnceLock<Regex> = OnceLock::new();
    RE.get_or_init(|| Regex::new(r"\[(\d+)/\d+\]").unwrap())
}

fn card_text(line: &str) -> String {
    strip_ansi_codes(line)
        .trim()
        .trim_start_matches('┃')
        .trim()
        .to_string()
}

/// 0-based page from Grok's `[N/M] ↑/↓ navigate · ←/→ question` footer.
/// A single-question card has the same footer without `[N/M]`.
fn grok_footer_page(line: &str) -> Option<usize> {
    if !line.contains("navigate") {
        return None;
    }
    if let Some(caps) = grok_footer_regex().captures(line) {
        let n: usize = caps[1].parse().ok()?;
        return (n >= 1).then_some(n - 1);
    }
    Some(0)
}

fn grok_key_number(key: &str) -> Option<u32> {
    key.chars().next().and_then(|c| match c {
        '1'..='9' | 'a'..='f' | 'A'..='F' => c.to_digit(16),
        _ => None,
    })
}

fn grok_option_label(rest: &str) -> String {
    let rest = rest.trim();
    if let Some(custom) = rest.strip_prefix('❯') {
        return custom.trim().to_string();
    }
    match rest.split_once("  ") {
        Some((label, _)) => label.trim().to_string(),
        None => rest.to_string(),
    }
}

fn parse_grok_option(text: &str, cursor: bool) -> Option<ScreenRow> {
    let caps = grok_option_regex().captures(text)?;
    let key = &caps[1];
    let mark = &caps[2];
    let checked = match mark {
        "(●)" | "[x]" | "[X]" => Some(true),
        "(○)" | "[ ]" => Some(false),
        _ => None,
    };
    Some(ScreenRow {
        number: grok_key_number(key).unwrap_or(0),
        checked,
        label: grok_option_label(&caps[3]),
        cursor,
    })
}

fn is_grok_card_noise(text: &str) -> bool {
    text.starts_with('◆')
        || text.starts_with("Tab:")
        || text.starts_with("Waiting on answers")
        || text.starts_with("Shift+x")
        || text.starts_with("Esc:")
}

/// Grok's question card: radio `(○)`/`(●)`, checkboxes `[ ]`/`[x]`,
/// free-text `z`, and a `[N/M]` page footer.
fn parse_grok(screen_content: &str) -> Option<QuestionScreen> {
    let lines: Vec<&str> = screen_content.lines().collect();
    let texts: Vec<String> = lines.iter().copied().map(card_text).collect();

    let (footer_idx, page_index) = texts
        .iter()
        .enumerate()
        .rev()
        .find_map(|(i, text)| grok_footer_page(text).map(|page| (i, page)))?;

    let mut rows = Vec::new();
    let mut idx = 0;
    for i in (0..footer_idx).rev() {
        idx = i;
        let text = &texts[i];
        if text.is_empty() {
            if !rows.is_empty() {
                break;
            }
            continue;
        }
        if let Some(row) = parse_grok_option(text, lines[i].contains("\x1b[7m")) {
            rows.push(row);
            continue;
        }
        if !rows.is_empty() {
            break;
        }
    }
    rows.reverse();
    if rows.is_empty() {
        return None;
    }

    let mut question_parts = Vec::new();
    for i in (0..=idx).rev() {
        let text = &texts[i];
        if text.is_empty() {
            if !question_parts.is_empty() {
                break;
            }
            continue;
        }
        if parse_grok_option(text, false).is_some()
            || grok_footer_page(text).is_some()
            || is_grok_card_noise(text)
        {
            break;
        }
        question_parts.push(text.as_str());
    }
    question_parts.reverse();

    // `z` parses as number 0. Give it the next row number, and treat a
    // filled free-text row as the cursor: Grok uses reverse-video only
    // while walking options, not while typing.
    let has_cursor = rows.iter().any(|r| r.cursor);
    let mut next = rows.iter().map(|r| r.number).max().unwrap_or(0);
    for row in &mut rows {
        if row.number != 0 {
            continue;
        }
        if !has_cursor && row.checked == Some(true) && !row.is_placeholder() {
            row.cursor = true;
        }
        next += 1;
        row.number = next;
    }

    Some(QuestionScreen::Page {
        question: question_parts.join(" "),
        rows,
        submit_row: None,
        page_index: Some(page_index),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const MULTI_PAGE: &str = "\
❯ Use the AskUserQuestion tool right now.
────────────────────────────────────────
←  ☒ Toppings  ✔ Submit  →
Which toppings do you want on the pizza?
  1. [✔] Cheese
  Classic mozzarella
  2. [ ] Pepperoni
  Spicy slices
  3. [✔] Mushrooms
  Fresh button mushrooms
  4. [ ] Olives
  Black olives
❯ 5. [✔] extra garlic
     Submit
────────────────────────────────────────
  6. Chat about this
Enter to select · ↑/↓ to navigate · ctrl+g to edit in VS Code · Esc to cancel
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
 Streaming 53730e0c » ? « Ask │ e2e ✓ Clean
";

    const SINGLE_PAGE: &str = "\
←  ☐ Crust  ☐ Toppings  ✔ Submit  →
Which crust?
❯ 1. Thin
     Crispy
  2. Thick
     Doughy
  3. Stuffed
     Cheese inside
  4. Type something.
────────────────────────────────────────
  5. Chat about this
Enter to select · Tab/Arrow keys to navigate · Esc to cancel
";

    const REVIEW_PAGE: &str = "\
←  ☒ Crust  ☒ Toppings  ✔ Submit  →
Review your answers
 ● Which crust?
   → Thick
 ● The dormant skip trims ~10%, so credits still run out around Sep 16. How do you want to get to
the Sep 30 renewal?
   → Give individual users a budget so that it lasts to the end of the month
Ready to submit your answers?
❯ 1. Submit answers
  2. Cancel
━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
";

    #[test]
    fn parses_multi_select_page() {
        let screen = QuestionScreen::parse(MULTI_PAGE).expect("dialog");
        let QuestionScreen::Page {
            question,
            rows,
            submit_row,
            page_index,
        } = screen
        else {
            panic!("expected a page");
        };
        assert_eq!(page_index, None);
        assert_eq!(question, "Which toppings do you want on the pizza?");
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].checked, Some(true));
        assert_eq!(rows[1].checked, Some(false));
        assert_eq!(rows[2].label, "Mushrooms");
        assert!(rows[4].cursor);
        assert_eq!(rows[4].label, "extra garlic");
        assert!(!rows[4].is_placeholder());
        assert_eq!(submit_row, Some(false));
        assert!(rows[..4].iter().all(|r| !r.cursor));
    }

    #[test]
    fn parses_cursor_on_submit_row() {
        let screen = MULTI_PAGE
            .replace("❯ 5. [✔] extra garlic", "  5. [ ] Type something")
            .replace("     Submit", "❯    Submit");
        let QuestionScreen::Page {
            rows,
            submit_row,
            page_index,
            ..
        } = QuestionScreen::parse(&screen).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(submit_row, Some(true));
        assert_eq!(page_index, None);
        assert!(rows.iter().all(|r| !r.cursor));
        assert!(rows[4].is_placeholder());
        assert_eq!(rows[4].checked, Some(false));
    }

    #[test]
    fn parses_next_row_before_the_last_question() {
        let screen = MULTI_PAGE
            .replace("☒ Toppings  ✔ Submit", "☐ Toppings  ☐ Crust  ✔ Submit")
            .replace("     Submit", "❯    Next")
            .replace("❯ 5. [✔] extra garlic", "  5. [ ] Type something");
        let QuestionScreen::Page { submit_row, .. } =
            QuestionScreen::parse(&screen).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(submit_row, Some(true));
    }

    #[test]
    fn parses_single_select_page() {
        let QuestionScreen::Page {
            question,
            rows,
            submit_row,
            page_index,
        } = QuestionScreen::parse(SINGLE_PAGE).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(page_index, None);
        assert_eq!(question, "Which crust?");
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|r| r.checked.is_none()));
        assert!(rows[0].cursor);
        assert_eq!(rows[3].label, "Type something.");
        assert!(rows[3].is_placeholder());
        assert_eq!(submit_row, None);
    }

    /// Real screens of a one-question dialog (Claude Code 2.1.301), with
    /// the terminal's escape codes, after Down moved the cursor.
    const ONE_QUESTION_ON_THIN: &str = include_str!("testdata/one_question_cursor_on_thin.txt");
    const ONE_QUESTION_ON_THICK: &str = include_str!("testdata/one_question_cursor_on_thick.txt");
    const ONE_QUESTION_ON_TYPE_SOMETHING: &str =
        include_str!("testdata/one_question_cursor_on_type_something.txt");

    #[test]
    fn follows_the_cursor_on_a_one_question_dialog() {
        for (screen, cursor_index) in [
            (ONE_QUESTION_ON_THIN, 0),
            (ONE_QUESTION_ON_THICK, 1),
            (ONE_QUESTION_ON_TYPE_SOMETHING, 3),
        ] {
            let Some(QuestionScreen::Page {
                question,
                rows,
                submit_row,
                page_index,
            }) = QuestionScreen::parse(screen)
            else {
                panic!(
                    "expected a page with the cursor on row {}",
                    cursor_index + 1
                );
            };
            assert_eq!(question, "Which crust?");
            assert_eq!(page_index, Some(0), "one question is always page 0");
            assert_eq!(submit_row, None);
            let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
            assert_eq!(labels, ["Thin", "Thick", "Stuffed", "Type something."]);
            assert!(rows.iter().all(|r| r.checked.is_none()));
            assert!(rows[3].is_placeholder());
            let cursors: Vec<usize> = (0..rows.len()).filter(|&i| rows[i].cursor).collect();
            assert_eq!(cursors, [cursor_index]);
        }
    }

    #[test]
    fn needs_the_key_hints_to_read_a_lone_header() {
        // A todo list draws `☐` lines too, and a permission menu numbers its rows.
        let todo = "\
────────────────────────────────────────
 ☐ Write the parser
  1. Yes
  2. No
";
        assert_eq!(QuestionScreen::parse(todo), None);
        let without_divider = ONE_QUESTION_ON_THICK.replacen('─', "x", 1);
        assert_eq!(QuestionScreen::parse(&without_divider), None);
    }

    #[test]
    fn parses_review_page() {
        let QuestionScreen::Review { answers } =
            QuestionScreen::parse(REVIEW_PAGE).expect("dialog")
        else {
            panic!("expected the review page");
        };
        assert_eq!(answers.len(), 2);
        assert_eq!(answers[0].question, "Which crust?");
        assert_eq!(answers[0].answer, "Thick");
        assert!(answers[1].question.ends_with("get to the Sep 30 renewal?"));
        assert!(answers[1].answer.starts_with("Give individual users"));
    }

    #[test]
    fn ignores_screens_without_the_dialog() {
        assert_eq!(QuestionScreen::parse("❯ \n  ⏵⏵ auto mode on\n"), None);
        assert_eq!(
            QuestionScreen::parse("Do you want to proceed?\n❯ 1. Yes\n  2. No\n"),
            None
        );
    }

    #[test]
    fn uses_the_last_dialog_on_screen() {
        let screen = format!("{SINGLE_PAGE}\n⏺ User answered\n{MULTI_PAGE}");
        let QuestionScreen::Page { question, .. } = QuestionScreen::parse(&screen).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(question, "Which toppings do you want on the pizza?");
    }

    #[test]
    fn strips_ansi_before_parsing() {
        let screen = "\x1b[36m←\x1b[m  ☐ Crust  \x1b[1m✔ Submit\x1b[m  →\nWhich crust?\n\x1b[36m❯\x1b[m 1. Thin\n  2. Thick\n";
        let QuestionScreen::Page { rows, .. } = QuestionScreen::parse(screen).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(rows.len(), 2);
        assert!(rows[0].cursor);
    }

    const GROK_RADIO: &str = "\
┃
┃  Which pizza crust do you want?
┃
┃  1 (○) Thin (crispy)            Thin (crispy)
┃  \x1b[7m2 (●) Thick (doughy)         \x1b[m  Thick (doughy)
┃  3 (○) Stuffed (cheese inside)  Stuffed (cheese inside)
┃  z (○) Type your answer here
┃
┃  [1/4] ↑/↓ navigate · ←/→ question · y copy                   Enter:select
";

    const GROK_CHECKBOX: &str = "\
┃
┃  Which toppings do you want?
┃
┃  1 [x] Cheese     Cheese
┃  \x1b[7m2 [x] Pepperoni\x1b[m  Pepperoni
┃  3 [ ] Mushrooms  Mushrooms
┃  4 [ ] Olives     Olives
┃  z [ ] Type your answer here
┃
┃  [2/4] ↑/↓ navigate · ←/→ question · y copy                   Enter:select
";

    const GROK_CUSTOM: &str = "\
┃
┃  When should we bake it?
┃
┃  1 (○) Now            Now
┃  2 (○) Later tonight  Later tonight
┃  3 (○) Never          Never
┃  z (●) ❯ extra garlic
┃
┃  [3/4] ↑/↓ navigate · ←/→ question · y copy                   Enter:edit
";

    fn grok_page(screen: &str) -> (String, Vec<ScreenRow>, Option<usize>) {
        let QuestionScreen::Page {
            question,
            rows,
            submit_row,
            page_index,
        } = QuestionScreen::parse(screen).expect("grok card")
        else {
            panic!("expected a page");
        };
        assert_eq!(submit_row, None);
        (question, rows, page_index)
    }

    #[test]
    fn parses_grok_radio_page() {
        let (question, rows, page_index) = grok_page(GROK_RADIO);
        assert_eq!(question, "Which pizza crust do you want?");
        assert_eq!(page_index, Some(0));
        assert_eq!(rows.len(), 4);
        assert_eq!(rows[0].checked, Some(false));
        assert_eq!(rows[1].checked, Some(true));
        assert!(rows[1].cursor);
        assert_eq!(rows[1].label, "Thick (doughy)");
        assert!(rows[3].is_placeholder());
        assert_eq!(rows[3].number, 4);
        assert!(rows.iter().enumerate().all(|(i, r)| r.cursor == (i == 1)));
    }

    #[test]
    fn parses_grok_checkbox_page() {
        let (question, rows, page_index) = grok_page(GROK_CHECKBOX);
        assert_eq!(question, "Which toppings do you want?");
        assert_eq!(page_index, Some(1));
        assert_eq!(rows.len(), 5);
        assert_eq!(rows[0].checked, Some(true));
        assert_eq!(rows[1].checked, Some(true));
        assert!(rows[1].cursor);
        assert_eq!(rows[2].checked, Some(false));
        assert_eq!(rows[3].label, "Olives");
        assert!(rows[4].is_placeholder());
    }

    #[test]
    fn parses_grok_custom_text() {
        let (question, rows, page_index) = grok_page(GROK_CUSTOM);
        assert_eq!(question, "When should we bake it?");
        assert_eq!(page_index, Some(2));
        assert_eq!(rows[3].label, "extra garlic");
        assert_eq!(rows[3].checked, Some(true));
        assert!(rows[3].cursor);
        assert!(!rows[3].is_placeholder());
    }

    #[test]
    fn grok_parse_ignores_screens_without_a_card() {
        assert_eq!(QuestionScreen::parse("Which pizza crust?\n1. Thin\n"), None);
    }

    const GROK_SINGLE: &str = "\
┃
┃  What is your favorite color?
┃
┃  1 (○) Red    Red
┃  2 (○) Blue   Blue
┃  3 (○) Green  Green
┃  z (●) ❯ periwinkle
┃
┃  ↑/↓ navigate · y copy                                  Enter:edit
";

    /// The last page of a two-question `request_user_input` call.
    const CODEX_SYNC_LAST: &str = "\
  Question 2/2 (1 unanswered)
  Which cheese?

  › 1. Mozzarella         Classic
    2. Cheddar            Sharp
    3. None of the above  Optionally, add details in notes (tab)

  tab to add notes | enter to submit all | ←/→ to navigate questions | esc to interrupt
";

    /// Notes open on the first page.
    const CODEX_SYNC_NOTES: &str = "\
  Question 1/2 (2 unanswered)
  Which crust?

    1. Thin               Crispy
    2. Thick              Doughy
  › 3. None of the above  Optionally, add details in notes (tab)

  › gluten free please

  tab or esc to clear notes | enter to submit answer
";

    /// `request_user_input_async` questions opened with ⇧←.
    const CODEX_ASYNC: &str = "\
• Working (22s • esc to interrupt) · 1 background terminal running · /ps to view · /stop to close

• Queued follow-up inputs

  1 of 2
  Which pizza size?

  › 1. Small
    2. Medium
    3. Large
    4. Other

  enter submit   ⌃] skip   ⇧→ main prompt   ⇧← next question


";

    /// An async question without options.
    const CODEX_ASYNC_TEXT: &str = "\
• Queued follow-up inputs

  2 of 2

  Any delivery notes?

  Type your answer

  enter submit   ⌃] skip   ⇧→ prev question
";

    fn codex_page(screen: &str) -> (String, Vec<ScreenRow>, Option<usize>) {
        let QuestionScreen::Page {
            question,
            rows,
            page_index,
            ..
        } = QuestionScreen::parse(screen).expect("codex question")
        else {
            panic!("expected a page");
        };
        (question, rows, page_index)
    }

    #[test]
    fn parses_codex_sync_pages() {
        assert_eq!(
            codex_question_view(CODEX_SYNC_LAST),
            Some(CodexQuestionView::Sync)
        );
        let (question, rows, page_index) = codex_page(CODEX_SYNC_LAST);
        assert_eq!(question, "Which cheese?");
        assert_eq!(page_index, Some(1));
        let labels: Vec<&str> = rows.iter().map(|r| r.label.as_str()).collect();
        assert_eq!(labels, ["Mozzarella", "Cheddar", "None of the above"]);
        assert!(rows[0].cursor && !rows[1].cursor);
        assert!(rows[2].is_placeholder());

        // The notes line is not a row, and its › is not the row cursor.
        let (question, rows, page_index) = codex_page(CODEX_SYNC_NOTES);
        assert_eq!(question, "Which crust?");
        assert_eq!(page_index, Some(0));
        assert_eq!(rows.len(), 3);
        assert!(rows[2].cursor && !rows[0].cursor);
    }

    #[test]
    fn parses_codex_async_pages() {
        assert_eq!(
            codex_question_view(CODEX_ASYNC),
            Some(CodexQuestionView::Async)
        );
        let (question, rows, page_index) = codex_page(CODEX_ASYNC);
        assert_eq!(question, "Which pizza size?");
        assert_eq!(page_index, None, "the counter skips answered questions");
        assert_eq!(rows.len(), 4);
        assert!(rows[0].cursor);
        assert!(rows[3].is_placeholder());

        let typed = CODEX_ASYNC
            .replace("  › 1. Small", "    1. Small")
            .replace("    4. Other", "  › 4. extra large");
        let (_, rows, _) = codex_page(&typed);
        assert_eq!(rows[3].label, "extra large");
        assert!(rows[3].cursor && !rows[3].is_placeholder());

        let (question, rows, _) = codex_page(CODEX_ASYNC_TEXT);
        assert_eq!(question, "Any delivery notes?");
        assert_eq!(rows.len(), 1);
        assert!(rows[0].cursor && rows[0].is_placeholder());
        let (_, rows, _) =
            codex_page(&CODEX_ASYNC_TEXT.replace("Type your answer", "ring the bell"));
        assert_eq!(rows[0].label, "ring the bell");
    }

    #[test]
    fn reads_the_question_from_a_codex_page_without_a_log_entry() {
        let page = QuestionScreen::parse(CODEX_ASYNC).unwrap();
        let question = page.codex_question(CodexQuestionView::Async).unwrap();
        assert_eq!(question.question, "Which pizza size?");
        let labels: Vec<&str> = question.options.iter().map(|o| o.label.as_str()).collect();
        assert_eq!(labels, ["Small", "Medium", "Large"]);
        assert_eq!(question.allows_other, None);

        let page = QuestionScreen::parse(CODEX_SYNC_LAST).unwrap();
        let question = page.codex_question(CodexQuestionView::Sync).unwrap();
        assert_eq!(question.options.len(), 2);
        assert_eq!(question.allows_other, Some(false));
    }

    #[test]
    fn sees_codex_questions_waiting_under_the_composer() {
        let waiting = "\
• Working (21s • esc to interrupt) · 1 background terminal running · /ps to view · /stop to close

• Queued follow-up inputs
  ? 2 questions · 12s
    ⇧← to answer

› Ask Codex to do anything

  GPT-6-Astra high · ~/scratch · ← for agents
";
        assert!(codex_questions_waiting(waiting));
        assert_eq!(codex_question_view(waiting), None);
        assert_eq!(QuestionScreen::parse(waiting), None);
        assert!(!codex_questions_waiting("› Ask Codex to do anything\n"));
    }

    #[test]
    fn ignores_answered_codex_questions() {
        let answered = "\
  Question 1/1 (1 unanswered)
• Questions 2/2 answered
  • Which cheese?
    answer: Cheddar

• Working (5s • esc to interrupt)

› Ask Codex to do anything
";
        assert_eq!(codex_question_view(answered), None);
        assert_eq!(QuestionScreen::parse(answered), None);
    }

    #[test]
    fn parses_grok_single_question_without_page_counter() {
        let (question, rows, page_index) = grok_page(GROK_SINGLE);
        assert_eq!(question, "What is your favorite color?");
        assert_eq!(page_index, Some(0));
        assert_eq!(rows[3].label, "periwinkle");
        assert_eq!(rows[3].checked, Some(true));
        assert!(rows[3].cursor);
        assert!(!rows[3].is_placeholder());
    }
}
