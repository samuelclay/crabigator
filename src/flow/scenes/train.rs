//! Passenger trains through a moving countryside. Each companion keeps its
//! own livery, arrives from behind and rests or leaves with its agent.
// Keep upstream geometry predicates and numeric constants exact for frame replay.
#![allow(
    non_snake_case,
    clippy::approx_constant,
    clippy::manual_range_contains,
    clippy::too_many_arguments
)]
use crate::flow::cells::{Cells, Rng};
use crate::flow::crew::{beckon, Crew, Mate};
use crate::flow::js::{hypot, i32_of, imul, round};
use crate::flow::night::{
    moon_cover, moon_pixel, moon_radius, MOON, NIGHT_HORIZON, NIGHT_ZENITH, STAR, STAR_DIM,
};
use crate::flow::palette::SceneHue;
use crate::flow::pixel_scene::{Painter, PixelDials, PixelScene};
use crate::flow::pixels::{clamp, clamp01, grey, hash, hash1, mix, noise1};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use crate::flow::waiting::breath;
use std::collections::{BTreeMap, BTreeSet};
mod above;
mod palette;
mod side;
use palette::{palette, Pal};
pub const DEF: SceneDef = SceneDef {
    name: "train",
    hue: SceneHue {
        // Keep land, signals and neutral details; turn the blue hues gently.
        key: 250.,
        range: 45.,
        spread: Some(45.),
    },
    figure: false,
    make: |seed| Box::new(Train::new(seed)),
};
const SPEED: [f64; 11] = [0., 0.3, 0.55, 0.85, 1.2, 1.6, 2.3, 3.4, 5., 7.2, 9.6];
const LOCO: i32 = 34;
const CAR: i32 = 30;
const GAP: i32 = 2;
const CARS: i32 = 5;
const TRAIN: i32 = LOCO + CARS * (GAP + CAR);
const UNIT: i32 = CAR + GAP + CAR;
const A_LOCO: i32 = 12;
const A_CAR: i32 = 10;
const A_GAP: i32 = 1;
const A_UNIT: i32 = A_CAR * 2 + A_GAP;
const ZONE: f64 = 900.;
const FIELDS: i32 = 0;
const VILLAGE: i32 = 1;
const WOODS: i32 = 2;
const RIVER: i32 = 3;
const STATION: i32 = 4;
const CROSS: f64 = 450.;
const ROAD: f64 = 5.;
const PLAT0: f64 = 250.;
const PLAT1: f64 = 650.;
const CANOPY0: f64 = 360.;
const CANOPY1: f64 = 540.;
const BRIDGE0: f64 = 410.;
const BRIDGE1: f64 = 520.;
const WATER0: f64 = 430.;
const WATER1: f64 = 500.;
const SIGNAL: f64 = 450.;
const SIGNAL0: f64 = 210.;
const F_CLOUD: f64 = 0.03;
const F_HILLFAR: f64 = 0.05;
const F_HILL: f64 = 0.1;
const F_TREES: f64 = 0.2;
const F_MID: f64 = 0.4;
const F_FIELD: f64 = 0.7;
const F_STATION: f64 = 0.6;
const F_FARPLAT: f64 = 0.86;
const F_FAR: f64 = 0.92;
const F_TRUSS: f64 = 1.05;
const F_POLE: f64 = 1.3;
const LIVERY: [u32; 4] = [0x2a6fd6, 0x2f9a4a, 0xe0781e, 0x8a4bc2];
const NIGHT_SHADE: u32 = 0x0a1428;
const FLASH: u32 = 0xffb020;
const RED: u32 = 0xff2a1a;
const GREEN: u32 = 0x2aff6a;
const RED_OFF: u32 = 0x3a1010;
const CAR_COLORS: [u32; 6] = [0xd8433a, 0x2f6fd0, 0xf0f0ea, 0x3a3d44, 0xe0b030, 0x4a9a5a];
fn raw_zone(z: f64) -> i32 {
    let h = hash1(z * 7919. + 1301.);
    if h < 0.4 {
        FIELDS
    } else if h < 0.58 {
        VILLAGE
    } else if h < 0.76 {
        WOODS
    } else if h < 0.88 {
        RIVER
    } else {
        STATION
    }
}
fn zone_of(z: f64) -> i32 {
    let k = raw_zone(z);
    if k != RIVER && k != STATION {
        return k;
    }
    let before = raw_zone(z - 1.);
    if before == RIVER || before == STATION {
        FIELDS
    } else {
        k
    }
}
fn crosses(zone: i32) -> bool {
    zone == FIELDS || zone == VILLAGE
}
fn speed_for(level: f64) -> f64 {
    let l = clamp(level, 0., 10.);
    let i = 9.0_f64.min(l.floor()) as usize;
    SPEED[i] + (SPEED[i + 1] - SPEED[i]) * (l - i as f64)
}
fn decel(v: f64) -> f64 {
    0.035 + 0.012 * v
}
fn q(k: f64, n: f64) -> f64 {
    round(clamp01(k) * n) / n
}
fn smear(buf: &mut [u32], w: usize, y: i32, run: f64) {
    let n = 10.0_f64.min(round(run * 0.8)) as usize;
    if n < 1 || y < 0 {
        return;
    }
    let o = y as usize * w;
    let last = o + w - 1;
    let (mut r, mut g, mut b) = (0i32, 0i32, 0i32);
    for x in 0..=n {
        let c = buf[last.min(o + x)];
        r += ((c >> 16) & 255) as i32;
        g += ((c >> 8) & 255) as i32;
        b += (c & 255) as i32;
    }
    let k = 1. / (n + 1) as f64;
    for x in 0..w {
        let c0 = buf[o + x];
        let c1 = buf[last.min(o + x + n + 1)];
        buf[o + x] = (((i32_of(r as f64 * k) >> 3) << 3) << 16
            | ((i32_of(g as f64 * k) >> 3) << 3) << 8
            | (i32_of(b as f64 * k) >> 3) << 3) as u32;
        r += ((c1 >> 16) & 255) as i32 - ((c0 >> 16) & 255) as i32;
        g += ((c1 >> 8) & 255) as i32 - ((c0 >> 8) & 255) as i32;
        b += (c1 & 255) as i32 - (c0 & 255) as i32;
    }
}
fn streak(buf: &mut [u32], w: usize, h: usize, n: i32) {
    let k = 1. / (n + 1) as f64;
    let mut col = vec![0; h];
    for x in 0..w {
        for y in 0..h {
            col[y] = buf[y * w + x];
        }
        let (mut r, mut g, mut b) = (0i32, 0i32, 0i32);
        for y in -n..h as i32 {
            let c = col[y.max(0) as usize];
            r += ((c >> 16) & 255) as i32;
            g += ((c >> 8) & 255) as i32;
            b += (c & 255) as i32;
            if y > 0 {
                let o = col[(y - n - 1).max(0) as usize];
                r -= ((o >> 16) & 255) as i32;
                g -= ((o >> 8) & 255) as i32;
                b -= (o & 255) as i32;
            }
            if y >= 0 {
                buf[y as usize * w + x] = (((i32_of(r as f64 * k) >> 3) << 3) << 16
                    | ((i32_of(g as f64 * k) >> 3) << 3) << 8
                    | (i32_of(b as f64 * k) >> 3) << 3)
                    as u32;
            }
        }
    }
}
#[derive(Clone)]
struct Puff {
    s: f64,
    up: f64,
    side: f64,
    r: f64,
    life: f64,
    vu: f64,
    vs: f64,
    wisp: bool,
    shade: f64,
}
struct Flock {
    x: f64,
    y: f64,
    vx: f64,
    n: i32,
    ph: f64,
}
#[derive(Clone)]
struct Motor {
    x: f64,
    dir: f64,
    v: f64,
    color: u32,
}
pub struct Train {
    base: PixelScene,
    crew: Crew,
    pos: f64,
    v: f64,
    started: bool,
    stop_at: Option<f64>,
    held: Option<f64>,
    extras: Vec<f64>,
    stood_at: Option<f64>,
    depart: f64,
    k_smoke: f64,
    k_storm: f64,
    puffs: Vec<Puff>,
    flock: Option<Flock>,
    next_flock: f64,
    roads: BTreeMap<i32, Vec<Motor>>,
    rng: Rng,
    w: i32,
    h: i32,
    tall: bool,
    nose_x: i32,
    tx: i32,
    nose_y: i32,
    canopy: Vec<u8>,
    row_v: Vec<i32>,
    solid: Vec<u8>,
    sky_row: Vec<u32>,
}
impl Train {
    pub fn new(seed: f64) -> Self {
        Self {
            base: PixelScene::new(seed),
            crew: Crew::new(4, 60., 70.),
            pos: 0.,
            v: 0.,
            started: false,
            stop_at: None,
            held: None,
            extras: vec![],
            stood_at: None,
            depart: 9.,
            k_smoke: 0.,
            k_storm: 0.,
            puffs: vec![],
            flock: None,
            next_flock: 200.,
            roads: BTreeMap::new(),
            rng: Rng::new(imul(i32_of(seed), 2654435761u32 as i32) as f64),
            w: 0,
            h: 0,
            tall: false,
            nose_x: 0,
            tx: 0,
            nose_y: 0,
            canopy: vec![],
            row_v: vec![],
            solid: vec![],
            sky_row: vec![],
        }
    }
    fn layout(&mut self, d: PixelDials) {
        self.w = (d.columns * 2) as i32;
        self.h = (d.rows * 2) as i32;
        self.tall = d.tall;
        let ahead = if self.small_box() {
            round(self.w as f64 * 0.25) as i32
        } else {
            26
        };
        self.nose_x = 2.max(self.w - ahead.max(round(self.w as f64 * 0.17) as i32)) & !1;
        self.tx = clamp(round(self.w as f64 * 0.36), 7., 7.max(self.w - 14) as f64) as i32;
        self.nose_y = 4.max(round(self.h as f64 * 0.22) as i32);
    }
    fn resize(&mut self, d: PixelDials) {
        self.layout(d);
        self.canopy = vec![0; (self.w * self.h) as usize];
        self.solid = vec![0; (self.w * self.h) as usize];
        self.row_v = vec![0; self.h as usize];
        self.roads.clear();
    }
    fn ahead(&self) -> f64 {
        if self.tall {
            (self.nose_y * 3) as f64
        } else {
            (self.w - self.nose_x) as f64
        }
    }
    fn far_behind(&self) -> f64 {
        -(if self.tall {
            (self.h - self.nose_y) * 3 + 6
        } else {
            self.nose_x + 6
        }) as f64
    }
    fn stopping_distance(&self) -> f64 {
        let mut d = 0.;
        let mut u = self.v;
        while u > 1.5 {
            d += u;
            u -= decel(u);
        }
        let a = self.v.min(1.5);
        d + (a * a) / (2. * 0.015) + 4.
    }
    fn platform_end(a: f64, b: f64) -> Option<f64> {
        for z in (a / ZONE).floor() as i32..=(b / ZONE).floor() as i32 + 1 {
            if zone_of(z as f64) != STATION {
                continue;
            }
            let at = z as f64 * ZONE + PLAT1 - 24.;
            if at >= a && at <= b {
                return Some(at);
            }
        }
        None
    }
    fn in_platform(s: f64) -> bool {
        let z = (s / ZONE).floor();
        let local = s - z * ZONE;
        zone_of(z) == STATION && local > PLAT0 && local < PLAT1
    }
    fn clear(mut s: f64) -> f64 {
        for _ in 0..6 {
            let mut moved = false;
            for z in
                ((s - TRAIN as f64 - 40.) / ZONE).floor() as i32..=((s + 40.) / ZONE).floor() as i32
            {
                let zone = zone_of(z as f64);
                let base = z as f64 * ZONE;
                let (a, b) = if crosses(zone) {
                    (base + CROSS - ROAD - 12., base + CROSS + ROAD + 12.)
                } else if zone == RIVER {
                    (base + BRIDGE0 - 12., base + BRIDGE1 + 12.)
                } else {
                    (0., -1.)
                };
                if b < a {
                    continue;
                }
                if s - (TRAIN as f64) < b && s > a {
                    s = b + TRAIN as f64;
                    moved = true;
                }
            }
            if !moved {
                break;
            }
        }
        s
    }
    fn choose_stop(&mut self) {
        let dmin = self.stopping_distance();
        let reach = dmin.max(self.ahead() + 12.);
        if let Some(platform) = Self::platform_end(self.pos + dmin, self.pos + reach + 260.) {
            self.stop_at = Some(platform);
            self.held = None;
            return;
        }
        let k = ((self.pos + dmin + 3. - SIGNAL0) / SIGNAL).ceil();
        let sg = k * SIGNAL + SIGNAL0;
        if sg - 3. - self.pos <= reach + 120. && Self::clear(sg - 3.) == sg - 3. {
            self.stop_at = Some(sg - 3.);
            self.held = Some(sg);
            return;
        }
        let at = Self::clear(round(self.pos + reach)) + 3.;
        if let Some(station) = Self::platform_end(at - 3., at - 3. + PLAT1 - PLAT0) {
            if station - (at - 3.) < PLAT1 - PLAT0 && Self::in_platform(at - 3.) {
                self.stop_at = Some(station);
                self.held = None;
                return;
            }
        }
        self.extras.push(at);
        self.stop_at = Some(at - 3.);
        self.held = Some(at);
    }
    fn update(&mut self, d: PixelDials) {
        self.layout(d);
        if self.w == 0 || self.h == 0 {
            return;
        }
        let want = clamp(self.base.dials.strength, 0., 10.);
        let dt = if self.base.dials.strength <= 1. && self.base.dials.tint == Tint::Normal {
            125. / 70.
        } else {
            1.
        };
        let idle = want <= 1.25 || self.base.dials.waiting;
        if !self.started {
            self.started = true;
            if idle {
                self.pos = Self::clear(self.pos);
                self.extras.push(self.pos + 3.);
                self.stop_at = Some(self.pos);
                self.held = Some(self.pos + 3.);
            } else {
                self.v = speed_for(want);
            }
        }
        let was = self.pos;
        let vt;
        if !idle {
            if self.stop_at.is_some() && self.v == 0. {
                self.stood_at = self.held;
            }
            self.stop_at = None;
            self.held = None;
            if self.v == 0. && self.depart < 9. {
                self.depart += dt;
                vt = 0.;
            } else {
                vt = speed_for(want);
            }
        } else {
            if self.stop_at.is_none() && self.v == 0. {
                self.stop_at = Some(self.pos);
                self.held = self.stood_at;
            } else if self.stop_at.is_none() {
                self.choose_stop();
            }
            let left = 0.0_f64.max(self.stop_at.unwrap() - self.pos);
            vt = if self.v == 0. {
                0.
            } else {
                clamp(self.v, 0.6, 1.5).min((2. * 0.015 * left).sqrt())
            };
        }
        let dv = vt - self.v;
        self.v += if dv > 0. {
            dv.min((0.022 + 0.01 * self.v) * dt)
        } else {
            dv.max(-decel(self.v) * dt)
        };
        self.pos += self.v * dt;
        if idle {
            if let Some(stop) = self.stop_at {
                if self.pos >= stop - 0.05 || (self.v < 0.02 && stop - self.pos < 0.6) {
                    self.pos = stop;
                    self.v = 0.;
                }
            }
        }
        if self.v == 0. && idle {
            self.depart = 0.;
        }
        let behind = self.pos - TRAIN as f64 - 1200.;
        self.extras.retain(|s| *s > behind);
        self.k_smoke = clamp01(
            self.k_smoke
                + (if self.base.dials.tint == Tint::Smoke {
                    0.2
                } else {
                    -0.035
                }) * dt,
        );
        self.k_storm = clamp01(
            self.k_storm
                + (if self.base.dials.tint == Tint::Blue {
                    0.05
                } else {
                    -0.04
                }) * dt,
        );
        self.move_companions(d);
        self.move_smoke(was, dt);
        if self.tall {
            self.move_traffic(dt);
        } else {
            self.move_birds(d, dt);
        }
    }
    fn move_birds(&mut self, d: PixelDials, dt: f64) {
        if let Some(f) = self.flock.as_mut() {
            f.x += (f.vx - self.v * F_MID) * dt;
            if f.x < -24. || f.x > self.w as f64 + 24. {
                self.flock = None;
                self.next_flock = 250. + self.rng.f() * 700.;
            }
        } else {
            self.next_flock -= dt;
            if self.next_flock > 0. || d.night > 0.5 || self.k_storm > 0.3 {
                return;
            }
            let dir = if self.rng.f() < 0.5 { 1. } else { -1. };
            let hz = self.h - 6;
            self.flock = Some(Flock {
                x: if dir > 0. { -8. } else { self.w as f64 + 8. },
                y: 1. + self.rng.f() * 1.max(hz * 2 - 4) as f64,
                vx: dir * (0.22 + self.rng.f() * 0.18),
                n: 2 + (self.rng.f() * 4.).floor() as i32,
                ph: self.rng.f() * 8.,
            });
        }
    }
    fn small_box(&self) -> bool {
        !self.tall && self.w <= 80 && self.h <= 24
    }
    fn station(slot: usize) -> f64 {
        -6. - slot as f64 * (UNIT as f64 + 14.)
    }
    fn in_view(&self) -> f64 {
        if self.tall {
            3. * ((self.nose_y - self.h) as f64 + A_UNIT as f64 / 2.)
        } else {
            (if self.small_box() {
                (UNIT as f64 / 2.).min(self.w as f64 * 0.3)
            } else {
                UNIT as f64 / 2.
            }) - (self.nose_x - 1) as f64
        }
    }
    fn move_companions(&mut self, d: PixelDials) {
        let least = self.in_view();
        let mut n = 0;
        let sway_scale = if self.small_box() { 1. / 3. } else { 1. };
        while n < 4 && Self::station(n) - 12. * sway_scale >= least {
            n += 1;
        }
        self.crew.room = n;
        self.crew.update(&self.base.dials.agents, d.boost);
        let far = self.far_behind();
        for m in &mut self.crew.mates {
            let ph = m.seed * 6.283;
            let station = Self::station(m.slot);
            let rest = least.max(station - 26.);
            let sway = ((d.t * 0.0037 + ph).sin() * 7. + (d.t * 0.0011 + ph * 2.3).sin() * 5.)
                * m.busy
                * sway_scale;
            let at = rest + (station - rest) * m.busy + sway;
            let out = far - UNIT as f64 - if m.leaving { 40. } else { 0. };
            m.x = out + (at - out) * m.here;
        }
    }
    fn cab(m: &Mate, t: f64) -> i32 {
        if m.waiting && !m.leaving {
            if beckon(m, t) {
                2
            } else {
                0
            }
        } else if m.busy >= 0.5 || m.leaving || m.here < 1. {
            1
        } else {
            0
        }
    }
    fn move_smoke(&mut self, was: f64, dt: f64) {
        if self.k_smoke > 0.3 && self.base.dials.tint == Tint::Smoke {
            let run = self.pos - was;
            let n = ((3. + run * 0.8) * dt).ceil() as usize;
            for _ in 0..n {
                self.puffs.push(Puff {
                    s: was + run * self.rng.f() - 14.5 + self.rng.f() * 1.5,
                    up: 0.,
                    side: (self.rng.f() - 0.5) * 0.6,
                    r: 0.9 + self.rng.f() * 0.5,
                    life: 1.,
                    vu: (0.06 + self.rng.f() * 0.08) / (1. + self.v * 0.5),
                    vs: 0.03 + self.rng.f() * 0.06,
                    wisp: false,
                    shade: self.rng.f(),
                });
            }
        } else if self.v < 1. && self.rng.f() < 0.12 * dt && self.puffs.len() < 40 {
            self.puffs.push(Puff {
                s: self.pos - 14.5,
                up: 0.,
                side: 0.,
                r: 0.6,
                life: 0.6,
                vu: 0.08 + self.rng.f() * 0.05,
                vs: 0.02,
                wisp: true,
                shade: 0.,
            });
        }
        for p in &mut self.puffs {
            p.up += p.vu * dt;
            p.vu *= 0.985;
            p.side += p.vs * dt;
            p.s -= 0.06 * dt;
            p.r = 3.4_f64.min(p.r + 0.035 * dt);
            p.life -= 0.011 * dt;
        }
        self.puffs.retain(|p| p.life > 0.);
        if self.puffs.len() > 300 {
            self.puffs.drain(..self.puffs.len() - 300);
        }
    }
    fn gate_shut(&self, vc: f64) -> bool {
        let nose = self.pos / 3.;
        let mut back = (self.pos - TRAIN as f64) / 3.;
        for m in &self.crew.mates {
            if !m.x.is_nan() {
                back = back.min((self.pos + m.x - UNIT as f64) / 3.);
            }
        }
        vc >= back - 3. && vc <= nose + 30.
    }
    fn move_traffic(&mut self, dt: f64) {
        let top = (self.pos / 3.).floor() + self.nose_y as f64 + 4.;
        let bottom = top - self.h as f64 - 8.;
        let mut seen = BTreeSet::new();
        for z in ((bottom * 3.) / ZONE).floor() as i32..=((top * 3.) / ZONE).floor() as i32 {
            if !crosses(zone_of(z as f64)) {
                continue;
            }
            let vc = ((z as f64 * ZONE + CROSS) / 3.).floor();
            if vc < bottom || vc > top {
                continue;
            }
            seen.insert(z);
            if !self.roads.contains_key(&z) {
                let mut cars = vec![];
                for i in 0..4 {
                    cars.push(Motor {
                        x: self.rng.f() * self.w as f64,
                        dir: if i % 2 == 0 { 1. } else { -1. },
                        v: 0.25 + self.rng.f() * 0.2,
                        color: CAR_COLORS
                            [(self.rng.f() * CAR_COLORS.len() as f64).floor() as usize],
                    });
                }
                self.roads.insert(z, cars);
            }
            let shut = self.gate_shut(vc);
            let left = (self.tx - 5) as f64;
            let right = (self.tx + 11) as f64;
            let cars = self.roads.get_mut(&z).unwrap();
            for i in 0..cars.len() {
                let c = &cars[i];
                let mut nx = c.x + c.dir * c.v * dt;
                if shut && c.dir > 0. && c.x + 1. < left && nx + 1. >= left {
                    nx = left - 1.01;
                }
                if shut && c.dir < 0. && c.x > right && nx <= right {
                    nx = right + 0.01;
                }
                for (j, o) in cars.iter().enumerate() {
                    if j != i
                        && o.dir == c.dir
                        && (o.x - nx) * c.dir > 0.
                        && (o.x - nx) * c.dir < 3.2
                    {
                        nx = c.x;
                    }
                }
                let c = &mut cars[i];
                c.x = nx;
                if c.x > self.w as f64 + 2. || c.x < -4. {
                    c.x = if c.dir > 0. {
                        -3. - self.rng.f() * 24.
                    } else {
                        self.w as f64 + 2. + self.rng.f() * 24.
                    };
                    c.color = CAR_COLORS[(self.rng.f() * CAR_COLORS.len() as f64).floor() as usize];
                }
            }
        }
        self.roads.retain(|z, _| seen.contains(z));
    }
    fn red(&self, sg: f64) -> bool {
        self.held == Some(sg) || sg < self.pos - 2.
    }
    fn lights(&self, d: PixelDials) -> f64 {
        q(d.night.max(self.k_storm * 0.8), 6.)
    }
    fn wait_glow(d: PixelDials) -> f64 {
        if d.wait > 0. {
            q(d.wait * (0.2 + 0.8 * breath(d.t)), 6.)
        } else {
            0.
        }
    }
    fn signal_lamp(&self, px: &mut Painter, x: i32, y: i32, sg: f64, glow: f64) {
        if !self.red(sg) || Some(sg) != self.held || glow <= 0. {
            px.lamp(x as f64, y as f64, if self.red(sg) { RED } else { GREEN });
            return;
        }
        for dy in -1..=1 {
            for dx in -2..=2 {
                let r = hypot(dx as f64, (dy * 2) as f64) / 2.6;
                if (dx == 0 && dy == 0)
                    || r >= 1.
                    || x + dx < 0
                    || y + dy < 0
                    || x + dx >= px.w as i32
                    || y + dy >= px.h as i32
                {
                    continue;
                }
                let a = q((1. - r) * glow * 0.85, 4.);
                if a > 0. {
                    let c = mix(px.get((x + dx) as f64, (y + dy) as f64), 0xff6a3a, a);
                    px.set((x + dx) as f64, (y + dy) as f64, c, false);
                }
            }
        }
        px.lamp(x as f64, y as f64, mix(RED, 0xffc8a8, q(glow * 0.7, 4.)));
    }
    fn paint(&mut self, px: &mut Painter, d: PixelDials) {
        if px.w == 0 || px.h == 0 {
            return;
        }
        self.layout(d);
        if self.solid.len() != px.w * px.h {
            self.resize(d);
        }
        self.solid.fill(0);
        let p = palette(d.night, self.k_smoke, self.k_storm);
        if self.tall {
            self.paint_above(px, d, p);
        } else {
            self.paint_side(px, d, p);
        }
    }
    fn puff(&self, px: &mut Painter, pal: Pal, p: &Puff, cx: f64, cy: f64, r: f64, ry: f64) {
        let a = p.life * if p.wisp { 0.4 } else { 0.95 };
        if a < 0.05 {
            return;
        }
        let c = if p.wisp {
            pal.haze
        } else {
            mix(pal.soot, pal.haze, q(p.shade * 0.45, 3.))
        };
        for y in 0.max((cy - ry).floor() as i32)..=(px.h as i32 - 1).min((cy + ry).ceil() as i32) {
            for x in 0.max((cx - r).floor() as i32)..=(px.w as i32 - 1).min((cx + r).ceil() as i32)
            {
                let dx = (x as f64 + 0.5 - cx) / r;
                let dy = (y as f64 + 0.5 - cy) / ry;
                let d2 = dx * dx + dy * dy;
                if d2 > 1. {
                    continue;
                }
                let k = q(a * 1.0_f64.min((1. - d2) * 2.5), 4.);
                if k > 0. {
                    let i = y as usize * px.w + x as usize;
                    px.px[i] = mix(px.px[i], c, k);
                }
            }
        }
    }
    fn rain(&self, px: &mut Painter, d: PixelDials, p: Pal) {
        let kb = self.k_storm;
        if kb < 0.05 {
            return;
        }
        let (dw, dh) = (px.dw as f64, px.dh as f64);
        let n = round(dw * dh * 0.012 * kb) as i32;
        let slant = if self.tall { 0.3 } else { 0.35 + self.v * 0.35 };
        let fall = if self.tall { 2.2 } else { 1.8 };
        for i in 0..n {
            let sp = 0.8 + hash1((i * 7 + 3) as f64) * 0.5;
            let y = (((hash1((i * 7 + 1) as f64) * dh + d.t * fall * sp) % dh) + dh) % dh;
            let x = (((hash1((i * 7 + 2) as f64) * dw
                - d.t * slant * fall * sp
                - if self.tall { 0. } else { self.pos * 0.6 })
                % dw)
                + dw)
                % dw;
            for k in 0..2 {
                let xx = (x + k as f64 * slant).floor() % dw;
                let yy = (y - k as f64).floor() as i32;
                if yy < 0 {
                    continue;
                }
                let c = (yy as usize >> 2) * 2 * px.w + (xx as usize & !1);
                if self.solid[c] != 0
                    || self.solid[c + 1] != 0
                    || self.solid[c + px.w] != 0
                    || self.solid[c + px.w + 1] != 0
                {
                    continue;
                }
                px.dot(xx, yy as f64, p.rain);
            }
        }
    }
}
impl Scene for Train {
    fn dials(&mut self) -> &mut Dials {
        &mut self.base.dials
    }
    fn ensure(&mut self, columns: usize, rows: usize) {
        if self.base.ensure(columns, rows) {
            self.resize(self.base.frame_dials());
        }
    }
    fn step(&mut self) {
        let d = self.base.step();
        self.update(d);
    }
    fn grid(&mut self) -> &Cells {
        if self.base.dials.strength <= 0. {
            return self.base.blank();
        }
        let mut p = self.base.begin_paint();
        self.paint(&mut p, self.base.frame_dials());
        self.base.finish_grid(p)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/train.json"), 0.0);
    }
}
