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

use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::OnceLock;

use super::permission_prompt::strip_ansi_codes;

/// Placeholder shown on the free-text row before the user types anything.
const TYPE_SOMETHING: &str = "Type something";

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
        self.label.trim_end_matches('.').trim() == TYPE_SOMETHING
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

fn is_divider(line: &str) -> bool {
    let trimmed = line.trim();
    !trimmed.is_empty() && trimmed.chars().all(|c| c == '─' || c == '━')
}

impl QuestionScreen {
    /// Parse the dialog from the terminal screen. Returns `None` when no
    /// AskUserQuestion dialog is visible.
    pub fn parse(screen_content: &str) -> Option<Self> {
        let stripped = strip_ansi_codes(screen_content);
        let lines: Vec<&str> = stripped.lines().collect();

        // The dialog is the last tab strip on screen; earlier ones are scrollback.
        let header_idx = lines.iter().rposition(|line| is_tab_header(line))?;
        let body: Vec<&str> = lines[header_idx + 1..]
            .iter()
            .take_while(|line| !line.trim().starts_with('━'))
            .copied()
            .collect();

        if body.iter().any(|line| line.trim() == "Review your answers") {
            return Some(Self::Review {
                answers: parse_review_answers(&body),
            });
        }

        parse_page(&body)
    }
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

fn parse_page(body: &[&str]) -> Option<QuestionScreen> {
    let mut question_lines: Vec<&str> = Vec::new();
    let mut rows: Vec<ScreenRow> = Vec::new();
    let mut submit_row: Option<bool> = None;

    for line in body {
        let trimmed = line.trim();
        // The rows end at the divider above "Chat about this" and the footer.
        if is_divider(line) || trimmed.starts_with("Enter to select") {
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
        } = screen
        else {
            panic!("expected a page");
        };
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
            rows, submit_row, ..
        } = QuestionScreen::parse(&screen).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(submit_row, Some(true));
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
        } = QuestionScreen::parse(SINGLE_PAGE).expect("dialog")
        else {
            panic!("expected a page");
        };
        assert_eq!(question, "Which crust?");
        assert_eq!(rows.len(), 4);
        assert!(rows.iter().all(|r| r.checked.is_none()));
        assert!(rows[0].cursor);
        assert_eq!(rows[3].label, "Type something.");
        assert!(rows[3].is_placeholder());
        assert_eq!(submit_row, None);
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
}
