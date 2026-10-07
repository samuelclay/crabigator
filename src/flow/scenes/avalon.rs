//! A colony ship (flow's `hooks/colony.ts`, the `avalon` scene) on the same
//! dials as the fire: the level is its speed. A long ship like the Avalon
//! holds steady (nose to the right in the band, nose up in a tall spine)
//! while three layers of stars drift past behind it: an engine block at the
//! stern, a bare truss spine, and three habitat blades twisted round that
//! spine, turning (the helix slides along the ship and each blade brightens
//! as it swings to face us, its windows showing, and darkens as it passes
//! behind). Ahead of the bow a deflector shield arcs across the lane. Rocks
//! drift in from ahead, strike the shield and burn up: an orange-white flash
//! where they hit, embers sprayed off the shield that cool and fade, and a
//! ripple of light running along it.
//!
//! At 1 the stars barely move, a slow rock comes by now and then and the
//! shield is faint. As the level climbs the stars stretch into streaks and
//! the rocks come thicker and faster; by 10 the stars blur past, the plume
//! burns long and impacts keep the shield lit. Subagents light more of the
//! habitat windows; smoke dims the engine to gray, coughs puffs out behind
//! it, and leaves the shield flickering weakly; a nearly-full context turns
//! the shield and the stars a deep, hard-glowing cyan.
//!
//! Everything is placed in a flight frame (a = along the direction of
//! travel, c = across it, both in braille dots: square, two a cell across and
//! four down) and mapped onto the screen only when drawn, so the band and the
//! spine share every rule. The hull is a pixel layer folded into quadrant
//! glyphs; the bare spine, the shield, rocks and embers are braille dots.
//!
//! Flow also sends each strike to be heard a little ahead of it; crabigator
//! plays no sound, so that (and the `heard` mark it keeps on a rock) is left
//! out: it never changed what is drawn.

use std::f64::consts::PI;

use crate::flow::cells::{is_tall, Cells, Rng, DEFAULT_COLOR};
use crate::flow::js::{fround, hypot, i32_of, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::{clamp, fit_quad, g, hash1 as hash, mix, QuadFit, BRAILLE, NEAR, QUAD};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};

pub const DEF: SceneDef = SceneDef {
    name: "avalon",
    hue: SceneHue {
        key: 240.0,
        range: 180.0,
        spread: None,
    },
    make: |seed| Box::new(Colony::new(seed)),
};

/// Cells per frame the nearest stars travel at each level (0 = off).
const SPEED: [f64; 11] = [
    0.0, 0.012, 0.03, 0.06, 0.11, 0.19, 0.32, 0.52, 0.85, 1.35, 2.1,
];
/// Depth layers: far stars drift slowest and dimmest.
const LAYERS: [f64; 3] = [0.25, 0.55, 1.0];

const ENGINE: [u32; 3] = [0x4c535c, 0x6c7580, 0x8e98a3];
const TRUSS: u32 = 0x7c8590;
const SPINE: u32 = 0x666e78;
const HUB: u32 = 0x7c8590;
/// A blade by how squarely it faces us: edge-on to full face.
const BLADE_FRONT: [u32; 3] = [0x7d8792, 0xa6b0bb, 0xd0d8e0];
const BLADE_BACK: [u32; 2] = [0x3b424b, 0x4d555f];
const WINDOW: u32 = 0xf2d06b;
const WINDOW_DIM: u32 = 0x8a7440;
const WINDOW_BACK: u32 = 0x6e5c30;
const BOW: [u32; 2] = [0x8e98a3, 0xc4ccd5];
const EMITTER: u32 = 0xbfe8ff;
const PLUME: [u32; 4] = [0xe8f4ff, 0x9fd8ff, 0x4aa8f0, 0x2a6fc0];
const PLUME_SMOKE: [u32; 2] = [0x9a9a9a, 0x707074];
const SMOKE: u32 = 0xb4b4b4;
const SMOKE_THIN: u32 = 0x606064;
/// The shield's light, faint to blazing.
const SHIELD: [u32; 6] = [0x1a3a58, 0x245a86, 0x3480bc, 0x5aaee4, 0x9ad6f6, 0xe2f6ff];
const SHIELD_BLUE: [u32; 6] = [0x06405a, 0x076a8c, 0x0a94b8, 0x12c0de, 0x5ae6f8, 0xc8fcff];
const SHIELD_SMOKE: [u32; 4] = [0x34485c, 0x4c6278, 0x6a8296, 0x8aa2b4];
/// The glow behind a bright stretch of shield (cell backgrounds).
const GLOW: [u32; 3] = [0x0c1a2a, 0x112840, 0x183a5c];
const GLOW_BLUE: [u32; 3] = [0x04222e, 0x063546, 0x094a60];
const FLASH_COLORS: [u32; 4] = [0xfff8e8, 0xffd480, 0xff9a3a, 0x8a3812];
const EMBER: [u32; 5] = [0xfff2c0, 0xffc04a, 0xff7a22, 0xc0401a, 0x6a2410];
/// Small, medium, big.
const ROCK: [u32; 3] = [0xa8a091, 0xc4baa9, 0xe0d7c6];
const HOT: u32 = 0xff9a50;
const STAR: [u32; 3] = [0x6a7488, 0xa8b2c4, 0xeef2fa];
const CYAN: [u32; 3] = [0x2a7ea0, 0x40b8dc, 0xb0f0ff];

/// Rock radii in dots, smallest first.
const ROCK_R: [f64; 3] = [2.2, 3.8, 6.2];
/// Draw order when a braille cell holds two things: the higher wins its color.
const P_SHIELD: u8 = 1;
const P_ROCK: u8 = 2;
const P_EMBER: u8 = 3;
/// Pixel kinds in the hull layer.
const K_NONE: u8 = 0;
const K_SPINE: u8 = 1;
const K_HULL: u8 = 2;
const K_WINDOW: u8 = 3;
/// Frames a strike's flash and ripple last.
const FLASH: f64 = 5.0;
const RIPPLE: f64 = 26.0;
const TAU: f64 = PI * 2.0;
/// The span of a star's twinkle phase: flow's literal, a shade under 2π.
#[allow(clippy::approx_constant)]
const PHASES: f64 = 6.283;
/// The ship and its shield, scaled from their original size.
const SHIP_SCALE: f64 = 0.9;

/// A star: `u` along the direction of travel, `v` across it, both in cells.
#[derive(Clone, Copy, Debug)]
struct Star {
    u: f64,
    v: f64,
    layer: usize,
    tw: f64,
}

/// A rock: `k` scales the level's drift speed.
#[derive(Clone, Copy, Debug)]
struct Rock {
    a: f64,
    c: f64,
    dc: f64,
    k: f64,
    size: usize,
    ang: f64,
    spin: f64,
    seed: f64,
}

/// An ember off the shield, or a puff of smoke.
#[derive(Clone, Copy, Debug)]
struct Mote {
    a: f64,
    c: f64,
    va: f64,
    vc: f64,
    life: f64,
    max: f64,
}

/// A strike on the shield: where, how long ago, and how big the rock was.
#[derive(Clone, Copy, Debug)]
struct Hit {
    a: f64,
    c: f64,
    age: f64,
    size: f64,
}

/// The ship's layout in the flight frame (dots), fixed per grid size.
#[derive(Clone, Copy, Debug, Default)]
struct Geo {
    /// The frame's length along (`A`)
    len_a: f64,
    /// and across (`Cd`).
    len_c: f64,
    /// The spine's centerline.
    c0: f64,
    /// The stern (`aS`).
    stern: f64,
    /// The nose (`aN`).
    nose: f64,
    /// Engine length (`E`).
    engine: f64,
    /// Habitat start, from the stern.
    h0: f64,
    /// Habitat end.
    h1: f64,
    /// Bow start.
    b0: f64,
    /// Blade radius (`R`).
    radius: f64,
    /// Engine half-width.
    hw: f64,
    /// The shield's vertex (`aV`).
    vertex: f64,
    /// The shield's half-width across, from the centerline.
    sw: f64,
    /// The shield's curvature radius (`Rc`).
    rc: f64,
}

impl Geo {
    /// The shield's surface at `c` across: an arc bending back round the bow.
    fn arc_a(&self, c: f64) -> f64 {
        let d = c - self.c0;
        self.vertex - (d * d) / (2.0 * self.rc)
    }
}

pub struct Colony {
    dials: Dials,
    columns: usize,
    rows: usize,
    out: Cells,
    stars: Vec<Star>,
    t: f64,
    seed_base: f64,
    rng: Rng,
    /// The eased level: changes glide rather than jump.
    level: f64,
    geo: Geo,
    // Braille layer: dot bits, color and the priority that set it, per cell.
    bits: Vec<u8>,
    dot_color: Vec<u32>,
    prio: Vec<u8>,
    /// The brightest shield light in each cell (0 = none; a `Float32Array` in flow).
    glow: Vec<f32>,
    // Smoke layer: a puff glyph and color per cell (0 = none).
    puff: Vec<u32>,
    puff_color: Vec<u32>,
    /// Cells a rock covers: the stars behind are hidden.
    solid: Vec<u8>,
    rocks: Vec<Rock>,
    embers: Vec<Mote>,
    smoke: Vec<Mote>,
    hits: Vec<Hit>,
    /// The habitat's turn (radians).
    spin: f64,
    fit: QuadFit,
}

impl Colony {
    pub fn new(seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            columns: 0,
            rows: 0,
            out: Cells::new(0, 0),
            stars: Vec::new(),
            t: 0.0,
            seed_base: seed % 100_000.0,
            rng: Rng::new(seed),
            level: f64::NAN,
            geo: Geo::default(),
            bits: Vec::new(),
            dot_color: Vec::new(),
            prio: Vec::new(),
            glow: Vec::new(),
            puff: Vec::new(),
            puff_color: Vec::new(),
            solid: Vec::new(),
            rocks: Vec::new(),
            embers: Vec::new(),
            smoke: Vec::new(),
            hits: Vec::new(),
            spin: 0.0,
            fit: QuadFit::default(),
        }
    }

    /// Tall grids (the spine) fly nose-up with the stars streaming down.
    fn is_vertical(&self) -> bool {
        is_tall(self.columns, self.rows)
    }

    /// Where the ship and its shield sit: the band's ship a little left of center, the spine's in its lower half.
    fn layout(&self) -> Geo {
        let v = self.is_vertical();
        let (columns, rows) = (self.columns as f64, self.rows as f64);
        let len_a = if v { rows * 4.0 } else { columns * 2.0 };
        let len_c = if v { columns * 2.0 } else { rows * 4.0 };
        // The spine's 4 dots fill whole cells: an even centerline across a column pair (or the band's middle row).
        let c0 = if v {
            2.0 * (columns / 2.0).floor()
        } else {
            2.0 * rows.floor()
        };
        // The shield holds its place; the ship stands well back from it, small
        // against it (the shield spans the whole frame across).
        let vertex = round(len_a * (if v { 0.52 } else { 0.45 })) + (if v { 7.0 } else { 8.0 });
        let nose = vertex - round((if v { 18.0 } else { 22.0 }) * SHIP_SCALE);
        let ls = round(
            SHIP_SCALE
                * if v {
                    len_a * 0.25
                } else {
                    clamp(len_a * 0.22, 22.0, 66.0)
                },
        );
        let engine = round((if v { 5.0 } else { 4.0 }) * SHIP_SCALE);
        let h0 = engine + round(ls * 0.1);
        let b0 = ls - round(ls * 0.11).max(6.0);
        let h1 = b0 - round(ls * 0.07).max(4.0);
        let radius = SHIP_SCALE
            * if v {
                (len_c * 0.19).min(8.0)
            } else {
                len_c * 0.22
            };
        // The shield spans a fixed width beside the ship, however wide the frame.
        let sw = (len_c / 2.0).min(radius * 2.6);
        Geo {
            len_a,
            len_c,
            c0,
            stern: nose - ls,
            nose,
            engine,
            h0,
            h1,
            b0,
            radius,
            hw: (if v { 2.5 } else { 2.0 }) * SHIP_SCALE,
            vertex,
            sw,
            // How far the arc bends back at its ends: about 2 rows in the spine, 2-3 cells in the band.
            rc: (sw * sw) / (2.0 * (if v { 9.0 } else { 6.0 }) * SHIP_SCALE),
        }
    }

    /// The stars' speed (cells per frame) at the eased level, between table steps.
    fn speed(&self) -> f64 {
        let l = clamp(self.level, 0.0, 10.0);
        let i = l.floor().min(9.0) as usize;
        SPEED[i] + (SPEED[i + 1] - SPEED[i]) * (l - i as f64)
    }

    /// How fast the rocks drift in (dots per frame): their own drift plus the ship's speed.
    fn rock_speed(&self) -> f64 {
        0.2 + self.speed() * 2.0
    }

    /// Rocks: spawn at the leading edge (rarely at 1, every second or so at 10), drift in, burn up on the shield.
    fn step_rocks(&mut self) {
        let geo = self.geo;
        // Rocks come at the shield, whole, never clipped by an edge.
        let lo = (geo.c0 - geo.sw).max(0.0);
        let hi = (geo.c0 + geo.sw).min(geo.len_c);
        let l = clamp(self.level, 0.0, 10.0);
        let rate = if l < 0.5 {
            0.0
        } else {
            0.0022 * (0.075_f64 / 0.0022).powf((l - 1.0) / 9.0)
        };
        if self.rng.f() < rate {
            let pick = self.rng.f();
            let size = if pick < 0.4 {
                2
            } else if pick < 0.75 {
                1
            } else {
                0
            };
            let r = ROCK_R[size];
            let a = geo.len_a + r + 1.0;
            let c = if hi - lo > r * 2.4 {
                lo + r * 1.15 + self.rng.f() * (hi - lo - r * 2.3)
            } else {
                geo.c0
            };
            let dc = (self.rng.f() - 0.5) * 0.08;
            let k = 0.7 + self.rng.f() * 0.6;
            let ang = self.rng.f() * TAU;
            let spin = (self.rng.f() - 0.5) * 0.09;
            let seed = f64::from(self.rng.int() % 10_000);
            self.rocks.push(Rock {
                a,
                c,
                dc,
                k,
                size,
                ang,
                spin,
                seed,
            });
        }
        let v = self.rock_speed();
        for mut rock in std::mem::take(&mut self.rocks) {
            rock.a -= v * rock.k;
            rock.c = clamp(rock.c + rock.dc, lo, hi - 1.0);
            rock.ang += rock.spin * (1.0 + v * 0.3);
            let r = ROCK_R[rock.size];
            let gap = rock.a - r * 0.75 - geo.arc_a(rock.c);
            if gap > 0.0 {
                self.rocks.push(rock);
            } else {
                self.burn(&rock);
            }
        }
    }

    /// A rock meets the shield: a flash, a ripple along it, and embers sprayed back off it.
    fn burn(&mut self, rock: &Rock) {
        let a = self.geo.arc_a(rock.c);
        let size = rock.size as f64;
        self.hits.push(Hit {
            a,
            c: rock.c,
            age: 0.0,
            size,
        });
        let n = 6 + rock.size * 5;
        let slope = (rock.c - self.geo.c0) / self.geo.rc; // the shield's tilt here
        for _ in 0..n {
            // Off the shield's face (outward, +a), fanned along it.
            let along = (self.rng.f() - 0.5) * (1.4 + size * 0.5);
            let out = 0.25 + self.rng.f() * (0.7 + size * 0.35);
            let life = 9.0 + (self.rng.f() * (10.0 + size * 6.0)).floor();
            self.embers.push(Mote {
                a: a + 0.5,
                c: rock.c,
                va: out - along * slope * 0.5,
                vc: along + out * slope * 0.5,
                life,
                max: life,
            });
        }
    }

    /// Embers cool and drift back onto the shield; (smoke tint) puffs coughed out of the engine.
    fn step_motes(&mut self) {
        let geo = self.geo;
        let rock_speed = self.rock_speed();
        let drift = 0.04 + rock_speed * 0.04;
        self.embers.retain_mut(|m| {
            m.va = m.va * 0.86 - drift;
            m.vc *= 0.88;
            m.a += m.va;
            m.c += m.vc;
            let floor = geo.arc_a(m.c) + 0.3;
            if m.a < floor {
                m.a = floor;
                m.va = 0.0;
            }
            m.life -= 1.0;
            m.life > 0.0
        });
        if self.dials.tint == Tint::Smoke && self.dials.strength > 0.0 && self.t % 3.0 == 0.0 {
            let life = 20.0 + (self.rng.f() * 16.0).floor();
            let c = geo.c0 + (self.rng.f() - 0.5) * 3.0;
            let vc = (self.rng.f() - 0.5) * 0.3;
            self.smoke.push(Mote {
                a: geo.stern - 3.0,
                c,
                va: -(0.35 + rock_speed * 0.3),
                vc,
                life,
                max: life,
            });
        }
        self.smoke.retain_mut(|m| {
            m.a += m.va;
            m.c += m.vc;
            m.life -= 1.0;
            m.life > 0.0 && m.a > -4.0
        });
    }

    fn star_color(&self, layer: usize, fade: f64) -> u32 {
        let ramp = if self.dials.tint == Tint::Blue {
            CYAN
        } else {
            STAR
        };
        let base = ramp[layer];
        let k = fade.clamp(0.15, 1.0);
        let ch = |sh: u32| round(f64::from((base >> sh) & 255) * k) as u32;
        (ch(16) << 16) | (ch(8) << 8) | ch(0)
    }

    /// The braille dot (x, y) and its cell for a flight-frame position, or `None` off the grid.
    fn dot(&self, a: f64, c: f64) -> Option<(i32, i32, usize)> {
        let vertical = self.is_vertical();
        let x = (if vertical { c } else { a }).floor();
        let y = (if vertical {
            (self.rows * 4) as f64 - a
        } else {
            c
        })
        .floor();
        if x < 0.0 || y < 0.0 || x >= (self.columns * 2) as f64 || y >= (self.rows * 4) as f64 {
            return None;
        }
        // As in flow, a NaN position falls through to dot (0, 0).
        let (x, y) = (i32_of(x), i32_of(y));
        Some((x, y, (y >> 2) as usize * self.columns + (x >> 1) as usize))
    }

    /// Light one dot at a flight-frame position.
    fn plot(&mut self, a: f64, c: f64, color: u32, prio: u8) {
        let Some((x, y, cell)) = self.dot(a, c) else {
            return;
        };
        self.bits[cell] |= BRAILLE[(x & 1) as usize][(y & 3) as usize] as u8;
        if prio >= self.prio[cell] {
            self.prio[cell] = prio;
            self.dot_color[cell] = color;
        }
    }

    /// The cell holding a flight-frame position, or `None` off the grid.
    fn cell_at(&self, a: f64, c: f64) -> Option<usize> {
        self.dot(a, c).map(|(_, _, cell)| cell)
    }

    /// A stroke, sampled finely enough that it never skips a dot.
    fn line(&mut self, a0: f64, c0: f64, a1: f64, c1: f64, color: u32, prio: u8) {
        let n = ((a1 - a0).abs().max((c1 - c0).abs()) * 1.5).ceil() + 1.0;
        let mut k = 0.0;
        while k <= n {
            self.plot(
                a0 + ((a1 - a0) * k) / n,
                c0 + ((c1 - c0) * k) / n,
                color,
                prio,
            );
            k += 1.0;
        }
    }

    /// The shield: an arc of light ahead of the bow, faint at 1 and brighter
    /// and busier with the level, shimmering along its length, with each
    /// strike's ripple running out from where it hit.
    fn draw_shield(&mut self, level: f64) {
        let geo = self.geo;
        let (c0, sw) = (geo.c0, geo.sw);
        let tint = self.dials.tint;
        let smoke = tint == Tint::Smoke;
        let blue = tint == Tint::Blue;
        let ramp: &[u32] = if smoke {
            &SHIELD_SMOKE
        } else if blue {
            &SHIELD_BLUE
        } else {
            &SHIELD
        };
        let base = if smoke {
            0.55 + level * 0.03
        } else {
            0.16 + level * 0.045 + if blue { 0.22 } else { 0.0 }
        };
        let busy = level / 10.0;
        let t = self.t;
        let ti = i32_of(t);
        // Smoke: the shield stutters, whole stretches dropping out and the rest guttering.
        let gutter = if smoke {
            if hash(self.seed_base + f64::from(ti >> 2) * 13.0) < 0.3 {
                0.35
            } else {
                0.75
            }
        } else {
            1.0
        };
        let end = geo.len_c.min(c0 + sw);
        let mut c = (c0 - sw).max(0.0) + 0.25;
        while c < end {
            let here = c;
            c += 0.5;
            if smoke
                && hash(
                    f64::from(i32_of(here / 3.0)) * 131.0
                        + f64::from(ti >> 1) * 7.0
                        + self.seed_base,
                ) < 0.4
            {
                continue;
            }
            let mut i = base;
            for hit in &self.hits {
                let d = (here - hit.c).abs();
                let front = hit.age * (0.9 + hit.size * 0.25);
                let fade = 1.0 - hit.age / RIPPLE;
                let power = 0.5 + hit.size * 0.25;
                i += (1.0 - (d - front).abs() / 2.2).max(0.0) * fade * power; // the ring running out
                if d < front {
                    i += 0.25 * fade * power; // and the lit stretch behind it
                }
                if hit.age < FLASH && d < 3.0 + hit.size {
                    i += 1.0;
                }
            }
            // The glow behind it follows the level and the strikes; the edge also shimmers, busier with the level.
            let light = i * gutter;
            i = (i
                + busy * 0.22 * (0.5 + 0.5 * (here * 0.45 - t * 0.31).sin())
                + busy * 0.12 * hash(f64::from(i32_of(here)) * 31.0 + t * 17.0))
                * gutter;
            // Its tips fade out rather than stop.
            let tip = ((sw - (here - c0).abs()) / 2.5).min(1.0);
            i *= tip;
            if i < 0.08 {
                continue;
            }
            let a = geo.arc_a(here);
            let color =
                ramp[((i * (ramp.len() as f64 - 0.5)).floor() as usize).min(ramp.len() - 1)];
            self.plot(a, here, color, P_SHIELD);
            self.plot(a + 1.0, here, color, P_SHIELD);
            if blue || i > 0.9 {
                self.plot(a + 2.0, here, color, P_SHIELD); // glowing hard: a thicker edge
            }
            if let Some(cell) = self.cell_at(a, here) {
                if !smoke && light * tip > f64::from(self.glow[cell]) {
                    self.glow[cell] = fround(light * tip) as f32;
                }
            }
        }
    }

    /// A strike's flash: an orange-white bloom on the shield, shrinking and cooling over a few frames.
    fn draw_flashes(&self, out: &mut Cells) {
        let w = self.columns;
        let vertical = self.is_vertical();
        for hit in &self.hits {
            if hit.age >= FLASH {
                continue;
            }
            let rf = (3.0 + hit.size * 1.6) * (1.0 - hit.age / (FLASH + 1.0));
            // Cell centers within rf dots of the strike (dots are square; a cell is 2 × 4).
            let x = if vertical { hit.c } else { hit.a };
            let y = if vertical {
                (self.rows * 4) as f64 - hit.a
            } else {
                hit.c
            };
            let c0 = ((x - rf) / 2.0).floor().max(0.0) as i64;
            let c1 = ((x + rf) / 2.0).floor().min(w as f64 - 1.0) as i64;
            let r0 = ((y - rf) / 4.0).floor().max(0.0) as i64;
            let r1 = ((y + rf) / 4.0).floor().min(self.rows as f64 - 1.0) as i64;
            for row in r0..=r1 {
                for col in c0..=c1 {
                    let d = hypot((col * 2 + 1) as f64 - x, (row * 4 + 2) as f64 - y) / rf;
                    if d >= 1.0 {
                        continue;
                    }
                    let cell = row as usize * w + col as usize;
                    let bg = FLASH_COLORS[((d * 2.0 + hit.age * 0.7).floor() as usize).min(3)];
                    let cp = out.code_point(cell);
                    let fg = if cp == 0x20 {
                        DEFAULT_COLOR
                    } else {
                        FLASH_COLORS[0]
                    };
                    out.set(cell, cp, fg, bg);
                }
            }
        }
    }

    /// Mark the cells within `rad` dots of (a, c) whose centers lie inside the rock.
    fn cover(&mut self, a: f64, c: f64, rad: f64) {
        let vertical = self.is_vertical();
        let h = (self.rows * 4) as f64;
        let x = if vertical { c } else { a };
        let y = if vertical { h - a } else { c };
        let c0 = ((x - rad) / 2.0).floor().max(0.0) as i64;
        let c1 = ((x + rad) / 2.0).floor().min(self.columns as f64 - 1.0) as i64;
        let r0 = ((y - rad) / 4.0).floor().max(0.0) as i64;
        let r1 = ((y + rad) / 4.0).floor().min(self.rows as f64 - 1.0) as i64;
        for row in r0..=r1 {
            for col in c0..=c1 {
                let dx = (col * 2 + 1) as f64 - x;
                let dy = (row * 4 + 2) as f64 - y;
                if dx * dx + dy * dy < rad * rad * 0.7 {
                    self.solid[row as usize * self.columns + col as usize] = 1;
                }
            }
        }
    }

    /// A lumpy rock: an outline of 6-10 corners at jittered radii, turning as it tumbles, glowing as it nears the shield.
    fn draw_rock(&mut self, rock: &Rock) {
        let r = ROCK_R[rock.size];
        let n = 6 + rock.size * 2;
        let gap = rock.a - r - self.geo.arc_a(rock.c);
        let heat = if gap < 12.0 {
            ((1.0 - gap / 12.0) * 3.0).ceil() / 3.0
        } else {
            0.0
        };
        let color = mix(ROCK[rock.size], HOT, heat * 0.8);
        self.cover(rock.a, rock.c, r);
        let (mut pa, mut pc) = (0.0, 0.0);
        for i in 0..=n {
            let j = i % n;
            let ang = rock.ang + (j as f64 / n as f64) * TAU;
            let rr = r * (0.7 + 0.45 * hash(rock.seed * 16.0 + j as f64));
            let a = rock.a + ang.cos() * rr;
            let c = rock.c + ang.sin() * rr;
            if i > 0 {
                self.line(pa, pc, a, c, color, P_ROCK);
            }
            pa = a;
            pc = c;
        }
        if rock.size == 2 {
            let ca = rock.ang + hash(rock.seed) * TAU;
            self.plot(
                rock.a + ca.cos() * r * 0.35,
                rock.c + ca.sin() * r * 0.35,
                ROCK[1],
                P_ROCK,
            );
        }
    }

    /// Smoke puffs as shade glyphs, dense near the engine, thinning as they drift off.
    fn draw_smoke(&mut self) {
        for i in 0..self.smoke.len() {
            let m = self.smoke[i];
            let Some(cell) = self.cell_at(m.a, m.c) else {
                continue;
            };
            let young = m.life / m.max > 0.5;
            if self.puff[cell] == 0x2592 {
                continue; // a thicker puff already holds the cell
            }
            self.puff[cell] = if young { 0x2592 } else { 0x2591 }; // ▒ ░
            self.puff_color[cell] = if young { SMOKE } else { SMOKE_THIN };
        }
    }

    /// The hull at a flight-frame point (s = dots from the stern, d = across
    /// from the spine): its kind and color. The habitat's three blades twist
    /// round the spine; each one's side-on height is its radius × sin(angle),
    /// and cos(angle) says how squarely it faces us.
    fn hull_at(&self, s: f64, d: f64, plume: f64, lit: f64) -> (u8, u32) {
        let geo = &self.geo;
        let ad = d.abs();
        if s < 0.0 {
            // The ion plume, tapering out behind the engine.
            if -s > plume {
                return (K_NONE, 0);
            }
            let f = -s / plume;
            if ad > 2.6 * (1.0 - f) + 0.6 {
                return (K_NONE, 0);
            }
            let ramp: &[u32] = if self.dials.tint == Tint::Smoke {
                &PLUME_SMOKE
            } else {
                &PLUME
            };
            let at = (f * ramp.len() as f64 + if ad > 1.2 { 1.0 } else { 0.0 }).floor() as usize;
            return (K_HULL, ramp[at.min(ramp.len() - 1)]);
        }
        let ls = geo.nose - geo.stern;
        if s >= ls {
            return (K_NONE, 0);
        }
        if s < geo.engine {
            // The engine block: a nozzle flaring at the very back.
            let hw = if s < 2.0 {
                geo.hw - 1.0 + s * 0.5
            } else {
                geo.hw
            };
            if ad >= hw {
                return (K_NONE, 0);
            }
            let shade = if ad > hw - 1.2 {
                0
            } else if s < 2.0 {
                1
            } else {
                2
            };
            return (K_HULL, ENGINE[shade]);
        }
        if s >= geo.b0 {
            // The bow: a rounded nose, the shield's emitter at its tip.
            let f = (s - geo.b0) / (ls - geo.b0);
            let hw = (0.9 + 2.4 * (1.0 - f * f).max(0.0).sqrt()) * SHIP_SCALE;
            if ad >= hw {
                return (K_NONE, 0);
            }
            let color = if f > 0.82 {
                EMITTER
            } else {
                BOW[if ad < hw * 0.5 { 1 } else { 0 }]
            };
            return (K_HULL, color);
        }
        let in_spine = ad < 2.0;
        if s >= geo.h0 && s < geo.h1 {
            // Hubs at both ends of the habitat, where the blades meet the spine.
            if (s < geo.h0 + 2.0 || s >= geo.h1 - 2.0) && ad < geo.radius * 0.4 {
                return (K_HULL, HUB);
            }
            let twist = (TAU * 0.55) / (geo.h1 - geo.h0);
            let (mut best_z, mut best_k, mut best_y) = (-2.0, -1.0, 0.0);
            for k in 0..3 {
                let k = f64::from(k);
                let phi = self.spin + (k * TAU) / 3.0 + twist * (s - geo.h0);
                let y = geo.radius * phi.sin();
                let z = phi.cos();
                // A blade is a broad ribbon: wider seen face-on than edge-on.
                if (d - y).abs() < 1.05 + 0.85 * z.abs() && z > best_z {
                    best_z = z;
                    best_k = k;
                    best_y = y;
                }
            }
            if best_k >= 0.0 && (best_z > 0.0 || !in_spine) {
                // A row of windows down the middle of each blade, every other step along it.
                let j = ((s - geo.h0) / 2.0).floor();
                let pane = (i32_of(j) & 1) == 0
                    && (d - best_y).abs() < 1.0
                    && hash((best_k * 977.0 + j) * 31.0 + 7.0) < lit;
                if best_z > 0.0 {
                    // Subagents switch more of them on; now and then one blinks off.
                    if pane
                        && best_z > 0.45
                        && f64::from(i32_of(self.t) >> 5) % 23.0 != (best_k * 7.0 + j) % 23.0
                    {
                        let color = if self.dials.tint == Tint::Smoke {
                            WINDOW_DIM
                        } else {
                            WINDOW
                        };
                        return (K_WINDOW, color);
                    }
                    return (
                        K_HULL,
                        BLADE_FRONT[((best_z * 3.0).floor() as usize).min(2)],
                    );
                }
                let color = if pane && best_z < -0.6 {
                    WINDOW_BACK
                } else {
                    BLADE_BACK[if best_z > -0.5 { 1 } else { 0 }]
                };
                return (K_HULL, color);
            }
        }
        if in_spine {
            return (K_SPINE, SPINE);
        }
        (K_NONE, 0)
    }

    /// The ship: its hull pixels folded into quadrant glyphs; cells holding only
    /// bare spine are drawn as an open braille truss instead.
    fn draw_ship(&mut self, out: &mut Cells, level: f64) {
        let geo = self.geo;
        let vertical = self.is_vertical();
        let w = self.columns;
        let h = self.rows;
        let hd = (h * 4) as f64;
        let smoke = self.dials.tint == Tint::Smoke;
        let flick = hash(self.seed_base + self.t * 5.0);
        let mut plume = (2.0 + level * 1.5) * (0.85 + flick * 0.3);
        if smoke {
            plume = if flick < 0.35 { 0.0 } else { plume * 0.5 }; // a failed command: the engine misfires
        }
        plume = plume.min(geo.stern - 1.0);
        let lit = (0.45 + self.dials.coverage_boost / 100.0).min(1.0);
        // The ship's box, in cells.
        let a0 = geo.stern - plume - 1.0;
        let a1 = geo.nose + 1.0;
        let span = (geo.radius + 2.5).max(geo.hw + 1.0);
        let c_lo = geo.c0 - span;
        let c_hi = geo.c0 + span;
        let (lo_x, hi_x) = if vertical { (c_lo, c_hi) } else { (a0, a1) };
        let (lo_y, hi_y) = if vertical {
            (hd - a1, hd - a0)
        } else {
            (c_lo, c_hi)
        };
        let x0 = (lo_x / 2.0).floor().max(0.0) as i64;
        let x1 = (hi_x / 2.0).floor().min(w as f64 - 1.0) as i64;
        let y0 = (lo_y / 4.0).floor().max(0.0) as i64;
        let y1 = (hi_y / 4.0).floor().min(h as f64 - 1.0) as i64;
        for row in y0..=y1 {
            for col in x0..=x1 {
                let cell = row as usize * w + col as usize;
                let (mut hull, mut spine) = (false, false);
                let mut keep = -1;
                let mut kinds = [K_NONE; 4];
                let mut colors = [0u32; 4];
                for p in 0..4 {
                    // The pixel's center in dots: half a cell across, two dots down.
                    let x = (col * 2 + (p & 1)) as f64 + 0.5;
                    let y = (row * 4 + if p & 2 != 0 { 2 } else { 0 }) as f64 + 1.0;
                    let a = if vertical { hd - y } else { x };
                    let c = if vertical { x } else { y };
                    let (kind, color) = self.hull_at(a - geo.stern, c - geo.c0, plume, lit);
                    kinds[p as usize] = kind;
                    colors[p as usize] = color;
                    if kind >= K_HULL {
                        hull = true;
                    } else if kind == K_SPINE {
                        spine = true;
                    }
                    if kind == K_WINDOW && keep < 0 {
                        keep = p as i32;
                    }
                }
                if !hull && !spine && self.inside(col, row) {
                    out.blank(cell); // the stars don't show through the ship
                    continue;
                }
                if hull {
                    let mut q = [0u32; 4];
                    for p in 0..4 {
                        q[p] = if kinds[p] != K_NONE { colors[p] } else { 0 };
                    }
                    fit_quad(&q, &mut self.fit, NEAR, keep);
                    let f = self.fit;
                    if f.spread == 0 {
                        out.set_fg(cell, 0x2588, q[0]);
                        continue;
                    }
                    let (mut mask, mut fg, mut bg) = (f.mask, f.fg, f.bg);
                    if fg == 0 {
                        // The empty pixels are the background: the hull is the glyph.
                        mask ^= 15;
                        fg = bg;
                        bg = 0;
                    }
                    out.set(
                        cell,
                        QUAD[mask as usize],
                        fg,
                        if bg == 0 { DEFAULT_COLOR } else { bg },
                    );
                } else if spine {
                    // An open truss: two rails and a zigzag between them.
                    let mut bits = 0;
                    for dx in 0..2 {
                        for dy in 0..4 {
                            let x = (col * 2 + dx) as f64 + 0.5;
                            let y = (row * 4 + dy) as f64 + 0.5;
                            let a = if vertical { hd - y } else { x };
                            let c = if vertical { x } else { y };
                            let j = (c - geo.c0 + 2.0).floor(); // 0..3 across the spine
                            if !(0.0..=3.0).contains(&j) {
                                continue;
                            }
                            let i = ((a.floor() % 6.0) + 6.0) % 6.0;
                            let rail = j == 0.0 || j == 3.0;
                            let brace = i == j + 1.0 || i == 6.0 - j - 1.0;
                            if rail || brace {
                                bits |= BRAILLE[dx as usize][dy as usize];
                            }
                        }
                    }
                    if bits != 0 {
                        out.set_fg(cell, 0x2800 | bits, TRUSS);
                    }
                }
            }
        }
    }

    /// Whether a cell's center lies inside the ship's silhouette (the habitat as a solid drum).
    fn inside(&self, col: i64, row: i64) -> bool {
        let geo = &self.geo;
        let x = (col * 2 + 1) as f64;
        let y = (row * 4 + 2) as f64;
        let vertical = self.is_vertical();
        let s = (if vertical {
            (self.rows * 4) as f64 - y
        } else {
            x
        }) - geo.stern;
        let d = ((if vertical { x } else { y }) - geo.c0).abs();
        if s < 0.0 || s >= geo.nose - geo.stern {
            return false;
        }
        if s >= geo.h0 && s < geo.h1 {
            return d < geo.radius + 1.0;
        }
        d < (if s < geo.engine { geo.hw } else { 2.0 })
    }

    /// The whole frame into `out`.
    fn draw(&mut self, out: &mut Cells) {
        let w = self.columns;
        let h = self.rows;
        for i in 0..w * h {
            out.blank(i);
        }
        if self.dials.strength <= 0.0 || w == 0 || h == 0 {
            return;
        }
        let level = clamp(self.level, 0.0, 10.0);
        let speed = self.speed();
        let vertical = self.is_vertical();
        let along = (if vertical { h } else { w }) as f64;
        let at = |u: f64, v: f64| -> usize {
            (if vertical {
                u.floor() * w as f64 + v
            } else {
                v * w as f64 + u.floor()
            }) as usize
        };
        let dir = if vertical { 1.0 } else { -1.0 };
        // The narrow spine gets shorter trails, or they fill the column.
        let max_trail = along * (if vertical { 0.25 } else { 0.6 });

        // Stars, far layers first; a fast star smears into a trail behind it.
        let streak = g(if vertical { '│' } else { '─' });
        for (layer, depth) in LAYERS.iter().enumerate() {
            for s in self.stars.iter().filter(|s| s.layer == layer) {
                let trail = max_trail.min(speed * depth * 9.0);
                let twinkle = if level <= 2.0 {
                    0.7 + 0.3 * (self.t * 0.3 + s.tw).sin()
                } else {
                    1.0
                };
                let head = s.u.floor();
                let glyph = if trail < 0.6 {
                    g(if layer == 2 { '*' } else { '·' })
                } else {
                    streak
                };
                out.set_fg(at(head, s.v), glyph, self.star_color(layer, twinkle));
                let mut k = 1.0;
                while k <= trail.floor() {
                    let u = (head - dir * k + along) % along; // the trail lies behind the star's travel
                    out.set_fg(
                        at(u, s.v),
                        streak,
                        self.star_color(layer, (1.0 - k / (trail + 1.0)) * 0.8),
                    );
                    k += 1.0;
                }
            }
        }

        // The braille layer: the shield, rocks and embers.
        self.bits.fill(0);
        self.prio.fill(0);
        self.glow.fill(0.0);
        self.puff.fill(0);
        self.solid.fill(0);
        self.draw_shield(level);
        for i in 0..self.rocks.len() {
            let rock = self.rocks[i];
            self.draw_rock(&rock);
        }
        for i in 0..self.embers.len() {
            let m = self.embers[i];
            let f = m.life / m.max;
            let shade = (((1.0 - f) * 5.0).floor() as usize).min(4);
            self.plot(m.a, m.c, EMBER[shade], P_EMBER);
        }
        self.draw_smoke();

        let blue = self.dials.tint == Tint::Blue;
        let glow_ramp = if blue { GLOW_BLUE } else { GLOW };
        let lo = if blue { 0.2 } else { 0.3 };
        for i in 0..w * h {
            let gl = f64::from(self.glow[i]);
            let bg = if gl > lo {
                glow_ramp[(((gl - lo) * 2.5).floor() as usize).min(2)]
            } else {
                DEFAULT_COLOR
            };
            let b = self.bits[i];
            if self.puff[i] != 0 {
                out.set_fg(i, self.puff[i], self.puff_color[i]);
            } else if b != 0 {
                out.set(i, 0x2800 | u32::from(b), self.dot_color[i], bg);
            } else if self.solid[i] != 0 {
                out.set(i, 0x20, DEFAULT_COLOR, bg);
            } else if bg != DEFAULT_COLOR {
                out.set(i, out.code_point(i), out.foreground(i), bg);
            }
        }
        self.draw_flashes(out);
        self.draw_ship(out, level);
    }
}

impl Scene for Colony {
    fn dials(&mut self) -> &mut Dials {
        &mut self.dials
    }

    fn ensure(&mut self, columns: usize, rows: usize) {
        if columns == self.columns && rows == self.rows {
            return;
        }
        self.columns = columns;
        self.rows = rows;
        let n = columns * rows;
        self.out = Cells::new(columns, rows);
        self.bits = vec![0; n];
        self.dot_color = vec![0; n];
        self.prio = vec![0; n];
        self.glow = vec![0.0; n];
        self.puff = vec![0; n];
        self.puff_color = vec![0; n];
        self.solid = vec![0; n];
        self.rocks.clear();
        self.embers.clear();
        self.smoke.clear();
        self.hits.clear();
        self.geo = self.layout();
        // u runs along the direction of travel, v across it; both in cells.
        let vertical = self.is_vertical();
        let along = (if vertical { rows } else { columns }) as f64;
        let across = (if vertical { columns } else { rows }) as f64;
        let count = round(n as f64 * 0.07).max(6.0) as usize;
        let base = self.seed_base;
        self.stars = (0..count)
            .map(|i| {
                let i = i as f64;
                Star {
                    u: hash(base + i * 3.0) * along,
                    v: (hash(base + i * 3.0 + 1.0) * across).floor(),
                    layer: (hash(base + i * 3.0 + 2.0) * LAYERS.len() as f64).floor() as usize,
                    tw: hash(base + i * 7.0) * PHASES,
                }
            })
            .collect();
    }

    fn step(&mut self) {
        self.t += 1.0;
        let want = clamp(self.dials.strength, 0.0, 10.0);
        if self.level.is_nan() {
            self.level = want;
        }
        self.level += if (want - self.level).abs() < 0.01 {
            want - self.level
        } else {
            (want - self.level) * 0.05
        };
        let speed = self.speed();
        let vertical = self.is_vertical();
        let along = (if vertical { self.rows } else { self.columns }) as f64;
        // Stars stream past the ship: leftward in the band (it flies right),
        // downward in the spine (it flies up).
        let dir = if vertical { 1.0 } else { -1.0 };
        for s in &mut self.stars {
            s.u += dir * speed * LAYERS[s.layer];
            if s.u < 0.0 {
                s.u += along;
            }
            if s.u >= along {
                s.u -= along;
            }
        }
        if self.columns == 0 || self.rows == 0 {
            return;
        }
        // The habitat turns at its own steady pace, whatever the speed.
        self.spin = (self.spin + 0.022) % TAU;
        for hit in &mut self.hits {
            hit.age += 1.0;
        }
        self.hits.retain(|hit| hit.age < RIPPLE);
        self.step_rocks();
        self.step_motes();
    }

    fn grid(&mut self) -> &Cells {
        let mut out = std::mem::take(&mut self.out);
        self.draw(&mut out);
        self.out = out;
        &self.out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/avalon.json"), 0.0);
    }
}
