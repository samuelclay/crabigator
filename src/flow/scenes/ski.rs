//! A skier on a mountain (flow's `hooks/ski.ts`), on the fire's dials: the
//! level is the speed and the steepness. At 1 the skier stands at the top of
//! the run, poles planted, snow drifting down (beside the lift hut in the
//! band; at the summit under the peaks in the spine). As the level rises they
//! push off and carve linked S-turns, faster and faster, leaving tracks in
//! the snow and throwing spray off every turn while the pines, slalom gates,
//! moguls, rocks and far peaks stream past. From 8 they tuck into a steep
//! powder run, a cloud of powder boiling up behind; at 10 they're flat out
//! and catching air off kickers, a little shadow on the snow below.
//!
//! Two layouts. The band (wide, a few rows) is a side view (`ski/band.rs`):
//! the slope runs across the band under a winter-blue sky, the skier's carves
//! read as a weave toward and away from you, tracks trailing across the
//! slope's face. The spine (tall, narrow) is a three-quarter view down the
//! fall line (`ski/spine.rs`): the skier carves S-turns down the column, the
//! slope scrolling up past them, forest along both edges and the tracks
//! winding away behind.
//!
//! Everything is painted into a pixel layer at quadrant resolution (2×2 per
//! cell; each cell keeps the two colors that best fit its four pixels), with
//! a braille layer on top (2×4 per cell) for the fine things: snowflakes,
//! spray specks and ski tracks, wherever the cell under them is plain.
//!
//! Dials: running subagents put more skiers on the slope (each in their own
//! jacket); a failed command makes the skier wipe out in a puff of snow (a
//! yard sale: skis crossed, one stuck upright) under a flurry until things
//! are fixed; a nearly-full context turns the light to dusk.
//!
//! flow's soundscape (each turn's swish, the wind) is left out: crabigator
//! plays no sound. Each turn's pace is still drawn at random, as flow does.

mod band;
mod spine;

use std::f64::consts::PI;

use crate::flow::cells::{is_tall, Cells, Rng, DEFAULT_COLOR};
use crate::flow::js::{hypot, i32_of, max, min, round};
use crate::flow::night::{moon_cover, moon_radius, MOON, NIGHT_HORIZON, NIGHT_ZENITH, STAR};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::{
    clamp, fit_quad, hash_murmur as hash, mix, QuadFit, BRAILLE, NEAR, QUAD,
};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};

pub const DEF: SceneDef = SceneDef {
    name: "ski",
    hue: SceneHue {
        key: 251.0,
        range: 45.0,
        spread: Some(60.0),
    },
    make: |seed| Box::new(Ski::new(seed)),
};

// ---------------------------------------------------------------- tables

/// Speed per level: band pixels (along the slope) per frame.
const SPEED: [f64; 11] = [0.0, 0.0, 0.45, 0.8, 1.2, 1.7, 2.2, 2.8, 3.5, 4.3, 5.2];
/// How wide the carves are (fraction of the room to swing in).
const AMP: [f64; 11] = [0.0, 0.0, 0.45, 0.65, 0.85, 0.95, 0.95, 0.85, 0.7, 0.55, 0.4];
/// Frames per full S (left turn + right turn).
const PERIOD: [f64; 11] = [
    60.0, 60.0, 64.0, 56.0, 48.0, 42.0, 38.0, 34.0, 30.0, 28.0, 26.0,
];
/// Spray particles per frame at a turn's edge change.
const SPRAY: [f64; 11] = [0.0, 0.0, 0.3, 0.6, 1.0, 1.5, 2.1, 2.8, 3.8, 4.8, 6.0];
/// Moguls by level: a bumpy mid-mountain run.
const MOGULS: [f64; 11] = [0.0, 0.0, 0.0, 0.4, 0.8, 1.0, 0.9, 0.6, 0.2, 0.0, 0.0];
/// Slalom gates by level.
const GATES: [f64; 11] = [0.0, 0.0, 0.15, 0.35, 0.45, 0.45, 0.35, 0.25, 0.1, 0.0, 0.0];
/// The spine scrolls in rows: a row is twice as tall as a pixel column is wide.
const SPINE: f64 = 0.42;
/// Frames to get up again after a fall.
const GET_UP: u32 = 18;

/// Particles (spray, billows, puffs), track points per skier, snowflakes.
const PMAX: usize = 520;
const TRK: usize = 400;
const FLAKES: usize = 220;
/// A flake's sway phase spans this (flow's literal, a shade under 2π).
#[allow(clippy::approx_constant)]
const SWAY_TURN: f64 = 6.283;

// ---------------------------------------------------------------- helpers

/// Smooth 1-D value noise in [0, 1).
fn noise(x: f64, s: f64) -> f64 {
    let i = x.floor();
    let f = x - i;
    let u = f * f * (3.0 - 2.0 * f);
    hash(i, 0.0, s) * (1.0 - u) + hash(i + 1.0, 0.0, s) * u
}

/// Smooth 2-D value noise in [0, 1).
fn noise2(x: f64, y: f64, s: f64) -> f64 {
    let i = x.floor();
    let j = y.floor();
    let fx = x - i;
    let fy = y - j;
    let u = fx * fx * (3.0 - 2.0 * fx);
    let v = fy * fy * (3.0 - 2.0 * fy);
    let a = hash(i, j, s) * (1.0 - u) + hash(i + 1.0, j, s) * u;
    let b = hash(i, j + 1.0, s) * (1.0 - u) + hash(i + 1.0, j + 1.0, s) * u;
    a * (1.0 - v) + b * v
}

/// `v` eased a fraction `k` of the way to `goal`, landing on it once close.
fn approach(v: f64, goal: f64, k: f64) -> f64 {
    let n = v + (goal - v) * k;
    if (goal - n).abs() < 0.01 {
        goal
    } else {
        n
    }
}

/// `x += d` on a `Float32Array` element: the sum in f64, stored narrowed.
fn add(x: &mut f32, d: f64) {
    *x = (f64::from(*x) + d) as f32;
}

/// `x *= k` on a `Float32Array` element.
fn scale(x: &mut f32, k: f64) {
    *x = (f64::from(*x) * k) as f32;
}

// ---------------------------------------------------------------- palettes

/// The scene's colors, eased between day, night, overcast and dusk.
#[derive(Clone, Copy)]
struct Pal {
    sky_top: u32,
    sky_low: u32,
    peak: u32,
    peak_shade: u32,
    cap: u32,
    cap_shade: u32,
    hill: u32,
    hill_pine: u32,
    snow: u32,
    snow_shade: u32,
    snow_deep: u32,
    track: u32,
    pine: u32,
    pine_lit: u32,
    pine_snow: u32,
    trunk: u32,
    rock: u32,
    spray: u32,
    spray_shade: u32,
    shadow: u32,
    flake: u32,
    hut: u32,
    roof: u32,
    window: u32,
}

impl Pal {
    /// Each color of `self` and `other` through `f`, key by key.
    fn zip(&self, other: &Pal, f: impl Fn(u32, u32) -> u32) -> Pal {
        Pal {
            sky_top: f(self.sky_top, other.sky_top),
            sky_low: f(self.sky_low, other.sky_low),
            peak: f(self.peak, other.peak),
            peak_shade: f(self.peak_shade, other.peak_shade),
            cap: f(self.cap, other.cap),
            cap_shade: f(self.cap_shade, other.cap_shade),
            hill: f(self.hill, other.hill),
            hill_pine: f(self.hill_pine, other.hill_pine),
            snow: f(self.snow, other.snow),
            snow_shade: f(self.snow_shade, other.snow_shade),
            snow_deep: f(self.snow_deep, other.snow_deep),
            track: f(self.track, other.track),
            pine: f(self.pine, other.pine),
            pine_lit: f(self.pine_lit, other.pine_lit),
            pine_snow: f(self.pine_snow, other.pine_snow),
            trunk: f(self.trunk, other.trunk),
            rock: f(self.rock, other.rock),
            spray: f(self.spray, other.spray),
            spray_shade: f(self.spray_shade, other.spray_shade),
            shadow: f(self.shadow, other.shadow),
            flake: f(self.flake, other.flake),
            hut: f(self.hut, other.hut),
            roof: f(self.roof, other.roof),
            window: f(self.window, other.window),
        }
    }
}

const DAY: Pal = Pal {
    sky_top: 0x2f74cf,
    sky_low: 0xa6d2f4,
    peak: 0x8aa0bf,
    peak_shade: 0x657c9e,
    cap: 0xf5f9fd,
    cap_shade: 0xc3d3ea,
    hill: 0xd5e3f2,
    hill_pine: 0x4f7d74,
    snow: 0xeef4fb,
    snow_shade: 0xc2d3e9,
    snow_deep: 0xa9bfdc,
    track: 0x93abcd,
    pine: 0x17482d,
    pine_lit: 0x2e7448,
    pine_snow: 0xe4edf6,
    trunk: 0x5b3a22,
    rock: 0x5f646e,
    spray: 0xffffff,
    spray_shade: 0xc9d8ec,
    shadow: 0x8098be,
    flake: 0xffffff,
    hut: 0x7a4a2a,
    roof: 0xf2f6fb,
    window: 0xffd36b,
};

/// A nearly-full context: dusk, alpenglow on the peaks, the snow gone lavender-blue.
const DUSK: Pal = Pal {
    sky_top: 0x1b2150,
    sky_low: 0xe08f74,
    peak: 0xb98aa0,
    peak_shade: 0x5a5480,
    cap: 0xffc9b5,
    cap_shade: 0x9a8ab6,
    hill: 0x9fa6cf,
    hill_pine: 0x2a3550,
    snow: 0xb9c2e4,
    snow_shade: 0x8f9acb,
    snow_deep: 0x7a85bb,
    track: 0x5f6aa3,
    pine: 0x10283a,
    pine_lit: 0x1d4050,
    pine_snow: 0xa9b3da,
    trunk: 0x3a2a2a,
    rock: 0x4a4a62,
    spray: 0xe2e6fa,
    spray_shade: 0xa3acd8,
    shadow: 0x5d679f,
    flake: 0xdfe4ff,
    hut: 0x4e3328,
    roof: 0xc4cbec,
    window: 0xffb347,
};

/// A failed command: an overcast whiteout sky.
const GREY: Pal = Pal {
    sky_top: 0x7d8796,
    sky_low: 0xc9cfd8,
    peak: 0xa3abb8,
    peak_shade: 0x8a93a2,
    cap: 0xe6e9ee,
    cap_shade: 0xc6ccd5,
    hill: 0xd7dce3,
    ..DAY
};

/// Night: moonlit snow under the near-black night sky (night.rs), the hut window glowing.
const NIGHT: Pal = Pal {
    sky_top: NIGHT_ZENITH,
    sky_low: NIGHT_HORIZON,
    peak: 0x34405e,
    peak_shade: 0x222b45,
    cap: 0xb4c2dc,
    cap_shade: 0x6e7c9c,
    hill: 0x4a5878,
    hill_pine: 0x0e1c26,
    snow: 0x8394b6,
    snow_shade: 0x5f6f90,
    snow_deep: 0x4c5a7a,
    track: 0x3e4b6a,
    pine: 0x081a14,
    pine_lit: 0x12301f,
    pine_snow: 0x8e9ec0,
    trunk: 0x2a1c14,
    rock: 0x30343e,
    spray: 0xd4e0f4,
    spray_shade: 0x8c9cbc,
    shadow: 0x46557a,
    flake: 0xdce6f8,
    hut: 0x3a2618,
    roof: 0x9aaac8,
    window: 0xffc35a,
};

/// A failed command by night: cloud over the moon, the stars gone.
const NIGHT_GREY: Pal = Pal {
    sky_top: 0x1c2230,
    sky_low: 0x3a4252,
    peak: 0x4a5262,
    peak_shade: 0x3a4252,
    cap: 0x8a92a2,
    cap_shade: 0x6a7282,
    ..NIGHT
};

/// What the skiers' kit leans toward by night.
const NIGHT_SHADE: u32 = 0x0a1428;

/// A skier's kit: hat, jacket, pants, skis.
#[derive(Clone, Copy)]
struct Kit {
    hat: u32,
    jacket: u32,
    pants: u32,
    skis: u32,
}

/// Each skier's kit. The first is the hero.
const KITS: [Kit; 5] = [
    Kit {
        hat: 0xffd23f,
        jacket: 0xe8302c,
        pants: 0x1f2a50,
        skis: 0xff9b1c,
    },
    Kit {
        hat: 0xffffff,
        jacket: 0x1e86e8,
        pants: 0x2b2b31,
        skis: 0xd9dde4,
    },
    Kit {
        hat: 0xff5a8a,
        jacket: 0x22b06a,
        pants: 0x2d2440,
        skis: 0x30343c,
    },
    Kit {
        hat: 0x2e2e36,
        jacket: 0xf4c20d,
        pants: 0x2a3a5a,
        skis: 0x8a1f1f,
    },
    Kit {
        hat: 0x5cd0ff,
        jacket: 0x9b55d6,
        pants: 0x232a3a,
        skis: 0x30343c,
    },
];
const SKIN: u32 = 0xf1c39b;
const POLE: u32 = 0x4c5058;

// ---------------------------------------------------------------- sprites

/// A small sprite, top row first; `ax` is the feet's column, the bottom row the feet's row.
struct Sprite {
    ax: usize,
    rows: &'static [&'static str],
}

// Side view (the band), facing downhill to the right.
// The body is two pixels wide at columns 2-3, drawn on an even pixel so it fills one cell column.
const B_STAND: Sprite = Sprite {
    ax: 2,
    rows: &["..HH...", "..RRR..", ".pRR.p.", ".pPP.p.", "SSSSSSS"],
};
const B_SKI: Sprite = Sprite {
    ax: 2,
    rows: &["..HH...", "..RRR..", ".pRR...", "p.PP...", "SSSSSSS"],
};
const B_CARVE: Sprite = Sprite {
    ax: 2,
    rows: &["..HH...", "..RRRp.", "..RR.p.", ".pPP...", "SSSSSSS"],
};
const B_TUCK: Sprite = Sprite {
    ax: 2,
    rows: &["..HH...", ".RRRRR.", "ppPP...", "SSSSSSS"],
};
const B_DOWN: Sprite = Sprite {
    ax: 4,
    rows: &["S.......", "S.......", "S..HRRPP", "S.p.SSSS"],
};

// Three-quarter view (the spine), facing down the hill; skis and poles are drawn apart.
const S_BODY: Sprite = Sprite {
    ax: 1,
    rows: &[".H.", ".F.", "RRR", "rRr", ".P."],
};
const S_TUCK: Sprite = Sprite {
    ax: 1,
    rows: &[".H.", "RFR", "rPr"],
};
const S_DOWN: Sprite = Sprite {
    ax: 3,
    rows: &[".RR....", "HRRRPP."],
};

// ---------------------------------------------------------------- the scene

/// Turn phase offsets, band anchors (place across the band) and spine
/// anchors (place down the column), per skier.
const OFFS: [f64; 5] = [0.0, 2.2, 4.1, 1.1, 3.3];
const ANCHORS: [f64; 5] = [0.28, 0.52, 0.12, 0.7, 0.86];
const SPINE_ANCHORS: [f64; 5] = [0.3, 0.62, 0.14, 0.82, 0.47];

struct Skier {
    kit: Kit,
    /// Phase offset of their turns.
    off: f64,
    /// Their tracks: a ring of (x, y) points (`Float32Array` in flow); NaN breaks the line.
    track: Vec<f32>,
    head: usize,
    len: usize,
    last_d: f64,
    /// This frame's feet, in the layout's world coords.
    fx: f64,
    fy: f64,
    lat: f64,
}

/// Snow in the air (flow's parallel `Float32Array`s): specks (kind 0) and billows (kind 1).
struct Particles {
    x: Vec<f32>,
    y: Vec<f32>,
    vx: Vec<f32>,
    vy: Vec<f32>,
    life: Vec<f32>,
    max_life: Vec<f32>,
    r1: Vec<f32>,
    a0: Vec<f32>,
    kind: Vec<u8>,
    next: usize,
}

/// Snowflakes, in dot coordinates: place, fall speed, sway phase.
struct Flakes {
    x: Vec<f32>,
    y: Vec<f32>,
    v: Vec<f32>,
    p: Vec<f32>,
}

/// Where `draw_particles` maps world coordinates to the screen.
#[derive(Clone, Copy)]
enum View {
    /// The band: x scrolls with the camera, y sits on the ground under it.
    Band { cam: f64 },
    /// The spine: the world row `top` is the grid's top.
    Spine { top: f64 },
}

pub struct Ski {
    dials: Dials,
    // The light, eased toward the dials (-1 until the first frame): night, overcast, dusk.
    k_night: f64,
    k_grey: f64,
    k_dusk: f64,
    palette: Pal,

    out: Cells,
    columns: usize,
    rows: usize,
    tall: bool,
    rng: Rng,

    // Motion.
    t: u32,
    d: f64,
    v: f64,
    steep: f64,
    amp: f64,
    rate: f64,
    phi: f64,
    /// This turn's pace and width against the level's, drawn afresh each turn: a skier's turns aren't a metronome.
    pace: f64,
    wide: f64,
    turn: f64,
    moguls: f64,
    gates: f64,
    // Air: height (pixels) and climb, and the kickers (distances where their lips are).
    air: f64,
    v_air: f64,
    jump_at: f64,
    last_kick: f64,
    kick_x: f64,
    landed: u32,
    // A wipe-out: 0 skiing, 1 down, 2 getting up.
    fall: u8,
    fall_t: u32,

    // Layers.
    pw: usize,
    ph: usize,
    pix: Vec<u32>,
    bits: Vec<u8>,
    dcol: Vec<u32>,
    /// Pixels the skiers drew this frame: their colors win in the cells they share.
    kit_mask: Vec<u8>,
    ground: Vec<f32>,
    fit: QuadFit,

    particles: Particles,
    flakes: Flakes,
    skiers: Vec<Skier>,
}

impl Ski {
    pub fn new(seed: f64) -> Self {
        let seed = seed % 100_000.0;
        let skiers = KITS
            .iter()
            .zip(OFFS)
            .map(|(&kit, off)| Skier {
                kit,
                off,
                track: vec![0.0; TRK * 2],
                head: 0,
                len: 0,
                last_d: -1e9,
                fx: 0.0,
                fy: 0.0,
                lat: 0.0,
            })
            .collect();
        Self {
            dials: Dials::default(),
            k_night: -1.0,
            k_grey: 0.0,
            k_dusk: 0.0,
            palette: DAY,
            out: Cells::new(0, 0),
            columns: 0,
            rows: 0,
            tall: false,
            rng: Rng::new(seed * 2_654_435_761.0 + 7.0),
            t: 0,
            d: 0.0,
            v: 0.0,
            steep: 0.0,
            amp: 0.0,
            rate: (2.0 * PI) / 60.0,
            phi: 0.0,
            pace: 1.0,
            wide: 1.0,
            turn: 0.0,
            moguls: 0.0,
            gates: 0.0,
            air: 0.0,
            v_air: 0.0,
            jump_at: -1.0,
            last_kick: -1e9,
            kick_x: 0.0,
            landed: 999,
            fall: 0,
            fall_t: 0,
            pw: 0,
            ph: 0,
            pix: Vec::new(),
            bits: Vec::new(),
            dcol: Vec::new(),
            kit_mask: Vec::new(),
            ground: Vec::new(),
            fit: QuadFit::default(),
            particles: Particles {
                x: vec![0.0; PMAX],
                y: vec![0.0; PMAX],
                vx: vec![0.0; PMAX],
                vy: vec![0.0; PMAX],
                life: vec![0.0; PMAX],
                max_life: vec![0.0; PMAX],
                r1: vec![0.0; PMAX],
                a0: vec![0.0; PMAX],
                kind: vec![0; PMAX],
                next: 0,
            },
            flakes: Flakes {
                x: vec![0.0; FLAKES],
                y: vec![0.0; FLAKES],
                v: vec![0.0; FLAKES],
                p: vec![0.0; FLAKES],
            },
            skiers,
        }
    }

    fn level(&self) -> usize {
        clamp(round(self.dials.strength), 0.0, 10.0) as usize
    }

    /// This frame's palette: day, night, overcast and dusk blended by the eased weights.
    fn pal(&mut self) -> Pal {
        if self.k_night < 0.0 {
            self.ease();
        }
        self.palette
    }

    /// Ease night, the overcast (a failed command) and dusk (a nearly-full
    /// context) toward the dials a little each frame, so the light changes
    /// over a second or two instead of cutting; a fresh start begins as asked.
    fn ease(&mut self) {
        let n = if self.dials.night { 1.0 } else { 0.0 };
        let g = if self.dials.tint == Tint::Smoke {
            1.0
        } else {
            0.0
        };
        let b = if self.dials.tint == Tint::Blue {
            1.0
        } else {
            0.0
        };
        if self.k_night < 0.0 {
            self.k_night = n;
            self.k_grey = g;
            self.k_dusk = b;
        }
        self.k_night = approach(self.k_night, n, 0.04);
        self.k_grey = approach(self.k_grey, g, 0.05);
        self.k_dusk = approach(self.k_dusk, b, 0.05);
        let (kn, kg, kd) = (self.k_night, self.k_grey, self.k_dusk);
        let clear = DAY.zip(&NIGHT, |a, b| mix(a, b, kn));
        let grey = GREY.zip(&NIGHT_GREY, |a, b| mix(a, b, kn));
        self.palette = clear
            .zip(&grey, |a, b| mix(a, b, kg))
            .zip(&DUSK, |a, b| mix(a, b, kd));
    }

    /// The company: one more skier per 15 of coverage, up to four.
    fn extras(&self) -> usize {
        let boost = self.dials.coverage_boost;
        if boost <= 0.0 {
            0
        } else {
            min((KITS.len() - 1) as f64, (boost / 15.0).ceil()) as usize
        }
    }

    // ------------------------------------------------------------ motion

    /// Each skier's feet this frame (layout coords), their tracks, and their spray.
    fn place(&mut self) {
        let n = self.extras();
        let tall = self.tall;
        for (i, spine_anchor) in SPINE_ANCHORS.into_iter().enumerate().take(n + 1) {
            let lat = if i == 0 && self.fall != 0 {
                self.skiers[i].lat
            } else {
                self.amp * (self.phi + self.skiers[i].off).sin()
            };
            // (Each turn's edge change is heard in flow: crabigator plays no sound.)
            let (fx, fy) = if tall {
                let half = self.room();
                (
                    self.pw as f64 / 2.0 + lat * half,
                    self.d * SPINE + round(self.ph as f64 * (spine_anchor - SPINE_ANCHORS[0])),
                )
            } else {
                // y is resolved against the ground at draw time.
                (self.d + self.anchor_x(i), 0.0)
            };
            let (d, v) = (self.d, self.v);
            // Tracks: a point per half pixel travelled.
            let on_ground = i != 0 || (self.air <= 0.0 && self.fall == 0);
            let s = &mut self.skiers[i];
            s.lat = lat;
            s.fx = fx;
            s.fy = fy;
            if on_ground && v > 0.02 && d - s.last_d >= 0.5 {
                s.last_d = d;
                let (a, b) = if tall { (fx, fy) } else { (fx, lat) };
                s.track[s.head * 2] = a as f32;
                s.track[s.head * 2 + 1] = b as f32;
                s.head = (s.head + 1) % TRK;
                s.len = TRK.min(s.len + 1);
            }
            if on_ground {
                self.spray(i, if i == 0 { 1.0 } else { 0.55 });
            }
        }
    }

    fn break_track(s: &mut Skier) {
        s.track[s.head * 2] = f32::NAN;
        s.track[s.head * 2 + 1] = f32::NAN;
        s.head = (s.head + 1) % TRK;
        s.len = TRK.min(s.len + 1);
    }

    /// The spine's half-width the skiers swing across, inside the forest.
    fn room(&self) -> f64 {
        max(2.0, self.pw as f64 / 2.0 - self.edge_w() - 3.0)
    }

    fn edge_w(&self) -> f64 {
        max(3.0, round(self.pw as f64 * 0.14))
    }

    /// A skier's place across the band; the company drift a little.
    fn anchor_x(&self, i: usize) -> f64 {
        let pw = self.pw as f64;
        let fi = i as f64;
        let drift = if i == 0 {
            0.0
        } else {
            (f64::from(self.t) * 0.007 * (1.0 + fi * 0.3) + fi).sin() * pw * 0.03
        };
        round(pw * ANCHORS[i] + drift)
    }

    #[allow(clippy::too_many_arguments)]
    fn spawn(&mut self, x: f64, y: f64, vx: f64, vy: f64, life: f64, r1: f64, a0: f64, kind: u8) {
        let p = &mut self.particles;
        let i = p.next;
        p.next = (i + 1) % PMAX;
        p.x[i] = x as f32;
        p.y[i] = y as f32;
        p.vx[i] = vx as f32;
        p.vy[i] = vy as f32;
        p.life[i] = life as f32;
        p.max_life[i] = life as f32;
        p.r1[i] = r1 as f32;
        p.a0[i] = a0 as f32;
        p.kind[i] = kind;
    }

    /// Spray off the edges: most at each turn's edge change, a powder cloud when it's deep.
    fn spray(&mut self, i: usize, k: f64) {
        let l = self.level();
        if self.v < 0.15 {
            return;
        }
        let (sfx, sfy, lat, off) = {
            let s = &self.skiers[i];
            (s.fx, s.fy, s.lat, s.off)
        };
        let edge = (self.phi + off).sin().abs().powf(6.0);
        let sp = self.v / SPEED[10];
        let rate = SPRAY[l] * (0.2 + 0.8 * edge) * k * min(1.0, self.v / max(0.3, SPEED[l]));
        let mut n = rate.floor();
        if self.rng.f() < rate - n {
            n += 1.0;
        }
        let deep = clamp((self.steep - 0.6) / 0.4, 0.0, 1.0);
        // `Math.sign(lat) || 1`: 0, -0 and NaN all go right.
        let out = if lat < 0.0 { -1.0 } else { 1.0 };
        let v = self.v;
        let mut j = 0.0;
        while j < n {
            j += 1.0;
            let billow = self.rng.f() < 0.12 + deep * 0.45;
            let r = &mut self.rng;
            // Arguments in flow's order: each draw where its argument is.
            let args = if self.tall {
                let vs = v * SPINE;
                if billow {
                    let x = sfx + (r.f() - 0.5) * 2.0;
                    let vx = out * (0.15 + r.f() * 0.35);
                    let vy = vs * (0.62 + r.f() * 0.25);
                    let life = 16.0 + r.f() * 22.0 * (0.5 + deep);
                    let r1 = 1.4 + deep * 2.6 + r.f();
                    (x, sfy - 0.5, vx, vy, life, r1, 0.8, 1)
                } else {
                    let vx = out * (0.25 + r.f() * 0.9) + (r.f() - 0.5) * 0.3;
                    let vy = vs * (0.15 + r.f() * 0.55);
                    let life = 8.0 + r.f() * 14.0;
                    (sfx + out * 1.5, sfy, vx, vy, life, 0.0, 1.0, 0)
                }
            } else if billow {
                let x = sfx - 1.5 - r.f() * 2.0;
                let y = -1.5 - r.f();
                let vx = v * (0.55 + r.f() * 0.25);
                let vy = -(0.08 + r.f() * 0.2) * (0.5 + deep);
                let life = 18.0 + r.f() * 26.0 * (0.5 + deep);
                let r1 = 1.5 + deep * 3.5 + sp + r.f();
                (x, y, vx, vy, life, r1, 0.85, 1)
            } else {
                let x = sfx - 1.0 - r.f() * 2.0;
                let vx = v * (0.3 + r.f() * 0.45);
                let vy = -(0.45 + r.f() * 0.7) * (0.6 + sp);
                let life = 10.0 + r.f() * 14.0;
                (x, -1.0, vx, vy, life, 0.0, 1.0, 0)
            };
            let (x, y, vx, vy, life, r1, a0, kind) = args;
            self.spawn(x, y, vx, vy, life, r1, a0, kind);
        }
    }

    /// A burst of snow at the hero's feet: a fall or a landing.
    fn puff(&mut self, n: usize, size: f64) {
        let (sfx, sfy) = (self.skiers[0].fx, self.skiers[0].fy);
        let v = self.v;
        for j in 0..n {
            let r = &mut self.rng;
            let a = r.f() * PI * 2.0;
            let sp = 0.2 + r.f() * 0.6;
            let kind = if j % 3 == 0 { 0 } else { 1 };
            if self.tall {
                let life = 20.0 + r.f() * 25.0;
                let r1 = (1.5 + r.f() * 2.5) * size;
                let vy = a.sin() * sp * 0.4 + v * SPINE * 0.6;
                self.spawn(
                    sfx + a.cos() * 1.5,
                    sfy - 1.0,
                    a.cos() * sp,
                    vy,
                    life,
                    r1,
                    0.85,
                    kind,
                );
            } else {
                let life = 20.0 + r.f() * 25.0;
                let r1 = (1.5 + r.f() * 2.2) * size;
                let vy = -a.sin().abs() * sp * 0.4;
                self.spawn(
                    sfx + a.cos() * 2.0,
                    -1.5,
                    a.cos() * sp + v * 0.6,
                    vy,
                    life,
                    r1,
                    0.85,
                    kind,
                );
            }
        }
    }

    fn wipeout(&mut self) {
        self.air = 0.0;
        self.v_air = 0.0;
        self.jump_at = -1.0;
        Self::break_track(&mut self.skiers[0]);
        self.puff(30, 1.3);
    }

    fn step_particles(&mut self) {
        let tall = self.tall;
        let p = &mut self.particles;
        for i in 0..PMAX {
            if p.life[i] <= 0.0 {
                continue;
            }
            add(&mut p.life[i], -1.0);
            let (vx, vy) = (f64::from(p.vx[i]), f64::from(p.vy[i]));
            add(&mut p.x[i], vx);
            add(&mut p.y[i], vy);
            if p.kind[i] == 0 {
                if tall {
                    scale(&mut p.vx[i], 0.9);
                    scale(&mut p.vy[i], 0.96);
                } else {
                    add(&mut p.vy[i], 0.07);
                    scale(&mut p.vx[i], 0.95);
                    if p.y[i] > 1.5 {
                        p.life[i] = 0.0;
                    }
                }
            } else {
                // A billow of powder hangs in the air, slowly losing the skier's speed.
                scale(&mut p.vx[i], if tall { 0.95 } else { 0.985 });
                scale(&mut p.vy[i], if tall { 0.985 } else { 0.9 });
                if !tall {
                    add(&mut p.vy[i], 0.004);
                }
            }
        }
    }

    fn step_flakes(&mut self) {
        let w = self.pw as f64;
        let depth = self.ph as f64 * 2.0;
        let flurry = self.dials.tint == Tint::Smoke;
        // The flakes stream back past the skier: left in the band, up the spine.
        let drift = if self.tall { 0.0 } else { -self.v * 0.45 };
        let climb = if self.tall {
            -self.v * SPINE * 2.0 * 0.45
        } else {
            0.0
        };
        let t = f64::from(self.t);
        let f = &mut self.flakes;
        for i in 0..FLAKES {
            let fall = f64::from(f.v[i]) * if flurry { 2.2 } else { 1.0 };
            add(&mut f.y[i], fall + climb);
            let sway = (t * 0.06 + f64::from(f.p[i])).sin() * 0.08;
            add(&mut f.x[i], drift + sway + if flurry { 0.35 } else { 0.0 });
            if f64::from(f.y[i]) >= depth {
                add(&mut f.y[i], -depth);
            }
            if f.y[i] < 0.0 {
                add(&mut f.y[i], depth);
            }
            if f64::from(f.x[i]) >= w {
                add(&mut f.x[i], -w);
            }
            if f.x[i] < 0.0 {
                add(&mut f.x[i], w);
            }
        }
    }

    // ------------------------------------------------------------ drawing

    /// A pixel. Off the layer does nothing (a NaN place too, as flow's typed array).
    fn put(&mut self, x: f64, y: f64, c: u32) {
        let (x, y) = (x.floor(), y.floor());
        if !(x >= 0.0 && y >= 0.0 && x < self.pw as f64 && y < self.ph as f64) {
            return;
        }
        self.pix[y as usize * self.pw + x as usize] = c;
    }

    /// A pixel moved `a` of the way to `c`.
    fn blend(&mut self, x: f64, y: f64, c: u32, a: f64) {
        let (x, y) = (x.floor(), y.floor());
        if !(x >= 0.0 && y >= 0.0 && x < self.pw as f64 && y < self.ph as f64) || a <= 0.02 {
            return;
        }
        let k = y as usize * self.pw + x as usize;
        self.pix[k] = mix(self.pix[k], c, a);
    }

    /// A braille dot at dot coords (one per pixel across, two per pixel down).
    fn dot(&mut self, x: f64, y: f64, c: u32) {
        let (x, y) = (x.floor(), y.floor());
        // As flow's: a NaN coordinate passes these and lands at 0 (`NaN >> 2` is 0).
        if x < 0.0 || y < 0.0 || x >= self.pw as f64 || y >= (self.ph * 2) as f64 {
            return;
        }
        let (xi, yi) = (i32_of(x) as usize, i32_of(y) as usize);
        let cell = (yi >> 2) * self.columns + (xi >> 1);
        self.bits[cell] |= BRAILLE[xi & 1][yi & 3] as u8;
        self.dcol[cell] = c;
    }

    fn dot_line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, c: u32) {
        let n = max((x1 - x0).abs(), (y1 - y0).abs()).ceil();
        if n > 60.0 {
            return;
        }
        let mut k = 0.0;
        while k <= n {
            let f = if n == 0.0 { 0.0 } else { k / n };
            self.dot(x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, c);
            k += 1.0;
        }
    }

    /// A sprite with its feet at (fx, fy); its top two rows shifted `lean` pixels.
    fn sprite(&mut self, s: &Sprite, fx: f64, fy: f64, kit: Kit, lean: f64) {
        let x0 = round(fx) - s.ax as f64;
        let y0 = round(fy) - (s.rows.len() - 1) as f64;
        let shade = mix(kit.jacket, 0, 0.32);
        for (r, row) in s.rows.iter().enumerate() {
            let sh = if r < 2 { lean } else { 0.0 };
            for (c, ch) in row.bytes().enumerate() {
                let col = match ch {
                    b'H' => kit.hat,
                    b'F' => SKIN,
                    b'R' => kit.jacket,
                    b'r' => shade,
                    b'P' => kit.pants,
                    b'S' => kit.skis,
                    b'p' => POLE,
                    _ => continue,
                };
                // In the band the skier is a few pixels tall: kept bright by night (a
                // shaded kit sinks into the dark snow); the spine has room to shade it.
                let px = x0 + c as f64 + sh;
                let py = y0 + r as f64;
                let col = if self.tall {
                    mix(col, NIGHT_SHADE, 0.3 * self.k_night)
                } else {
                    col
                };
                self.put(px, py, col);
                if px >= 0.0 && py >= 0.0 && px < self.pw as f64 && py < self.ph as f64 {
                    self.kit_mask[py as usize * self.pw + px as usize] = 1;
                }
            }
        }
    }

    /// Billows (soft, shaded underneath) and specks (braille) of snow.
    fn draw_particles(&mut self, view: View) {
        let pal = self.pal();
        let tall = self.tall;
        let w = self.pw;
        for i in 0..PMAX {
            let life = f64::from(self.particles.life[i]);
            if life <= 0.0 {
                continue;
            }
            let f = 1.0 - life / f64::from(self.particles.max_life[i]);
            let (wx, wy) = (
                f64::from(self.particles.x[i]),
                f64::from(self.particles.y[i]),
            );
            let (x, y) = match view {
                View::Band { cam } => {
                    let sxp = clamp(round(wx - cam), 0.0, (w - 1) as f64);
                    (wx - cam, f64::from(self.ground[sxp as usize]) + 1.0 + wy)
                }
                View::Spine { top } => (wx, wy - top),
            };
            if self.particles.kind[i] == 0 {
                // Specks: white against the sky, blue-grey against the snow seen from above.
                if f < 0.85 {
                    self.dot(
                        x,
                        y * 2.0,
                        if tall {
                            mix(pal.track, pal.spray, 0.25)
                        } else {
                            pal.spray
                        },
                    );
                }
                continue;
            }
            let rad = 0.6 + (f64::from(self.particles.r1[i]) - 0.6) * f.sqrt();
            let a = f64::from(self.particles.a0[i]) * (1.0 - f * f);
            let rx = rad.ceil();
            let ry = (rad / 2.0).ceil();
            let inv = 1.0 / (rad * rad);
            let cx = round(x);
            let cy = round(y);
            // Seen from above, a billow casts a soft shadow down-right on the snow.
            if tall {
                let mut dy = -ry;
                while dy <= ry {
                    let mut dx = -rx;
                    while dx <= rx {
                        let d2 = (dx * dx + 4.0 * dy * dy) * inv;
                        if d2 <= 1.0 {
                            self.blend(
                                cx + dx + 1.0,
                                cy + dy + 1.0,
                                pal.shadow,
                                a * 0.3 * (1.0 - d2),
                            );
                        }
                        dx += 1.0;
                    }
                    dy += 1.0;
                }
            }
            let mut dy = -ry;
            while dy <= ry {
                let mut dx = -rx;
                while dx <= rx {
                    let d2 = (dx * dx + 4.0 * dy * dy) * inv;
                    if d2 <= 1.0 {
                        let c = if dy > 0.0 || dx > rx / 2.0 {
                            pal.spray_shade
                        } else {
                            pal.spray
                        };
                        self.blend(cx + dx, cy + dy, c, a * (1.0 - 0.55 * d2));
                    }
                    dx += 1.0;
                }
                dy += 1.0;
            }
        }
    }

    fn draw_flakes(&mut self) {
        let l = self.level();
        let flurry = self.dials.tint == Tint::Smoke;
        let want = if flurry {
            FLAKES
        } else {
            let per = if l <= 1 { 0.035 } else { 0.02 };
            round(min(FLAKES as f64, (self.columns * self.rows) as f64 * per)) as usize
        };
        let pal = self.pal();
        let c = if self.tall {
            mix(pal.track, pal.flake, 0.4)
        } else {
            pal.flake
        };
        for i in 0..want {
            let (x, y) = (f64::from(self.flakes.x[i]), f64::from(self.flakes.y[i]));
            self.dot(x, y, c);
        }
    }

    /// A sky pixel by night: a star now and then (`sx` is its column on the
    /// slowly scrolling sky; hidden by the overcast) and the moon, a small
    /// disc at (mx, my), with a faint halo in the spine.
    #[allow(clippy::too_many_arguments)]
    fn night_sky(&self, c: u32, x: f64, y: f64, sx: f64, mx: f64, my: f64) -> u32 {
        let r = moon_radius(self.tall);
        let moon = moon_cover(x + 0.5, y + 0.5, mx, my, r);
        if moon >= 1.0 {
            return MOON;
        }
        let mut c = c;
        if self.tall {
            let d = hypot(x + 0.5 - mx, (y + 0.5 - my) * 2.0);
            if d < r * 3.0 {
                c = mix(c, MOON, 0.18 * (1.0 - d / (r * 3.0)));
            }
        }
        if hash(sx, y, 77.0) > 0.972 {
            let twinkle = 0.5 + 0.5 * hash(sx, y + f64::from(self.t >> 3), 78.0);
            c = mix(c, STAR, twinkle * (1.0 - self.k_grey));
        }
        mix(c, MOON, moon)
    }

    // ------------------------------------------------------------ compositing

    /// Each cell: plain (with braille on it if any) or the two colors that best fit its pixels.
    fn composite(&mut self) {
        let w = self.columns;
        let pw = self.pw;
        for r in 0..self.rows {
            for c in 0..w {
                let cell = r * w + c;
                let k0 = 2 * r * pw + 2 * c;
                let pix = &self.pix;
                let q = [pix[k0], pix[k0 + 1], pix[k0 + pw], pix[k0 + pw + 1]];
                let kit = &self.kit_mask;
                let keep = if kit[k0] != 0 {
                    0
                } else if kit[k0 + 1] != 0 {
                    1
                } else if kit[k0 + pw] != 0 {
                    2
                } else if kit[k0 + pw + 1] != 0 {
                    3
                } else {
                    -1
                };
                fit_quad(&q, &mut self.fit, NEAR, keep);
                let f = self.fit;
                let b = self.bits[cell];
                if f.spread <= 150 || (b != 0 && f.spread <= 2600) {
                    let avg = mix(mix(q[0], q[1], 0.5), mix(q[2], q[3], 0.5), 0.5);
                    if b != 0 {
                        self.out
                            .set(cell, 0x2800 + u32::from(b), self.dcol[cell], avg);
                    } else {
                        self.out.set(cell, 0x20, DEFAULT_COLOR, avg);
                    }
                    continue;
                }
                self.out.set(cell, QUAD[f.mask as usize], f.fg, f.bg);
            }
        }
    }
}

impl Scene for Ski {
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
        self.tall = is_tall(columns, rows);
        self.pw = columns * 2;
        self.ph = rows * 2;
        self.pix = vec![0; self.pw * self.ph];
        self.bits = vec![0; columns * rows];
        self.dcol = vec![0; columns * rows];
        self.ground = vec![0.0; self.pw];
        for s in &mut self.skiers {
            s.len = 0;
            s.last_d = -1e9;
        }
        self.particles.life.fill(0.0);
        let (pw, ph) = (self.pw as f64, self.ph as f64);
        for i in 0..FLAKES {
            self.flakes.x[i] = (self.rng.f() * pw) as f32;
            self.flakes.y[i] = (self.rng.f() * ph * 2.0) as f32;
            self.flakes.v[i] = (0.12 + self.rng.f() * 0.25) as f32;
            self.flakes.p[i] = (self.rng.f() * SWAY_TURN) as f32;
        }
    }

    fn step(&mut self) {
        if self.columns == 0 {
            return;
        }
        self.ease();
        self.t += 1;
        let l = self.level();
        if l == 0 {
            return;
        }
        // A failed command: down in a puff of snow until it's fixed, then up again.
        let smoke = self.dials.tint == Tint::Smoke;
        if smoke && self.fall != 1 {
            self.fall = 1;
            self.fall_t = 0;
            self.wipeout();
        } else if !smoke && self.fall == 1 {
            self.fall = 2;
            self.fall_t = 0;
        }
        self.fall_t += 1;
        if self.fall == 2 && self.fall_t > GET_UP {
            self.fall = 0;
        }

        let vt = if self.fall != 0 { 0.0 } else { SPEED[l] };
        if self.fall == 1 {
            self.v *= 0.86;
        } else {
            self.v += (vt - self.v) * if vt > self.v { 0.022 } else { 0.04 };
        }
        if self.v < 0.004 {
            self.v = if vt > 0.0 { self.v } else { 0.0 };
        }
        self.steep += (l as f64 / 10.0 - self.steep) * 0.03;
        let moving = min(1.0, self.v / 0.35);
        let turn = (self.phi / PI).floor();
        if turn != self.turn {
            self.turn = turn;
            self.pace = 0.75 + 0.55 * self.rng.f();
            self.wide = 0.8 + 0.4 * self.rng.f();
        }
        self.amp += (AMP[l] * self.wide * moving - self.amp) * 0.03;
        self.rate += (((2.0 * PI) / PERIOD[l]) * self.pace - self.rate) * 0.05;
        self.moguls += (MOGULS[l] - self.moguls) * 0.02;
        self.gates += (GATES[l] - self.gates) * 0.02;
        self.d += self.v;
        if self.fall == 0 && self.air <= 0.0 {
            self.phi += self.rate * moving;
        }

        // Air time off the kickers at full speed.
        self.landed += 1;
        if self.air > 0.0 || self.v_air > 0.0 {
            self.air += self.v_air;
            self.v_air -= if self.tall { 0.045 } else { 0.05 };
            if self.air <= 0.0 {
                self.air = 0.0;
                self.v_air = 0.0;
                self.landed = 0;
                self.puff(14, 1.0);
            }
        } else if self.jump_at >= 0.0 && self.d >= self.jump_at {
            self.v_air = if self.tall { 0.62 } else { 0.5 };
            self.air = 0.01;
            self.last_kick = self.jump_at;
            self.jump_at = -1.0;
            for s in &mut self.skiers {
                Self::break_track(s);
            }
        } else if l == 10
            && self.fall == 0
            && self.jump_at < 0.0
            && self.landed > 75
            && self.v > SPEED[9]
        {
            // A kicker far enough ahead to come into view.
            let ahead = if self.tall {
                (self.ph as f64 * 0.7 + 8.0) / SPINE
            } else {
                self.pw as f64 * 0.75 + 8.0
            };
            self.jump_at = self.d + ahead;
            // Where the skier will be when they reach it.
            let frames = ahead / self.v;
            self.kick_x = self.amp * (self.phi + self.rate * frames).sin();
        }

        self.place();
        self.step_particles();
        self.step_flakes();
    }

    fn grid(&mut self) -> &Cells {
        let n = self.columns * self.rows;
        if self.level() == 0 || n == 0 {
            for i in 0..n {
                self.out.blank(i);
            }
            return &self.out;
        }
        self.bits.fill(0);
        if self.kit_mask.len() != self.pw * self.ph {
            self.kit_mask = vec![0; self.pw * self.ph];
        } else {
            self.kit_mask.fill(0);
        }
        if self.tall {
            self.draw_spine();
        } else {
            self.draw_band();
        }
        self.draw_flakes();
        self.composite();
        &self.out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/ski.json"), 0.0);
    }
}
