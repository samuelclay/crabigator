//! The launch site's particles, in world pixels: vapour venting off the
//! fuelled rocket, steam and smoke billowing off the pad, the contrail, the
//! Ship's fireball and spray, plasma off a heat shield coming home, and the
//! puffs a tint sends off the rocket aloft. Stored as flow stores them, in
//! `Float32Array`s, so they move by the same roundings.

use crate::flow::pixels::mix;
use crate::flow::scene::Tint;
use crate::flow::sky::SkyScene;

use super::{LaunchSite, Part, State};

/// The most particles alive at once: the oldest is reused.
pub(super) const PMAX: usize = 384;

pub(super) struct Particles {
    pub px: Vec<f32>,
    pub py: Vec<f32>,
    pub vx: Vec<f32>,
    pub vy: Vec<f32>,
    pub life: Vec<f32>,
    pub max_life: Vec<f32>,
    pub r0: Vec<f32>,
    pub r1: Vec<f32>,
    pub a0: Vec<f32>,
    pub col: Vec<u32>,
    next: usize,
}

impl Particles {
    pub fn new() -> Self {
        Self {
            px: vec![0.0; PMAX],
            py: vec![0.0; PMAX],
            vx: vec![0.0; PMAX],
            vy: vec![0.0; PMAX],
            life: vec![0.0; PMAX],
            max_life: vec![0.0; PMAX],
            r0: vec![0.0; PMAX],
            r1: vec![0.0; PMAX],
            a0: vec![0.0; PMAX],
            col: vec![0; PMAX],
            next: 0,
        }
    }

    /// A new particle: where, its velocity, its life in frames, its radius at
    /// birth and death, its colour and its opacity at birth.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        x: f64,
        y: f64,
        vx: f64,
        vy: f64,
        life: f64,
        r0: f64,
        r1: f64,
        color: u32,
        a0: f64,
    ) {
        let i = self.next;
        self.next = (i + 1) % PMAX;
        self.px[i] = x as f32;
        self.py[i] = y as f32;
        self.vx[i] = vx as f32;
        self.vy[i] = vy as f32;
        self.life[i] = life as f32;
        self.max_life[i] = life as f32;
        self.r0[i] = r0 as f32;
        self.r1[i] = r1 as f32;
        self.col[i] = color;
        self.a0[i] = a0 as f32;
    }

    /// Move every particle up (or down) by `dy` world pixels: the sky folding or unfolding under them.
    pub fn shift(&mut self, dy: f64) {
        for y in &mut self.py {
            *y = (f64::from(*y) + dy) as f32;
        }
    }

    /// A frame older: the living drift, slowing sideways.
    fn age(&mut self) {
        for i in 0..PMAX {
            if self.life[i] <= 0.0 {
                continue;
            }
            self.life[i] = (f64::from(self.life[i]) - 1.0) as f32;
            self.px[i] = (f64::from(self.px[i]) + f64::from(self.vx[i])) as f32;
            self.py[i] = (f64::from(self.py[i]) + f64::from(self.vy[i])) as f32;
            self.vx[i] = (f64::from(self.vx[i]) * 0.96) as f32;
        }
    }
}

impl LaunchSite {
    /// Age the particles, then make this frame's new ones.
    pub(super) fn step_particles(&mut self) {
        self.parts.age();
        let s = self.spec();
        let (a, cx, _) = self.nozzle();
        let tall = self.tall;
        let scale = if tall { 1.0 } else { 0.6 };
        let tint = self.world.dials.tint;
        let smoky = tint == Tint::Smoke;
        let len = if self.thr > 0.0 {
            s.plume * self.thr
        } else {
            0.0
        };
        let state = self.state;
        let apx = self.apx();

        // Steam and smoke billowing off the pad while the plume reaches it.
        if len > 0.0 && a - len < 2.0 {
            let n = (if tall { 6 } else { 2 }) + if state == State::Ignite { 2 } else { 0 };
            for _ in 0..n {
                let side = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
                let out = s.body_w / 2.0 + self.rng.f() * (len - a + 2.0) * 1.2;
                let shade = if smoky {
                    mix(0x9a9a9a, 0x4a4a4a, self.rng.f())
                } else {
                    mix(0xffffff, 0xb4bac2, self.rng.f() * 0.7)
                };
                self.parts.spawn(
                    cx + side * out,
                    self.rng.f() * 2.0 - 0.5,
                    side * (0.35 + self.rng.f() * 1.3) * scale,
                    0.03 + self.rng.f() * 0.12,
                    40.0 + self.rng.f() * 50.0,
                    s.puff.0,
                    s.puff.1 * (0.7 + self.rng.f() * 0.5),
                    shade,
                    0.85,
                );
            }
        }
        // A contrail left hanging in the air: it falls away below as the rocket climbs.
        if state == State::Fly
            && len > 0.0
            && a - len > 1.0
            && self.orbit < 0.3
            && self.layer < 80.0
        {
            let thick = if self.layer < 40.0 { 1.0 } else { 0.6 };
            self.parts.spawn(
                cx + (self.rng.f() - 0.5) * s.body_w,
                a - len - 1.0,
                (self.rng.f() - 0.5) * 0.1,
                0.0,
                30.0 + self.rng.f() * 30.0,
                s.puff.0,
                s.puff.1 * 0.6 * thick,
                if smoky { 0x5a5a5a } else { 0xd9dee5 },
                if smoky { 0.6 } else { 0.55 * thick },
            );
        }
        // Fuelled and waiting: cold vapour venting, sinking as it drifts. More with subagents.
        let venting = state == State::Rest
            || state == State::Ignite
            || (state == State::Landed && self.timer > 30.0)
            || (state == State::Release && self.world.alt <= s.mount / 2.0);
        if venting {
            let p = (0.16
                + self.world.dials.coverage_boost.min(60.0) * 0.01
                + if smoky { 0.25 } else { 0.0 })
                * if tall { 1.0 } else { 0.6 };
            let sp = &s.fly;
            // A stage on its own vents only from its own tanks.
            let (from, to) = self.part_rows(self.part);
            let x0 = self.ox + self.pos;
            for &(col, row, dir) in s.vents {
                if row < from || row >= to || self.rng.f() >= p {
                    continue;
                }
                self.parts.spawn(
                    x0 + col + dir * 0.5,
                    apx + (sp.hf() - 1.0 - row),
                    dir * (0.12 + self.rng.f() * 0.3) * if tall { 1.0 } else { 0.7 },
                    -(0.02 + self.rng.f() * 0.05),
                    22.0 + self.rng.f() * 26.0,
                    0.5,
                    (if tall { 2.2 } else { 1.1 }) * (0.7 + self.rng.f() * 0.6),
                    if smoky {
                        0x8a8a8e
                    } else if tint == Tint::Blue {
                        0xbcd8ff
                    } else {
                        0xf2f6fb
                    },
                    0.7,
                );
            }
        }
        // The Ship's end at sea: a fireball rolling up off the water.
        if self.boom > 0.0 {
            if self.boom < 5.0 {
                for _ in 0..(if tall { 10 } else { 4 }) {
                    let hot = self.rng.f();
                    self.parts.spawn(
                        cx + (self.rng.f() - 0.5) * s.stage,
                        1.0 + self.rng.f() * 2.0,
                        (self.rng.f() - 0.5) * 1.2 * scale,
                        0.2 + self.rng.f() * 0.8,
                        14.0 + self.rng.f() * 16.0,
                        s.puff.0,
                        s.puff.1 * 1.4,
                        if hot < 0.3 {
                            0xfff2c0
                        } else if hot < 0.7 {
                            0xff9a2a
                        } else {
                            0xd8402a
                        },
                        1.0,
                    );
                }
            }
            self.boom = if self.boom > 30.0 {
                0.0
            } else {
                self.boom + 1.0
            };
        }
        // The Ship coming in: plasma streaming up off its belly as it falls.
        if self.look().catches
            && self.part == Part::Upper
            && self.burn > 6.0
            && self.burn < 84.0
            && self.flop > 0.5
        {
            let len = s.stage;
            let mid = apx + (s.fly.hf() - s.stage) + s.stage / 2.0;
            for _ in 0..(if tall { 4 } else { 2 }) {
                let hot = self.rng.f();
                self.parts.spawn(
                    cx + (self.rng.f() - 0.5) * len * 2.0,
                    mid - s.body_w / 2.0 - 1.0 - self.rng.f(),
                    (self.rng.f() - 0.5) * 0.4,
                    0.5 + self.rng.f() * 1.1,
                    6.0 + self.rng.f() * 8.0,
                    0.5,
                    1.2,
                    if hot < 0.4 {
                        0xff5aa0
                    } else if hot < 0.75 {
                        0xff7a2a
                    } else {
                        0xffc46a
                    },
                    0.85,
                );
            }
        }
        // Dragon coming in: plasma and sparks streaming back off its heat shield.
        if self.part == Part::Capsule && self.burn > 28.0 && self.burn < 88.0 {
            let n = if self.burn > 40.0 && self.burn < 75.0 {
                4
            } else {
                2
            };
            for _ in 0..n {
                let hot = self.rng.f();
                self.parts.spawn(
                    cx + (self.rng.f() - 0.5) * s.body_w * 1.5,
                    a + self.rng.f() * 3.0,
                    (self.rng.f() - 0.5) * 0.6,
                    0.6 + self.rng.f() * 1.2,
                    8.0 + self.rng.f() * 10.0,
                    0.5,
                    1.2,
                    if hot < 0.35 {
                        0xff5aa0
                    } else if hot < 0.7 {
                        0xff7a2a
                    } else {
                        0xffc46a
                    },
                    0.9,
                );
            }
        }
        // Coming down in the sea: a burst of spray, then a little wash.
        if self.splash > 0.0 {
            let n = if self.splash < 3.0 {
                if tall {
                    18
                } else {
                    8
                }
            } else if self.splash < 40.0 && self.rng.f() < 0.3 {
                1
            } else {
                0
            };
            for _ in 0..n {
                let side = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
                self.parts.spawn(
                    cx + side * (s.body_w / 2.0 + self.rng.f() * 2.0),
                    0.0,
                    side * (0.2 + self.rng.f() * 0.6) * scale,
                    0.3 + self.rng.f() * if tall { 0.9 } else { 0.4 },
                    10.0 + self.rng.f() * 14.0,
                    0.6,
                    s.puff.1 * 0.6,
                    0xeef6ff,
                    0.9,
                );
            }
            self.splash = if self.state == State::Landed {
                self.splash + 1.0
            } else {
                0.0
            };
        }
        // A failed command: the climbing rocket coughs dark smoke.
        if smoky && state == State::Fly && self.orbit <= 0.3 && self.rng.f() < 0.4 {
            self.parts.spawn(
                cx,
                a - 1.0,
                (self.rng.f() - 0.5) * 0.4,
                -0.1,
                25.0 + self.rng.f() * 20.0,
                s.puff.0,
                s.puff.1,
                0x55555a,
                0.75,
            );
        }
        // A tint aloft: puffs off the rocket as drawn (posed, in orbit), riding
        // along with its climb a while before falling behind. Smoke trails off
        // the tail; a near-full context fires blue-white thruster puffs off the nose.
        if state == State::Fly && self.world.target() > 0.0 && tint != Tint::Normal {
            self.base = 2.0 * (self.scroll() + self.rows() - 2.0) + 1.0;
            let (px, py, th, sc) = self.pose(&s.fly, self.pos, true);
            let sn = th.sin();
            let cs = th.cos();
            let reach = s.fly.hf() * sc;
            // Climbing, puffs keep pace (the world scrolls 2 pixels a row) and drift off sideways.
            let ride = self.v.max(0.0) * 2.0;
            let base = self.base;
            if smoky && self.rng.f() < 0.7 {
                let yh = base - (py + (reach * cs) / 2.0);
                let drift =
                    (if self.rng.f() < 0.5 { -1.0 } else { 1.0 }) * cs * (0.2 + self.rng.f() * 0.5);
                self.parts.spawn(
                    px - reach * sn + (self.rng.f() - 0.5) * 2.0,
                    yh,
                    drift - sn * (0.3 + self.rng.f() * 0.5),
                    ride - cs * 0.05,
                    18.0 + self.rng.f() * 14.0,
                    s.puff.0,
                    s.puff.1 * 1.3,
                    0x5c5c62,
                    0.85,
                );
            } else if !smoky && self.rng.f() < 0.6 {
                let side = if self.rng.f() < 0.5 { -1.0 } else { 1.0 };
                let at = 0.55 + self.rng.f() * 0.35;
                let yh = base - (py - (reach * at * cs) / 2.0);
                self.parts.spawn(
                    px + reach * at * sn + side * cs * 1.5,
                    yh,
                    side * cs * (0.5 + self.rng.f() * 0.4),
                    ride - side * sn * 0.3,
                    9.0 + self.rng.f() * 6.0,
                    0.8,
                    s.puff.1,
                    0xe4f0ff,
                    1.0,
                );
            }
        }
    }
}
