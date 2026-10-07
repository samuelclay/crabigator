//! A Victorian steam engine room (flow's `hooks/engine.ts`) on the same
//! dials as the fire; the level is how hard it is being driven. At 1 it
//! stands cold, a dim glow in the firebox and the odd wisp from the safety
//! valve. As the level climbs, everything turns together off one crank angle
//! (its speed eases toward each level's target, so the room spins up and
//! runs down smoothly): the flywheel, the piston in its cut-away cylinder,
//! the connecting rod and valve gear, the governor's flying balls, a line
//! shaft with its pulleys and leather belts, trains of meshing cogs (each
//! turning at its gear ratio, neighbours in opposite directions), a
//! cam-driven trip hammer, a rocking beam pump, a grindstone throwing sparks,
//! gauges, brick smokestacks. By 10 spokes and teeth blur, belts race, steam
//! pours and sparks fly.
//!
//! The band lays it out sideways: the mill engine on the left, then a line
//! shaft along the top driving a run of machinery modules chosen to fill the
//! width. A tall spine stacks a vertical engine (flywheel at the foot,
//! cylinder, boiler, chimney) with belts running up both sides to a line
//! shaft overhead, gear trains beside the crosshead, and steam room at the
//! top.
//!
//! Drawing happens on two sub-cell canvases: solid parts on a quadrant grid
//! (2×2 a cell, rendered as quadrant blocks with a fg and bg color) and the
//! moving parts on a braille dot grid (2×4 a cell, the same x resolution).
//! Steam is a density field at dot resolution, drawn as dithered braille.
//! Subagents (the coverage boost) light lamps along the bed plate and thicken
//! the steam; smoke makes the engine sputter, the chimney pour sooty black
//! smoke (even cold) and the lamps burn low; a nearly-full context turns the
//! firebox to a blue gas flame, the steam and gauges blue-white, and blinks a
//! blue lamp on the boiler.
//!
//! flow's scene also queues sounds (the chuffs follow the crank, the hammer
//! clanks); crabigator plays none, so only the draws and state behind them
//! are kept.

mod draw;
mod layout;

use std::f64::consts::{PI, TAU};

use crate::flow::cells::{Cells, Rng};
use crate::flow::js::{round, u32_of};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::{clamp01, mix};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};

pub const DEF: SceneDef = SceneDef {
    name: "engine",
    hue: SceneHue {
        key: 71.0,
        range: 180.0,
        spread: None,
    },
    make: |seed| Box::new(Engine::new(seed)),
};

/// Crank radians per frame at each level (0 = off, 1 = cold and still).
const SPEED: [f64; 11] = [
    0.0, 0.0, 0.035, 0.07, 0.11, 0.15, 0.2, 0.27, 0.36, 0.47, 0.6,
];
const MAX_SPEED: f64 = SPEED[10];
const SPOKES: f64 = 6.0;
/// The line shaft turns this many times per crank turn.
const SHAFT_RATIO: f64 = 1.6;
/// Gear pitch radius per tooth, in dots (all gears share one tooth pitch, so they mesh).
const PITCH: f64 = 0.36;

/// The engine room's palette (flow's `C`).
mod c {
    pub const BRASS: u32 = 0xc8a24a;
    pub const BRASS_HI: u32 = 0xe8c66a;
    pub const BRASS_DK: u32 = 0x8f7230;
    pub const COPPER: u32 = 0xb8733a;
    pub const COPPER_DK: u32 = 0x8a5228;
    pub const IRON: u32 = 0x5a5f66;
    pub const IRON_LT: u32 = 0x8a9199;
    pub const IRON_DK: u32 = 0x3a3e44;
    pub const STEEL: u32 = 0xc6ced6;
    pub const CAVITY: u32 = 0x1d1f23;
    pub const FACE: u32 = 0xe6dcc0;
    pub const NEEDLE: u32 = 0x2b211a;
    pub const NEEDLE_HOT: u32 = 0xc0301c;
    pub const LAMP_ON: u32 = 0xffb43a;
    pub const LAMP_LOW: u32 = 0x6a4818;
    pub const LAMP_BLUE: u32 = 0x8cc4ff;
    pub const BLUE: u32 = 0x3c8cff;
    pub const BLUE_OFF: u32 = 0x2a3440;
    pub const SPARK: u32 = 0xffb040;
    pub const LEATHER: u32 = 0x9a6236;
    pub const LEATHER_HI: u32 = 0xc48a52;
    pub const WOOD: u32 = 0x8c6236;
    pub const BRICK: u32 = 0x8a3a28;
    pub const BRICK_DK: u32 = 0x6a2c20;
    pub const STONE: u32 = 0xa89878;
    pub const STONE_DK: u32 = 0x6e6450;
    pub const WALL_BRICK: u32 = 0x3c2520;
    pub const WALL_MORTAR: u32 = 0x221714;
}

/// Firebox ramp, cold to roaring: deep red → orange → yellow-white.
const FIRE: [u32; 8] = [
    0x2a0805, 0x4a0d08, 0x7a160c, 0xb02a12, 0xd8501a, 0xf08a24, 0xffc246, 0xffe9a8,
];
/// A nearly-full context: the firebox burns like a gas flame, deep blue → blue-white.
const BLUE_FIRE: [u32; 8] = [
    0x0c1640, 0x13286a, 0x1c3c9a, 0x2a58c8, 0x3c7cf0, 0x6aa6ff, 0xa8d0ff, 0xeef6ff,
];

const BRAILLE_BASE: u32 = 0x2800;
/// 4×4 ordered dither, 0..15.
const BAYER: [f64; 16] = [
    0.0, 8.0, 2.0, 10.0, 12.0, 4.0, 14.0, 6.0, 3.0, 11.0, 1.0, 9.0, 15.0, 7.0, 13.0, 5.0,
];

const MAX_PUFFS: usize = 200;
const MAX_SPARKS: usize = 64;

/// Machinery modules along the band (and the spine's bell).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Train,
    Hammer,
    Stack,
    Beam,
    Panel,
    Grind,
    Pipe,
    /// Spine only: a cam lifting a striker against a bell.
    Bell,
}

impl Kind {
    /// A fixed-width machine's width in dots (a train or pipe takes what it's given).
    fn width(self) -> f64 {
        match self {
            Kind::Hammer => 34.0,
            Kind::Stack => 12.0,
            Kind::Beam => 44.0,
            Kind::Panel => 26.0,
            Kind::Grind => 28.0,
            Kind::Train | Kind::Pipe | Kind::Bell => 0.0,
        }
    }
}

/// A cog. `rim`: only the teeth are drawn (a ring gear cut on the flywheel's rim).
#[derive(Clone, Copy, Debug)]
struct Gear {
    x: f64,
    y: f64,
    n: f64,
    r: f64,
    phase: f64,
    s: f64,
    color: u32,
    rim: bool,
}

#[derive(Clone, Copy, Debug)]
struct Pulley {
    x: f64,
    y: f64,
    r: f64,
    phase: f64,
    s: f64,
    color: u32,
    spokes: f64,
}

#[derive(Clone, Copy, Debug)]
struct Belt {
    x1: f64,
    y1: f64,
    r1: f64,
    x2: f64,
    y2: f64,
    r2: f64,
    v: f64,
    crossed: bool,
}

#[derive(Clone, Copy, Debug)]
struct Module {
    kind: Kind,
    x: f64,
    y: f64,
    w: f64,
    s: f64,
    phase: f64,
    /// The cam's lift last frame (the hammer falls when it drops).
    prev: f64,
}

#[derive(Clone, Copy, Debug)]
struct Shaft {
    y: f64,
    x0: f64,
    x1: f64,
    s: f64,
}

#[derive(Clone, Copy, Debug)]
struct Gauge {
    col: f64,
    row: f64,
    wc: f64,
    bias: f64,
}

/// Where steam leaves: 0 the engine's chimney, 1 a smokestack, 2 a leaky valve.
#[derive(Clone, Copy, Debug)]
struct Emitter {
    x: f64,
    y: f64,
    kind: u8,
}

/// Steam puffs, as parallel `Float32Array`s (no per-frame allocation).
struct Puffs {
    n: usize,
    x: [f32; MAX_PUFFS],
    y: [f32; MAX_PUFFS],
    vx: [f32; MAX_PUFFS],
    vy: [f32; MAX_PUFFS],
    r: [f32; MAX_PUFFS],
    grow: [f32; MAX_PUFFS],
    amp: [f32; MAX_PUFFS],
    age: [f32; MAX_PUFFS],
    life: [f32; MAX_PUFFS],
}

impl Puffs {
    fn new() -> Self {
        Self {
            n: 0,
            x: [0.0; MAX_PUFFS],
            y: [0.0; MAX_PUFFS],
            vx: [0.0; MAX_PUFFS],
            vy: [0.0; MAX_PUFFS],
            r: [0.0; MAX_PUFFS],
            grow: [0.0; MAX_PUFFS],
            amp: [0.0; MAX_PUFFS],
            age: [0.0; MAX_PUFFS],
            life: [0.0; MAX_PUFFS],
        }
    }

    /// Puff `j` moved into slot `i`.
    fn copy(&mut self, j: usize, i: usize) {
        self.x[i] = self.x[j];
        self.y[i] = self.y[j];
        self.vx[i] = self.vx[j];
        self.vy[i] = self.vy[j];
        self.r[i] = self.r[j];
        self.grow[i] = self.grow[j];
        self.amp[i] = self.amp[j];
        self.age[i] = self.age[j];
        self.life[i] = self.life[j];
    }
}

/// Sparks, as parallel `Float32Array`s.
struct Sparks {
    n: usize,
    x: [f32; MAX_SPARKS],
    y: [f32; MAX_SPARKS],
    vx: [f32; MAX_SPARKS],
    vy: [f32; MAX_SPARKS],
    life: [f32; MAX_SPARKS],
}

impl Sparks {
    fn new() -> Self {
        Self {
            n: 0,
            x: [0.0; MAX_SPARKS],
            y: [0.0; MAX_SPARKS],
            vx: [0.0; MAX_SPARKS],
            vy: [0.0; MAX_SPARKS],
            life: [0.0; MAX_SPARKS],
        }
    }
}

/// `n - floor(n)`.
fn frac(n: f64) -> f64 {
    n - n.floor()
}

fn fire_color(h: f64, ramp: &[u32; 8]) -> u32 {
    let x = clamp01(h) * (ramp.len() - 1) as f64;
    let i = x.floor().min((ramp.len() - 2) as f64);
    mix(ramp[i as usize], ramp[i as usize + 1], x - i)
}

/// Solid colors are stored +1 so 0 can mean "empty" (and pure black still works).
const SOLID: u32 = 0x100_0000;

/// The level the dials ask for: 0..10, whole.
fn level_of(strength: f64) -> f64 {
    round(strength).clamp(0.0, 10.0)
}

pub struct Engine {
    dials: Dials,
    columns: usize,
    rows: usize,
    out: Cells,
    rng: Rng,
    seed: u32,
    t: f64,

    // Canvases: quadrant solids, braille linkage, steam density, sparks.
    quad: Vec<u32>,
    bits: Vec<u8>,
    bcol: Vec<u32>,
    bpri: Vec<u8>,
    steam: Vec<f32>,
    spark: Vec<u8>,

    // Motion state.
    angle: f64,
    omega: f64,
    heat: f64,
    pressure: f64,
    last_half: f64,
    sputter: f64,

    puffs: Box<Puffs>,
    sparks: Box<Sparks>,

    // The machinery, built by layout().
    gears: Vec<Gear>,
    pulleys: Vec<Pulley>,
    belts: Vec<Belt>,
    modules: Vec<Module>,
    gauges: Vec<Gauge>,
    emitters: Vec<Emitter>,
    hangers: Vec<f64>,
    shafts: Vec<Shaft>,
    shaft_y: f64,
    /// Spine: cells with the engine-house brick wall behind them.
    wall: Vec<u8>,
    /// Spine: the gauge riser pipe (x, top y, bottom y in dots), x < 0 = none.
    riser_x: f64,
    riser_y0: f64,
    riser_y1: f64,
    shaft2_y: f64,

    // The main engine's geometry (dots), recomputed by layout().
    vertical: bool,
    ey: f64,
    cx: f64,
    cy: f64,
    ux: f64,
    uy: f64,
    vx: f64,
    vy: f64,
    radius: f64,
    crank: f64,
    rod_len: f64,
    cyl0: f64,
    cyl1: f64,
    piston_offset: f64,
    half_t: f64,
    chim_x: f64,
    chim_y: f64,
    valve_x: f64,
    valve_y: f64,
    gov_x: f64,
    gov_y: f64,
    gov_arm: f64,
    gov_base: f64,
    boiler_top: f64,
    boiler_bottom: f64,
}

impl Engine {
    pub fn new(seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            columns: 0,
            rows: 0,
            out: Cells::new(0, 0),
            rng: Rng::new(seed),
            seed: u32_of(seed),
            t: 0.0,
            quad: Vec::new(),
            bits: Vec::new(),
            bcol: Vec::new(),
            bpri: Vec::new(),
            steam: Vec::new(),
            spark: Vec::new(),
            angle: 0.0,
            omega: 0.0,
            heat: 0.05,
            pressure: 0.0,
            last_half: 0.0,
            sputter: 0.0,
            puffs: Box::new(Puffs::new()),
            sparks: Box::new(Sparks::new()),
            gears: Vec::new(),
            pulleys: Vec::new(),
            belts: Vec::new(),
            modules: Vec::new(),
            gauges: Vec::new(),
            emitters: Vec::new(),
            hangers: Vec::new(),
            shafts: Vec::new(),
            shaft_y: 0.0,
            wall: Vec::new(),
            riser_x: -1.0,
            riser_y0: 0.0,
            riser_y1: 0.0,
            shaft2_y: -1.0,
            vertical: false,
            ey: 0.0,
            cx: 0.0,
            cy: 0.0,
            ux: 1.0,
            uy: 0.0,
            vx: 0.0,
            vy: -1.0,
            radius: 9.6,
            crank: 5.2,
            rod_len: 21.0,
            cyl0: 0.0,
            cyl1: 0.0,
            piston_offset: 0.0,
            half_t: 4.0,
            chim_x: 0.0,
            chim_y: 0.0,
            valve_x: 0.0,
            valve_y: 0.0,
            gov_x: 0.0,
            gov_y: 0.0,
            gov_arm: 5.5,
            gov_base: 0.0,
            boiler_top: 0.0,
            boiler_bottom: 0.0,
        }
    }

    // ---- motion -------------------------------------------------------------

    fn advance(&mut self) {
        self.t += 1.0;
        let level = level_of(self.dials.strength);
        let smoke = self.dials.tint == Tint::Smoke;
        let boost = self.dials.coverage_boost;
        // A failed command makes the engine sputter: its target speed stumbles.
        if smoke && self.rng.f() < 0.04 {
            self.sputter = 6.0 + self.rng.f() * 10.0;
        }
        if self.sputter > 0.0 {
            self.sputter -= 1.0;
        }
        let stumble = if self.sputter > 0.0 { 0.45 } else { 1.0 };
        let target = SPEED[level as usize] * stumble;
        self.omega += (target - self.omega) * 0.045;
        if (target - self.omega).abs() < 0.0005 {
            self.omega = target;
        }
        self.angle += self.omega;
        if self.angle > TAU * 1000.0 {
            // Every ratio here times 1000 turns is a whole number of tooth/belt
            // periods only approximately, so re-base rarely: once every ~2000 turns.
            self.angle -= TAU * 1000.0;
        }

        let heat_target = if level <= 0.0 {
            0.0
        } else {
            0.1 + (level - 1.0) * 0.1
        };
        let damp = if smoke { 0.75 } else { 1.0 };
        self.heat += (heat_target * damp - self.heat) * 0.03;
        let p_target = if level <= 0.0 {
            0.0
        } else {
            (0.06 + (level - 1.0) * 0.1 + boost * 0.001).min(0.97)
        };
        self.pressure += (p_target - self.pressure) * 0.025;

        // Chuffs: a double-acting cylinder exhausts at each dead centre.
        let half = (self.angle / PI).floor();
        if half != self.last_half {
            self.last_half = half;
            if self.omega > 0.012 && level > 0.0 && !(smoke && self.rng.f() < 0.35) {
                self.chuff(level);
            }
        }
        // Above 5 the chimney pours more and more between chuffs, steeply toward 10.
        let extra = (level - 5.0).max(0.0).powf(1.4) * 0.1 + boost * 0.004;
        if level >= 2.0 && self.rng.f() < extra {
            let amp = 0.35 + self.rng.f() * 0.25;
            self.puff(self.chim_x, self.chim_y, amp, 1.2, level);
        }
        // A failed command: the fire smoulders and the chimney pours soot, even cold.
        if smoke && level > 0.0 && self.rng.f() < 0.3 {
            let amp = 0.75 + self.rng.f() * 0.3;
            self.puff(
                self.chim_x,
                self.chim_y,
                amp,
                1.5 + level * 0.08,
                level.max(2.0),
            );
        }
        // Cold, the safety valve lets a faint wisp go now and then.
        let leak = if level == 1.0 { 0.06 } else { 0.03 };
        if level > 0.0 && level <= 3.0 && self.rng.f() < leak {
            self.wisp(self.valve_x, self.valve_y);
        }
        if level >= 7.0 && self.rng.f() < (level - 6.0) * 0.15 && !smoke {
            self.chimney_spark(level);
        }
        // Smokestacks and leaky valves.
        for i in 1..self.emitters.len() {
            let e = self.emitters[i];
            if e.kind == 1 && level >= 2.0 && self.rng.f() < 0.05 + level * 0.035 + boost * 0.002 {
                self.puff(e.x, e.y, 0.35 + level * 0.04, 1.0 + level * 0.1, level);
            } else if e.kind == 2 && level >= 6.0 && self.rng.f() < (level - 5.0) * 0.02 {
                self.wisp(e.x, e.y);
            }
        }
        self.machine_events(level, smoke);
        self.move_puffs();
        self.move_sparks();
    }

    /// The trip hammer's blows and the grindstone's sparks.
    fn machine_events(&mut self, level: f64, smoke: bool) {
        for i in 0..self.modules.len() {
            let m = self.modules[i];
            if m.kind == Kind::Hammer {
                let lift = self.cam_lift(&m);
                // (flow hears a clank here.)
                if lift < m.prev - 0.9 && level >= 4.0 {
                    let n = if smoke {
                        1.0
                    } else {
                        1.0 + (level / 3.0).floor()
                    };
                    let mut k = 0.0;
                    while k < n {
                        let vx = (self.rng.f() - 0.5) * 1.6;
                        let vy = -(0.3 + self.rng.f() * 0.6);
                        let life = 4.0 + self.rng.f() * 5.0;
                        self.emit_spark(m.x + 28.0, self.ey + 15.5, vx, vy, life);
                        k += 1.0;
                    }
                }
                self.modules[i].prev = lift;
            } else if m.kind == Kind::Grind
                && level >= 4.0
                && !smoke
                && self.rng.f() < (level - 3.0) * 0.12
            {
                let vx = 0.5 + self.rng.f() * 1.1;
                let vy = -0.2 + self.rng.f() * 0.6;
                let life = 3.0 + self.rng.f() * 5.0;
                self.emit_spark(m.x + 18.5, self.ey + 10.5, vx, vy, life);
            }
        }
    }

    /// How far a snail cam has lifted its follower (0..1.8).
    fn cam_lift(&self, m: &Module) -> f64 {
        let a = m.phase + m.s * self.angle;
        1.8 * (1.0 - frac((2.0 * (-PI / 2.0 - a)) / TAU))
    }

    fn chuff(&mut self, level: f64) {
        // (flow hears a chuff here.)
        let amp = (0.55 + level * 0.05 + self.dials.coverage_boost * 0.004).min(1.25);
        self.puff(self.chim_x, self.chim_y, amp, 1.6 + level * 0.16, level);
    }

    /// A new puff of steam, or None when there are already as many as there may be.
    fn puff(&mut self, x: f64, y: f64, amp: f64, r: f64, level: f64) -> Option<usize> {
        if self.puffs.n >= MAX_PUFFS {
            return None;
        }
        let i = self.puffs.n;
        self.puffs.n += 1;
        let vertical = self.vertical;
        let rng = &mut self.rng;
        let p = &mut self.puffs;
        p.x[i] = (x + (rng.f() - 0.5)) as f32;
        p.y[i] = y as f32;
        if !vertical {
            p.vx[i] = (0.35 + level * 0.07 + rng.f() * 0.15) as f32;
            p.vy[i] = (-0.05 - rng.f() * 0.08) as f32;
        } else {
            p.vx[i] = ((rng.f() - 0.5) * 0.45) as f32;
            p.vy[i] = (-(0.3 + level * 0.05 + rng.f() * 0.15)) as f32;
        }
        p.r[i] = r as f32;
        p.grow[i] =
            ((0.06 + rng.f() * 0.05 + level * 0.01) * if vertical { 1.4 } else { 1.0 }) as f32;
        p.amp[i] = amp as f32;
        p.age[i] = 0.0;
        let life = if vertical { 40.0 } else { 50.0 };
        p.life[i] = (life + rng.f() * 24.0) as f32;
        Some(i)
    }

    fn wisp(&mut self, x: f64, y: f64) {
        let Some(i) = self.puff(x, y - 1.0, 0.45, 0.7, 1.0) else {
            return;
        };
        let vx = if self.vertical {
            (self.rng.f() - 0.5) * 0.2
        } else {
            0.12 + self.rng.f() * 0.1
        };
        self.puffs.vx[i] = vx as f32;
        self.puffs.vy[i] = (-0.12 - self.rng.f() * 0.08) as f32;
        self.puffs.grow[i] = 0.03;
        self.puffs.life[i] = (22.0 + self.rng.f() * 10.0) as f32;
    }

    fn move_puffs(&mut self) {
        let wd = (self.columns * 2) as f64;
        let hd = (self.rows * 4) as f64;
        let sway = (self.t * 0.045).sin() * 0.1;
        let vertical = self.vertical;
        let floor = self.ey + 1.5;
        let p = &mut self.puffs;
        let mut i = 0;
        while i < p.n {
            p.age[i] = (f64::from(p.age[i]) + 1.0) as f32;
            p.x[i] = (f64::from(p.x[i]) + (f64::from(p.vx[i]) + if vertical { sway } else { 0.0 }))
                as f32;
            p.y[i] = (f64::from(p.y[i]) + f64::from(p.vy[i])) as f32;
            p.r[i] = (f64::from(p.r[i]) + f64::from(p.grow[i])) as f32;
            if vertical {
                p.vy[i] = (f64::from(p.vy[i]) * 0.985) as f32;
            } else {
                // In the band the steam has nowhere to rise: it rolls along
                // the top and slowly spreads down as it thins.
                p.vy[i] = (f64::from(p.vy[i]) + 0.008) as f32;
                if f64::from(p.vy[i]) > 0.15 {
                    p.vy[i] = 0.15;
                }
                if f64::from(p.y[i]) < floor {
                    p.y[i] = floor as f32;
                }
            }
            let r = f64::from(p.r[i]);
            let (x, y) = (f64::from(p.x[i]), f64::from(p.y[i]));
            let gone =
                p.age[i] >= p.life[i] || x - r > wd || x + r < 0.0 || y + r < 0.0 || y - r > hd;
            if gone {
                p.n -= 1;
                p.copy(p.n, i);
            } else {
                i += 1;
            }
        }
    }

    fn emit_spark(&mut self, x: f64, y: f64, vx: f64, vy: f64, life: f64) {
        let s = &mut self.sparks;
        if s.n >= MAX_SPARKS {
            return;
        }
        let i = s.n;
        s.n += 1;
        s.x[i] = x as f32;
        s.y[i] = y as f32;
        s.vx[i] = vx as f32;
        s.vy[i] = vy as f32;
        s.life[i] = life as f32;
    }

    fn chimney_spark(&mut self, level: f64) {
        let rng = &mut self.rng;
        let (x, y, vx, vy, life) = if self.vertical {
            let x = self.chim_x + (rng.f() - 0.5) * 2.0;
            let vx = (rng.f() - 0.5) * 0.9;
            let vy = -(0.9 + rng.f() * 0.8);
            (x, self.chim_y, vx, vy, 6.0 + rng.f() * 10.0)
        } else {
            let vx = 0.5 + rng.f() * 0.9 + level * 0.04;
            let vy = -(0.2 + rng.f() * 0.5);
            (self.chim_x, self.chim_y, vx, vy, 6.0 + rng.f() * 10.0)
        };
        self.emit_spark(x, y, vx, vy, life);
    }

    fn move_sparks(&mut self) {
        let fall = if self.vertical { 0.04 } else { 0.06 };
        let s = &mut self.sparks;
        let mut i = 0;
        while i < s.n {
            s.x[i] = (f64::from(s.x[i]) + f64::from(s.vx[i])) as f32;
            s.y[i] = (f64::from(s.y[i]) + f64::from(s.vy[i])) as f32;
            s.vy[i] = (f64::from(s.vy[i]) + fall) as f32;
            s.life[i] = (f64::from(s.life[i]) - 1.0) as f32;
            if s.life[i] <= 0.0 {
                s.n -= 1;
                let j = s.n;
                s.x[i] = s.x[j];
                s.y[i] = s.y[j];
                s.vx[i] = s.vx[j];
                s.vy[i] = s.vy[j];
                s.life[i] = s.life[j];
            } else {
                i += 1;
            }
        }
    }
}

impl Scene for Engine {
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
        self.quad = vec![0; columns * 2 * rows * 2];
        self.bits = vec![0; columns * rows];
        self.bcol = vec![0; columns * rows];
        self.bpri = vec![0; columns * rows];
        self.spark = vec![0; columns * rows];
        self.steam = vec![0.0; columns * 2 * rows * 4];
        self.puffs.n = 0;
        self.sparks.n = 0;
        self.layout();
    }

    fn step(&mut self) {
        self.advance();
    }

    fn grid(&mut self) -> &Cells {
        self.draw();
        &self.out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/engine.json"), 0.0);
    }
}
