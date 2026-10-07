//! Checks a ported scene against flow's own frames (`testdata/<scene>.json`,
//! recorded by `make flow-reference`): the same seed, size and dials, step
//! by step, must give the same cells.

use base64::Engine;
use serde::Deserialize;

use super::scene::{scene_def, Tint};

#[derive(Deserialize)]
struct Reference {
    scene: String,
    cases: Vec<Case>,
}

#[derive(Deserialize)]
struct Case {
    columns: usize,
    rows: usize,
    seed: f64,
    script: Vec<(u32, Dials)>,
    snaps: Vec<u32>,
    frames: Vec<String>,
}

#[derive(Deserialize, Default, Clone, Copy)]
struct Dials {
    level: Option<f64>,
    tint: Option<TintName>,
    night: Option<bool>,
    coverage: Option<f64>,
}

#[derive(Deserialize, Clone, Copy)]
#[serde(rename_all = "lowercase")]
enum TintName {
    Normal,
    Smoke,
    Blue,
}

/// Run the scene through every recorded case and compare each snapshot.
/// Cells may differ only where floating-point functions round their last
/// bit differently from JavaScript's; `max_mismatch` (a fraction of cells)
/// bounds that.
pub fn check(json: &str, max_mismatch: f64) {
    let reference: Reference = serde_json::from_str(json).expect("reference frames");
    let def = scene_def(&reference.scene).expect("a ported scene");
    for (case_index, case) in reference.cases.iter().enumerate() {
        let mut scene = (def.make)(case.seed);
        let (mut level, mut tint, mut night, mut coverage) = (8.0, Tint::Normal, false, 0.0);
        let last = *case.snaps.iter().max().unwrap();
        let mut frames = case.frames.iter();
        for n in 1..=last {
            for (at, d) in &case.script {
                if *at == n - 1 {
                    level = d.level.unwrap_or(level);
                    night = d.night.unwrap_or(night);
                    coverage = d.coverage.unwrap_or(coverage);
                    tint = match d.tint {
                        Some(TintName::Normal) => Tint::Normal,
                        Some(TintName::Smoke) => Tint::Smoke,
                        Some(TintName::Blue) => Tint::Blue,
                        None => tint,
                    };
                }
            }
            let dials = scene.dials();
            dials.strength = level;
            dials.tint = tint;
            dials.night = night;
            dials.coverage_boost = coverage;
            scene.ensure(case.columns, case.rows);
            scene.step();
            if !case.snaps.contains(&n) {
                continue;
            }
            let want = base64::engine::general_purpose::STANDARD
                .decode(frames.next().expect("a frame per snapshot"))
                .unwrap();
            let want: Vec<u32> = want
                .chunks_exact(4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                .collect();
            let got = &scene.grid().words;
            assert_eq!(
                got.len(),
                want.len(),
                "case {case_index} step {n}: grid size"
            );
            let cells = want.len() / 3;
            let diffs: Vec<usize> = (0..cells)
                .filter(|&i| got[i * 3..i * 3 + 3] != want[i * 3..i * 3 + 3])
                .collect();
            let allowed = (cells as f64 * max_mismatch).floor() as usize;
            if diffs.len() > allowed {
                let shown: Vec<String> = diffs
                    .iter()
                    .take(8)
                    .map(|&i| {
                        format!(
                            "  ({}, {}): got {:?} want {:?}",
                            i % case.columns,
                            i / case.columns,
                            describe(&got[i * 3..i * 3 + 3]),
                            describe(&want[i * 3..i * 3 + 3])
                        )
                    })
                    .collect();
                panic!(
                    "{} case {case_index} ({}×{}) step {n}: {} of {cells} cells differ (allowed {allowed})\n{}",
                    reference.scene,
                    case.columns,
                    case.rows,
                    diffs.len(),
                    shown.join("\n")
                );
            }
        }
    }
}

fn describe(cell: &[u32]) -> String {
    let glyph = char::from_u32(cell[0]).unwrap_or('?');
    format!("{glyph} {:06x}/{:06x}", cell[1], cell[2])
}
