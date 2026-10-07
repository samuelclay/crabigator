//! A hot-air balloon in the sky world (flow's `hooks/balloon.ts`): the level
//! is its target altitude. At 1 it sits on the grass among trees and houses;
//! it climbs past birds and the layered clouds into a thinning sky; at 10 it
//! floats in space among stars, Earth's blue rim below. Subagents fly as
//! small companion balloons, a failure trails sooty smoke, and a
//! nearly-full context turns the stripes blue.

use crate::flow::cells::Cells;
use crate::flow::js::{i32_of, round};
use crate::flow::palette::{tint_to, SceneHue};
use crate::flow::pixels::{g, hash, mix};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use crate::flow::sky::{SkyScene, SkyWorld};

pub const DEF: SceneDef = SceneDef {
    name: "balloon",
    hue: SceneHue {
        key: 251.0,
        range: 45.0,
        spread: Some(60.0),
    },
    figure: true,
    make: |seed| Box::new(Balloon::new(seed)),
};

const STRIPE_A: u32 = 0xe04a3a;
const STRIPE_B: u32 = 0xf2c14e;
const BLUE_A: u32 = 0x3a7be0;
const BLUE_B: u32 = 0xe8eef8;
const ROPE: u32 = 0x9a9a9a;
const BASKET: u32 = 0x8b5a2b;
const FLAME: [u32; 3] = [0xffd166, 0xff9f1c, 0xff6b1a];
const SMOKE: u32 = 0x9a9a9a;
const SOOT: u32 = 0x38383c;
const COMPANION: [u32; 4] = [0xe06a3a, 0x3ab0e0, 0x9a6ae0, 0x5ac46a];

/// The balloon, 7 wide × 4 tall; ' ' cells let the sky through.
const SPRITE: [&str; 4] = [" ▄▆█▆▄ ", "███████", " ▀█▀█▀ ", "  ╲█╱  "];

pub struct Balloon {
    world: SkyWorld,
}

impl Balloon {
    pub fn new(seed: f64) -> Self {
        Self {
            world: SkyWorld::new(seed),
        }
    }

    /// The balloon's left column: it wanders the middle of the band in the wind.
    fn balloon_x(&self) -> f64 {
        let w = self.world.columns as f64;
        let t = self.world.t;
        let wander = (t * 0.0045).sin() * 0.28 + (t * 0.0017).sin() * 0.12;
        let x = round(w / 2.0 - 3.0 + wander * w);
        x.min((w - 7.0).max(0.0)).max(0.0)
    }

    fn draw_smoke(&self, out: &mut Cells, top: f64) {
        let (w, h) = (self.world.columns as f64, self.world.rows as f64);
        let bx = self.balloon_x() + 3.0;
        let puff = f64::from(i32_of(self.world.t) >> 2);
        for k in 1..=9 {
            let k = f64::from(k);
            let x = bx - k - 1.0;
            let r = top + 2.0 - (k / 3.0).floor();
            if x < 0.0 || x >= w || r < 0.0 || r >= h {
                continue;
            }
            let n = hash(k - puff, 0.0, 91.0);
            if n < 0.2 {
                continue;
            }
            let i = (r * w + x) as usize;
            let behind = out.behind(i);
            let thin = k / 10.0;
            let glyph = if k < 5.0 && n > 0.45 { '▓' } else { '▒' };
            out.set(i, g(glyph), mix(SOOT, behind, thin * 0.5), behind);
        }
    }

    fn draw_companions(&self, out: &mut Cells) {
        let n = round(self.world.dials.coverage_boost / 15.0).min(4.0);
        let (w, h) = (self.world.columns as f64, self.world.rows as f64);
        let t = self.world.t;
        let mut k = 0.0;
        while k < n {
            let x = round(((k + 1.0) / (n + 1.0)) * w + (t * 0.01 + k * 2.0).sin() * 4.0);
            let r = (1.0 + ((k + f64::from(i32_of(t) >> 6)) % 2.0))
                .min(h - 2.0)
                .max(0.0);
            if x >= 0.0 && x < w {
                let i = (r * w + x) as usize;
                out.set(
                    i,
                    g('●'),
                    COMPANION[k as usize % COMPANION.len()],
                    out.behind(i),
                );
                let j = ((r + 1.0) * w + x) as usize;
                out.set(j, g('╵'), ROPE, out.behind(j));
            }
            k += 1.0;
        }
    }

    fn draw_balloon(&self, out: &mut Cells, top: f64) {
        let w = self.world.columns as f64;
        let bx = self.balloon_x();
        let tint = self.world.dials.tint;
        let blue = tint == Tint::Blue;
        let burning =
            self.world.target() > self.world.alt + 0.3 || self.world.dials.strength >= 6.0;
        // The stripes in the session's hue, light and dark (crabigator's).
        let (stripe_a, stripe_b) = match self.world.dials.accent {
            Some(hue) => (tint_to(STRIPE_A, hue, 0.0), tint_to(STRIPE_B, hue, 0.0)),
            None => (STRIPE_A, STRIPE_B),
        };
        for (sr, row) in SPRITE.iter().enumerate() {
            let r = top + sr as f64;
            if r < 0.0 {
                continue;
            }
            for (sc, ch) in row.chars().enumerate() {
                let x = bx + sc as f64;
                if ch == ' ' || x < 0.0 || x >= w {
                    continue;
                }
                let color = if sr == 3 {
                    if ch == '█' {
                        BASKET
                    } else {
                        ROPE
                    }
                } else if sr == 2 && sc == 3 {
                    // The burner's mouth: a flicker of flame while it climbs.
                    if tint == Tint::Smoke {
                        SMOKE
                    } else if burning {
                        FLAME[((i32_of(self.world.t) >> 1) % 3) as usize]
                    } else if blue {
                        BLUE_A
                    } else {
                        stripe_a
                    }
                } else {
                    let even = sc % 2 == 0;
                    match (blue, even) {
                        (true, true) => BLUE_A,
                        (true, false) => BLUE_B,
                        (false, true) => stripe_a,
                        (false, false) => stripe_b,
                    }
                };
                let i = (r * w + x) as usize;
                if i < out.len() {
                    out.set(i, g(ch), color, out.behind(i));
                }
            }
        }
    }
}

impl SkyScene for Balloon {
    fn world(&self) -> &SkyWorld {
        &self.world
    }

    fn world_mut(&mut self) -> &mut SkyWorld {
        &mut self.world
    }

    fn vehicle_height(&self) -> f64 {
        SPRITE.len() as f64
    }

    fn draw_vehicle(&mut self, out: &mut Cells, top: f64) {
        self.draw_companions(out);
        if self.world.dials.tint == Tint::Smoke {
            self.draw_smoke(out, top);
        }
        self.draw_balloon(out, top);
    }
}

impl Scene for Balloon {
    fn dials(&mut self) -> &mut Dials {
        &mut self.world.dials
    }

    fn ensure(&mut self, columns: usize, rows: usize) {
        self.world.ensure(columns, rows);
    }

    fn step(&mut self) {
        self.sky_step();
    }

    fn grid(&mut self) -> &Cells {
        self.sky_grid()
    }
}

#[cfg(test)]
mod tests {
    use crate::flow::palette::tint_to;
    use crate::flow::scene::Scene;

    #[test]
    fn the_stripes_wear_the_session_hue() {
        let mut b = super::Balloon::new(7.0);
        b.dials().accent = Some(305.0);
        b.ensure(24, 8);
        for _ in 0..30 {
            b.step();
        }
        let want = tint_to(super::STRIPE_A, 305.0, 0.0);
        let grid = b.grid();
        assert!((0..grid.len()).any(|i| grid.foreground(i) == want));
        assert!((0..grid.len()).all(|i| grid.foreground(i) != super::STRIPE_A));
    }

    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/balloon.json"), 0.0);
    }
}
