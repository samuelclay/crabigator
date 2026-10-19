//! The fire (flow's `hooks/fire.ts`, `hooks/fire-palette.ts` and the `Ember`
//! scene in `hooks/styles.ts`).
//!
//! A Doom-fire–style automaton: heat is seeded along the bottom row by a row
//! of burners that each live a random 1.5–8 s, then propagates upward each
//! step with random cooling and sideways drift. Glyphs (░▒▓█) come from the
//! live heat; each is coloured half from a 256-colour ramp and half from a
//! smooth truecolor black-body. Sparks break off the tips and cool into
//! smoke. Strength 1 is a bed of dark embers; a nearly-full context turns a
//! low fire blue.

use std::sync::OnceLock;

use crate::flow::cells::{is_tall, Cells, Rng};
use crate::flow::crew::{failed, Crew, Mate};
use crate::flow::js::{fround, i32_of, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::{hash1, lower_block, mix, ramp_at, ramp_stops, smooth, BRAILLE};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use crate::flow::waiting::{breath, ease_wait, wait_tone};

pub const DEF: SceneDef = SceneDef {
    name: "fire",
    hue: SceneHue {
        key: 43.0,
        range: 180.0,
        spread: None,
    },
    figure: false,
    make: |seed| Box::new(Ember::new(seed)),
};

const GLYPHS: [u32; 4] = [0x2591, 0x2592, 0x2593, 0x2588]; // ░ ▒ ▓ █
/// Rows of body; any above are headroom.
const BODY_ROWS: usize = 4;
/// Extra cooling for heat rising into a headroom row: 0..HEADROOM_COOL-1 more.
const HEADROOM_COOL: u32 = 14;
const HEADROOM_COOL_MIN: u32 = 4;
/// Under smoke, only the cooler tips go gray; the core keeps its color.
const SMOKE_TIPS: f64 = 0.5;

/// Map strength to (peak heat, base coverage %).
pub fn params(strength: f64) -> (u32, u32) {
    if strength <= 0.0 {
        return (0, 0);
    }
    const PEAKS: [u32; 11] = [0, 2, 4, 7, 11, 14, 17, 20, 24, 29, 35];
    const SEEDS: [u32; 11] = [0, 10, 24, 42, 60, 74, 86, 94, 100, 100, 100];
    let s = strength.min(10.0) as usize;
    (PEAKS[s], SEEDS[s])
}

/// Burner lifetimes, in frames: ~1.5 s to ~8 s before a re-roll.
const LIFE_MIN: u32 = 20;
const LIFE_SPAN: u32 = 95;
/// Below this level a burner is out (its column seeds no heat).
const LEVEL_OUT: f64 = 0.08;

/// xterm 256-color index → 0xRRGGBB.
fn xterm(idx: u32) -> u32 {
    if idx >= 232 {
        let g = 8 + (idx - 232) * 10;
        return (g << 16) | (g << 8) | g;
    }
    if idx >= 16 {
        const LV: [u32; 6] = [0, 95, 135, 175, 215, 255];
        let i = idx - 16;
        return (LV[(i / 36) as usize] << 16)
            | (LV[((i / 6) % 6) as usize] << 8)
            | LV[(i % 6) as usize];
    }
    0xffffff
}

/// Level 1: a fire burned down to its embers, deep red to dark orange.
const EMBERS: [u32; 3] = [52, 88, 130];
const ORANGE: [u32; 6] = [130, 166, 172, 208, 214, 220];
const MIX5: [u32; 6] = [124, 160, 166, 208, 214, 220];
const RED67: [u32; 6] = [88, 124, 160, 196, 202, 208];
const RED8: [u32; 8] = [52, 88, 124, 160, 196, 202, 208, 214];
const RED9: [u32; 8] = [88, 124, 160, 196, 202, 208, 214, 220];
const RED10: [u32; 8] = [124, 160, 196, 202, 208, 214, 220, 226];
const SMOKE: [u32; 8] = [236, 238, 240, 242, 244, 246, 248, 250];
/// Context nearly full: the hottest core burns blue-white, and a low fire's embers burn blue.
const BLUE_CORE: [u32; 6] = [33, 39, 45, 51, 87, 159];
const BLUE_EMBERS: [u32; 6] = [17, 19, 26, 27, 33, 39];

fn pick(ramp: &[u32], r: f64) -> u32 {
    let n = ramp.len();
    let i = (r.clamp(0.0, 1.0) * n as f64).floor() as usize;
    ramp[i.min(n - 1)]
}

/// Color by (strength, heat ratio): embers → orange → red inferno.
pub fn color_for(strength: f64, r: f64, tint: Tint) -> u32 {
    let idx = if tint == Tint::Smoke && r < SMOKE_TIPS {
        pick(&SMOKE, r / SMOKE_TIPS)
    } else if tint == Tint::Blue && strength <= 5.0 {
        pick(&BLUE_EMBERS, r)
    } else if tint == Tint::Blue && r >= 0.6 {
        pick(&BLUE_CORE, (r - 0.6) / 0.4)
    } else if strength == 1.0 {
        pick(&EMBERS, r)
    } else if strength <= 4.0 {
        pick(&ORANGE, r)
    } else if strength == 5.0 {
        pick(&MIX5, r)
    } else if strength <= 7.0 {
        pick(&RED67, r)
    } else if strength == 8.0 {
        pick(&RED8, r)
    } else if strength == 9.0 {
        pick(&RED9, r)
    } else {
        pick(&RED10, r)
    };
    xterm(idx)
}

pub fn glyph_for(r: f64) -> u32 {
    if r < 0.18 {
        GLYPHS[0]
    } else if r < 0.52 {
        GLYPHS[1]
    } else if r < 0.86 {
        GLYPHS[2]
    } else {
        GLYPHS[3]
    }
}

// ── The truecolor ramps (fire-palette.ts) ─────────────────────────────────

type Stop = (f64, u32);

const BLACKBODY: &[Stop] = &[
    (0.0, 0x300603),
    (0.18, 0x761005),
    (0.38, 0xcc3008),
    (0.58, 0xf67c14),
    (0.78, 0xffcc48),
    (1.0, 0xfff9da),
];
/// A fire burned down: embers glowing deep red to dark orange, never yellow.
const EMBERS_RAMP: &[Stop] = &[(0.0, 0x280602), (0.5, 0x701a06), (1.0, 0xbe4a0e)];
/// A low fire (to 5) with the context nearly full: all of it burns blue.
const BLUE_EMBERS_RAMP: &[Stop] = &[(0.0, 0x0a124a), (0.5, 0x2258e2), (1.0, 0xacd8ff)];
const BLUE_CORE_RAMP: &[Stop] = &[
    (0.0, 0x300603),
    (0.3, 0x961a08),
    (0.55, 0xee7016),
    (0.75, 0x7cacff),
    (1.0, 0xe4f2ff),
];
const SMOKE_RAMP: &[Stop] = &[(0.0, 0x2c2c2e), (1.0, 0xc4c4c8)];
const WISP_SMOKE: &[Stop] = &[(0.0, 0x2e2c2e), (1.0, 0x807a78)];

const STEPS: usize = 48;

struct Luts {
    blackbody: [u32; STEPS + 1],
    embers: [u32; STEPS + 1],
    blue_embers: [u32; STEPS + 1],
    blue_core: [u32; STEPS + 1],
    smoke: [u32; STEPS + 1],
    wisp: [u32; STEPS + 1],
}

fn luts() -> &'static Luts {
    static LUTS: OnceLock<Luts> = OnceLock::new();
    LUTS.get_or_init(|| {
        let lut =
            |stops: &[Stop]| std::array::from_fn(|i| ramp_stops(stops, i as f64 / STEPS as f64));
        Luts {
            blackbody: lut(BLACKBODY),
            embers: lut(EMBERS_RAMP),
            blue_embers: lut(BLUE_EMBERS_RAMP),
            blue_core: lut(BLUE_CORE_RAMP),
            smoke: lut(SMOKE_RAMP),
            wisp: lut(WISP_SMOKE),
        }
    })
}

fn at(lut: &[u32; STEPS + 1], x: f64) -> u32 {
    lut[(round(x).max(0.0) as usize).min(STEPS)]
}

/// Drifting smoke, `t` 0 (nearly gone) .. 1 (just turned from a spark).
pub fn smoke_color(t: f64) -> u32 {
    at(&luts().wisp, t.clamp(0.0, 1.0) * STEPS as f64)
}

/// Color for heat `t` (0..1) at `strength`: low strengths never reach the
/// white-hot end, strength 1 is a bed of embers.
pub fn heat_color(strength: f64, t: f64, tint: Tint) -> u32 {
    let l = luts();
    if tint == Tint::Smoke && t < SMOKE_TIPS {
        return at(&l.smoke, (t / SMOKE_TIPS) * STEPS as f64);
    }
    let low = strength <= 5.0 && tint == Tint::Blue;
    let lut = if low {
        &l.blue_embers
    } else if tint == Tint::Blue {
        &l.blue_core
    } else if strength == 1.0 {
        &l.embers
    } else {
        &l.blackbody
    };
    let reach = if strength == 1.0 || low {
        1.0
    } else {
        0.52 + 0.048 * strength.min(10.0)
    };
    at(lut, t.clamp(0.0, 1.0) * reach * STEPS as f64)
}

// ── The automaton (fire.ts) ───────────────────────────────────────────────

pub struct AsciiFire {
    pub w: usize,
    pub h: usize,
    pub cells: Vec<u8>,
    pub strength: f64,
    gain: Vec<f32>,
    cover: Vec<f32>,
    shut: Vec<u8>,
    pub peak: u32,
    t: f64,
    // One burner per column (Float32Array / Uint16Array in flow).
    rank: Vec<f32>,
    target: Vec<f32>,
    level: Vec<f32>,
    rate: Vec<f32>,
    life: Vec<u16>,
    rng: Rng,
}

impl AsciiFire {
    pub fn new(seed: f64) -> Self {
        Self {
            w: 0,
            h: 0,
            cells: Vec::new(),
            strength: 8.0,
            gain: Vec::new(),
            cover: Vec::new(),
            shut: Vec::new(),
            peak: 1,
            t: 0.0,
            rank: Vec::new(),
            target: Vec::new(),
            level: Vec::new(),
            rate: Vec::new(),
            life: Vec::new(),
            rng: Rng::new(seed),
        }
    }

    /// Give burner `x` a new life: where it ranks, how bright, how fast it fades.
    fn reroll(&mut self, x: usize) {
        self.rank[x] = self.rng.f() as f32;
        self.target[x] = (0.6 + 0.4 * self.rng.f()) as f32;
        self.rate[x] = (0.03 + 0.07 * self.rng.f()) as f32;
        self.life[x] = (f64::from(LIFE_MIN) + (self.rng.f() * f64::from(LIFE_SPAN)).floor()) as u16;
    }

    /// Re-allocate (and reset) the grid when the render area changes size.
    pub fn ensure(&mut self, w: usize, h: usize) {
        if w == self.w && h == self.h {
            return;
        }
        self.w = w;
        self.h = h;
        self.cells = vec![0; w * h];
        self.rank = vec![0.0; w];
        self.target = vec![0.0; w];
        self.level = vec![0.0; w];
        self.rate = vec![0.0; w];
        self.life = vec![0; w];
        self.gain = vec![1.0; w];
        self.cover = vec![-1.0; w];
        self.shut = vec![0; w];
        for x in 0..w {
            self.reroll(x);
            // Stagger first lives so the burners never re-roll in lockstep.
            self.life[x] = (self.rng.f() * f64::from(LIFE_MIN + LIFE_SPAN)).floor() as u16;
        }
    }

    /// Slowly-moving height profile so flames ripple horizontally over time.
    fn base_dip(&self, x: usize, span: f64) -> f64 {
        let a = (x as f64 * 0.3 + self.t * 0.22).sin();
        let b = (x as f64 * 0.11 - self.t * 0.15).sin();
        ((((a + b) * 0.5 + 1.0) * 0.5) * span).floor()
    }

    pub fn step(&mut self) {
        let (w, h) = (self.w, self.h);
        if w == 0 || h < 2 {
            return;
        }
        let (peak, base_seed) = params(self.strength);
        let seed_pct = if peak == 0 { 0.0 } else { f64::from(base_seed) };
        if peak > 0 && peak < self.peak {
            let k = f64::from(peak) / f64::from(self.peak);
            for c in &mut self.cells {
                *c = round(f64::from(*c) * k) as u8;
            }
        }
        self.peak = peak.max(1);
        if peak == 0 {
            self.cells.fill(0);
            return;
        }
        let peak_f = f64::from(peak);
        // Seed the source row from the burners.
        let bottom = (h - 1) * w;
        let span = peak_f * 0.18;
        for x in 0..w {
            if self.life[x] == 0 {
                self.reroll(x);
            } else {
                self.life[x] -= 1;
            }
            let is_lit = f64::from(self.rank[x]) * 100.0
                < if self.cover[x] < 0.0 {
                    seed_pct
                } else {
                    f64::from(self.cover[x])
                };
            let goal = if is_lit {
                f64::from(self.target[x])
            } else {
                0.0
            };
            let lv = f64::from(self.level[x]);
            let step = f64::from(self.rate[x]);
            self.level[x] = (if lv < goal {
                goal.min(lv + step)
            } else {
                goal.max(lv - step)
            }) as f32;
            if f64::from(self.level[x]) < LEVEL_OUT {
                self.cells[bottom + x] = 0;
                continue;
            }
            let v = (peak_f - self.base_dip(x, span) - f64::from(self.rng.int() % 3))
                .max((peak_f / 2.0).floor());
            let g = if self.shut[x] != 0 {
                0.0
            } else {
                f64::from(self.gain[x])
            };
            self.cells[bottom + x] = if g <= 0.0 {
                0
            } else {
                round(v * f64::from(self.level[x]) * g).max(1.0) as u8
            };
        }
        // Propagate upward with random cooling and wind drift; rising into a
        // headroom row cools harder, so only the tallest get there.
        let stretched = h > BODY_ROWS + 1;
        let headroom = if stretched {
            1
        } else {
            h.saturating_sub(BODY_ROWS)
        };
        let stretch = if stretched {
            (BODY_ROWS as f64 / (h - 1) as f64 * 1.8).min(1.0)
        } else {
            1.0
        };
        for x in 0..w {
            for y in 1..h {
                let src = y * w + x;
                let p = self.cells[src];
                if p == 0 {
                    self.cells[src - w] = 0;
                } else {
                    let extra = if y - 1 < headroom {
                        HEADROOM_COOL_MIN + self.rng.int() % HEADROOM_COOL
                    } else {
                        0
                    };
                    let roll = self.rng.int() % 11;
                    let cool = if stretched {
                        round(f64::from(roll) * stretch) as u32
                    } else {
                        roll
                    } + extra;
                    let nx = x as i64 + (1 - i64::from(self.rng.int() % 3));
                    if nx >= 0 && (nx as usize) < w {
                        self.cells[(y - 1) * w + nx as usize] = if self.shut[nx as usize] != 0 {
                            0
                        } else {
                            u32::from(p).saturating_sub(cool) as u8
                        };
                    }
                }
            }
        }
        self.t += 1.0;
    }
}

// ── The scene (Ember, styles.ts) ──────────────────────────────────────────

struct Spark {
    x: f64,
    y: f64,
    vy: f64,
    heat: f64,
    cool: f64,
    phase: f64,
}

/// A spark's sway starts somewhere in this span: flow's own literal, not quite TAU.
#[allow(clippy::approx_constant)]
const PHASE_SPAN: f64 = 6.283;
/// Below this heat a spark has cooled into smoke: gray, slower, wider sway.
const SMOKE_AT: f64 = 0.38;
/// Cells fainter than this read as black on a dark terminal: drop them.
const FAINTEST: f64 = 0.1;

pub struct Ember {
    dials: Dials,
    crew: Crew,
    owner: Vec<i8>,
    flame_at: Vec<f32>,
    span: [f32; 12],
    puffed: std::collections::HashSet<String>,
    k_wait: f64,
    core: AsciiFire,
    rng: Rng,
    /// The heat softened: blurred over five columns, eased across frames.
    soft: Vec<f32>,
    sparks: Vec<Spark>,
    out: Cells,
    bits: Vec<u8>,
    spark: Vec<f32>,
    smoke: Vec<f32>,
    columns: usize,
    rows: usize,
    t: f64,
}

impl Ember {
    pub fn new(seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            crew: Crew::new(6, 30.0, 48.0),
            owner: Vec::new(),
            flame_at: Vec::new(),
            span: [0.0; 12],
            puffed: Default::default(),
            k_wait: 0.0,
            core: AsciiFire::new(seed),
            rng: Rng::new(f64::from(i32_of(seed) ^ 0x27d4_eb2f)),
            soft: Vec::new(),
            sparks: Vec::new(),
            out: Cells::new(0, 0),
            bits: Vec::new(),
            spark: Vec::new(),
            smoke: Vec::new(),
            columns: 0,
            rows: 0,
            t: 0.0,
        }
    }
}

impl Scene for Ember {
    fn dials(&mut self) -> &mut Dials {
        &mut self.dials
    }

    fn ensure(&mut self, columns: usize, rows: usize) {
        self.core.ensure(columns, rows);
        if columns == self.columns && rows == self.rows {
            return;
        }
        self.columns = columns;
        self.rows = rows;
        let n = columns * rows;
        self.soft = vec![0.0; n];
        self.out = Cells::new(columns, rows);
        self.bits = vec![0; n];
        self.spark = vec![0.0; n];
        self.smoke = vec![0.0; n];
        self.sparks.clear();
        self.owner = vec![-1; columns];
        self.flame_at = vec![1.0; columns];
    }

    fn step(&mut self) {
        let (w, h) = (self.columns, self.rows);
        if w == 0 || h == 0 {
            return;
        }
        self.core.strength = self.dials.strength;
        self.crew.room = self.crew_room();
        self.crew
            .update(&self.dials.agents, self.dials.coverage_boost);
        self.layout();
        self.core.step();
        self.t += 1.0;
        self.k_wait = ease_wait(self.k_wait, self.dials.waiting);
        let peak = f64::from(self.core.peak.max(1));
        let cells = &self.core.cells;
        for y in 0..h {
            for x in 0..w {
                let (mut sum, mut wt) = (0.0, 0.0);
                for dx in -2i64..=2 {
                    let nx = x as i64 + dx;
                    if nx < 0 || nx >= w as i64 {
                        continue;
                    }
                    let k = if dx == 0 {
                        3.0
                    } else if dx.abs() == 1 {
                        2.0
                    } else {
                        1.0
                    };
                    sum += (f64::from(cells[y * w + nx as usize]) / peak).min(1.0) * k;
                    wt += k;
                }
                let i = y * w + x;
                self.soft[i] = (f64::from(self.soft[i]) * 0.45 + (sum / wt) * 0.55) as f32;
            }
        }

        // Sparks off the tips: the topmost cell of each column hot enough.
        let s = self.dials.strength;
        if s == 0.0 {
            self.sparks.clear();
        }
        if s > 0.0 {
            let chance = if s == 1.0 { 0.0008 } else { 0.0022 * s };
            for x in 0..w {
                let tip = (0..h).find(|&y| f64::from(cells[y * w + x]) / peak > 0.3);
                let Some(tip) = tip else { continue };
                if self.rng.f() >= chance {
                    continue;
                }
                // Embers only let off the odd wisp of smoke, never a spark.
                let heat = if s == 1.0 {
                    SMOKE_AT
                } else {
                    0.72 + 0.25 * self.rng.f()
                };
                let x = x as f64 * 2.0 + self.rng.f() * 2.0;
                let vy = 0.2 + 0.18 * self.rng.f();
                let cool = 0.016 + 0.012 * self.rng.f();
                let phase = self.rng.f() * PHASE_SPAN;
                self.sparks.push(Spark {
                    x,
                    y: tip as f64 * 4.0,
                    vy,
                    heat,
                    cool,
                    phase,
                });
            }
        }
        // Move, cool, and compact the live sparks in place.
        let t = self.t;
        let width = (w * 2) as f64;
        self.sparks.retain_mut(|p| {
            let is_smoke = p.heat < SMOKE_AT;
            if is_smoke {
                p.vy = (p.vy * 0.985).max(0.05);
            }
            p.y -= p.vy;
            p.x += (p.y * 0.5 + t * 0.15 + p.phase).sin() * if is_smoke { 0.3 } else { 0.14 };
            p.heat -= if is_smoke { p.cool * 0.6 } else { p.cool };
            p.heat > 0.04 && p.y >= 0.0 && p.x >= 0.0 && p.x < width
        });
    }

    fn grid(&mut self) -> &Cells {
        let w = self.columns;
        let n = w * self.rows;
        let peak = f64::from(self.core.peak.max(1));
        let s = self.dials.strength;
        let tint = self.dials.tint;
        self.bits.fill(0);
        self.spark.fill(0.0);
        self.smoke.fill(0.0);
        for p in &self.sparks {
            let px = p.x.floor() as i64;
            let py = p.y.floor() as i64;
            let c = (py >> 2) * w as i64 + (px >> 1);
            if c < 0 || c >= n as i64 {
                continue;
            }
            let c = c as usize;
            self.bits[c] |= BRAILLE[(px & 1) as usize][(py & 3) as usize] as u8;
            if p.heat >= SMOKE_AT {
                self.spark[c] = fround(f64::from(self.spark[c]).max(p.heat)) as f32;
            } else {
                self.smoke[c] = fround(f64::from(self.smoke[c]).max(p.heat / SMOKE_AT)) as f32;
            }
        }
        for i in 0..n {
            let r = f64::from(self.core.cells[i]) / peak;
            if r >= FAINTEST {
                let smooth = ((r + f64::from(self.soft[i])) * 0.55).min(1.0);
                let owner = self.owner[i % w];
                let mate = if owner >= 0 {
                    self.crew.in_slot(owner as usize)
                } else {
                    None
                };
                let ramp = if self.k_wait > 0.5 { s.max(5.0) } else { s };
                let fg = if failed(mate) {
                    heat_color(s, r.min(0.98) * SMOKE_TIPS, Tint::Smoke)
                } else {
                    mix(color_for(ramp, r, tint), heat_color(s, smooth, tint), 0.5)
                };
                self.out.set_fg(i, glyph_for(r), fg);
            } else if self.bits[i] != 0 {
                let fg = if self.spark[i] > 0.0 {
                    if tint == Tint::Smoke {
                        smoke_color(1.0)
                    } else {
                        heat_color(s, f64::from(self.spark[i]), tint)
                    }
                } else {
                    smoke_color(f64::from(self.smoke[i]))
                };
                self.out.set_fg(i, 0x2800 | u32::from(self.bits[i]), fg);
            } else {
                self.out.blank(i);
            }
        }
        if self.k_wait > 0.0 && s > 0.0 {
            self.draw_coals();
        }
        if s > 0.0 {
            self.draw_crew();
        }
        wait_tone(&mut self.out, self.k_wait, self.t, 0.0, None);
        &self.out
    }
}

const COALS: [u32; 6] = [0x3a0a04, 0x781806, 0xb8320c, 0xe85a18, 0xff8c2a, 0xffbe58];
const PULSE: [u32; 6] = [0x4a1206, 0x9a2a0a, 0xe85a18, 0xffa83a, 0xffd878, 0xfff2c0];
const SHY: f64 = 4.999 / 5.0;
impl Ember {
    fn crew_size(&self) -> (f64, f64) {
        let w = self.columns as f64;
        if is_tall(self.columns, self.rows) {
            (round(w * 0.2).max(3.0), 2.0)
        } else {
            (round((w * 0.07).clamp(6.0, 14.0)), 3.0)
        }
    }
    fn crew_room(&self) -> usize {
        if is_tall(self.columns, self.rows) {
            if self.columns >= 20 {
                2
            } else if self.columns >= 13 {
                1
            } else {
                0
            }
        } else {
            let (f, g) = self.crew_size();
            ((self.columns as f64 * 0.6 / (f + g)).floor() as usize).min(6)
        }
    }
    fn flame(m: &Mate) -> f64 {
        smooth(if failed(Some(m)) {
            ((m.p - 0.75) / 0.25).clamp(0.0, 1.0)
        } else {
            ((m.p - 0.35) / 0.65).clamp(0.0, 1.0)
        })
    }
    fn layout(&mut self) {
        let w = self.columns;
        self.owner.fill(-1);
        self.flame_at.fill(1.0);
        let (fire, gap) = self.crew_size();
        let mut left = 0.0;
        let mut right = 0.0;
        for slot in 0..6 {
            let Some(m) = self.crew.in_slot(slot).cloned() else {
                continue;
            };
            let o = smooth((m.p * 2.0).clamp(0.0, 1.0));
            let g = gap * o;
            let f = fire * o;
            let (a, b, ga) = if slot % 2 == 0 {
                let b = w as f64 - right;
                let a = b - f;
                right += g + f;
                (a, b, a - g)
            } else {
                let a = left;
                let b = left + f;
                left += g + f;
                (a, b, b)
            };
            self.span[slot * 2] = a as f32;
            self.span[slot * 2 + 1] = b as f32;
            let lit = Self::flame(&m);
            for x in 0..w {
                let c = x as f64 + 0.5;
                if c >= a && c < b {
                    self.owner[x] = slot as i8;
                    self.flame_at[x] = lit as f32;
                } else if c >= ga && c < ga + g {
                    self.owner[x] = -2;
                    self.flame_at[x] = 0.0;
                }
            }
            if !m.leaving
                && lit > 0.0
                && lit < 0.9
                && self.dials.strength > 0.0
                && self.rng.f() < 0.3
            {
                self.sparks.push(Spark {
                    x: (a + self.rng.f() * (b - a)) * 2.0,
                    y: (self.rows - 1) as f64 * 4.0 + 2.0,
                    vy: 0.25 + 0.2 * self.rng.f(),
                    heat: 0.75 + 0.2 * self.rng.f(),
                    cool: 0.03 + 0.02 * self.rng.f(),
                    phase: self.rng.f() * PHASE_SPAN,
                });
            }
            if m.leaving
                && !self.puffed.contains(&m.id)
                && (!m.ok || lit < 0.15)
                && self.dials.strength > 0.0
            {
                self.puffed.insert(m.id.clone());
                self.puff(a, b, if m.ok { 6 } else { 12 });
            }
        }
        self.puffed
            .retain(|id| self.crew.mates.iter().any(|m| &m.id == id));
        let dense = (params(self.dials.strength).1 + 40).min(100) as f64;
        for x in 0..w {
            match self.owner[x] {
                -1 => {
                    self.core.gain[x] = 1.0;
                    self.core.cover[x] = -1.0;
                    self.core.shut[x] = 0;
                }
                -2 => {
                    self.core.gain[x] = 0.0;
                    self.core.cover[x] = 0.0;
                    self.core.shut[x] = 1;
                }
                o => {
                    let m = self.crew.in_slot(o as usize).unwrap();
                    let lit = f64::from(self.flame_at[x]);
                    self.core.gain[x] = (lit * (0.35 + (0.75 - 0.35) * m.busy)) as f32;
                    self.core.cover[x] = (dense * lit * (0.75 + 0.25 * m.busy)) as f32;
                    self.core.shut[x] = 0;
                }
            }
        }
    }
    fn puff(&mut self, a: f64, b: f64, n: usize) {
        let h = self.rows as f64;
        let reach = if is_tall(self.columns, self.rows) {
            round(h * 0.25).max(2.0)
        } else {
            2.0
        };
        for _ in 0..n {
            self.sparks.push(Spark {
                x: (a + self.rng.f() * (b - a).max(1.0)) * 2.0,
                y: (h - 1.0 - self.rng.f() * reach) * 4.0 + 3.0,
                vy: 0.12 + 0.14 * self.rng.f(),
                heat: SMOKE_AT * (0.8 + 0.18 * self.rng.f()),
                cool: 0.016 + 0.012 * self.rng.f(),
                phase: self.rng.f() * PHASE_SPAN,
            });
        }
    }
    fn draw_coals(&mut self) {
        let w = self.columns;
        let h = self.rows;
        let b = breath(self.t);
        let peak = f64::from(self.core.peak.max(1));
        let tall = h > 8;
        for x in 0..w {
            let xf = x as f64;
            if hash1(xf * 31.0 + 7.0) >= self.k_wait {
                continue;
            }
            let o = self.owner[x];
            if self.flame_at[x] < 0.3
                || (o >= 0 && self.crew.in_slot(o as usize).is_some_and(|m| m.waiting))
            {
                continue;
            }
            let i = (h - 1) * w + x;
            if f64::from(self.core.cells[i]) / peak >= 0.6 {
                continue;
            }
            let own = 0.45
                + 0.4 * hash1(xf * 131.0 + 3.0)
                + 0.15 * hash1(xf * 977.0 + f64::from(i32_of(self.t) >> 3));
            let heat = own * (0.4 + 0.6 * b);
            let fg = ramp_at(&COALS, (round(heat.min(1.0) * 20.0) / 20.0) * SHY);
            self.out.set_fg(
                i,
                if tall {
                    0x2588
                } else {
                    lower_block(4.0 + (hash1(xf * 17.0 + 1.0) * 5.0).floor())
                },
                fg,
            );
            if tall
                && hash1(xf * 53.0 + 11.0) < 0.7
                && f64::from(self.core.cells[i - w]) / peak < 0.45
            {
                self.out.set_fg(
                    i - w,
                    lower_block(1.0 + (hash1(xf * 59.0 + 5.0) * 4.0).floor()),
                    mix(COALS[0], fg, 0.75),
                );
            }
        }
    }
    fn draw_crew(&mut self) {
        let w = self.columns;
        let h = self.rows;
        let tall = is_tall(w, h);
        for m in &self.crew.mates {
            let x0 = round(f64::from(self.span[m.slot * 2])).max(0.0) as usize;
            let x1 = (round(f64::from(self.span[m.slot * 2 + 1])) as usize).min(w);
            if m.waiting && !m.leaving {
                let k = Self::flame(m);
                let pulse = 0.5 - 0.5 * ((self.t % 14.0 / 14.0) * 2.0 * std::f64::consts::PI).cos();
                for x in x0..x1 {
                    let own =
                        0.8 + 0.2 * hash1(x as f64 * 131.0 + 3.0 + f64::from(i32_of(self.t) >> 2));
                    let fg = ramp_at(
                        &PULSE,
                        (round((own * k * (0.45 + 0.55 * pulse)).clamp(0.0, 1.0) * 15.0) / 15.0)
                            * SHY,
                    );
                    let i = (h - 1) * w + x;
                    self.out.set_fg(
                        i,
                        if tall {
                            0x2588
                        } else {
                            lower_block(3.0 + round(pulse * 5.0))
                        },
                        fg,
                    );
                    if tall && pulse > 0.25 {
                        self.out
                            .set_fg(i - w, lower_block(1.0 + round(pulse * 6.0)), fg);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/fire.json"), 0.0);
    }
}
