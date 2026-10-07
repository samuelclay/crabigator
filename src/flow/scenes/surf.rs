//! Surf (flow's `hooks/surf.ts`): a surfer and the ocean on the same dials as
//! the fire; the level is the swell. At 1 the sea is glassy under a dawn sky
//! and the surfer sits on their board, bobbing on the ripples, waiting. As
//! the level climbs the swell builds: the surfer paddles, pops up and rides;
//! the waves grow taller, steeper and faster, crests spill white, spray blows
//! off the lip, foam trails off the back, and the world (foam, clouds, the
//! headland, gulls) streams past. By 8 it is big clean faces with the surfer
//! carving top to bottom, snapping off the lip; at 10 the lip throws all the
//! way over and the surfer crouches inside the barrel.
//!
//! The band is a side-on cross-section (the wave travels right, the camera
//! rides with it; the set's following swells come in behind). A tall spine
//! is the same wave seen up close: a towering concave face with sky above.
//!
//! Everything solid is painted into a 2 × 2-a-cell pixel layer and folded
//! into quadrant glyphs with the two colors that best fit each cell; spray,
//! foam fizz and sun glints are a braille dot layer on top. Subagents (the
//! coverage boost) put more surfers in the line-up; smoke is a wipeout under
//! a grey overcast sky; a nearly-full context turns the sea storm-blue and
//! raises a red warning flag.
//!
//! flow also hears each wave come in; crabigator plays no sound, but the
//! draw that times the next wave stays, so the frames match flow's.

use std::f64::consts::PI;

use crate::flow::cells::{is_tall, Cells, Rng};
use crate::flow::js::{hypot, i32_of, round};
use crate::flow::night::{moon_pixel, moon_radius, MOON, NIGHT_HORIZON, NIGHT_ZENITH, STAR};
use crate::flow::palette::{tint_to, SceneHue};
use crate::flow::pixels::{
    clamp, clamp01, fit_quad, g, hash1 as hash, mix, QuadFit, BRAILLE, NEAR, QUAD,
};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};

pub const DEF: SceneDef = SceneDef {
    name: "surf",
    hue: SceneHue {
        key: 244.0,
        range: 45.0,
        spread: Some(60.0),
    },
    figure: true,
    make: |seed| Box::new(Surf::new(seed)),
};

/// Pixels of water texture that stream past per frame at each level.
const SPEED: [f64; 11] = [
    0.0, 0.05, 0.12, 0.22, 0.34, 0.48, 0.64, 0.82, 1.02, 1.26, 1.55,
];

/// flow's round turn for a gull's wingbeat phase (kept as written, not
/// `TAU`, so the phases match flow's).
#[allow(clippy::approx_constant)]
const TURN: f64 = 6.28;

const GULL_UP: u32 = g('v');
const GULL_DOWN: u32 = g('^');

/// A palette's colors, by the indices below (in flow's key order, which
/// `update_palette` blends in).
type Palette = [u32; 16];
const SKY_TOP: usize = 0;
const SKY_HZ: usize = 1;
const SUN: usize = 2;
const SUN_GLOW: usize = 3;
const CLOUD: usize = 4;
const CLOUD_SHADE: usize = 5;
const HEAD: usize = 6;
const SEA_TOP: usize = 7;
const SEA_DEEP: usize = 8;
const FACE_DEEP: usize = 9;
const FACE_LIP: usize = 10;
const BACK: usize = 11;
const TUBE: usize = 12;
const FOAM: usize = 13;
const FOAM_SHADE: usize = 14;
const SPRAY: usize = 15;

const DAY: Palette = [
    0x2f7fd0, // sky top
    0xa9daf4, // sky at the horizon
    0xfff6d0, // sun
    0xffe39a, // sun glow
    0xf7fbff, // cloud
    0xdbe8f3, // cloud shade
    0x2f5a5a, // headland
    0x1f8ab8, // sea top
    0x0a2f5a, // sea deep
    0x0b4670, // face deep
    0x66d2ec, // face lip
    0x2c96c4, // back
    0x123f66, // tube
    0xf4fbff, // foam
    0xbfe0ec, // foam shade
    0xeaf7ff, // spray
];
/// A calm sea's dawn: only the sky, the sun and the headland change.
const DAWN: [Option<u32>; 16] = {
    let mut p = [None; 16];
    p[SKY_TOP] = Some(0x5a86c8);
    p[SKY_HZ] = Some(0xf8c79c);
    p[SUN] = Some(0xfff0c8);
    p[SUN_GLOW] = Some(0xffb070);
    p[HEAD] = Some(0x6f7486);
    p
};
/// A wipeout's grey overcast: the sky's colors; the rest of the scene greys.
const OVERCAST: [Option<u32>; 16] = {
    let mut p = [None; 16];
    p[SKY_TOP] = Some(0x6e7680);
    p[SKY_HZ] = Some(0xadb2b8);
    p[SUN] = Some(0xc8c8c4);
    p[SUN_GLOW] = Some(0xa8acb0);
    p[CLOUD] = Some(0xc4c8cc);
    p[CLOUD_SHADE] = Some(0x8e949a);
    p[HEAD] = Some(0x5d6466);
    p
};
const STORM: Palette = [
    0x1b2540, 0x4f6284, 0x9fb0c8, 0x5a6c8a, 0x8090a8, 0x55627a, 0x2c3a4e, 0x23517c, 0x07142c,
    0x0b2346, 0x6ea2b8, 0x2e5c86, 0x10283c, 0xdfe8f0, 0x9fb2c4, 0xd0dcea,
];

/// By night: a moonlit sea under the near-black night sky every scene shares
/// (night.rs; the sun's disc is the moon, high up). The sea is a dark teal
/// and a moonlit rim runs along the water's surface, so the two never merge.
const NIGHT: Palette = [
    NIGHT_ZENITH,
    NIGHT_HORIZON,
    MOON,
    0x2e3e60,
    0x3a4660,
    0x232c42,
    0x080c18,
    0x0a2c3c,
    0x010610,
    0x041a28,
    0x2f7c8e,
    0x0f3e52,
    0x03121c,
    0xc8d8e8,
    0x6c84a0,
    0xb8cce0,
];
/// What the surfers and their kit lean toward by night.
const NIGHT_SHADE: u32 = 0x0a1428;
/// Moonlight caught along the top of the water by night: the line between sea and sky.
const MOON_RIM: u32 = 0x4a7c94;
/// The band's sea by night: one even teal under the black sky (darkening a little with depth).
const BAND_SEA: u32 = 0x0c3448;

const SKIN: u32 = 0xe9b48a;
const SUIT: u32 = 0x16181e;
const BOARD: u32 = 0xf5c842;
const STRIPE: u32 = 0xe2513a;
const EXTRA_SUITS: [u32; 5] = [0x8a2424, 0x203f86, 0x2f6a35, 0x6a4320, 0x5a2a72];
const EXTRA_BOARDS: [u32; 5] = [0xf3f3ee, 0x4fc3f7, 0xff8a65, 0xa5d6a7, 0xf48fb1];
const FLAG: u32 = 0xe5302a;
const POLE: u32 = 0xdedede;
const BUOY: u32 = 0xf07a1a;

/// What the surfer is doing.
#[derive(Clone, Copy, PartialEq, Eq)]
enum Pose {
    Sit,
    Paddle,
    Ride,
    Crouch,
}

impl Pose {
    /// The surfer, facing right: rows above the board, top first. h = head,
    /// w = wetsuit; anything else lets the scene through. Each pose has a
    /// small cut (the band) and a large one (the spine).
    fn rows(self, large: bool) -> &'static [&'static str] {
        match (self, large) {
            (Pose::Sit, false) => &["  hh  ", "  ww  ", "  ww  "],
            (Pose::Sit, true) => &["  hh  ", "  hh  ", " wwww ", " wwww ", "  ww  "],
            (Pose::Paddle, false) => &["wwwwhh"],
            (Pose::Paddle, true) => &["    hh", "wwwwhh"],
            (Pose::Ride, false) => &["  hh  ", " wwww ", "  ww  "],
            (Pose::Ride, true) => &[
                "  hh  ", "  hh  ", " wwwww", "w ww  ", "  ww  ", " w  w ", "w    w",
            ],
            (Pose::Crouch, false) => &["    hh", "  www ", "  ww  "],
            (Pose::Crouch, true) => &["    hh", "  wwhh", " wwww ", "ww  w ", "w   w "],
        }
    }
}

const PMAX: usize = 480;
/// Set pixels in each quadrant mask.
const BITS: [u32; 16] = [0, 1, 1, 2, 1, 2, 2, 3, 1, 2, 2, 3, 2, 3, 3, 4];

/// What a particle is: spray, white water, or a sun glint (which stays put).
const SPRAY_DOT: u8 = 0;
const WHITE_WATER: u8 = 1;
const GLINT: u8 = 2;

fn grey(c: u32, k: f64) -> u32 {
    let l = i32_of(
        f64::from((c >> 16) & 255) * 0.3
            + f64::from((c >> 8) & 255) * 0.59
            + f64::from(c & 255) * 0.11,
    ) as u32;
    mix(c, (l << 16) | (l << 8) | l, k)
}

/// Smooth 1-D value noise in [0, 1).
fn vnoise(x: f64, seed: f64) -> f64 {
    let i = x.floor();
    let f = x - i;
    let a = hash(i * 7919.0 + seed);
    let b = hash((i + 1.0) * 7919.0 + seed);
    a + (b - a) * f * f * (3.0 - 2.0 * f)
}

/// The speed table at a level, read only at whole levels: flow indexes the
/// table with the raw level, so a fractional one reads `undefined` (NaN).
fn speed_at(level: f64) -> f64 {
    if level.fract() == 0.0 {
        SPEED[level as usize]
    } else {
        f64::NAN
    }
}

/// Spray, splash and glints, in pixels (Float32Arrays in flow).
struct Particles {
    x: Vec<f32>,
    y: Vec<f32>,
    vx: Vec<f32>,
    vy: Vec<f32>,
    life: Vec<f32>,
    kind: Vec<u8>,
    next: usize,
}

impl Particles {
    fn new() -> Self {
        Self {
            x: vec![0.0; PMAX],
            y: vec![0.0; PMAX],
            vx: vec![0.0; PMAX],
            vy: vec![0.0; PMAX],
            life: vec![0.0; PMAX],
            kind: vec![0; PMAX],
            next: 0,
        }
    }

    fn emit(&mut self, x: f64, y: f64, vx: f64, vy: f64, life: f64, kind: u8) {
        let i = self.next;
        self.next = (i + 1) % PMAX;
        self.x[i] = x as f32;
        self.y[i] = y as f32;
        self.vx[i] = vx as f32;
        self.vy[i] = vy as f32;
        self.life[i] = life as f32;
        self.kind[i] = kind;
    }
}

/// A gull in world pixels: where, how high (0..1 of the sky), its wingbeat's phase, its speed.
struct Gull {
    x: f64,
    y: f64,
    ph: f64,
    v: f64,
}

/// A cloud in world pixels: where, how high (0..1), its half-width and half-height.
struct Cloud {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

pub struct Surf {
    dials: Dials,
    /// The frame the next wave comes in on (flow hears it).
    next_wave: f64,
    columns: usize,
    rows: usize,
    out: Cells,
    rng: Rng,
    seed: f64,
    t: f64,
    started: bool,

    // Eased dials.
    s: f64,
    speed: f64,
    scroll: f64,
    k_grey: f64,
    k_storm: f64,
    k_night: f64,
    pal: Palette,

    // Pixel layer (2 × 2 a cell) and per-cell overlays.
    pw: usize,
    ph: usize,
    pc: Vec<u32>,
    water: Vec<u8>,
    dots: Vec<u8>,
    dot_color: Vec<u32>,
    solid: Vec<u8>,
    dominant: Vec<u32>,
    fit: QuadFit,

    // Wave geometry (pixels), refreshed by layout().
    vertical: bool,
    sea_y: f64,
    cx: f64,
    /// The main wave's height.
    wave_h: f64,
    /// Its front (face) and back widths.
    wf: f64,
    wb: f64,
    pow: f64,
    steep: f64,
    curl: f64,
    crest_y: f64,
    /// The barrel's ellipse: center and radii, and where its lip ends (degrees).
    ex: f64,
    ey: f64,
    rx: f64,
    ry: f64,
    phi_end: f64,
    /// The following swells (Float32Arrays in flow): where, how tall, how wide.
    fx: [f32; 2],
    f_h: [f32; 2],
    f_w: [f32; 2],
    n_f: usize,

    // The surfer.
    phase: f64,
    u: f64,
    du: f64,
    wipe: f64,
    recover: f64,
    last_tint: Tint,
    board_x: f64,
    board_y: f64,
    board_vx: f64,
    board_vy: f64,
    board_spin: f64,

    particles: Particles,

    // Scenery in world pixels.
    gulls: Vec<Gull>,
    clouds: Vec<Cloud>,
}

impl Surf {
    pub fn new(seed: f64) -> Self {
        let seed = seed % 100_000.0;
        Self {
            dials: Dials::default(),
            next_wave: 0.0,
            columns: 0,
            rows: 0,
            out: Cells::new(0, 0),
            rng: Rng::new(seed + 17.0),
            seed,
            t: 0.0,
            started: false,
            s: 0.0,
            speed: 0.0,
            scroll: 0.0,
            k_grey: 0.0,
            k_storm: 0.0,
            k_night: 0.0,
            pal: DAY,
            pw: 0,
            ph: 0,
            pc: Vec::new(),
            water: Vec::new(),
            dots: Vec::new(),
            dot_color: Vec::new(),
            solid: Vec::new(),
            dominant: Vec::new(),
            fit: QuadFit::default(),
            vertical: false,
            sea_y: 0.0,
            cx: 0.0,
            wave_h: 0.0,
            wf: 1.0,
            wb: 1.0,
            pow: 1.0,
            steep: 0.0,
            curl: 0.0,
            crest_y: 0.0,
            ex: 0.0,
            ey: 0.0,
            rx: 1.0,
            ry: 1.0,
            phi_end: 0.0,
            fx: [0.0; 2],
            f_h: [0.0; 2],
            f_w: [0.0; 2],
            n_f: 0,
            phase: 0.0,
            u: 0.5,
            du: 0.0,
            wipe: 0.0,
            recover: 0.0,
            last_tint: Tint::Normal,
            board_x: 0.0,
            board_y: 0.0,
            board_vx: 0.0,
            board_vy: 0.0,
            board_spin: 0.0,
            particles: Particles::new(),
            gulls: Vec::new(),
            clouds: Vec::new(),
        }
    }

    fn level(&self) -> f64 {
        self.dials.strength.clamp(0.0, 10.0)
    }

    /// The wave's shape for the current (eased) swell.
    fn layout(&mut self) {
        let s = self.s;
        let (pw, ph) = (self.pw as f64, self.ph as f64);
        let e = clamp01((s - 1.2) / 8.8).powf(1.1);
        self.steep = clamp01((s - 2.5) / 6.0);
        self.pow = 1.3 + 0.12 * s;
        self.curl = clamp01((s - 9.0) / 0.9);
        if self.vertical {
            self.sea_y = ph * (0.56 + 0.036 * s);
            self.wave_h = ((self.sea_y - ph * 0.2) * e).max(0.3);
            self.cx = pw * 0.3;
            self.wf = (pw - self.cx + 3.0).min(self.wave_h * (2.4 - 0.12 * s) + 4.0);
            self.wb = self.wave_h * 3.0 + 8.0;
            self.n_f = 0;
        } else {
            self.sea_y = ph * (0.52 + 0.033 * s);
            self.wave_h = ((self.sea_y - 0.7) * e).max(0.3);
            self.cx = pw * if pw > 200.0 { 0.6 } else { 0.55 };
            self.wf = self.wave_h * 3.2 + 2.0;
            self.wb = self.wave_h * 7.0 + 8.0;
            let span = (pw * 0.3).max(self.wb + self.wf + 30.0);
            self.n_f = 0;
            for i in 0..2 {
                let fi = i as f64;
                let x = self.cx - span * (fi + 1.0) * (1.0 - fi * 0.12);
                let h = self.wave_h * if i == 0 { 0.6 } else { 0.4 };
                let w = h * 6.0 + 8.0;
                if x - w * 0.5 < 0.0 {
                    break;
                }
                self.fx[i] = x as f32;
                self.f_h[i] = h as f32;
                self.f_w[i] = w as f32;
                self.n_f += 1;
            }
        }
        self.crest_y = self.sea_y - self.wave_h;
        let c = self.curl;
        self.ry = (self.wave_h * (0.22 + 0.22 * c)).max(1.0);
        self.rx = if self.vertical {
            self.wf * (0.3 + 0.2 * c)
        } else {
            (self.wf * 0.4).max(self.ry * 2.6)
        }
        .max(1.0);
        self.ex = self.cx + self.rx * 0.95;
        self.ey = self.crest_y + self.ry;
        self.phi_end = -90.0 + c * 135.0;
    }

    /// The main wave's height above sea level at pixel column x (center).
    fn main_hump(&self, x: f64) -> f64 {
        let d = x - self.cx;
        let h = self.wave_h;
        if d < 0.0 {
            let u = -d / self.wb;
            return if u < 1.0 {
                h * (0.5 + 0.5 * (PI * u).cos())
            } else {
                0.0
            };
        }
        let u = d / self.wf;
        if u < 1.0 {
            let hc = h * (0.5 + 0.5 * (PI * u).cos());
            let hp = h * (1.0 - u).powf(self.pow);
            return hc + (hp - hc) * self.steep;
        }
        if u < 1.5 {
            return -h * 0.1 * self.steep * ((PI * (u - 1.0)) / 0.5).sin();
        }
        0.0
    }

    fn ripple(&self, x: f64) -> f64 {
        let wx = x + self.scroll;
        let t = self.t;
        let a = if self.vertical {
            0.5 + 0.06 * self.s
        } else {
            0.3 + 0.04 * self.s
        };
        a * (0.6 * (wx * 0.23 + t * 0.09).sin() + 0.4 * (wx * 0.083 - t * 0.05).sin())
    }

    /// The following swell `i`'s height at pixel column x, if x is on it.
    fn swell(&self, i: usize, x: f64) -> Option<f64> {
        let d = x - f64::from(self.fx[i]);
        let w = if d < 0.0 {
            f64::from(self.f_w[i])
        } else {
            f64::from(self.f_w[i]) * 0.7
        };
        (d.abs() < w).then(|| f64::from(self.f_h[i]) * (0.5 + 0.5 * ((PI * d) / w).cos()))
    }

    /// Water surface (pixel row) at column center x, the hood over a barrel ignored.
    fn surface_at(&self, x: f64) -> f64 {
        let mut h = self.main_hump(x);
        for i in 0..self.n_f {
            if let Some(fh) = self.swell(i, x) {
                h = h.max(fh);
            }
        }
        self.sea_y - h + self.ripple(x) * (1.0 - clamp01(h.abs() / 2.0))
    }

    fn start_wipeout(&mut self) {
        self.wipe = 1.0;
        let k = if self.vertical { 1.6 } else { 1.0 };
        let x = self.surfer_x();
        let y = self.surface_at(x);
        self.board_x = x;
        self.board_y = y - 1.0;
        self.board_vx = -0.35 * k;
        self.board_vy = -0.9 * k;
        self.board_spin = 0.0;
        let mut i = 0.0;
        while i < 40.0 * k {
            let a = PI * (1.0 + self.rng.f());
            let v = (0.3 + self.rng.f() * 0.9) * k;
            let life = 10.0 + self.rng.f() * 16.0;
            self.particles
                .emit(x, y - 0.5, a.cos() * v, a.sin() * v * 1.2, life, SPRAY_DOT);
            i += 1.0;
        }
    }

    fn pose(&self) -> Pose {
        if self.recover > 0.0 {
            Pose::Paddle
        } else if self.s < 1.7 {
            Pose::Sit
        } else if self.s < 2.7 {
            Pose::Paddle
        } else if self.curl > 0.55 {
            Pose::Crouch
        } else {
            Pose::Ride
        }
    }

    fn surfer_x(&self) -> f64 {
        match self.pose() {
            Pose::Sit => self.cx,
            Pose::Paddle => self.cx + self.wf * 0.45,
            _ => self.cx + self.u * self.wf,
        }
    }

    /// The sun's column by day, easing over to the moon's (night.rs) by night.
    fn sun_x(&self) -> f64 {
        let pw = self.pw as f64;
        let day = if self.vertical { pw * 0.74 } else { pw * 0.85 };
        day + (self.moon().0 - day) * self.k_night
    }

    fn moon(&self) -> (f64, f64) {
        moon_pixel(self.columns as f64, self.rows as f64)
    }

    fn update_palette(&mut self) {
        let dawn = clamp01((4.0 - self.s) / 3.0);
        for key in 0..DAY.len() {
            let mut c = DAY[key];
            if let Some(d) = DAWN[key] {
                c = mix(c, d, dawn);
            }
            c = mix(c, STORM[key], self.k_storm);
            c = mix(c, NIGHT[key], self.k_night);
            c = match OVERCAST[key] {
                Some(o) => mix(c, o, self.k_grey),
                None => grey(c, self.k_grey * 0.55),
            };
            self.pal[key] = c;
        }
    }

    /// Sky, sun, clouds, headland, sea and the waves into the pixel layer.
    fn paint_scene(&mut self) {
        self.paint_world();
        // The pitching lip pours down as a curtain from its tip to the water.
        if self.curl > 0.5 {
            let a = (self.phi_end * PI) / 180.0;
            let tx = self.ex + self.rx * a.cos();
            let ty = self.ey + self.ry * a.sin();
            let bottom = self.sea_y;
            let pw = self.pw;
            let mut y = ty.floor();
            while y < bottom {
                let f = (y - ty) / (bottom - ty).max(1.0);
                let x = tx + f * self.rx * 0.25;
                let xi = x.floor();
                if !(xi < 0.0 || xi >= pw as f64 || y < 0.0 || y >= self.ph as f64) {
                    let xi = xi as usize;
                    let k = y as usize * pw + xi;
                    self.pc[k] = mix(self.pal[FACE_LIP], self.pal[FOAM], 0.3 + 0.5 * f);
                    if self.vertical && xi + 1 < pw {
                        self.pc[k + 1] = mix(self.pc[k + 1], self.pal[FOAM_SHADE], 0.6);
                    }
                }
                y += 1.0;
            }
        }
    }

    fn paint_world(&mut self) {
        let (pw, ph) = (self.pw, self.ph);
        let (pwf, phf) = (pw as f64, ph as f64);
        let p = self.pal;
        let s = self.s;
        let vertical = self.vertical;
        let sea_y = self.sea_y;
        let t = self.t;
        let (k_grey, k_storm) = (self.k_grey, self.k_storm);
        // Sun. By night the moon hangs high where every scene's moon does, about a cell across.
        let k_night = self.k_night;
        let sun_x = self.sun_x();
        let day_sun_y = if vertical {
            phf * (0.36 - 0.022 * s)
        } else {
            (sea_y - 1.6 - (s - 1.0) * 0.5).max(1.2)
        };
        let sun_y = day_sun_y + (self.moon().1 - day_sun_y) * k_night;
        let day_r = if vertical { 2.6 } else { 1.3 };
        let sun_r = day_r + (moon_radius(vertical) / 2.0 - day_r) * k_night;
        let glow_r = 9.0 - 4.0 * k_night;
        let band_night = if vertical { 0.0 } else { k_night };
        // Headland: one hazy ridge far off, drifting slowly.
        let head_scroll = self.scroll * 0.06 + self.seed;
        let head_max = if vertical { 5.0 } else { (phf * 0.24).max(1.2) };
        let foam_amt = clamp01((s - 3.5) / 4.0);
        let spill = clamp01((s - 4.2) / 3.0) * (1.0 - self.curl) * 0.4;
        let curl = self.curl;
        let cx = self.cx;
        let (ex, ey, rx, ry, phi_end) = (self.ex, self.ey, self.rx, self.ry, self.phi_end);
        let (wave_h, wf) = (self.wave_h, self.wf);
        for x in 0..pw {
            let xf = x as f64;
            let xc = xf + 0.5;
            let surf = self.surface_at(xc);
            // Which wave owns this column, for shading.
            let mut wave = wave_h;
            let mut hump = self.main_hump(xc);
            let mut face = xc > cx;
            for i in 0..self.n_f {
                if let Some(fh) = self.swell(i, xc) {
                    if fh > hump {
                        hump = fh;
                        wave = f64::from(self.f_h[i]);
                        face = xc - f64::from(self.fx[i]) > 0.0;
                    }
                }
            }
            let is_main = wave == wave_h;
            let hood = if curl > 0.0 && xc >= cx && xc <= ex {
                self.crest_y
            } else {
                1e9
            };
            let head_h = self.headland(xf + head_scroll, head_max);
            let wx = xf + self.scroll;
            for y in 0..ph {
                let yf = y as f64;
                let k = y * pw + x;
                let yc = yf + 0.5;
                // Sky.
                let mut c = mix(p[SKY_TOP], p[SKY_HZ], clamp01(yf / sea_y.max(1.0)));
                let sd = hypot(xc - sun_x, (yc - sun_y) * 2.0);
                if sd < glow_r {
                    let near =
                        (1.0 - sd / glow_r) * (1.0 - sd / glow_r) * 0.55 * (1.0 - k_grey * 0.6);
                    c = mix(c, p[SUN_GLOW], near);
                }
                let mut star = 0.0;
                if sd < sun_r * 2.0 {
                    c = mix(c, p[SUN], clamp01(sun_r * 2.0 - sd) * (1.0 - k_grey * 0.75));
                } else if k_night > 0.05 && yc < sea_y - 1.0 {
                    // Stars, drifting slowly with the swell, fading toward the horizon.
                    let sx = (xf + self.scroll * 0.02).floor();
                    if hash(sx * 7919.0 + yf * 104_729.0 + self.seed) > 0.985 {
                        let twinkle =
                            0.55 + 0.45 * hash(sx * 31.0 + yf * 17.0 + f64::from(i32_of(t) >> 3));
                        star = twinkle * (1.0 - clamp01(yc / sea_y) * 0.7) * (1.0 - k_grey);
                        c = mix(c, STAR, k_night * star);
                    }
                }
                for cl in &self.clouds {
                    let span = pwf + 40.0;
                    let cxp =
                        (((cl.x - self.scroll * 0.15 - t * 0.01) % span) + span) % span - 20.0;
                    let cy = if vertical {
                        2.0 + cl.y * phf * 0.22
                    } else {
                        0.7 + cl.y * (sea_y * 0.35).max(0.3)
                    };
                    let dx = (xc - cxp) / cl.w;
                    let dy = (yc - cy) / cl.h;
                    let r = dx * dx + dy * dy;
                    if r < 1.0 {
                        let shade = if yc > cy { p[CLOUD_SHADE] } else { p[CLOUD] };
                        c = mix(c, shade, clamp01((1.0 - r) * 2.5) * (1.0 - k_storm * 0.3));
                    }
                }
                if yc < sea_y && yc > sea_y - head_h {
                    c = mix(
                        p[HEAD],
                        c,
                        if yc < sea_y - head_h + 1.0 {
                            0.45
                        } else {
                            0.25
                        },
                    );
                }
                // In the band by night the sky is plain black but for the moon and the
                // stars: in so few rows its glow, clouds and headland read as stripes.
                if band_night > 0.01 {
                    let mut k = 0x000000;
                    if sd < sun_r * 2.0 {
                        k = mix(k, p[SUN], clamp01(sun_r * 2.0 - sd));
                    } else if star > 0.0 {
                        k = mix(k, STAR, star);
                    }
                    c = mix(c, k, band_night);
                }
                // Water.
                let mut wet = clamp01(yf + 1.0 - surf);
                let mut in_tube = false;
                let mut lip = false;
                if curl > 0.0 && is_main && (xc - ex).abs() < rx + 1.0 && yc < sea_y + 1.0 {
                    let dx = (xc - ex) / rx;
                    let dy = (yc - ey) / ry;
                    let r = (dx * dx + dy * dy).sqrt();
                    let phi = (dy.atan2(dx) * 180.0) / PI;
                    if r < 1.02 {
                        let prog = clamp01((phi + 180.0) / (phi_end + 180.0));
                        let ca = ((phi * PI) / 180.0).cos();
                        let sa = ((phi * PI) / 180.0).sin();
                        let rr = hypot(rx * ca, 2.0 * ry * sa);
                        let th = ((if vertical { 5.0 } else { 2.6 })
                            * (1.0 - 0.45 * prog)
                            * (0.6 + 0.4 * curl))
                            / rr;
                        if phi <= phi_end && r > 1.0 - th && r < 1.0 && (yc < surf || phi > -140.0)
                        {
                            lip = true;
                        } else if yc < surf && r < 1.0 {
                            in_tube = true;
                        }
                    } else if yc >= hood {
                        wet = 1.0;
                    }
                } else if yc >= hood && yc < surf {
                    wet = clamp01(yf + 1.0 - hood);
                }
                if lip {
                    // The lip: lit through, pale green-blue; frothing at the tip.
                    let dyc = (yc - ey) / ry;
                    c = mix(p[FACE_LIP], p[FOAM], clamp01(0.25 + dyc * 0.5) * 0.7);
                    if band_night > 0.01 {
                        c = mix(c, mix(BAND_SEA, MOON_RIM, 0.45), 0.7 * band_night);
                    }
                    self.water[k] = 1;
                } else if in_tube {
                    let dx = (xc - ex) / rx;
                    let dy = (yc - ey) / ry;
                    // Light comes in through the open end: shadowed at the back of
                    // the curl, close to the face's own color toward the mouth.
                    let lit = clamp01((dx + dy) * 0.4 + 0.55);
                    c = mix(
                        mix(p[TUBE], p[SEA_TOP], 0.4),
                        mix(p[SEA_TOP], p[FACE_LIP], 0.5),
                        lit * 0.8,
                    );
                    if band_night > 0.01 {
                        c = mix(c, mix(BAND_SEA, p[SEA_DEEP], 0.3), 0.7 * band_night);
                    }
                    self.water[k] = 0;
                } else if wet > 0.0 {
                    let hf = (sea_y - yc) / wave.max(0.5);
                    let mut wc;
                    if hump > 0.4 && hf > 0.0 {
                        if !is_main {
                            let (toward, by) = if face {
                                (p[FACE_LIP], 0.45)
                            } else {
                                (p[BACK], 0.8)
                            };
                            wc = mix(p[SEA_TOP], toward, clamp01(hf) * by);
                        } else {
                            // Ease from the back into the face over a few pixels past the
                            // crest, so the face never starts as a hard-edged dark block.
                            let back_c = mix(p[SEA_TOP], p[BACK], clamp01(hf));
                            let face_c = mix(
                                mix(p[FACE_DEEP], p[SEA_TOP], 0.45),
                                p[FACE_LIP],
                                clamp01(hf).powf(1.25),
                            );
                            let ramp = if vertical { (wf * 0.45).max(4.0) } else { 5.0 };
                            let fk = clamp01((xc - cx) / ramp + 0.35);
                            wc = mix(back_c, face_c, fk);
                            // The barrel's shadow fades out ahead of it rather than stopping on a line.
                            if curl > 0.0 {
                                let ahead = clamp01((ex + rx - xc) / (rx * 0.8).max(2.0));
                                wc = mix(wc, p[TUBE], 0.28 * curl * ahead * fk);
                            }
                        }
                    } else {
                        wc = mix(
                            p[SEA_TOP],
                            p[SEA_DEEP],
                            clamp01(((yc - sea_y) / (phf - sea_y).max(2.0)) * 1.1),
                        );
                    }
                    if band_night > 0.01 {
                        let even = mix(
                            BAND_SEA,
                            p[SEA_DEEP],
                            clamp01((yc - surf) / (phf - surf).max(2.0)) * 0.35,
                        );
                        wc = mix(wc, even, 0.85 * band_night);
                    }
                    let depth = yc - surf;
                    if depth < 1.0 {
                        let k = if hump > 0.4 && hf > 0.5 {
                            0.18 + 0.4 * clamp01(hf - 0.5)
                        } else {
                            0.18 * (1.0 - k_night)
                        };
                        wc = mix(wc, p[FOAM_SHADE], k);
                    }
                    // Moonlight along the crests, strongest on the tallest faces; on a calm
                    // sea only a faint line, or the swell reads as dark blocks on a bright stripe.
                    if depth < 1.0 && k_night > 0.05 {
                        wc = mix(
                            wc,
                            MOON_RIM,
                            k_night * (0.1 + 0.4 * clamp01(hf * 1.6 - 0.2)),
                        );
                    }
                    // Streaky texture that travels with the water (faded by night,
                    // when it reads as blocks on the dark water).
                    let tn = hash(
                        ((wx + yf * 3.0) * if vertical { 0.5 } else { 0.34 }).floor() * 131.0 + yf,
                    );
                    if tn > 0.86 {
                        wc = mix(wc, p[FOAM_SHADE], 0.12 * (1.0 - 0.8 * k_night));
                    } else if tn < 0.1 {
                        wc = mix(wc, p[SEA_DEEP], 0.12 * (1.0 - 0.8 * k_night));
                    }
                    // Foam: trailing off the back, spilling down the crest.
                    if depth < 1.6 && depth > -1.0 {
                        if is_main && !face && hump < 0.35 * wave_h + 1.0 && foam_amt > 0.0 {
                            let dd = (cx - xc) / (self.wb * 1.4 + 10.0);
                            let fn_ = vnoise(wx * if vertical { 0.4 } else { 0.22 }, 99.0) * 0.7
                                + vnoise(wx * 0.9 + t * 0.05, 7.0) * 0.3;
                            if dd < 1.0 && fn_ < foam_amt * (1.0 - dd) * 0.75 {
                                wc =
                                    mix(wc, if depth < 0.8 { p[FOAM] } else { p[FOAM_SHADE] }, 0.9);
                            }
                        } else if !is_main && hump > 0.4 && foam_amt > 0.5 && hf > 0.8 {
                            wc = mix(wc, p[FOAM_SHADE], 0.5);
                        }
                    }
                    if is_main && face && spill > 0.0 && hf > 1.0 - spill && depth < 2.2 {
                        let fl = vnoise(wx * 0.5 + yf * 1.7 - t * 0.12, 31.0);
                        if fl < 0.6 {
                            wc = mix(wc, if fl < 0.4 { p[FOAM] } else { p[FOAM_SHADE] }, 0.9);
                        }
                    }
                    // White water boiling at the foot of a big face.
                    if is_main
                        && s > 6.5
                        && xc > cx + wf * 0.7
                        && xc < cx + wf * (1.15 + 0.15 * curl)
                    {
                        let top =
                            sea_y - (s - 6.2) * (if vertical { 1.4 } else { 0.35 }) * (1.0 + curl);
                        if yc > top && yc < sea_y + if vertical { 3.0 } else { 1.0 } {
                            let fl = vnoise(xf * 0.45 + t * 0.21, 53.0 + yf) * 0.75
                                + hash(xf * 17.0 + yf * 113.0 + t * 31.0) * 0.25;
                            if fl < 0.62 {
                                wc = mix(wc, if fl < 0.4 { p[FOAM] } else { p[FOAM_SHADE] }, 0.9);
                            }
                        }
                    }
                    // A path of moonlight across the water under the moon.
                    if k_night > 0.05 && yc > sea_y {
                        let gd = (xc - sun_x).abs()
                            / (1.5 + (yc - sea_y) * if vertical { 0.5 } else { 1.6 });
                        if gd < 1.0
                            && hash(
                                (wx * 0.8).floor() * 31.0 + yf * 977.0 + f64::from(i32_of(t) >> 2),
                            ) > 0.45 + gd * 0.45
                        {
                            wc = mix(wc, p[SUN], 0.6 * k_night * (1.0 - gd));
                        }
                    }
                    c = mix(c, wc, wet);
                    self.water[k] = u8::from(wet > 0.5);
                } else {
                    self.water[k] = 0;
                }
                self.pc[k] = c;
            }
        }
    }

    /// Distant headlands and islands: a few humps strung out along the horizon.
    fn headland(&self, wx: f64, max: f64) -> f64 {
        let sp = if self.vertical { 70.0 } else { 150.0 };
        let seg = (wx / sp).floor();
        let seed = self.seed;
        let mut best: f64 = 0.0;
        for k in [seg - 1.0, seg, seg + 1.0] {
            if hash(k * 41.0 + seed) < 0.45 {
                continue;
            }
            let w = sp * (0.1 + 0.2 * hash(k * 43.0 + seed));
            let c = k * sp + sp * 0.5 + (hash(k * 47.0 + seed) - 0.5) * sp * 0.4;
            let d = (wx - c) / w;
            if d <= -1.0 || d >= 1.0 {
                continue;
            }
            // A cliff on one side, a long slope down the other.
            let shape = if d < 0.0 {
                (1.0 - d * d).sqrt()
            } else {
                1.0 - d * d
            };
            best = best.max(max * (0.55 + 0.45 * hash(k * 53.0 + seed)) * shape);
        }
        best
    }

    fn put(&mut self, x: f64, y: f64, c: u32) {
        let (xi, yi) = (x.floor(), y.floor());
        // (NaN passes these tests, as in flow: its pixel write goes nowhere, but
        // it marks cell 0 solid.)
        if xi < 0.0 || yi < 0.0 || xi >= self.pw as f64 || yi >= self.ph as f64 {
            return;
        }
        if !(xi.is_nan() || yi.is_nan()) {
            self.pc[yi as usize * self.pw + xi as usize] = if self.k_night > 0.01 {
                mix(c, NIGHT_SHADE, 0.4 * self.k_night)
            } else {
                c
            };
        }
        let cell = (i32_of(yi) >> 1) as usize * self.columns + (i32_of(xi) >> 1) as usize;
        self.solid[cell] = 1;
    }

    /// A board: a line centered at (x, y) along the slope, nose (facing) highlighted.
    /// A board (its nose in `nose`), centred at (x, y), along `slope`.
    #[allow(clippy::too_many_arguments)]
    fn board(&mut self, x: f64, y: f64, slope: f64, half: f64, facing: f64, color: u32, nose: u32) {
        // Work in screen units where a pixel is 1 wide and 2 tall.
        let sx = 1.0;
        let sy = slope * 2.0;
        let n = hypot(sx, sy);
        let c = sx / n;
        let sn = sy / n;
        let steps = (half * 4.0).ceil();
        let mut i = -steps;
        while i <= steps {
            let k = (i / steps) * half;
            let at_nose = k * facing > half * 0.55;
            self.put(
                x + k * c,
                y + (k * sn) / 2.0,
                if at_nose { nose } else { color },
            );
            i += 1.0;
        }
    }

    fn sprite(&mut self, pose: Pose, large: bool, x: f64, y: f64, facing: f64, suit: u32) {
        let rows = pose.rows(large);
        let width = rows.iter().map(|r| r.len()).max().unwrap_or(0) as f64;
        // Snapped so the head fills whole cells: three colors never share one.
        let x0 = 2.0 * round((x - width / 2.0) / 2.0);
        let yb = y.floor();
        for (r, row) in rows.iter().enumerate() {
            let py = yb - (rows.len() - r) as f64;
            for (c, ch) in row.bytes().enumerate() {
                if ch != b'h' && ch != b'w' {
                    continue;
                }
                let c = c as f64;
                let pxl = if facing > 0.0 {
                    x0 + c
                } else {
                    x0 + width - 1.0 - c
                };
                self.put(pxl, py, if ch == b'h' { SKIN } else { suit });
            }
        }
    }

    fn paint_surfer(&mut self) {
        // The hero's board, and its nose, in the session's hue (crabigator's).
        let (board, stripe) = match self.dials.accent {
            Some(hue) => (tint_to(BOARD, hue, 0.0), tint_to(STRIPE, hue, 0.0)),
            None => (BOARD, STRIPE),
        };
        let large = self.vertical && self.ph >= 32;
        let half = if large { 5.0 } else { 2.6 };
        if self.wipe > 0.0 {
            self.board(
                self.board_x,
                self.board_y,
                self.board_spin.tan(),
                half,
                1.0,
                board,
                stripe,
            );
            return;
        }
        let pose = self.pose();
        let x = self.surfer_x();
        let surf = self.surface_at(x);
        let lim = if self.vertical { 0.7 } else { 0.12 };
        let slope = clamp(
            (self.surface_at(x + 1.0) - self.surface_at(x - 1.0)) / 2.0,
            -lim,
            lim,
        );
        let bob = if pose == Pose::Sit {
            (self.t * 0.12).sin() * 0.35
        } else {
            0.0
        };
        let by = f64::from(i32_of((surf - 0.6 + bob).floor()) | 1) + 0.5;
        let flat = pose == Pose::Sit || pose == Pose::Paddle;
        let facing = if flat || self.du >= 0.0 || self.s < 5.0 {
            1.0
        } else {
            -1.0
        };
        self.board(
            x,
            by,
            if flat { slope * 0.5 } else { slope },
            half,
            facing,
            board,
            stripe,
        );
        let lean = if !flat && large { -slope * 0.8 } else { 0.0 };
        self.sprite(pose, large, x + lean, by, facing, SUIT);
    }

    /// Subagents: more surfers in the line-up.
    fn paint_extras(&mut self) {
        let n = round(self.dials.coverage_boost / 12.0).min(5.0);
        if n == 0.0 {
            return;
        }
        let mut slot = 0.0;
        let mut i = 0;
        while (i as f64) < n {
            let fi = i as f64;
            let x;
            let mut pose = Pose::Sit;
            if !self.vertical && i < self.n_f && self.s >= 3.0 {
                let u = 0.35 + 0.2 * (self.phase * 0.8 + fi * 2.1).sin();
                x = f64::from(self.fx[i]) + u * f64::from(self.f_w[i]) * 0.7;
                pose = Pose::Ride;
            } else if self.vertical {
                x = 3.0 + slot * 6.0;
                slot += 1.0;
                if x > self.cx - 6.0 {
                    break;
                }
            } else {
                let pw = self.pw as f64;
                let mut xx =
                    self.cx + self.wf * 1.6 + 10.0 + slot * 14.0 + hash(slot * 3.0 + 1.0) * 10.0;
                if xx > pw - 4.0 {
                    xx = self.cx - self.wb - 6.0 - (xx - pw) * 1.0;
                }
                x = xx;
                slot += 1.0;
                if x < 3.0 {
                    break;
                }
                pose = if self.s >= 2.0 {
                    Pose::Paddle
                } else {
                    Pose::Sit
                };
            }
            let surf = self.surface_at(x);
            let slope = (self.surface_at(x + 1.0) - self.surface_at(x - 1.0)) / 2.0;
            let bob = if pose == Pose::Sit {
                (self.t * 0.11 + fi * 1.7).sin() * 0.3
            } else {
                0.0
            };
            let by = f64::from(i32_of((surf - 0.6 + bob).floor()) | 1) + 0.5;
            let tilt = if pose == Pose::Ride {
                slope
            } else {
                slope * 0.5
            };
            self.board(x, by, tilt, 2.4, 1.0, EXTRA_BOARDS[i], STRIPE);
            self.sprite(pose, false, x, by, 1.0, EXTRA_SUITS[i]);
            i += 1;
        }
    }

    /// A nearly-full context: a red warning flag on a buoy.
    fn paint_flag(&mut self) {
        if self.k_storm < 0.35 {
            return;
        }
        let pw = self.pw as f64;
        let x = if self.vertical { pw * 0.1 } else { pw * 0.94 }.floor();
        let surf = self.surface_at(x + 0.5);
        let base = surf.floor();
        let tall = if self.vertical {
            7.0
        } else {
            round(self.sea_y * 0.6).clamp(3.0, 5.0)
        };
        self.put(x, base, BUOY);
        self.put(x + 1.0, base, BUOY);
        let mut k = 1.0;
        while k <= tall {
            self.put(x, base - k, POLE);
            k += 1.0;
        }
        let wave = (i32_of(self.t) >> 2) & 1 == 1;
        let (fw, fh) = if self.vertical {
            (4.0, 3.0)
        } else {
            (3.0, 2.0)
        };
        let mut r = 0.0;
        while r < fh {
            let end = fw - if r == fh - 1.0 && wave { 1.0 } else { 0.0 };
            let mut c = 1.0;
            while c <= end {
                let dip = if c > 2.0 && wave { 1.0 } else { 0.0 };
                self.put(x + c, base - tall + r + dip, FLAG);
                c += 1.0;
            }
            r += 1.0;
        }
    }

    /// Fold each cell's four pixels into the two colors that best fit them.
    fn composite(&mut self) {
        let (w, pw) = (self.columns, self.pw);
        for cell in 0..w * self.rows {
            let r = cell / w;
            let c = cell - r * w;
            let k0 = 2 * r * pw + 2 * c;
            let q = [
                self.pc[k0],
                self.pc[k0 + 1],
                self.pc[k0 + pw],
                self.pc[k0 + pw + 1],
            ];
            let f = &mut self.fit;
            fit_quad(&q, f, NEAR, -1);
            if f.spread == 0 {
                self.out.set(cell, 0x20, q[0], q[0]);
                self.dominant[cell] = q[0];
                continue;
            }
            self.out.set(cell, QUAD[f.mask as usize], f.fg, f.bg);
            // The color covering most of the cell: what overlays put behind themselves.
            self.dominant[cell] = if BITS[f.mask as usize] > 2 {
                f.fg
            } else {
                f.bg
            };
        }
    }

    /// Spray, fizz and glints as braille dots over the cells they fall in.
    fn overlay_dots(&mut self) {
        let w = self.columns;
        let (pwf, phf) = (self.pw as f64, self.ph as f64);
        self.dots.fill(0);
        let ps = &self.particles;
        for i in 0..PMAX {
            if ps.life[i] <= 0.0 {
                continue;
            }
            let x = f64::from(ps.x[i]);
            let y = f64::from(ps.y[i]);
            if x < 0.0 || y < 0.0 || x >= pwf || y >= phf {
                continue;
            }
            let col = i32_of(x / 2.0) as usize;
            let row = i32_of(y / 2.0) as usize;
            let cell = row * w + col;
            if self.solid[cell] != 0 {
                continue;
            }
            let dc = (x.floor() - (col * 2) as f64) as usize;
            let dr = ((y - (row * 2) as f64) * 2.0).floor().min(3.0) as usize;
            if self.dots[cell] == 0 {
                self.dot_color[cell] = match ps.kind[i] {
                    GLINT => mix(self.pal[SUN], 0xffffff, 0.4),
                    WHITE_WATER => self.pal[FOAM],
                    _ => self.pal[SPRAY],
                };
            }
            self.dots[cell] |= BRAILLE[dc][dr] as u8;
        }
        for cell in 0..w * self.rows {
            if self.dots[cell] != 0 {
                self.out.set(
                    cell,
                    0x2800 + u32::from(self.dots[cell]),
                    self.dot_color[cell],
                    self.dominant[cell],
                );
            }
            self.solid[cell] = 0;
        }
    }

    fn overlay_gulls(&mut self) {
        let (pw, phf) = (self.pw, self.ph as f64);
        let pwf = pw as f64;
        let span = pwf + 20.0;
        let t = self.t;
        let top = if self.vertical { phf * 0.05 } else { 0.0 };
        let low = if self.vertical {
            self.crest_y.min(self.sea_y) - 3.0
        } else {
            self.sea_y - 1.5
        };
        let bottom = (top + 1.0).max(low);
        for gl in &self.gulls {
            let x = (((gl.x - self.scroll * 0.35 - t * gl.v) % span + span) % span) - 10.0;
            let y = top + gl.y * (bottom - top) + (t * 0.05 + gl.ph).sin() * 0.6;
            if x < 0.0 || x >= pwf || y < 0.0 || y >= phf {
                continue;
            }
            let (xi, yi) = (x.floor() as usize, y.floor() as usize);
            if self.water[yi * pw + xi] != 0 {
                continue;
            }
            let cell = (yi >> 1) * self.columns + (xi >> 1);
            if self.dots[cell] != 0 {
                continue;
            }
            let up = (t * 0.35 + gl.ph).sin() > 0.0;
            let col = if self.k_storm > 0.5 {
                0xd8dde6
            } else {
                0x2a3340
            };
            self.out.set(
                cell,
                if up { GULL_UP } else { GULL_DOWN },
                col,
                self.dominant[cell],
            );
        }
    }
}

impl Scene for Surf {
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
        self.vertical = is_tall(columns, rows);
        self.pw = columns * 2;
        self.ph = rows * 2;
        let n = self.pw * self.ph;
        self.pc = vec![0; n];
        self.water = vec![0; n];
        self.dots = vec![0; columns * rows];
        self.dot_color = vec![0; columns * rows];
        self.solid = vec![0; columns * rows];
        self.dominant = vec![0; columns * rows];
        self.particles.life.fill(0.0);
        let (seed, pw) = (self.seed, self.pw as f64);
        let vertical = self.vertical;
        let ng = if vertical {
            3.0
        } else {
            round(columns as f64 / 45.0).max(2.0)
        };
        self.gulls = (0..ng as usize)
            .map(|i| {
                let i = i as f64;
                Gull {
                    x: hash(seed + i * 13.0) * pw,
                    y: hash(seed + i * 13.0 + 1.0),
                    ph: hash(seed + i * 13.0 + 2.0) * TURN,
                    v: 0.04 + hash(seed + i * 13.0 + 3.0) * 0.08,
                }
            })
            .collect();
        let nc = if vertical {
            3.0
        } else {
            round(columns as f64 / 40.0).max(2.0)
        };
        self.clouds = (0..nc as usize)
            .map(|i| {
                let i = i as f64;
                let (w, h) = (hash(seed + i * 29.0 + 7.0), hash(seed + i * 29.0 + 8.0));
                Cloud {
                    x: hash(seed + i * 29.0 + 5.0) * (pw + 40.0),
                    y: hash(seed + i * 29.0 + 6.0),
                    w: if vertical {
                        5.0 + w * 6.0
                    } else {
                        6.0 + w * 14.0
                    },
                    h: if vertical {
                        1.6 + h * 1.6
                    } else {
                        0.6 + h * 0.4
                    },
                }
            })
            .collect();
    }

    fn step(&mut self) {
        if self.columns == 0 {
            return;
        }
        let level = self.level();
        // A wave coming in: about every 12 s on a calm sea, every 5.5 s on a big
        // one, never on a beat: each one sets the next at random around that.
        // (flow hears it; only the draw for the next one matters here.)
        if self.s >= 1.0 && self.t >= self.next_wave {
            self.next_wave =
                self.t + round((12.6 - 0.7 * self.s) * 14.0 * (0.6 + 0.8 * self.rng.f()));
        }
        let on = |b: bool| if b { 1.0 } else { 0.0 };
        let tint = self.dials.tint;
        if !self.started {
            self.s = level;
            self.speed = speed_at(level);
            self.k_grey = on(tint == Tint::Smoke);
            self.k_storm = on(tint == Tint::Blue);
            self.k_night = on(self.dials.night);
            self.started = true;
        }
        self.t += 1.0;
        self.s += (level - self.s) * 0.035;
        let lo = self.s.floor();
        let l = lo as usize;
        let sp = SPEED[l] + (SPEED[(l + 1).min(10)] - SPEED[l]) * (self.s - lo);
        self.speed += (sp - self.speed) * 0.1;
        self.scroll += self.speed * if self.vertical { 1.4 } else { 1.0 };
        self.k_grey += (on(tint == Tint::Smoke) - self.k_grey) * 0.05;
        self.k_storm += (on(tint == Tint::Blue) - self.k_storm) * 0.05;
        self.k_night += (on(self.dials.night) - self.k_night) * 0.04;
        if level <= 0.0 {
            return;
        }
        self.layout();

        // Wipeout when a command fails.
        if tint == Tint::Smoke && self.last_tint != Tint::Smoke && self.wipe == 0.0 {
            self.start_wipeout();
        }
        self.last_tint = tint;
        let wiping = self.wipe > 0.0 && {
            self.wipe += 1.0;
            self.wipe > 46.0
        };
        if wiping {
            self.wipe = 0.0;
            self.recover = 34.0;
        } else if self.recover > 0.0 {
            self.recover -= 1.0;
        }

        // The surfer's line on the face.
        let s = self.s;
        let omega = 0.035 + 0.011 * s;
        self.phase += omega;
        let in_tube = self.curl > 0.55;
        let amp = if in_tube {
            0.05
        } else {
            clamp(0.08 + 0.04 * (s - 3.0), 0.06, 0.36)
        };
        let mid = if in_tube { 0.4 } else { 0.52 };
        let nu = mid - amp * self.phase.cos();
        let ndu = nu - self.u;
        // Snapping off the top: a fan of spray from the tail.
        let k = if self.vertical { 1.6 } else { 1.0 };
        if self.du < 0.0 && ndu >= 0.0 && s >= 5.0 && !in_tube && self.wipe == 0.0 {
            let x = self.cx + self.u * self.wf;
            let y = self.surface_at(x);
            let n = round((s - 3.0) * 2.5 * k);
            let mut i = 0.0;
            while i < n {
                let vx = (-0.2 - self.rng.f() * 0.9) * k;
                let vy = (-0.25 - self.rng.f() * 0.5) * k;
                let life = 8.0 + self.rng.f() * 10.0;
                self.particles.emit(x, y - 1.0, vx, vy, life, SPRAY_DOT);
                i += 1.0;
            }
        }
        self.u = nu;
        self.du = ndu;

        // Spray blown back off the lip.
        if s >= 5.5 {
            let n = (s - 5.0) * 0.7 * k;
            let whole = n.floor()
                + if self.rng.f() < n - n.floor() {
                    1.0
                } else {
                    0.0
                };
            let mut i = 0.0;
            while i < whole {
                let x = (if self.curl > 0.0 { self.ex } else { self.cx })
                    + (self.rng.f() - 0.6) * 2.0 * k;
                let y = self.crest_y + self.rng.f() * 0.6;
                let vx = (-0.3 - self.rng.f() * (0.4 + s * 0.09)) * k;
                let vy = (-0.08 - self.rng.f() * 0.3) * k;
                let life = 8.0 + self.rng.f() * 14.0;
                self.particles.emit(x, y, vx, vy, life, SPRAY_DOT);
                i += 1.0;
            }
        }
        // The pitching lip: falling curtain at its tip and an explosion where it lands.
        if self.curl > 0.3 {
            let a = (self.phi_end * PI) / 180.0;
            let tx = self.ex + self.rx * a.cos();
            let ty = self.ey + self.ry * a.sin();
            let mut i = 0.0;
            while i < 2.0 * k {
                let x = tx + (self.rng.f() - 0.5);
                let vx = (self.rng.f() - 0.3) * 0.4;
                let vy = 0.15 + self.rng.f() * 0.3;
                let life = 6.0 + self.rng.f() * 6.0;
                self.particles.emit(x, ty, vx, vy, life, SPRAY_DOT);
                i += 1.0;
            }
            let bx = self.ex + self.rx * (0.9 + self.rng.f() * 0.6);
            let mut i = 0.0;
            while i < 3.0 * k * self.curl {
                let x = bx + self.rng.f() * 3.0 * k;
                let vx = (0.1 + self.rng.f() * 0.8) * k;
                let vy = (-0.3 - self.rng.f() * 0.8) * k;
                let life = 6.0 + self.rng.f() * 10.0;
                self.particles
                    .emit(x, self.sea_y - 0.5, vx, vy, life, SPRAY_DOT);
                i += 1.0;
            }
        }
        // White water churning at the foot of a breaking face.
        if s >= 6.5 {
            let n = (s - 6.0) * 0.6 * k;
            let mut i = 0.0;
            while i < n {
                let x = self.cx + self.wf * (0.75 + self.rng.f() * 0.5);
                let y = self.sea_y - self.rng.f();
                let vx = (self.rng.f() - 0.3) * 0.5;
                let vy = -0.1 - self.rng.f() * 0.4 * k;
                let life = 4.0 + self.rng.f() * 8.0;
                self.particles.emit(x, y, vx, vy, life, WHITE_WATER);
                i += 1.0;
            }
        }
        // Paddling: little splashes off the hands.
        if self.pose() == Pose::Paddle && self.t % 5.0 == 0.0 {
            let sx = self.surfer_x();
            let x = sx + (if self.vertical { 4.0 } else { 1.5 }) + self.rng.f();
            let y = self.surface_at(sx) - 0.3;
            self.particles.emit(x, y, 0.1, -0.25, 5.0, WHITE_WATER);
        }
        // Sun glints on calm water.
        if s < 4.5 {
            let n = (4.5 - s)
                * 0.5
                * if self.vertical {
                    1.0
                } else {
                    self.pw as f64 / 120.0
                };
            let mut i = 0.0;
            while i < n {
                let x =
                    self.sun_x() + (self.rng.f() - 0.5) * if self.vertical { 14.0 } else { 30.0 };
                let y =
                    self.surface_at(x) + 0.3 + self.rng.f() * if self.vertical { 4.0 } else { 1.6 };
                let life = 2.0 + self.rng.f() * 3.0;
                self.particles.emit(x, y, 0.0, 0.0, life, GLINT);
                i += 1.0;
            }
        }
        // Move particles (Float32Arrays in flow: each store rounds to f32).
        let ps = &mut self.particles;
        for i in 0..PMAX {
            if ps.life[i] <= 0.0 {
                continue;
            }
            ps.life[i] = (f64::from(ps.life[i]) - 1.0) as f32;
            if ps.kind[i] == GLINT {
                continue;
            }
            ps.x[i] = (f64::from(ps.x[i]) + f64::from(ps.vx[i])) as f32;
            ps.y[i] = (f64::from(ps.y[i]) + f64::from(ps.vy[i])) as f32;
            ps.vy[i] = (f64::from(ps.vy[i]) + 0.04) as f32;
            ps.vx[i] = (f64::from(ps.vx[i]) * 0.97) as f32;
        }
        // The tumbling board.
        if self.wipe > 0.0 {
            self.board_x += self.board_vx;
            self.board_y += self.board_vy;
            self.board_vy += 0.07 * k;
            self.board_spin += 0.45;
            let floor = self.surface_at(self.board_x) - 0.5;
            if self.board_y > floor {
                self.board_y = floor;
                self.board_vy = -self.board_vy * 0.3;
                self.board_vx *= 0.6;
                self.board_spin *= 0.5;
            }
        }
    }

    fn grid(&mut self) -> &Cells {
        let (w, h) = (self.columns, self.rows);
        if self.level() <= 0.0 || w == 0 || h == 0 {
            for i in 0..w * h {
                self.out.blank(i);
            }
            return &self.out;
        }
        if !self.started {
            self.step();
        }
        self.layout();
        self.update_palette();
        self.paint_scene();
        self.paint_flag();
        self.paint_extras();
        self.paint_surfer();
        self.composite();
        self.overlay_dots();
        self.overlay_gulls();
        &self.out
    }
}

#[cfg(test)]
mod tests {
    use crate::flow::palette::tint_to;
    use crate::flow::scene::Scene;

    #[test]
    fn the_hero_board_wears_the_session_hue() {
        let mut s = super::Surf::new(7.0);
        s.dials().accent = Some(305.0);
        s.dials().strength = 3.0;
        s.ensure(30, 9);
        for _ in 0..60 {
            s.step();
        }
        let want = tint_to(super::BOARD, 305.0, 0.0);
        let grid = s.grid();
        let colours: Vec<u32> = (0..grid.len())
            .flat_map(|i| [grid.foreground(i), grid.background(i)])
            .collect();
        assert!(colours.contains(&want));
        assert!(!colours.contains(&super::BOARD));
    }

    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/surf.json"), 0.0);
    }
}
