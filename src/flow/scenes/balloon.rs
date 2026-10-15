//! A hot-air balloon in the sky world (flow's `hooks/balloon.ts`): the level
//! is its target altitude. At 1 it sits on the grass among trees and houses;
//! it climbs past birds and the layered clouds into a thinning sky; at 10 it
//! floats in space among stars, Earth's blue rim below. Subagents fly as
//! small companion balloons, a failure trails sooty smoke, and a
//! nearly-full context turns the stripes blue.

use crate::flow::cells::{is_tall, Cells};
use crate::flow::crew::{beckon, finished, Crew, Mate};
use crate::flow::js::{i32_of, round};
use crate::flow::palette::{tint_to, SceneHue};
use crate::flow::pixels::{
    dist, fit_quad, g, hash, hash1, mix, noise1, smooth, QuadFit, NEAR, QUAD,
};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use crate::flow::sky::{SkyScene, SkyWorld};
use crate::flow::waiting::breath;

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
const COMPANION: [[u32; 2]; 4] = [
    [0x2e9e5e, 0xf4efd8],
    [0x8a52dc, 0xf6a8cc],
    [0xe8862a, 0x23407e],
    [0xd2366e, 0x7fe0e8],
];
const MINI_TALL: [&[u8]; 8] = [
    b"..EEEEEE..",
    b".EEEEEEEE.",
    b"EEEEEEEEEE",
    b"EEEEEEEEEE",
    b".EEEEEEEE.",
    b"..EEFFEE..",
    b"...R..R...",
    b"....KK....",
];
const MINI_BAND: [&[u8]; 6] = [
    b"..EEEEEE..",
    b".EEEEEEEE.",
    b"EEEEEEEEEE",
    b".EEEEEEEE.",
    b"...EFFE...",
    b"....KK....",
];

/// The balloon, 7 wide × 4 tall; ' ' cells let the sky through.
const SPRITE: [&str; 4] = [" ▄▆█▆▄ ", "███████", " ▀█▀█▀ ", "  ╲█╱  "];

pub struct Balloon {
    world: SkyWorld,
    crew: Crew,
    vy: [f64; 4],
    lift: [f64; 4],
    main_x: f64,
    main_top: f64,
}

impl Balloon {
    pub fn new(seed: f64) -> Self {
        Self {
            world: SkyWorld::new(seed),
            crew: Crew::new(4, 24.0, 36.0),
            vy: [0.0; 4],
            lift: [0.0; 4],
            main_x: 0.0,
            main_top: 0.0,
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

    fn draw_balloon(&self, out: &mut Cells, top: f64) {
        let w = self.world.columns as f64;
        let bx = self.balloon_x();
        let tint = self.world.dials.tint;
        let blue = tint == Tint::Blue;
        let burning = !self.world.dials.waiting
            && (self.world.goal.max(self.world.target()) > self.world.alt + 0.3
                || self.world.dials.strength >= 6.0);
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
                        mix(BLUE_A, FLAME[0], self.world.k_wait * breath(self.world.t))
                    } else {
                        mix(stripe_a, FLAME[0], self.world.k_wait * breath(self.world.t))
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
        self.crew.room = self.room_for();
        self.crew
            .update(&self.world.dials.agents, self.world.dials.coverage_boost);
        self.main_x = self.balloon_x();
        self.main_top = self.vehicle_top();
        for i in 0..self.crew.mates.len() {
            let mut m = self.crew.mates[i].clone();
            let rank = self.crew.mates.iter().filter(|o| o.slot < m.slot).count();
            self.fly(&mut m, rank, self.crew.mates.len());
            self.crew.mates[i] = m;
        }
    }

    fn grid(&mut self) -> &Cells {
        self.sky_grid()
    }
}

impl Balloon {
    fn mini(&self) -> &'static [&'static [u8]] {
        if is_tall(self.world.columns, self.world.rows) {
            &MINI_TALL
        } else {
            &MINI_BAND
        }
    }
    fn across(&self) -> usize {
        if self.world.columns >= 19 {
            2
        } else {
            1
        }
    }
    fn lane_bottom(&self) -> f64 {
        ((self.world.rows as f64 * 0.45).floor() * 2.0 - 8.0).max(2.0)
    }
    fn room_for(&self) -> usize {
        if !is_tall(self.world.columns, self.world.rows) {
            (self.world.columns / 12).clamp(1, 4)
        } else {
            (self.across() * (((self.lane_bottom() - 2.0) / 9.0).floor() as usize + 1)).clamp(1, 4)
        }
    }
    fn burner_lit(&self, m: &Mate) -> bool {
        m.busy > 0.5
            && !m.leaving
            && noise1(
                self.world.t * 0.06,
                f64::from(i32_of(m.seed * 2147483647.0)) + 17.0,
            ) > 0.22
    }
    fn fly(&mut self, m: &mut Mate, rank: usize, n: usize) {
        let w = self.world.columns as f64;
        let h = self.world.rows as f64;
        let sh = self.mini().len() as f64;
        let cols = 5.0;
        let s = f64::from(i32_of(m.seed * 2147483647.0));
        let u1 = hash1(s + 1.0);
        let u2 = hash1(s + 2.0);
        let u3 = hash1(s + 3.0);
        let pace = 0.012 + 0.014 * u3;
        let t = self.world.t;
        let wy = (noise1(t * pace, s) * 0.7 + noise1(t * pace * 2.3, s + 5.0) * 0.3 - 0.5) * 2.0;
        let wx = (noise1(t * pace * 0.6, s + 11.0) * 0.7 + noise1(t * pace * 1.7, s + 13.0) * 0.3
            - 0.5)
            * 2.0;
        let mut lift = self.lift[m.slot];
        if m.y.is_nan() {
            lift = m.busy;
        } else {
            lift += (m.busy - lift) * (0.01 + 0.14 * hash1(s + 8.0).powi(2));
        }
        self.lift[m.slot] = lift;
        let lit = self.burner_lit(m);
        let mx0 = self.main_x - 1.0;
        let mx1 = self.main_x + 8.0;
        let my0 = self.main_top * 2.0 - 1.0;
        let my1 = (self.main_top + 4.0) * 2.0 + 1.0;
        let x = m.x;
        let y = m.y;
        let mut goal;
        let mut gx;
        let mut ease = 0.02 + 0.03 * hash1(s + 6.0);
        if !is_tall(self.world.columns, self.world.rows) {
            let floor = (2.0 * h - sh).max(0.0);
            let hi = 1.0 + u1 * 0.8;
            goal = (floor + (hi - floor) * lift + wy * (0.4 + 1.1 * lift)
                - if lit { 0.6 } else { 0.0 })
            .min(floor)
            .max(0.0);
            let a = (mx0 - cols).max(0.0);
            let b = (w - cols - mx1).max(0.0);
            let spacing = (a + b) / (n + 1) as f64;
            let amp = ((spacing - cols - 1.0) / 2.0).clamp(0.0, 4.0);
            let u = (rank + 1) as f64 * spacing + (u2 - 0.5) * amp * 0.6 + wx * amp * 0.7;
            gx = (if u <= a { u } else { mx1 + u - a })
                .min(w - cols)
                .max(0.0);
            if (gx - x).abs() > spacing / 2.0 || (x < mx1 && x + cols > mx0) {
                ease = 0.2;
            }
        } else {
            let floor = (2.0 * (h - 1.0) - sh).max(0.0);
            let across = self.across();
            let rows = ((self.crew.room.max(1) as f64 / across as f64).ceil()).max(1.0);
            let f = if rows > 1.0 {
                (m.slot / across) as f64 / (rows - 1.0)
            } else {
                0.5
            };
            let lo_top = floor.min((h * 0.55).floor() * 2.0);
            let hi = (2.0 + f * (self.lane_bottom() - 2.0) + (u1 - 0.5) * 2.0)
                .min(floor)
                .max(0.0);
            let lo = hi.max(lo_top + f * (floor - lo_top) + (u1 - 0.5) * 2.0);
            goal = lo + (hi - lo) * lift + wy * 3.0 - if lit { 1.5 } else { 0.0 };
            let on_left = across == 1 || m.slot.is_multiple_of(2);
            let tx = if across == 1 {
                (w - cols) / 2.0 + (u2 - 0.5) * 2.0
            } else if on_left {
                0.5 + u2 * 1.5
            } else {
                w - cols - 0.5 - u2 * 1.5
            };
            gx = (tx + wx).min(w - cols).max(0.0);
            for o in &self.crew.mates {
                if o.slot == m.slot || o.y.is_nan() || (gx - o.x).abs() >= cols + 1.0 {
                    continue;
                }
                let d = goal - o.y;
                let gap = sh + 1.0 - d.abs();
                if gap > 0.0 {
                    goal += (if d < 0.0 || (d == 0.0 && m.slot < o.slot) {
                        -gap
                    } else {
                        gap
                    }) * 0.5;
                }
            }
            goal = goal.min(floor).max(0.0);
            let yy = if y.is_nan() { goal } else { y };
            let xx = if x.is_nan() { gx } else { x };
            if (goal < my1 && goal + sh > my0) || (yy < my1 && yy + sh > my0) {
                let right = xx + cols / 2.0 >= self.main_x + 3.5;
                let was = gx;
                if right && mx1 + cols <= w {
                    gx = gx.max(mx1);
                } else if !right && mx0 - cols >= 0.0 {
                    gx = gx.min(mx0 - cols);
                } else if gx < mx1 && gx + cols > mx0 {
                    let above = my0 - sh;
                    let up = above >= 0.0 && (goal + sh / 2.0 < (my0 + my1) / 2.0 || my1 > floor);
                    if up {
                        goal = above;
                    } else if my1 <= floor {
                        goal = my1;
                    }
                }
                if gx != was {
                    ease = 0.15;
                }
            }
        }
        if x.is_nan() || y.is_nan() {
            m.x = gx;
            m.y = goal;
            self.vy[m.slot] = 0.0;
            return;
        }
        let k = if goal < y {
            0.004 + 0.016 * hash1(s + 4.0)
        } else {
            0.0015 + 0.008 * hash1(s + 5.0)
        };
        let v = (self.vy[m.slot] + k * (goal - y)) * (1.0 - 1.5 * k.sqrt());
        m.y = (y + v).min(2.0 * h - sh).max(0.0);
        self.vy[m.slot] = if m.y == y + v { v } else { 0.0 };
        m.x = x + (gx - x) * ease;
    }
    fn draw_companions(&mut self, out: &mut Cells) {
        let w = self.world.columns;
        let h = self.world.rows;
        let sp = self.mini();
        let sh = sp.len() as f64;
        for j in 0..self.crew.mates.len() {
            let m = self.crew.mates[j].clone();
            if m.x.is_nan() {
                continue;
            }
            let s = f64::from(i32_of(m.seed * 2147483647.0));
            let away = 1.0 - smooth((m.p * (1.0 + 0.8 * hash1(s + 7.0))).clamp(0.0, 1.0));
            let py = round(if finished(Some(&m)) {
                m.y - away * (m.y + sh + 2.0)
            } else {
                m.y + away * (2.0 * h as f64 + 2.0 - m.y)
            });
            let x = round(
                m.x + if m.leaving {
                    away * if is_tall(w, h) { 4.0 } else { 10.0 }
                } else {
                    0.0
                },
            );
            if py >= 2.0 * h as f64 || py + sh <= 0.0 {
                continue;
            }
            let pair = COMPANION[m.slot % 4];
            let blue = self.world.dials.tint == Tint::Blue;
            let mut a = if blue {
                mix(pair[0], BLUE_A, 0.7)
            } else {
                pair[0]
            };
            let mut b = if blue {
                mix(pair[1], BLUE_B, 0.7)
            } else {
                pair[1]
            };
            let blink = m.waiting && beckon(&m, self.world.t);
            if blink {
                a = mix(a, 0xffffff, 0.45);
                b = mix(b, 0xffffff, 0.45);
            }
            let burner = if !self.burner_lit(&m) {
                a
            } else if self.world.dials.tint == Tint::Smoke {
                SMOKE
            } else {
                FLAME[(hash(f64::from(i32_of(self.world.t) >> 1), m.slot as f64, 5.0) * 3.0).floor()
                    as usize]
            };
            let part_color = |part: u8, c: usize| match part {
                b'E' => {
                    if c.is_multiple_of(2) {
                        a
                    } else {
                        b
                    }
                }
                b'F' => burner,
                b'R' => ROPE,
                _ => BASKET,
            };
            let r0 = (py / 2.0).floor().max(0.0) as usize;
            let r1 = ((py + sh - 1.0) / 2.0).floor().min(h as f64 - 1.0) as usize;
            for r in r0..=r1 {
                for c in 0..5 {
                    let xx = x + c as f64;
                    if xx < 0.0 || xx >= w as f64 {
                        continue;
                    }
                    let i = r * w + xx as usize;
                    let cp = out.code_point(i);
                    let fg = out.foreground(i);
                    let bg = out.background(i);
                    let under = if (0x2580..0x25a0).contains(&cp) {
                        QUAD.iter()
                            .position(|&q| q == cp)
                            .map(|x| x as i32)
                            .unwrap_or(-1)
                    } else if cp == 0x20 {
                        0
                    } else {
                        -1
                    };
                    let flat = out.behind(i);
                    let mut q = [0; 4];
                    let mut mine = 0;
                    let mut keep = -1;
                    let mut keep_part = 0;
                    let mut envelope = false;
                    for j in 0..4 {
                        q[j] = if under < 0 {
                            flat
                        } else if (under >> j) & 1 != 0 {
                            fg
                        } else {
                            bg
                        };
                        let sy = r as f64 * 2.0 + (j >> 1) as f64 - py;
                        if sy < 0.0 || sy >= sh {
                            continue;
                        }
                        let part = sp[sy as usize][c * 2 + (j & 1)];
                        let role = match part {
                            b'E' => 1,
                            b'F' => 2,
                            b'R' => 3,
                            b'K' => 4,
                            _ => 0,
                        };
                        if role == 0 {
                            continue;
                        }
                        q[j] = part_color(part, c);
                        mine |= 1 << j;
                        if role == 1 {
                            envelope = true;
                        }
                        if role > keep_part {
                            keep_part = role;
                            keep = j as i32;
                        }
                    }
                    if mine == 0 {
                        continue;
                    }
                    let mut fit = QuadFit::default();
                    fit_quad(&q, &mut fit, NEAR, keep);
                    if fit.spread == 0 {
                        out.set(i, QUAD[15], q[0], q[0]);
                    } else {
                        out.set(i, QUAD[fit.mask as usize], fit.fg, fit.bg);
                    }
                    if blink && envelope {
                        let e = part_color(b'E', c);
                        let f0 = out.foreground(i);
                        let b0 = out.background(i);
                        self.world.lamps[i] = if dist(f0, e) <= dist(b0, e) {
                            f0 as i32
                        } else {
                            b0 as i32
                        };
                    }
                }
            }
        }
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
