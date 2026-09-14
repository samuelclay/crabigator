//! Per-session identity chip: a short drawing with one foreground and one
//! background color, stable for the life of a session.
//!
//! The preferred glyph is hashed from the local crabigator session id, then
//! walked forward so two live sessions never share a drawing until every
//! drawing is already in use (more than 30 live sessions).

use std::collections::HashSet;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use unicode_width::UnicodeWidthStr;

use crate::terminal::escape::{bg_rgb, fg_rgb, RESET};

/// Mirrors the PR board's stale cutoff: inspect.json older than this is not
/// a live session for occupancy.
const LIVE_SESSION_SECS: f64 = 300.0;
/// One-column chip padding. Regular spaces lose their background at the end
/// of a color run; NBSP does not.
const PAD: char = '\u{00A0}';

/// A 2–5 column drawing plus its colors.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
pub struct SessionMark {
    pub glyph: &'static str,
    pub fg: (u8, u8, u8),
    pub bg: (u8, u8, u8),
}

type Rgb = (u8, u8, u8);

/// Glyphs and palettes live in `session_mark.json`. The dashboard and MCP
/// server read that same file.
#[derive(Deserialize)]
struct MarkData {
    glyphs: Vec<String>,
    palettes: Vec<(Rgb, Rgb)>,
}

fn mark_data() -> &'static MarkData {
    static DATA: OnceLock<MarkData> = OnceLock::new();
    DATA.get_or_init(|| {
        serde_json::from_str(include_str!("session_mark.json"))
            .expect("session_mark.json")
    })
}

fn glyphs() -> &'static [String] {
    &mark_data().glyphs
}

fn palettes() -> &'static [(Rgb, Rgb)] {
    &mark_data().palettes
}

fn glyph_at(index: usize) -> &'static str {
    glyphs()[index].as_str()
}

impl SessionMark {
    /// Pick a stable mark from a session id (the local crabigator id).
    pub fn from_seed(seed: &str) -> Self {
        Self::claim(seed, &[])
    }

    /// Prefer the hashed drawing unless another live session already has it.
    /// Glyphs repeat only when every drawing is taken; colors still change
    /// first so two sessions with the same drawing stay distinguishable.
    pub fn claim(seed: &str, taken: &[Self]) -> Self {
        let hash = fnv1a64(seed.as_bytes());
        let glyph_count = glyphs().len();
        let palette_count = palettes().len();
        let preferred_glyph = (hash as usize) % glyph_count;
        let preferred_palette = ((hash >> 8) as usize) % palette_count;
        let used_glyphs: HashSet<&str> = taken.iter().map(|mark| mark.glyph).collect();
        let glyph = if used_glyphs.len() >= glyph_count {
            glyph_at(preferred_glyph)
        } else {
            (0..glyph_count)
                .map(|offset| glyph_at((preferred_glyph + offset) % glyph_count))
                .find(|glyph| !used_glyphs.contains(glyph))
                .unwrap_or_else(|| glyph_at(preferred_glyph))
        };
        let used_colors: HashSet<_> = taken
            .iter()
            .filter(|mark| mark.glyph == glyph)
            .map(|mark| (mark.bg, mark.fg))
            .collect();
        let (bg, fg) = (0..palette_count)
            .map(|offset| palettes()[(preferred_palette + offset) % palette_count])
            .find(|pair| !used_colors.contains(pair))
            .unwrap_or_else(|| palettes()[preferred_palette]);
        Self { glyph, fg, bg }
    }

    /// Occupancy-aware pick for a new session: reuse a stored mark when this
    /// session already published one, otherwise take a drawing no live
    /// session is using.
    pub fn assign(session_id: &str) -> Self {
        if let Some(mark) = stored_mark(session_id) {
            return mark;
        }
        let mut mark = Self::claim(session_id, &live_taken_marks(session_id));
        publish_stub(session_id, mark);
        let taken = live_taken_marks(session_id);
        let used_glyphs = taken.iter().map(|other| other.glyph).collect::<HashSet<_>>();
        if used_glyphs.contains(mark.glyph) && used_glyphs.len() < glyphs().len() {
            mark = Self::claim(session_id, &taken);
            publish_stub(session_id, mark);
        }
        mark
    }

    /// Prefer the local crabigator id; fall back to the cloud id.
    pub fn from_ids(client_session_id: &str, session_id: &str) -> Self {
        Self::from_seed(match client_session_id {
            "" => session_id,
            id => id,
        })
    }

    /// Rebuild a mark stored in inspect.json. Unknown glyphs fall back to a
    /// fresh pick from `seed`.
    pub fn from_mirror(value: Option<&serde_json::Value>, seed: &str) -> Self {
        value
            .and_then(Self::from_json)
            .unwrap_or_else(|| Self::from_seed(seed))
    }

    fn from_json(value: &serde_json::Value) -> Option<Self> {
        let glyph = value.get("glyph")?.as_str()?;
        Some(Self {
            glyph: glyphs().iter().map(|item| item.as_str()).find(|item| *item == glyph)?,
            fg: rgb_array(value.get("fg")?)?,
            bg: rgb_array(value.get("bg")?)?,
        })
    }

    /// Colored chip with one column of padding on each side.
    ///
    /// Padding is a non-breaking space so terminals keep the background on
    /// the trailing cell. A normal space at the end of a color run is dropped,
    /// which made the chip look heavier on the left.
    pub fn chip(self) -> String {
        format!(
            "{}{}{PAD}{}{PAD}{RESET}",
            bg_rgb(self.bg),
            fg_rgb(self.fg),
            self.glyph
        )
    }

    /// Terminal columns the chip occupies, including padding.
    pub fn width(self) -> usize {
        1 + self.glyph.width() + 1
    }
}

fn rgb_array(value: &serde_json::Value) -> Option<(u8, u8, u8)> {
    let items = value.as_array()?;
    Some((
        items.first()?.as_u64()? as u8,
        items.get(1)?.as_u64()? as u8,
        items.get(2)?.as_u64()? as u8,
    ))
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash: u64 = 0xcbf2_9ce4_8422_2325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
    hash
}

fn now_secs() -> f64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs_f64()
}

fn inspect_path(session_id: &str) -> std::path::PathBuf {
    std::path::PathBuf::from(format!("/tmp/crabigator-{session_id}/inspect.json"))
}

fn stored_mark(session_id: &str) -> Option<SessionMark> {
    let data: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(inspect_path(session_id)).ok()?).ok()?;
    SessionMark::from_json(data.get("session_mark")?)
}

fn live_taken_marks(except_session_id: &str) -> Vec<SessionMark> {
    let Ok(entries) = glob::glob("/tmp/crabigator-*/inspect.json") else {
        return Vec::new();
    };
    let now = now_secs();
    let mut seen = HashSet::new();
    let mut taken = Vec::new();
    for path in entries.flatten() {
        let Ok(content) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(data) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let Some(session_id) = data.get("session_id").and_then(|value| value.as_str()) else {
            continue;
        };
        if session_id == except_session_id || !seen.insert(session_id.to_string()) {
            continue;
        }
        let last_updated = data.get("last_updated").and_then(|value| value.as_f64()).unwrap_or(0.0);
        if now - last_updated > LIVE_SESSION_SECS {
            continue;
        }
        if let Some(mark) = data.get("session_mark").and_then(SessionMark::from_json) {
            taken.push(mark);
        }
    }
    taken
}

fn publish_stub(session_id: &str, mark: SessionMark) {
    let path = inspect_path(session_id);
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let body = serde_json::json!({
        "session_id": session_id,
        "session_mark": mark,
        "last_updated": now_secs(),
    });
    if let Ok(json) = serde_json::to_string_pretty(&body) {
        let _ = std::fs::write(path, json);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::utils::strip_ansi_len;

    #[test]
    fn glyphs_fit_the_chip_budget() {
        for glyph in glyphs() {
            let width = glyph.as_str().width();
            assert!(
                (2..=5).contains(&width),
                "{glyph:?} is {width} columns, want 2–5"
            );
        }
    }

    #[test]
    fn seed_is_stable() {
        let mark = SessionMark::from_seed("crabigator-test");
        assert_eq!(mark, SessionMark::from_seed("crabigator-test"));
        assert_ne!(mark, SessionMark::from_seed("other-session"));
    }

    #[test]
    fn chip_paints_one_color_pair_and_counts_width() {
        let mark = SessionMark::from_seed("chip-width");
        let chip = mark.chip();
        assert!(chip.contains(mark.glyph));
        assert!(chip.contains(&format!(
            "\x1b[38;2;{};{};{}m",
            mark.fg.0, mark.fg.1, mark.fg.2
        )));
        assert!(chip.contains(&format!(
            "\x1b[48;2;{};{};{}m",
            mark.bg.0, mark.bg.1, mark.bg.2
        )));
        assert_eq!(strip_ansi_len(&chip), mark.width());
        assert_eq!(mark.width(), 1 + mark.glyph.width() + 1);
        assert!(
            chip.contains(&format!("{PAD}{}{PAD}", mark.glyph)),
            "padding is equal on both sides"
        );
    }

    #[test]
    fn mirror_round_trips_and_rejects_unknown_glyphs() {
        let mark = SessionMark::from_seed("mirror-roundtrip");
        let json = serde_json::to_value(mark).unwrap();
        assert_eq!(SessionMark::from_json(&json), Some(mark));

        let mut bad = json.clone();
        bad["glyph"] = serde_json::json!("nope");
        assert_eq!(
            SessionMark::from_mirror(Some(&bad), "mirror-roundtrip"),
            mark
        );
    }

    #[test]
    fn known_seed_is_stable() {
        let mark = SessionMark::from_seed("crabigator-test");
        assert_eq!(mark.glyph, "⊏◆⊐");
        assert_eq!(mark.bg, (122, 16, 36));
        assert_eq!(mark.fg, (255, 210, 168));
    }

    #[test]
    fn ids_prefer_the_local_session_id() {
        let local = SessionMark::from_seed("local-id");
        assert_eq!(SessionMark::from_ids("local-id", "cloud-id"), local);
        assert_eq!(SessionMark::from_ids("", "local-id"), local);
    }

    #[test]
    fn claim_skips_a_taken_preferred_glyph() {
        let preferred = SessionMark::from_seed("glyph-owner");
        let next = SessionMark::claim("glyph-owner", &[preferred]);
        assert_ne!(next.glyph, preferred.glyph);
    }

    #[test]
    fn live_sessions_get_unique_glyphs_until_the_set_is_full() {
        let mut taken = Vec::new();
        for index in 0..glyphs().len() {
            let mark = SessionMark::claim(&format!("session-{index}"), &taken);
            assert!(
                taken.iter().all(|other: &SessionMark| other.glyph != mark.glyph),
                "{} collided at {index}",
                mark.glyph
            );
            taken.push(mark);
        }
        assert_eq!(taken.len(), glyphs().len());
        let extra = SessionMark::claim("session-overflow", &taken);
        assert!(taken.iter().any(|mark| mark.glyph == extra.glyph));
        assert!(
            taken
                .iter()
                .filter(|mark| mark.glyph == extra.glyph)
                .all(|mark| (mark.bg, mark.fg) != (extra.bg, extra.fg)),
            "a repeated drawing still changes color"
        );
    }
}
