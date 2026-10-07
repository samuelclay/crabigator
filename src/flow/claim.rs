//! Which scene each session shows, and in what colour.
//!
//! Scenes are claimed like session marks: the preferred scene is hashed from
//! the session id, then walked forward past scenes other live sessions have
//! claimed, so sessions side by side show different scenes until every scene
//! is in use. The claim is published in `inspect.json` (`flow_scene`). The
//! colour comes from the session's mark, so the scene and the chip match.

use std::collections::HashSet;
use std::sync::OnceLock;

use super::scene::scenes;
use crate::session_mark::{fnv1a64, inspect_path, live_session_mirrors, publish_stub, SessionMark};

/// Below this chroma (the brightest channel minus the dimmest), a mark's
/// colours are grey: the scene keeps its natural colours.
const MIN_ACCENT_CHROMA: u8 = 48;

/// The scenes there are, in flow's order.
pub fn scene_names() -> &'static [String] {
    static NAMES: OnceLock<Vec<String>> = OnceLock::new();
    NAMES.get_or_init(|| scenes().iter().map(|def| def.name.to_string()).collect())
}

/// The preferred scene for a seed, walked forward past the taken ones. When
/// every scene is taken, the preferred one.
pub fn claim_scene(seed: &str, taken: &[String]) -> String {
    let names = scene_names();
    let preferred = ((fnv1a64(seed.as_bytes()) >> 32) as usize) % names.len();
    let taken: HashSet<&str> = taken.iter().map(String::as_str).collect();
    (0..names.len())
        .map(|offset| &names[(preferred + offset) % names.len()])
        .find(|name| !taken.contains(name.as_str()))
        .unwrap_or(&names[preferred])
        .clone()
}

/// This session's scene: the one it already claimed, or a fresh claim that
/// no live session has, published at once (with the mark) so sessions
/// starting at the same moment see it.
pub fn assign_scene(session_id: &str, mark: SessionMark) -> String {
    if let Some(scene) = stored_scene(session_id) {
        return scene;
    }
    let mut scene = claim_scene(session_id, &live_taken_scenes(session_id));
    publish_stub(session_id, mark, Some(&scene));
    // Another session may have claimed the same scene a moment ago: claim
    // again past it, while there are scenes left.
    let taken = live_taken_scenes(session_id);
    let distinct: HashSet<&str> = taken.iter().map(String::as_str).collect();
    if distinct.contains(scene.as_str()) && distinct.len() < scene_names().len() {
        scene = claim_scene(session_id, &taken);
        publish_stub(session_id, mark, Some(&scene));
    }
    scene
}

/// The colour to turn the scene to: whichever of the mark's colours is more
/// colourful, or `None` when both are near grey.
pub fn mark_accent(mark: SessionMark) -> Option<[u8; 3]> {
    let chroma = |(r, g, b): (u8, u8, u8)| r.max(g).max(b) - r.min(g).min(b);
    let pick = if chroma(mark.fg) >= chroma(mark.bg) {
        mark.fg
    } else {
        mark.bg
    };
    (chroma(pick) >= MIN_ACCENT_CHROMA).then_some([pick.0, pick.1, pick.2])
}

/// A seed for the scene's own randomness, stable for the session.
pub fn scene_seed(session_id: &str) -> u32 {
    (fnv1a64(session_id.as_bytes()) >> 16) as u32
}

fn is_scene(name: &str) -> bool {
    scene_names().iter().any(|known| known == name)
}

fn stored_scene(session_id: &str) -> Option<String> {
    let data: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(inspect_path(session_id)).ok()?).ok()?;
    let scene = data.get("flow_scene")?.as_str()?;
    is_scene(scene).then(|| scene.to_string())
}

fn live_taken_scenes(session_id: &str) -> Vec<String> {
    live_session_mirrors(session_id)
        .iter()
        .filter_map(|data| data.get("flow_scene")?.as_str())
        .filter(|scene| is_scene(scene))
        .map(str::to_string)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_scene_is_named() {
        let names = scene_names();
        assert!(names.iter().any(|name| name == "fire"), "{names:?}");
    }

    #[test]
    fn a_seed_prefers_the_same_scene_every_time() {
        assert_eq!(claim_scene("session-a", &[]), claim_scene("session-a", &[]));
    }

    #[test]
    fn taken_scenes_are_skipped_until_every_scene_is_in_use() {
        let mut taken: Vec<String> = Vec::new();
        for index in 0..scene_names().len() {
            let scene = claim_scene(&format!("session-{index}"), &taken);
            assert!(!taken.contains(&scene), "{scene} claimed twice");
            taken.push(scene);
        }
        let preferred = claim_scene("one-too-many", &[]);
        assert_eq!(claim_scene("one-too-many", &taken), preferred);
    }

    #[test]
    fn the_accent_is_the_more_colourful_mark_colour_or_none_for_grey() {
        let mark = |bg, fg| SessionMark {
            glyph: "◆◇◆",
            bg,
            fg,
        };
        // A pale fg on a saturated bg: the bg.
        assert_eq!(
            mark_accent(mark((122, 16, 36), (255, 210, 168))),
            Some([122, 16, 36])
        );
        // A dark bg under a bright fg: the fg.
        assert_eq!(
            mark_accent(mark((0, 24, 72), (94, 240, 255))),
            Some([94, 240, 255])
        );
        // Charcoal and cream: grey, so natural colours.
        assert_eq!(mark_accent(mark((26, 26, 26), (232, 220, 192))), None);
    }

    #[test]
    fn every_session_mark_palette_but_the_grey_one_has_an_accent() {
        type Rgb = (u8, u8, u8);
        #[derive(serde::Deserialize)]
        struct Data {
            palettes: Vec<(Rgb, Rgb)>,
        }
        let data: Data = serde_json::from_str(include_str!("../session_mark.json")).unwrap();
        let grey: Vec<usize> = data
            .palettes
            .iter()
            .enumerate()
            .filter(|(_, (bg, fg))| {
                mark_accent(SessionMark {
                    glyph: "◆◇◆",
                    bg: *bg,
                    fg: *fg,
                })
                .is_none()
            })
            .map(|(index, _)| index)
            .collect();
        assert_eq!(grey, vec![6]);
    }
}
