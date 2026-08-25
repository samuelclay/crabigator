//! Cooldown tints for the PR board.
//!
//! When a session's state badge or one of a PR's GitHub status cells changes,
//! that cell lights up bright purple and fades over two minutes to a barely
//! visible purple-gray, then back to plain. One hue, so a glance at the board
//! reads simply: the brighter the purple, the more recent the change.
//! A session entering the thinking state stays quiet (that is the user's own
//! prompt at work), and the unresolved-comment count only glows when it
//! rises.

use std::collections::HashMap;

use crate::terminal::escape::{bg_rgb, fg_rgb, RESET_BG, RESET_FG};

/// How long a changed cell stays tinted.
pub(crate) const COOLDOWN_MS: u64 = 120_000;

/// How many distinct shades the cooldown steps through.
const SHADES: u64 = 128;

/// One step of the cooldown: a truecolor background and a foreground that
/// stays legible on it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Tint {
    pub(crate) bg: (u8, u8, u8),
    pub(crate) fg: (u8, u8, u8),
}

/// Fade anchors, brightest first; the shades interpolate between them. One
/// purple from bright to a near-background purple-gray, so brightness alone
/// says how recent the change is.
const STOPS: &[(u8, u8, u8)] = &[
    (214, 178, 255), // bright purple
    (175, 135, 255), // the board's PR purple
    (135, 95, 215),  // mid purple
    (95, 65, 150),   // dim purple
    (55, 45, 80),    // ember
];

/// The tint for a change `age_ms` old; `None` once the cooldown has run out.
pub(crate) fn tint_for_age(age_ms: u64) -> Option<Tint> {
    if age_ms >= COOLDOWN_MS {
        return None;
    }
    let step = age_ms * SHADES / COOLDOWN_MS;
    // Walk the gradient: which pair of anchors this shade falls between, and
    // how far along it sits.
    let along = step as f32 / (SHADES - 1) as f32 * (STOPS.len() - 1) as f32;
    let segment = (along as usize).min(STOPS.len() - 2);
    let fraction = along - segment as f32;
    let (from, to) = (STOPS[segment], STOPS[segment + 1]);
    let mix = |a: u8, b: u8| (f32::from(a) + (f32::from(b) - f32::from(a)) * fraction) as u8;
    let bg = (mix(from.0, to.0), mix(from.1, to.1), mix(from.2, to.2));
    // Dark text on the bright end, light text once the shade dims.
    let luma = 0.299 * f32::from(bg.0) + 0.587 * f32::from(bg.1) + 0.114 * f32::from(bg.2);
    let fg = if luma >= 150.0 {
        (0, 0, 0)
    } else {
        (255, 255, 255)
    };
    Some(Tint { bg, fg })
}

/// Paint `text` on the tint, leaving the row's colors untouched afterwards.
pub(crate) fn tint_text(text: &str, tint: Tint) -> String {
    format!(
        "{}{}{text}{RESET_BG}{RESET_FG}",
        bg_rgb(tint.bg),
        fg_rgb(tint.fg)
    )
}

/// Tints for a PR row's GitHub status cells.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(crate) struct StatusTints {
    pub(crate) state: Option<Tint>,
    pub(crate) ci: Option<Tint>,
    pub(crate) comments: Option<Tint>,
    pub(crate) review: Option<Tint>,
    pub(crate) merge: Option<Tint>,
}

struct Observed {
    signature: String,
    /// Unix ms of the last change; 0 for a cell seen only in one state, so
    /// the board's first frame does not light up every cell.
    changed_at_ms: u64,
}

/// Remembers what each watched cell last showed and when it changed.
#[derive(Default)]
pub(crate) struct Cooldowns {
    seen: HashMap<String, Observed>,
}

impl Cooldowns {
    /// Record what a cell shows now. A cell seen for the first time starts
    /// cold; a cell whose text differs from last time starts a cooldown.
    pub(crate) fn observe(&mut self, key: String, signature: &str, now_ms: u64) {
        self.record(key, signature, now_ms);
    }

    /// Record a change that should not glow — a session entering the thinking
    /// state is the user's own prompt, not news. Any active glow goes out.
    pub(crate) fn observe_quiet(&mut self, key: String, signature: &str) {
        self.record(key, signature, 0);
    }

    /// Record a count that only glows when it rises — comment threads getting
    /// resolved is quiet; new unresolved ones light up.
    pub(crate) fn observe_counter(&mut self, key: String, count: i64, now_ms: u64) {
        let rose = self.seen.get(&key).is_some_and(|observed| {
            observed
                .signature
                .parse::<i64>()
                .is_ok_and(|old| count > old)
        });
        self.record(key, &count.to_string(), if rose { now_ms } else { 0 });
    }

    fn record(&mut self, key: String, signature: &str, changed_at_ms: u64) {
        // A cell seen for the first time lands cold, so only a signature that
        // differs from the one already stored starts a cooldown.
        let observed = self.seen.entry(key).or_insert_with(|| Observed {
            signature: signature.to_string(),
            changed_at_ms: 0,
        });
        if observed.signature != signature {
            observed.signature = signature.to_string();
            observed.changed_at_ms = changed_at_ms;
        }
    }

    /// When the cell last changed; `None` for an unknown cell or one only
    /// ever seen in a single state.
    fn changed_at(&self, key: &str) -> Option<u64> {
        let observed = self.seen.get(key)?;
        (observed.changed_at_ms > 0).then_some(observed.changed_at_ms)
    }

    /// The cell's current tint, if it changed within the cooldown.
    pub(crate) fn tint(&self, key: &str, now_ms: u64) -> Option<Tint> {
        tint_for_age(now_ms.saturating_sub(self.changed_at(key)?))
    }

    /// The hottest tint across several cells — for a badge that stands for
    /// more than one session.
    pub(crate) fn hottest_tint<I>(&self, keys: I, now_ms: u64) -> Option<Tint>
    where
        I: IntoIterator<Item = String>,
    {
        let changed_at = keys
            .into_iter()
            .filter_map(|key| self.changed_at(&key))
            .max()?;
        tint_for_age(now_ms.saturating_sub(changed_at))
    }

    /// Whether any cell is still cooling, so the board keeps repainting.
    pub(crate) fn active(&self, now_ms: u64) -> bool {
        self.seen.values().any(|observed| {
            observed.changed_at_ms > 0
                && now_ms.saturating_sub(observed.changed_at_ms) < COOLDOWN_MS
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_change_glows_hottest_then_cools_and_fades() {
        let mut cooldowns = Cooldowns::default();
        cooldowns.observe("s".to_string(), "ready", 1_000);
        assert_eq!(
            cooldowns.tint("s", 1_000),
            None,
            "first sight is not a change"
        );
        assert!(!cooldowns.active(1_000));

        cooldowns.observe("s".to_string(), "ready", 5_000);
        assert_eq!(cooldowns.tint("s", 5_000), None, "same state, still cold");

        cooldowns.observe("s".to_string(), "complete", 10_000);
        let hot = cooldowns
            .tint("s", 10_000)
            .expect("a change starts the cooldown");
        assert_eq!(hot.bg, STOPS[0]);
        assert!(cooldowns.active(10_000));

        let warm = cooldowns.tint("s", 10_000 + COOLDOWN_MS / 2).unwrap();
        assert_ne!(warm, hot, "the tint moves along the gradient");
        let ember = cooldowns.tint("s", 10_000 + COOLDOWN_MS - 1).unwrap();
        assert_eq!(
            ember.bg,
            *STOPS.last().unwrap(),
            "the last shade is the dimmest"
        );

        assert_eq!(
            cooldowns.tint("s", 10_000 + COOLDOWN_MS),
            None,
            "then it fades out"
        );
        assert!(!cooldowns.active(10_000 + COOLDOWN_MS));
        assert_eq!(cooldowns.tint("unknown", 10_000), None);
    }

    #[test]
    fn a_shared_badge_takes_the_freshest_change() {
        let mut cooldowns = Cooldowns::default();
        for key in ["a", "b"] {
            cooldowns.observe(key.to_string(), "ready", 0);
        }
        cooldowns.observe("a".to_string(), "interrupted", 1_000);
        cooldowns.observe("b".to_string(), "complete", 31_000);
        let keys = || ["a".to_string(), "b".to_string()];
        assert_eq!(
            cooldowns.hottest_tint(keys(), 31_000).map(|tint| tint.bg),
            Some(STOPS[0]),
            "b just changed"
        );
        assert_eq!(
            cooldowns.hottest_tint(keys(), 31_000 + COOLDOWN_MS),
            None,
            "both cooled off"
        );
    }

    #[test]
    fn the_gradient_covers_the_whole_cooldown_in_many_shades() {
        assert_eq!(tint_for_age(0).unwrap().bg, STOPS[0]);
        assert_eq!(
            tint_for_age(COOLDOWN_MS - 1).unwrap().bg,
            *STOPS.last().unwrap()
        );
        assert_eq!(tint_for_age(COOLDOWN_MS), None);
        let step_ms = COOLDOWN_MS / SHADES;
        let shades: Vec<_> = (0..SHADES)
            .map(|step| tint_for_age(step * step_ms).unwrap())
            .collect();
        let mut distinct = shades.clone();
        distinct.dedup();
        assert!(
            distinct.len() > 100,
            "the gradient really has ~{SHADES} shades, not a handful: {}",
            distinct.len()
        );
        assert!(
            shades.iter().take(20).all(|tint| tint.fg == (0, 0, 0)),
            "dark text on the bright end"
        );
        assert!(
            shades
                .iter()
                .rev()
                .take(20)
                .all(|tint| tint.fg == (255, 255, 255)),
            "light text on the dim end"
        );
    }

    #[test]
    fn thinking_updates_are_quiet_and_comment_counts_glow_only_upward() {
        let mut cooldowns = Cooldowns::default();
        cooldowns.observe("s".to_string(), "Complete", 0);
        cooldowns.observe("s".to_string(), "Question", 1_000);
        assert!(cooldowns.tint("s", 1_000).is_some());
        cooldowns.observe_quiet("s".to_string(), "Thinking");
        assert_eq!(
            cooldowns.tint("s", 1_000),
            None,
            "entering thinking puts the glow out"
        );
        cooldowns.observe("s".to_string(), "Complete", 2_000);
        assert!(
            cooldowns.tint("s", 2_000).is_some(),
            "finishing glows again"
        );

        cooldowns.observe_counter("c".to_string(), 4, 3_000);
        assert_eq!(cooldowns.tint("c", 3_000), None, "first sight stays cold");
        cooldowns.observe_counter("c".to_string(), 2, 4_000);
        assert_eq!(
            cooldowns.tint("c", 4_000),
            None,
            "threads getting resolved is quiet"
        );
        cooldowns.observe_counter("c".to_string(), 5, 5_000);
        assert!(
            cooldowns.tint("c", 5_000).is_some(),
            "new unresolved threads light up"
        );
    }
}
