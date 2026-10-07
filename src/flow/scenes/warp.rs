//! Warp (flow's `hooks/starfield.ts`): a starfield on the same dials as the
//! fire, the level is the ship's speed.
//!
//! Stars sit in 3D (x, y in -1..1, depth z in 0..1) and fly toward the
//! viewer; each frame they're projected through the band's center onto a
//! braille sub-pixel grid (2×4 dots a cell). At rest the field drifts and
//! twinkles; from level 6 every star draws a streak back along its path, so
//! by 10 the band is hyperspace. Subagents (the coverage boost) add stars;
//! smoke dims the field to gray, a nearly-full context tints it cyan.

use crate::flow::cells::{Cells, Rng};
use crate::flow::js::{fround, i32_of, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::BRAILLE;
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use crate::flow::scenes::fire::params;

pub const DEF: SceneDef = SceneDef {
    name: "warp",
    hue: SceneHue {
        key: 264.0,
        range: 180.0,
        spread: None,
    },
    figure: false,
    make: |seed| Box::new(Starfield::new(seed)),
};

/// Depth travelled per frame at each level (0 = stopped).
const SPEED: [f64; 11] = [
    0.0, 0.0025, 0.004, 0.006, 0.009, 0.013, 0.018, 0.025, 0.034, 0.046, 0.062,
];
/// From this level stars draw streaks: trail length in frames of travel.
const STREAK_FROM: f64 = 6.0;
const NEAR: f64 = 0.04;
/// The span of a twinkle's phase: flow's literal, a shade under 2π.
#[allow(clippy::approx_constant)]
const PHASES: f64 = 6.283;

/// A star: across (x), down (y), depth (z, 1 = farthest) and its twinkle phase.
#[derive(Clone, Copy, Debug)]
struct Star {
    x: f64,
    y: f64,
    z: f64,
    tw: f64,
}

/// Brightness 0..1 as a star color: dim blue-gray far, white near.
fn star_color(b: f64, tint: Tint, warp: f64) -> u32 {
    let v = b.clamp(0.0, 1.0);
    if tint == Tint::Smoke {
        let g = round(60.0 + v * 110.0) as u32;
        return (g << 16) | (g << 8) | g;
    }
    // Far stars sit blue-gray; near ones bleach to white. Warp adds blue.
    let mut r = 70.0 + v * 185.0;
    let mut g = 80.0 + v * 175.0;
    let bl = 120.0 + v * 135.0;
    if tint == Tint::Blue {
        r *= 0.55;
        g = (g * 1.05).min(255.0);
    }
    r -= warp * 40.0 * (1.0 - v);
    g -= warp * 15.0 * (1.0 - v);
    let c = |n: f64| round(n).clamp(0.0, 255.0) as u32;
    (c(r) << 16) | (c(g) << 8) | c(bl)
}

pub struct Starfield {
    dials: Dials,
    columns: usize,
    rows: usize,
    stars: Vec<Star>,
    out: Cells,
    /// Braille dot bits per cell.
    bits: Vec<u8>,
    /// The brightest dot in each cell (a `Float32Array` in flow).
    bright: Vec<f32>,
    rng: Rng,
    t: f64,
}

impl Starfield {
    pub fn new(seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            columns: 0,
            rows: 0,
            stars: Vec::new(),
            out: Cells::new(0, 0),
            bits: Vec::new(),
            bright: Vec::new(),
            rng: Rng::new(seed),
            t: 0.0,
        }
    }

    /// Stars live in a volume shaped like the grid: flat for the band, tall for the spine.
    fn aspect_x(&self) -> f64 {
        let (w, h) = (self.columns as f64, self.rows as f64);
        ((w * 2.0) / (h * 4.0).max(1.0)).max(1.0)
    }

    fn aspect_y(&self) -> f64 {
        let (w, h) = (self.columns as f64, self.rows as f64);
        ((h * 4.0) / (w * 2.0).max(1.0)).max(1.0)
    }

    /// A new star: anywhere in depth at the start, at the far end afterwards.
    fn spawn(&mut self, any_depth: bool) -> Star {
        let x = (self.rng.f() * 2.0 - 1.0) * self.aspect_x();
        let y = (self.rng.f() * 2.0 - 1.0) * self.aspect_y();
        let z = if any_depth {
            NEAR + self.rng.f() * (1.0 - NEAR)
        } else {
            1.0
        };
        let tw = self.rng.f() * PHASES;
        Star { x, y, z, tw }
    }

    /// How many stars the band holds at this level: denser as it climbs.
    fn wanted(&self) -> f64 {
        let strength = self.dials.strength;
        if strength <= 0.0 {
            return 0.0;
        }
        let area = (self.columns * self.rows) as f64;
        let (_, seed) = params(strength);
        let coverage = (f64::from(seed) + self.dials.coverage_boost).min(160.0) / 100.0;
        round(area * (0.035 + 0.035 * coverage) * (0.7 + strength * 0.05))
    }

    /// A star's position in braille dots, or `None` off the band.
    fn project(&self, s: &Star, z: f64) -> Option<(f64, f64)> {
        let pw = (self.columns * 2) as f64;
        let ph = (self.rows * 4) as f64;
        // One scale for both axes (x already spans the band's aspect), so stars
        // stream out radially instead of squashing into a bow-tie.
        let scale = (pw.min(ph) / 2.0) * 0.9;
        let px = pw / 2.0 + (s.x / z) * scale;
        let py = ph / 2.0 + (s.y / z) * scale;
        if px < 0.0 || px >= pw || py < 0.0 || py >= ph {
            return None;
        }
        Some((px, py))
    }

    /// The level as an index into `SPEED` (the dial is a whole number 0..10).
    fn level(&self) -> usize {
        self.dials.strength.clamp(0.0, 10.0) as usize
    }

    /// Light one braille dot, keeping the cell's brightest.
    fn plot(&mut self, px: f64, py: f64, b: f64) {
        let x = i32_of(px.floor());
        let y = i32_of(py.floor());
        let c = i64::from(y >> 2) * self.columns as i64 + i64::from(x >> 1);
        if c < 0 || c >= self.bits.len() as i64 {
            return;
        }
        let c = c as usize;
        self.bits[c] |= BRAILLE[(x & 1) as usize][(y & 3) as usize] as u8;
        if b > f64::from(self.bright[c]) {
            self.bright[c] = fround(b) as f32;
        }
    }
}

impl Scene for Starfield {
    fn dials(&mut self) -> &mut Dials {
        &mut self.dials
    }

    fn ensure(&mut self, columns: usize, rows: usize) {
        if columns == self.columns && rows == self.rows {
            return;
        }
        self.columns = columns;
        self.rows = rows;
        self.out = Cells::new(columns, rows);
        self.bits = vec![0; columns * rows];
        self.bright = vec![0.0; columns * rows];
        self.stars.clear();
    }

    fn step(&mut self) {
        self.t += 1.0;
        let want = self.wanted();
        while (self.stars.len() as f64) < want {
            let star = self.spawn(true);
            self.stars.push(star);
        }
        if self.stars.len() as f64 > want {
            self.stars.truncate(want as usize);
        }
        let speed = SPEED[self.level()];
        for i in 0..self.stars.len() {
            self.stars[i].z -= speed;
            let s = self.stars[i];
            if s.z <= NEAR || self.project(&s, s.z).is_none() {
                self.stars[i] = self.spawn(false);
            }
        }
    }

    fn grid(&mut self) -> &Cells {
        self.bits.fill(0);
        self.bright.fill(0.0);
        let level = self.level();
        let speed = SPEED[level];
        let lf = level as f64;
        let trail = if lf >= STREAK_FROM {
            (lf - STREAK_FROM + 1.0) * 1.4
        } else {
            0.0
        };
        for i in 0..self.stars.len() {
            let s = self.stars[i];
            let Some(head) = self.project(&s, s.z) else {
                continue;
            };
            // Nearer is brighter; at rest the field twinkles.
            let twinkle = if level <= 2 {
                0.75 + 0.25 * (self.t * 0.35 + s.tw).sin()
            } else {
                1.0
            };
            let b = (1.0 - s.z) * twinkle;
            self.plot(head.0, head.1, b);
            if trail > 0.0 {
                // A streak back along the star's path, fading toward its tail.
                if let Some(tail) = self.project(&s, (s.z + speed * trail).min(1.0)) {
                    let steps = (head.0 - tail.0).abs().max((head.1 - tail.1).abs()).ceil();
                    let mut k = 1.0;
                    while k <= steps {
                        let f = k / (steps + 1.0);
                        self.plot(
                            head.0 + (tail.0 - head.0) * f,
                            head.1 + (tail.1 - head.1) * f,
                            b * (1.0 - f) * 0.55,
                        );
                        k += 1.0;
                    }
                }
            }
        }
        let warp = ((lf - 7.0) / 3.0).max(0.0);
        let tint = self.dials.tint;
        for i in 0..self.columns * self.rows {
            if self.bits[i] == 0 {
                self.out.blank(i);
            } else {
                let fg = star_color(f64::from(self.bright[i]), tint, warp);
                self.out.set_fg(i, 0x2800 | u32::from(self.bits[i]), fg);
            }
        }
        &self.out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/warp.json"), 0.0);
    }
}
