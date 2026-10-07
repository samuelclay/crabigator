//! Bubbles (flow's `hooks/bubbles.ts`): a glass of fizz on the same dials as
//! the fire; the level is the fizz. At 1 a couple of airstones let lazy
//! bubbles drift up through still water. As the level climbs more streams
//! open, the bubbles come faster and bigger, wobbling side to side, swelling
//! as they rise and catching a white glint up-left, and each one pops at the
//! surface with a ripple and a little splash of droplets. By 10 the whole
//! width is a rolling, fizzing boil.
//!
//! Everything is braille dots (2 × 4 a cell, so the dots sit square and a
//! bubble reads round): a bubble is a ring once it's big enough, a dot or a
//! speck before. White bubbles in blue water: the water fills the band,
//! lighter up by the surface and deeper toward the bottom, with slow swells
//! along it, and a head of foam fizzing along the top (thicker as it gets
//! busier), heaving where bubbles burst. Subagents (the coverage boost) open
//! more streams; smoke stirs a cloud of sediment up off the bottom and turns
//! the water murky; a nearly-full context runs the water cold, a deep indigo
//! welling up from below. By night it's a stout: near-black liquid with pale
//! tan bubbles rising, and a creamy line of head at the top.
//!
//! flow also hears each pop; crabigator plays no sound, so the scene keeps
//! only what it draws.

use crate::flow::cells::{Cells, Rng};
use crate::flow::js::{i32_of, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::{clamp, clamp01, hash1, mix, BRAILLE};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};

pub const DEF: SceneDef = SceneDef {
    name: "bubbles",
    hue: SceneHue {
        key: 254.0,
        range: 90.0,
        spread: Some(60.0),
    },
    figure: false,
    make: |seed| Box::new(Bubbles::new(seed)),
};

/// The surface's rest height, in dots from the top: room above for the splash.
const SURFACE: f64 = 2.0;
/// Bubbles in flight, droplets and sediment at most: a boil in a 250-wide band.
const MAX_BUBBLES: usize = 700;
const MAX_DROPS: usize = 400;
const MAX_SILT: usize = 900;

/// Rise, in dots a frame, at each level (a tall spine scales it up).
const RISE: [f64; 11] = [
    0.0, 0.16, 0.24, 0.32, 0.42, 0.52, 0.62, 0.74, 0.9, 1.1, 1.35,
];
/// Streams per 100 dots of width at each level.
const STREAMS: [f64; 11] = [0.0, 0.8, 1.4, 2.2, 3.0, 3.8, 4.7, 6.0, 8.0, 11.0, 15.0];
/// Bubbles a stream lets go per frame at each level.
const RATE: [f64; 11] = [
    0.0, 0.06, 0.07, 0.08, 0.09, 0.1, 0.11, 0.13, 0.16, 0.21, 0.28,
];

/// A bubble's color by brightness, deep (0) to the glint (1): pale blue-white to white.
const WATER: [u32; 5] = [0x7fb4e4, 0xa8cff0, 0xcfe4f8, 0xeef6fe, 0xffffff];
const MURK: [u32; 5] = [0x6a7068, 0x868b80, 0xa3a69a, 0xc2c3b8, 0xdedfd6];
const COLD: [u32; 5] = [0x6f86d8, 0x95a8e6, 0xbac8f2, 0xdde5fa, 0xf6f8ff];
const SILT: [u32; 3] = [0x4a3f30, 0x6b5c45, 0x8c7a5c];
/// The water: up by the surface, and down at the bottom.
const WATER_TOP: u32 = 0x2f86d0;
const WATER_BOTTOM: u32 = 0x14468e;
/// By night: a stout, near-black with a hint of brown, its bubbles pale tan up to cream.
const STOUT_TOP: u32 = 0x1c140d;
const STOUT_BOTTOM: u32 = 0x070504;
const TAN: [u32; 5] = [0x5e4630, 0x84694c, 0xab9070, 0xd2bc98, 0xf2e6cc];
/// The water by smoke (murky) and by a nearly-full context (cold), at the same two depths.
const MURK_TOP: u32 = 0x4c6670;
const MURK_BOTTOM: u32 = 0x2c3c3e;
const COLD_TOP: u32 = 0x26357e;
const COLD_BOTTOM: u32 = 0x0c1238;
/// Brightness steps, so the frame holds few distinct colors.
const STEPS: f64 = 12.0;
/// flow's round turn for a random phase (kept as written, not `TAU`, so the
/// phases match flow's).
#[allow(clippy::approx_constant)]
const TURN: f64 = 6.283;

/// What lit a cell's brightest dot.
const LIT_BUBBLE: u8 = 0;
const LIT_SURFACE: u8 = 1;
const LIT_SILT: u8 = 2;

#[derive(Clone, Copy)]
struct Bubble {
    x: f64,
    y: f64,
    /// Where it wobbles about.
    bx: f64,
    r: f64,
    /// Its radius when let go: it swells from there as it rises.
    r0: f64,
    vy: f64,
    ph: f64,
    fq: f64,
}

/// An airstone on the bottom.
struct Stream {
    x: f64,
    life: f64,
    rate: f64,
    wait: f64,
}

struct Droplet {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
}

struct Silt {
    x: f64,
    y: f64,
    vy: f64,
    ph: f64,
    life: f64,
}

struct Ripple {
    x: f64,
    age: f64,
    amp: f64,
}

/// A ramp's color at brightness `b` (0..1), quantized to STEPS.
fn ramp(stops: &[u32], b: f64) -> u32 {
    let v = (round(clamp01(b) * STEPS) / STEPS) * (stops.len() - 1) as f64;
    let i = ((stops.len() - 2) as f64).min(v.floor());
    mix(stops[i as usize], stops[i as usize + 1], v - i)
}

/// The frame's braille dots: each cell's bits, its brightest dot, and what lit it.
struct Dots {
    columns: usize,
    /// Width and height in dots.
    w: f64,
    h: f64,
    bits: Vec<u8>,
    lum: Vec<f32>,
    kind: Vec<u8>,
}

impl Dots {
    fn new(columns: usize, rows: usize) -> Self {
        Self {
            columns,
            w: (columns * 2) as f64,
            h: (rows * 4) as f64,
            bits: vec![0; columns * rows],
            lum: vec![0.0; columns * rows],
            kind: vec![0; columns * rows],
        }
    }

    /// Light one dot at (x, y), brightness v (0..1), lit by `kind`.
    fn dot(&mut self, x: f64, y: f64, v: f64, kind: u8) {
        let (px, py) = (x.floor(), y.floor());
        // (NaN passes these tests and lands in cell 0, as in flow.)
        if px < 0.0 || py < 0.0 || px >= self.w || py >= self.h {
            return;
        }
        let (px, py) = (i32_of(px), i32_of(py));
        let c = (py >> 2) as usize * self.columns + (px >> 1) as usize;
        self.bits[c] |= BRAILLE[(px & 1) as usize][(py & 3) as usize] as u8;
        if v > f64::from(self.lum[c]) {
            self.lum[c] = v as f32;
            self.kind[c] = kind;
        }
    }
}

pub struct Bubbles {
    dials: Dials,
    columns: usize,
    rows: usize,
    out: Cells,
    rng: Rng,
    /// Where the foam's fizz pattern starts (the seed as an integer).
    seed_n: i32,
    t: f64,
    /// A fresh glass (new, or resized) starts full of bubbles, not empty.
    fresh: bool,

    // Eased dials.
    s: f64,
    k_murk: f64,
    k_cold: f64,
    /// Night, eased; -1 until the first step, which starts it as asked.
    k_night: f64,

    bubbles: Vec<Bubble>,
    streams: Vec<Stream>,
    drops: Vec<Droplet>,
    silt: Vec<Silt>,
    ripples: Vec<Ripple>,

    /// Per-frame buffers, reused.
    dots: Dots,
}

impl Bubbles {
    pub fn new(seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            columns: 0,
            rows: 0,
            out: Cells::new(0, 0),
            rng: Rng::new(seed),
            seed_n: i32_of(seed),
            t: 0.0,
            fresh: true,
            s: 0.0,
            k_murk: 0.0,
            k_cold: 0.0,
            k_night: -1.0,
            bubbles: Vec::new(),
            streams: Vec::new(),
            drops: Vec::new(),
            silt: Vec::new(),
            ripples: Vec::new(),
            dots: Dots::new(0, 0),
        }
    }

    /// Width and height in dots.
    fn w(&self) -> f64 {
        (self.columns * 2) as f64
    }

    fn h(&self) -> f64 {
        (self.rows * 4) as f64
    }

    /// Rise speed now: a tall spine's column is deeper, so bubbles cross it a bit quicker.
    fn rise(&self) -> f64 {
        self.at(&RISE) * (self.h() / 20.0).max(1.0).powf(0.35)
    }

    /// The level's table value, read between whole levels.
    fn at(&self, table: &[f64; 11]) -> f64 {
        let lo = self.s.floor();
        let l = lo as usize;
        table[l] + (table[(l + 1).min(10)] - table[l]) * (self.s - lo)
    }

    /// An airstone somewhere along the bottom, clear of the others where there's room.
    fn new_stream(&mut self) -> Stream {
        let w = self.w();
        let mut x = 0.0;
        for _ in 0..6 {
            x = 2.0 + self.rng.f() * (w - 4.0);
            if self.streams.iter().all(|o| (o.x - x).abs() > 5.0) {
                break;
            }
        }
        // Airstones live ~3-12 s, then bubble up somewhere else.
        let life = 40.0 + self.rng.f() * 130.0;
        let rate = 0.6 + self.rng.f() * 0.8;
        Stream {
            x,
            life,
            rate,
            wait: 0.0,
        }
    }

    /// A bubble let go at dot x, height y (just under the bottom); fizz is a fast speck.
    fn emit(&mut self, x: f64, y: f64, fizz: bool) -> Option<Bubble> {
        if self.bubbles.len() >= MAX_BUBBLES {
            return None;
        }
        let s = self.s;
        // Bigger at higher levels: specks at 1, fat bubbles in a boil.
        let big = if fizz {
            0.0
        } else {
            self.rng.f() * self.rng.f()
        };
        let r0 = if fizz {
            0.3
        } else {
            0.55 + big * (0.3 + s * 0.12) + s * 0.03
        };
        let pace = if fizz { 1.3 } else { 0.75 + 0.3 * self.rng.f() };
        let vy = pace * self.rise() * (1.0 + r0 * 0.15);
        let ph = self.rng.f() * TURN;
        let fq = 0.12 + self.rng.f() * 0.14;
        let b = Bubble {
            x,
            y,
            bx: x,
            r: r0,
            r0,
            vy,
            ph,
            fq,
        };
        self.bubbles.push(b);
        Some(b)
    }

    /// One frame of the simulation.
    fn advance(&mut self) {
        let (w, h, s) = (self.w(), self.h(), self.s);
        let rise = self.rise();
        self.t += 1.0;
        let t = self.t;

        // Streams: as many as the level and the subagents ask for, opened and
        // closed one at a time so a change of level fades in.
        let tall = if h > w { 1.8 } else { 1.0 };
        let want =
            round(w / 100.0 * tall * self.at(&STREAMS) * (1.0 + self.dials.coverage_boost / 30.0))
                .max(if s < 1.5 { 2.0 } else { 3.0 });
        let turn = i32_of(t) & 3 == 0;
        if (self.streams.len() as f64) < want && turn {
            let st = self.new_stream();
            self.streams.push(st);
        }
        if (self.streams.len() as f64) > want && turn {
            self.streams.pop();
        }
        let rate = self.at(&RATE);
        for i in 0..self.streams.len() {
            self.streams[i].life -= 1.0;
            if self.streams[i].life <= 0.0 {
                let st = self.new_stream();
                self.streams[i] = st;
                continue;
            }
            self.streams[i].wait -= 1.0;
            if self.streams[i].wait <= 0.0 && self.rng.f() < rate * self.streams[i].rate {
                // The next one waits until this one's clear (at the size it will
                // swell to), so a stream is a string of bubbles, not a solid column.
                let x = self.streams[i].x + (self.rng.f() - 0.5) * 1.2;
                if let Some(b) = self.emit(x, h + 1.0, false) {
                    self.streams[i].wait = (b.r0 * (1.6 + s * 0.06) * 2.0 + 1.5) / b.vy;
                }
            }
        }
        // From 5 up, fizz rises from everywhere along the bottom, thickening fast
        // from 7; from 7 a boil throws up fat bubbles too.
        let mut n = w / 100.0 * ((s - 4.5).max(0.0) * 0.35 + (s - 7.0).max(0.0) * 0.5);
        while n > 0.0 {
            if self.rng.f() < n {
                let x = self.rng.f() * w;
                self.emit(x, h + 1.0, true);
            }
            n -= 1.0;
        }
        if s > 7.0 {
            let mut n = w / 100.0 * (s - 7.0) * 0.8;
            while n > 0.0 {
                if self.rng.f() < n {
                    let x = self.rng.f() * w;
                    self.emit(x, h + 2.0, false);
                }
                n -= 1.0;
            }
        }

        // Rise, swell, wobble; pop at the surface.
        let cap = 1.2 + s * 0.2 + (s - 7.0).max(0.0) * 0.15;
        let mut live = 0;
        for i in 0..self.bubbles.len() {
            let mut b = self.bubbles[i];
            b.y -= b.vy;
            b.vy = (rise * 1.5).min(b.vy * 1.003);
            // Swell as the pressure drops: the farther up, the bigger, to a cap.
            let up = clamp01((h - b.y) / h.max(20.0));
            b.r = cap.min(b.r0 * (1.0 + up * (0.6 + s * 0.06)));
            // A boil rolls: base positions drift; every bubble wobbles, more as it grows.
            if s > 6.0 {
                b.bx += (b.y * 0.07 + t * 0.03 + b.ph).sin() * (s - 6.0) * 0.03;
            }
            b.x = b.bx + (b.ph + t * b.fq).sin() * (0.15 + b.r * 0.35);
            if b.y - b.r > SURFACE {
                self.bubbles[live] = b;
                live += 1;
                continue;
            }
            self.pop(&b);
        }
        self.bubbles.truncate(live);

        // Droplets fly up and fall back into the surface.
        self.drops.retain_mut(|d| {
            d.x += d.vx;
            d.y += d.vy;
            d.vy += 0.09;
            d.y < SURFACE + 0.5 && d.x >= 0.0 && d.x < w
        });

        // Ripples spread and fade.
        self.ripples.retain_mut(|rp| {
            rp.age += 1.0;
            rp.amp *= 0.93;
            rp.amp > 0.08
        });

        // Smoke: sediment billows up off the bottom in a few puffs (each moving
        // on every ~6 s) and hangs low, swirling, settling.
        if self.k_murk > 0.05 {
            let puffs = round(w / 70.0).max(1.0);
            let mut k = w / 100.0 * 1.6 * self.k_murk;
            while k > 0.0 {
                if !(self.rng.f() >= k || self.silt.len() >= MAX_SILT) {
                    let j = f64::from(i32_of(self.rng.f() * puffs));
                    let cx = w * hash1(j * 31.0 + ((t + j * 37.0) / 90.0).floor());
                    let x = cx + (self.rng.f() + self.rng.f() - 1.0) * 8.0;
                    let y = h - self.rng.f() * 2.0;
                    let vy = (0.15 + 0.85 * self.rng.f() * self.rng.f()) * h * 0.0075;
                    let ph = self.rng.f() * TURN;
                    self.silt.push(Silt {
                        x,
                        y,
                        vy,
                        ph,
                        life: 1.0,
                    });
                }
                k -= 1.0;
            }
        }
        let fade = 0.007 + (1.0 - self.k_murk) * 0.03;
        self.silt.retain_mut(|p| {
            p.y -= p.vy;
            p.vy *= 0.985;
            p.x += (p.y * 0.3 + t * 0.05 + p.ph).sin() * 0.25;
            p.life -= fade;
            p.life > 0.0 && p.y > SURFACE + 1.0
        });
    }

    /// A bubble bursts at the surface: a ripple, and a splash for the bigger ones.
    fn pop(&mut self, b: &Bubble) {
        let amp = 0.35 + b.r * 0.35;
        if self.ripples.len() < 120 {
            self.ripples.push(Ripple {
                x: b.x,
                age: 0.0,
                amp,
            });
        }
        let n = if b.r < 0.7 {
            if self.rng.f() < 0.3 {
                1.0
            } else {
                0.0
            }
        } else {
            round(1.0 + b.r * 1.2 + self.rng.f() * 2.0)
        };
        let mut i = 0.0;
        while i < n && self.drops.len() < MAX_DROPS {
            let a = (self.rng.f() - 0.5) * 2.2;
            let v = 0.45 + self.rng.f() * 0.45 + b.r * 0.12;
            self.drops.push(Droplet {
                x: b.x,
                y: SURFACE,
                vx: a.sin() * v * 0.9,
                vy: -a.cos() * v,
            });
            i += 1.0;
        }
    }

    /// The surface's height at dot x: a slow swell plus every ripple's ring.
    fn surface_at(&self, x: f64) -> f64 {
        let t = self.t;
        let mut y =
            SURFACE + (x * 0.11 + t * 0.07).sin() * (x * 0.037 - t * 0.03).sin() * self.s * 0.06;
        for rp in &self.ripples {
            let d = (x - rp.x).abs();
            let front = rp.age * 0.7;
            if d > front + 2.0 {
                continue;
            }
            y += ((d - front) * 0.9).cos() * rp.amp * (-d * 0.08).exp();
        }
        y
    }
}

impl Scene for Bubbles {
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
        self.dots = Dots::new(columns, rows);
        self.bubbles.clear();
        self.streams.clear();
        self.drops.clear();
        self.silt.clear();
        self.ripples.clear();
        self.fresh = true;
    }

    fn step(&mut self) {
        let h = self.h();
        if self.w() == 0.0 || h == 0.0 {
            return;
        }
        let target = clamp(self.dials.strength, 0.0, 10.0);
        if target <= 0.0 {
            // Off is off: the glass empties at once. Back on, it fizzes up from
            // the bottom as the level eases in.
            self.s = 0.0;
            self.bubbles.clear();
            self.drops.clear();
            self.silt.clear();
            self.ripples.clear();
            self.streams.clear();
            return;
        }
        if self.fresh {
            self.s = target;
        }
        // Ease the level, the murk and the cold: a change takes a second or two.
        let on = |b: bool| if b { 1.0 } else { 0.0 };
        self.s += clamp(target - self.s, -0.06, 0.06);
        self.k_murk += clamp(
            on(self.dials.tint == Tint::Smoke) - self.k_murk,
            -0.05,
            0.05,
        );
        self.k_cold += clamp(on(self.dials.tint == Tint::Blue) - self.k_cold, -0.04, 0.04);
        if self.k_night < 0.0 {
            self.k_night = on(self.dials.night);
        }
        self.k_night += clamp(on(self.dials.night) - self.k_night, -0.04, 0.04);
        if self.fresh {
            // Begin mid-fizz, not with an empty glass: run the column full once.
            self.fresh = false;
            let n = (h / self.rise().max(0.1)).ceil().min(600.0);
            for _ in 0..n as usize {
                self.advance();
            }
            return;
        }
        self.advance();
    }

    fn grid(&mut self) -> &Cells {
        let n = self.columns * self.rows;
        self.dots.bits.fill(0);
        self.dots.lum.fill(0.0);
        if self.s <= 0.0 || self.dials.strength <= 0.0 {
            for i in 0..n {
                self.out.blank(i);
            }
            return &self.out;
        }
        let (w, h) = (self.columns * 2, self.h());
        let t = self.t;
        let s = self.s;

        // The surface: a faint dotted line, brighter where it's been disturbed.
        let creep = i64::from(i32_of(t) >> 3);
        for x in 0..w {
            let y = self.surface_at(x as f64 + 0.5);
            let lift = (y - SURFACE).abs();
            // Two dots in three when still (the gaps creep along), solid where it heaves.
            if lift < 0.35 && (x as i64 + creep) % 3 == 0 {
                continue;
            }
            self.dots.dot(
                x as f64,
                y + 0.5,
                0.12 + (lift * 0.4).min(0.45),
                LIT_SURFACE,
            );
        }
        // The head: foam fizzing along the surface, densest at the top and
        // thinning out below, a few dots frothing above; thicker as it gets busier.
        let head = 1.5 + s * 0.35;
        let fizz = f64::from(i32_of(t) >> 2);
        let seed_n = f64::from(self.seed_n);
        for x in 0..w {
            let x = x as f64;
            let y0 = self.surface_at(x + 0.5);
            if hash1(x * 7919.0 + fizz * 104_729.0 + seed_n) < 0.3 {
                self.dots.dot(x, y0 - 0.5, 0.6, LIT_SURFACE);
            }
            let mut d = 0.0;
            while d < head {
                let p = 0.85 * (1.0 - d / head).powf(1.3) + 0.08;
                if hash1(x * 7919.0 + (d + 1.0) * 15_485_863.0 + fizz * 104_729.0 + seed_n) < p {
                    self.dots
                        .dot(x, y0 + 0.5 + d, 0.82 - (d / head) * 0.3, LIT_SURFACE);
                }
                d += 1.0;
            }
        }
        for d in &self.drops {
            self.dots.dot(d.x, d.y, 0.85, LIT_BUBBLE);
        }
        for p in &self.silt {
            self.dots.dot(p.x, p.y, 0.3 + p.life * 0.7, LIT_SILT);
        }

        // Bubbles: a ring with a glint up-left once big enough, a dot before.
        for b in &self.bubbles {
            // Deeper bubbles sit dimmer; they brighten toward the surface.
            let depth = clamp01((b.y - SURFACE) / (h - SURFACE).max(1.0));
            let base = 0.16 + (1.0 - depth) * 0.22 + (b.r * 0.04).min(0.1);
            if b.r < 0.8 {
                self.dots.dot(b.x, b.y, base, LIT_BUBBLE);
                continue;
            }
            if b.r < 1.3 {
                // A small bubble: a diamond of four dots round an empty middle, lit
                // from the top and the left.
                self.dots.dot(b.x, b.y - 1.0, base + 0.3, LIT_BUBBLE);
                self.dots.dot(b.x - 1.0, b.y, base + 0.2, LIT_BUBBLE);
                self.dots.dot(b.x + 1.0, b.y, base - 0.05, LIT_BUBBLE);
                self.dots.dot(b.x, b.y + 1.0, base - 0.1, LIT_BUBBLE);
                continue;
            }
            let reach = (b.r + 1.0).ceil();
            let (cx, cy) = (b.x.floor(), b.y.floor());
            let mut dy = -reach;
            while dy <= reach {
                let mut dx = -reach;
                while dx <= reach {
                    let ox = cx + dx + 0.5 - b.x;
                    let oy = cy + dy + 0.5 - b.y;
                    let d = (ox * ox + oy * oy).sqrt();
                    if (d - b.r).abs() <= 0.5 {
                        // Light from up-left: that side of the rim shines, the far side dims.
                        let facing = if d > 0.0 {
                            -(ox + oy) / (d * 1.414)
                        } else {
                            0.0
                        };
                        let glint = if facing > 0.8 { 0.35 } else { 0.0 };
                        self.dots
                            .dot(cx + dx, cy + dy, base + facing * 0.2 + glint, LIT_BUBBLE);
                    }
                    dx += 1.0;
                }
                dy += 1.0;
            }
            // The glint: a dot inside, up-left of center, on the bigger ones.
            if b.r >= 2.2 {
                self.dots
                    .dot(b.x - b.r * 0.42, b.y - b.r * 0.45, 1.0, LIT_BUBBLE);
            }
        }

        // Colors: the water behind every cell, deeper by row with slow swells
        // along it (stepped, so the frame holds few colors); the cell's
        // brightest dot picks from its bubble ramp. Murk and cold blend both.
        let km = round(self.k_murk * 4.0) / 4.0;
        let kc = round(self.k_cold * 4.0) / 4.0;
        let kn = round(self.k_night.max(0.0) * 8.0) / 8.0;
        let top = mix(
            mix(mix(WATER_TOP, STOUT_TOP, kn), MURK_TOP, km),
            COLD_TOP,
            kc,
        );
        let bottom = mix(
            mix(mix(WATER_BOTTOM, STOUT_BOTTOM, kn), MURK_BOTTOM, km),
            COLD_BOTTOM,
            kc,
        );
        let deepest = ((self.rows as f64) - 1.0).max(1.0);
        for i in 0..n {
            let row = i / self.columns;
            let col = (i - row * self.columns) as f64;
            let swell = (col * 0.11 + t * 0.03).sin() * (col * 0.047 - t * 0.019).sin();
            let depth = clamp01(row as f64 / deepest + swell * 0.08);
            let bg = mix(top, bottom, round(depth * 12.0) / 12.0);
            let bits = self.dots.bits[i];
            if bits == 0 {
                self.out.set(i, 0x20, bg, bg);
                continue;
            }
            let v = f64::from(self.dots.lum[i]);
            let fg = if self.dots.kind[i] == LIT_SILT {
                ramp(&SILT, v)
            } else {
                let mut fg = ramp(&WATER, v);
                if kn > 0.0 {
                    fg = mix(fg, ramp(&TAN, v), kn);
                }
                if km > 0.0 {
                    fg = mix(fg, ramp(&MURK, v * 0.85), km);
                }
                if kc > 0.0 {
                    fg = mix(fg, ramp(&COLD, v), kc);
                }
                fg
            };
            self.out.set(i, 0x2800 | u32::from(bits), fg, bg);
        }
        &self.out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/bubbles.json"), 0.0);
    }
}
