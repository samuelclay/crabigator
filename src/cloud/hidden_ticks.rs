//! Ticks on rows of a question page that have scrolled above the screen.
//!
//! Claude Code draws only the part of a tall page that fits. Once a row is
//! above the screen it is never drawn again, not even when it is ticked,
//! and the terminal's scrollback keeps the row as it looked when it left.
//! A digit key still ticks the row, so we follow those keys instead: every
//! key for the dialog passes through crabigator, typed here or sent from a
//! phone. The screen stays the authority for every row it shows.

use std::collections::BTreeSet;

use super::events::shown_question;
use crate::parsers::question_prompt::ScreenRow;
use crate::parsers::QuestionScreen;
use crate::platforms::Question;

/// The page the dialog showed when we last looked.
#[derive(Debug, PartialEq, Eq)]
struct Page {
    question: usize,
    labels: Vec<String>,
    /// Option rows above the screen.
    hidden: Vec<u32>,
    /// Digits type into the free-text row while the cursor is on it.
    cursor_on_text_row: bool,
}

#[derive(Debug, Default)]
pub struct HiddenTicks {
    /// The dialog these ticks belong to, by its questions.
    dialog: Vec<String>,
    /// `(question, row)` of each hidden row we believe is ticked.
    ticked: BTreeSet<(usize, u32)>,
    page: Option<Page>,
}

impl HiddenTicks {
    /// Take in the dialog as the screen shows it now. Rows on screen
    /// replace what we believed about them.
    pub fn see(&mut self, questions: &[Question], screen: Option<&QuestionScreen>) {
        let dialog: Vec<String> = questions.iter().map(|q| q.question.clone()).collect();
        if dialog != self.dialog {
            *self = Self {
                dialog,
                ..Self::default()
            };
        }
        self.page = None;
        let Some(QuestionScreen::Page { rows, .. }) = screen else {
            return;
        };
        let Some(question) = shown_question(questions, screen) else {
            return;
        };
        let q = &questions[question];
        if !q.multi_select {
            return;
        }
        let options = q.options.len() as u32;
        for row in rows.iter().filter(|row| row.number <= options) {
            if row.checked == Some(true) {
                self.ticked.insert((question, row.number));
            } else {
                self.ticked.remove(&(question, row.number));
            }
        }
        self.page = Some(Page {
            question,
            labels: q.options.iter().map(|o| o.label.clone()).collect(),
            hidden: (1..=options)
                .filter(|n| !rows.iter().any(|row| row.number == *n))
                .collect(),
            cursor_on_text_row: rows
                .iter()
                .any(|row| row.cursor && row.number == options + 1),
        });
    }

    /// Note a key on its way to the dialog. Returns true when it ticked or
    /// unticked a hidden row, which the screen will not show.
    pub fn key(&mut self, bytes: &[u8]) -> bool {
        let (Some(page), [digit @ b'1'..=b'9']) = (&self.page, bytes) else {
            return false;
        };
        let row = u32::from(digit - b'0');
        if page.cursor_on_text_row || !page.hidden.contains(&row) {
            return false;
        }
        let tick = (page.question, row);
        if !self.ticked.remove(&tick) {
            self.ticked.insert(tick);
        }
        true
    }

    /// The page with its hidden ticked rows put back above the rows on screen.
    pub fn fill(&self, screen: &mut QuestionScreen) {
        let (Some(page), QuestionScreen::Page { rows, .. }) = (&self.page, screen) else {
            return;
        };
        let hidden = page
            .hidden
            .iter()
            .filter(|row| self.ticked.contains(&(page.question, **row)))
            .map(|&number| ScreenRow {
                number,
                checked: Some(true),
                label: page.labels[number as usize - 1].clone(),
                cursor: false,
            });
        rows.splice(0..0, hidden);
        rows.sort_by_key(|row| row.number);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::platforms::QuestionOption;

    fn questions() -> Vec<Question> {
        let option = |label: &str| QuestionOption {
            preview: None,
            label: label.to_string(),
            description: None,
        };
        vec![Question {
            question: "Which toppings?".to_string(),
            options: vec![option("Cheese"), option("Olives"), option("Mushrooms")],
            multi_select: true,
            ..Default::default()
        }]
    }

    /// The page with rows 1 and 2 above the screen.
    const CUT_OFF: &str = "\
  3. [ ] Mushrooms
  4. [ ] Type something
     Submit
────────────
  5. Chat about this

Enter to select · Tab/Arrow keys to navigate · Esc to cancel
";

    fn ticked(ticks: &HiddenTicks, screen: &str) -> Vec<u32> {
        let mut page = QuestionScreen::parse(screen).unwrap();
        ticks.fill(&mut page);
        let QuestionScreen::Page { rows, .. } = page else {
            panic!("expected a page");
        };
        rows.iter()
            .filter(|row| row.checked == Some(true))
            .map(|row| row.number)
            .collect()
    }

    #[test]
    fn follows_digit_keys_onto_rows_above_the_screen() {
        let mut ticks = HiddenTicks::default();
        ticks.see(&questions(), QuestionScreen::parse(CUT_OFF).as_ref());
        assert!(ticks.key(b"1"));
        assert!(ticks.key(b"2"));
        assert!(ticks.key(b"2"));
        assert_eq!(ticked(&ticks, CUT_OFF), [1]);
        // Row 3 is on screen, and row 4 is the text row: the screen shows those.
        assert!(!ticks.key(b"3"));
        assert!(!ticks.key(b"4"));
        // Arrows, pasted text and several digits at once are not a tick.
        assert!(!ticks.key(b"\x1b[B"));
        assert!(!ticks.key(b"12"));
    }

    #[test]
    fn the_screen_wins_for_rows_it_shows() {
        let mut ticks = HiddenTicks::default();
        ticks.see(&questions(), QuestionScreen::parse(CUT_OFF).as_ref());
        assert!(ticks.key(b"1"));
        // The page fits again and shows row 1 unticked.
        let whole = format!("  1. [ ] Cheese\n  2. [✔] Olives\n{CUT_OFF}");
        ticks.see(&questions(), QuestionScreen::parse(&whole).as_ref());
        ticks.see(&questions(), QuestionScreen::parse(CUT_OFF).as_ref());
        assert_eq!(ticked(&ticks, CUT_OFF), [2]);
    }

    #[test]
    fn digits_type_text_while_the_cursor_is_on_the_text_row() {
        let mut ticks = HiddenTicks::default();
        let typing = CUT_OFF.replace("  4. [ ] Type", "❯ 4. [ ] Type");
        ticks.see(&questions(), QuestionScreen::parse(&typing).as_ref());
        assert!(!ticks.key(b"1"));
    }

    #[test]
    fn forgets_the_ticks_of_an_earlier_dialog() {
        let mut ticks = HiddenTicks::default();
        ticks.see(&questions(), QuestionScreen::parse(CUT_OFF).as_ref());
        assert!(ticks.key(b"1"));
        let mut next = questions();
        next[0].question = "Which sauces?".to_string();
        ticks.see(&next, QuestionScreen::parse(CUT_OFF).as_ref());
        ticks.see(&questions(), QuestionScreen::parse(CUT_OFF).as_ref());
        assert!(ticked(&ticks, CUT_OFF).is_empty());
    }
}
