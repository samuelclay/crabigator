//! Doom and Doom3d: the original procedural techbase, combat and co-op squad,
//! ported from Flow's doom.ts. The sound lead and volley clocks remain part of
//! the simulation because they determine when muzzle flashes and hits occur.
// Keep the upstream coordinate predicates directly comparable to its renderer.
#![allow(clippy::too_many_arguments, clippy::manual_range_contains)]
mod draw;
mod sprites;
mod world;
use crate::flow::cells::{Cells, Rng};
use crate::flow::crew::{beckon, ease_to, failed, finished, Crew, Mate};
use crate::flow::js::{hypot, i32_of, imul, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixel_scene::{Painter, PixelDials, PixelScene};
use crate::flow::pixels::{approach, clamp, clamp01, hash, hash1, mix, noise2};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use sprites::*;
use world::*;
pub const DEF: SceneDef = SceneDef {
    name: "doom",
    hue: SceneHue {
        // Brown architecture takes the hue; red, blue and green combat cues stay.
        key: 80.0,
        range: 90.0,
        spread: Some(10.0),
    },
    figure: false,
    make: |s| Box::new(Doom::new(s, false)),
};
pub const DOOM3D: SceneDef = SceneDef {
    name: "doom3d",
    hue: SceneHue {
        // Brown architecture takes the hue; red, blue and green combat cues stay.
        key: 80.0,
        range: 90.0,
        spread: Some(10.0),
    },
    figure: false,
    make: |s| Box::new(Doom::new(s, true)),
};
const PISTOL: usize = 0;
const SHOTGUN: usize = 1;
const CHAINGUN: usize = 2;
const PLASMA: usize = 3;
const ZOMBIE: usize = 0;
const IMP: usize = 1;
const PINKY: usize = 2;
const CACO: usize = 3;
const WALK: u8 = 0;
const ATTACK: u8 = 1;
const PAIN: u8 = 2;
const DIE: u8 = 3;
const DEAD: u8 = 4;
const FLEE: u8 = 5;
const BALL_FIRE: u8 = 0;
const BALL_CACO: u8 = 1;
const BALL_PLASMA: u8 = 2;
const BLOOD: u8 = 0;
const DUST: u8 = 1;
const BOOM: u8 = 2;
const SPARK: u8 = 3;
const LOOK_LEFT: usize = 0;
const LOOK_AHEAD: usize = 1;
const LOOK_RIGHT: usize = 2;
const GRIN: usize = 3;
const OUCH: usize = 4;
const RAGE: usize = 5;
const SQUAD: usize = 4;
const ARRIVE: f64 = 30.0;
const LEAVE: f64 = 34.0;
const SWITCH: f64 = 14.0;
const DIE_T: f64 = 8.0;
const SPEED: [f64; 11] = [
    0.0, 0.15, 0.24, 0.34, 0.46, 0.6, 0.78, 1.0, 1.28, 1.62, 2.05,
];
const RATE: [[f64; 2]; 4] = [[20.0, 12.0], [16.0, 11.0], [5.0, 3.0], [3.0, 2.0]];
const DAMAGE: [f64; 4] = [1.0, 4.0, 1.0, 1.6];
const GUN_FROM: [f64; 5] = [0.0, 2.5, 5.5, 8.5, 10.0];
#[derive(Clone)]
struct Foe {
    kind: usize,
    x: f64,
    u: f64,
    hp: f64,
    st: u8,
    t: f64,
    walk: f64,
    cool: f64,
    flash: f64,
    gib: bool,
    seed: f64,
}
#[derive(Clone)]
struct Missile {
    kind: u8,
    x: f64,
    u: f64,
    h: f64,
    v: f64,
    t: f64,
}
#[derive(Clone)]
struct Puff {
    kind: u8,
    x: f64,
    u: f64,
    h: f64,
    t: f64,
    life: f64,
    seed: f64,
}
#[derive(Clone)]
struct Barrel {
    x: f64,
    u: f64,
    fuse: f64,
}
#[derive(Clone)]
struct Door {
    x: f64,
    open: f64,
}
#[derive(Clone, Copy)]
enum Target {
    Foe(usize),
    Barrel(usize),
}
fn speed_at(level: f64) -> f64 {
    let l = clamp(level, 0.0, 10.0);
    let i = (l.floor() as usize).min(9);
    SPEED[i] + (SPEED[i + 1] - SPEED[i]) * (l - i as f64)
}
fn gun_for(level: f64, now: usize) -> usize {
    let mut g = now;
    while g < PLASMA && level > GUN_FROM[g + 1] + 0.25 {
        g += 1;
    }
    while g > PISTOL && level < GUN_FROM[g] - 0.25 {
        g -= 1;
    }
    g
}
fn rate_of(gun: usize, level: f64) -> f64 {
    let k = clamp01((level - GUN_FROM[gun]) / (GUN_FROM[gun + 1] - GUN_FROM[gun]).max(0.5));
    round(RATE[gun][0] + (RATE[gun][1] - RATE[gun][0]) * k)
}
pub struct Doom {
    base: PixelScene,
    seed: f64,
    first_person: bool,
    crew: Crew,
    rng: Rng,
    pos: f64,
    v: f64,
    walk: f64,
    gun: usize,
    next_gun: usize,
    switch_t: f64,
    cool: f64,
    flash: f64,
    recoil: f64,
    since_shot: f64,
    since_sighted: f64,
    ammo: [f64; 4],
    hp: f64,
    ouch: f64,
    grin: f64,
    look: usize,
    look_t: f64,
    spawn_t: f64,
    foes: Vec<Foe>,
    missiles: Vec<Missile>,
    puffs: Vec<Puff>,
    barrels: Vec<Barrel>,
    doors: Vec<Door>,
    laid: f64,
    k_smk: f64,
    k_blu: f64,
    hold: bool,
    w: usize,
    h: usize,
    tall: bool,
    mx: f64,
    hv: usize,
    sb: usize,
    zb: Vec<f32>,
    pano: bool,
    fy: f64,
    fth: f64,
    ux: Vec<f32>,
    uz: Vec<f32>,
    ps: f64,
    pt: f64,
    pk: f64,
    shots: usize,
    primed: bool,
    tick: f64,
    heard_to: f64,
    volley_r: f64,
    volley_from: f64,
    lights: [f32; 7],
    hz: f64,
    bx: f64,
    by: f64,
    bw: f64,
    bh: f64,
}
impl Doom {
    pub fn new(seed: f64, first_person: bool) -> Self {
        Self {
            base: PixelScene::new(seed),
            seed,
            first_person,
            crew: Crew::new(SQUAD, ARRIVE, LEAVE),
            rng: Rng::new((imul(i32_of(seed), 2654435761_u32 as i32) ^ 0x51ed27) as f64),
            pos: 0.0,
            v: 0.0,
            walk: 0.0,
            gun: PISTOL,
            next_gun: PISTOL,
            switch_t: 0.0,
            cool: 0.0,
            flash: 0.0,
            recoil: 0.0,
            since_shot: 999.0,
            since_sighted: 999.0,
            ammo: [50.0, 20.0, 120.0, 100.0],
            hp: 100.0,
            ouch: 0.0,
            grin: 0.0,
            look: LOOK_AHEAD,
            look_t: 0.0,
            spawn_t: 40.0,
            foes: vec![],
            missiles: vec![],
            puffs: vec![],
            barrels: vec![],
            doors: vec![],
            laid: f64::NAN,
            k_smk: 0.0,
            k_blu: 0.0,
            hold: false,
            w: 0,
            h: 0,
            tall: false,
            mx: 0.0,
            hv: 0,
            sb: 0,
            zb: vec![],
            pano: false,
            fy: 1.0,
            fth: 1.0,
            ux: vec![],
            uz: vec![],
            ps: 0.0,
            pt: 0.0,
            pk: 0.0,
            shots: 0,
            primed: false,
            tick: 0.0,
            heard_to: 0.0,
            volley_r: 0.0,
            volley_from: 0.0,
            lights: [0.0; 7],
            hz: 0.0,
            bx: 0.0,
            by: 0.0,
            bw: 0.0,
            bh: 0.0,
        }
    }
    fn resize(&mut self, d: PixelDials) {
        self.w = d.columns * 2;
        self.h = d.rows * 2;
        self.tall = d.tall || self.first_person;
        self.mx = clamp(
            round(self.w as f64 * 0.32),
            8.0,
            (self.w as f64 - 16.0).max(8.0),
        );
        self.sb = if self.tall && self.h >= 50 {
            clamp(round(self.h as f64 * 0.13), 12.0, 14.0) as usize
        } else {
            0
        };
        self.hv = self.h - self.sb;
        self.zb = vec![0.0; self.w * self.hv.max(1)];
        self.project();
        self.laid = f64::NAN;
        self.doors.clear();
        self.barrels.clear();
    }
    fn project(&mut self) {
        let w = self.w as f64;
        let hv = self.hv.max(1) as f64;
        self.pano = self.tall && w > hv;
        self.ux = vec![0.0; self.w];
        self.uz = vec![0.0; self.w];
        if self.pano {
            let fov = clamp(0.9 + 0.27 * (w / (2.0 * hv)), 1.4, 2.4);
            self.fth = w / fov;
            self.fy = hv;
            for x in 0..self.w {
                let a = (x as f64 + 0.5 - w / 2.0) / self.fth;
                self.ux[x] = a.sin() as f32;
                self.uz[x] = a.cos() as f32;
            }
        } else {
            self.fy = w * 0.72;
            self.fth = self.fy;
            for x in 0..self.w {
                self.ux[x] = ((x as f64 + 0.5 - w / 2.0) / self.fy) as f32;
                self.uz[x] = 1.0;
            }
        }
    }
    fn on_view(&mut self, x: f64, u: f64, nearest: f64) -> bool {
        let z = (x - self.pos) / UNIT;
        if z < nearest || z > 12.0 {
            return false;
        }
        if self.pano {
            self.pt = hypot(u, z);
            self.ps = self.w as f64 / 2.0 + u.atan2(z) * self.fth;
        } else {
            self.pt = z;
            self.ps = self.w as f64 / 2.0 + (self.fth * u) / z;
        }
        self.pk = self.fy / self.pt;
        true
    }
    fn ahead(&self) -> f64 {
        if self.tall {
            11.0 * UNIT
        } else {
            self.w as f64 - self.mx
        }
    }
    fn range(&self) -> f64 {
        if self.tall {
            6.5 * UNIT
        } else {
            (self.ahead() * 0.82).max(36.0)
        }
    }
    fn behind(&self) -> f64 {
        if self.tall {
            8.0
        } else {
            self.mx + 16.0
        }
    }
    fn room(&self) -> usize {
        if self.tall {
            if self.w >= 28 {
                SQUAD
            } else {
                2
            }
        } else {
            clamp(((self.mx - 6.0) / 11.0).floor(), 0.0, SQUAD as f64) as usize
        }
    }
    fn blocked(&self, x: f64) -> bool {
        self.doors
            .iter()
            .any(|dr| dr.x > self.pos && dr.x < x && dr.open < 0.7)
    }
    fn update(&mut self, d: PixelDials) {
        let l = d.level;
        self.k_smk = approach(
            self.k_smk,
            if d.tint == Tint::Smoke { 1.0 } else { 0.0 },
            0.1,
        );
        self.k_blu = approach(
            self.k_blu,
            if d.tint == Tint::Blue { 1.0 } else { 0.0 },
            0.1,
        );
        self.hold = d.wait > 0.3;
        self.lay();
        self.update_gun(l);
        self.update_him(l);
        self.update_foes(l);
        self.update_squad(l, d);
        self.update_missiles();
        self.update_doors();
        for p in &mut self.puffs {
            p.t += 1.0;
        }
        self.puffs.retain(|p| p.t < p.life);
        for i in 0..self.barrels.len() {
            if self.barrels[i].fuse > 0.0 {
                self.barrels[i].fuse -= 1.0;
                if self.barrels[i].fuse == 0.0 {
                    self.explode(i);
                }
            }
        }
        let gone = self.pos - self.behind() - 24.0;
        self.barrels.retain(|b| b.fuse >= 0.0 && b.x > gone);
        let pos = self.pos;
        let ahead = self.ahead();
        self.foes.retain(|f| {
            if f.st == DEAD {
                f.x > gone
            } else {
                f.st != FLEE || f.x - pos < ahead + 20.0
            }
        });
        let mut dead = 0;
        for i in (0..self.foes.len()).rev() {
            if self.foes[i].st == DEAD {
                dead += 1;
                if dead > 30 {
                    self.foes.remove(i);
                }
            }
        }
        self.update_face(d);
    }
    fn lay(&mut self) {
        let to = ((self.pos + self.ahead() + 40.0) / SEG).floor();
        if self.laid.is_nan() {
            self.laid = ((self.pos - self.behind()) / SEG).floor() - 1.0;
        }
        while self.laid < to {
            self.laid += 1.0;
            let seg = self.laid;
            if hash1(seg * 53.0 + self.seed) < 0.24 {
                self.barrels.push(Barrel {
                    x: seg * SEG + 6.0 + (hash1(seg * 59.0 + self.seed) * 20.0).floor(),
                    u: if hash1(seg * 61.0 + self.seed) < 0.5 {
                        -0.78
                    } else {
                        0.78
                    },
                    fuse: 0.0,
                });
            }
            if self.tall
                && seg * SEG > self.pos + 2.0 * UNIT
                && hash1(seg * 41.0 + self.seed + 9.0) < 0.2
            {
                self.doors.push(Door {
                    x: seg * SEG,
                    open: 0.0,
                });
            }
        }
    }
    fn update_gun(&mut self, l: f64) {
        let want = gun_for(l, self.next_gun);
        if want != self.next_gun && self.switch_t == 0.0 {
            self.next_gun = want;
            self.switch_t = 1.0;
        }
        if self.switch_t > 0.0 {
            self.switch_t += 1.0;
            if self.switch_t == 7.0 {
                self.gun = self.next_gun;
                self.grin = 22.0;
            }
            if self.switch_t >= SWITCH {
                self.switch_t = 0.0;
            }
        }
    }
    fn nearest(&self) -> f64 {
        let mut best = f64::INFINITY;
        for f in &self.foes {
            if f.st < DIE && f.x - self.pos > -4.0 {
                best = best.min(f.x - self.pos);
            }
        }
        best
    }
    fn target(&self, from: f64, barrels: bool) -> Option<Target> {
        let range = self.range();
        if barrels {
            for (i, b) in self.barrels.iter().enumerate() {
                let db = b.x - from;
                if b.fuse != 0.0 || db < 34.0 || db > range || self.blocked(b.x) {
                    continue;
                }
                if self
                    .foes
                    .iter()
                    .any(|f| f.st < DIE && (f.x - b.x).abs() < 16.0)
                {
                    return Some(Target::Barrel(i));
                }
            }
        }
        let mut best = None;
        for (i, f) in self.foes.iter().enumerate() {
            if f.st >= DIE {
                continue;
            }
            let df = f.x - from;
            if df < -2.0 || df > range || self.blocked(f.x) {
                continue;
            }
            if best.is_none_or(|j: usize| f.x < self.foes[j].x) {
                best = Some(i);
            }
        }
        best.map(Target::Foe)
    }
    fn update_him(&mut self, l: f64) {
        let mut goal = if self.hold { 0.0 } else { speed_at(l) };
        if self.nearest() < if self.tall { 22.0 } else { 28.0 } {
            goal = 0.0;
        }
        self.v += clamp(goal - self.v, -0.06, 0.035);
        if self.v.abs() < 0.005 {
            self.v = 0.0;
        }
        self.pos += self.v;
        self.walk += self.v;
        self.flash = (self.flash - 1.0).max(0.0);
        self.recoil = (self.recoil - 1.0).max(0.0);
        self.cool -= 1.0;
        self.tick += 1.0;
        self.since_shot += 1.0;
        self.since_sighted += 1.0;
        // The sound clock also gates the first shot and each volley, even when muted.
        let due = (self.tick - self.since_shot + self.volley_r).max(self.volley_from);
        if self.heard_to > self.tick && self.tick > due {
            self.heard_to = self.tick;
        }
        if self.hold || l <= 0.0 || self.switch_t > 0.0 {
            self.primed = false;
            return;
        }
        let Some(tg) = self.target(self.pos, true) else {
            self.primed = false;
            return;
        };
        self.since_sighted = 0.0;
        let lead = 380.0
            / if self.base.dials.strength <= 1.0 && self.base.dials.tint == Tint::Normal {
                125.0
            } else {
                70.0
            };
        if self.gun >= CHAINGUN {
            if self.tick + lead >= self.heard_to {
                if self.heard_to <= self.tick {
                    self.cool = self.cool.max((lead - 1e-9_f64).ceil());
                }
                let r = clamp(rate_of(self.gun, l), RATE[self.gun][1], RATE[self.gun][0]);
                self.volley_r = r;
                self.volley_from = self.heard_to.max(self.tick + self.cool);
                self.heard_to = self.volley_from + round(1400.0 / (r * 70.0)) * r;
            }
        } else if !self.primed && self.cool <= lead {
            self.primed = true;
            if self.cool <= 0.0 {
                self.cool = (lead - 1e-9_f64).ceil();
            }
        }
        if self.cool > 0.0 {
            return;
        }
        self.primed = false;
        self.cool = rate_of(self.gun, l);
        self.flash = if self.gun == CHAINGUN || self.gun == PLASMA {
            1.0
        } else {
            2.0
        };
        self.recoil = if self.gun == SHOTGUN { 5.0 } else { 2.0 };
        self.since_shot = 0.0;
        self.shots += 1;
        self.ammo[self.gun] -= 1.0;
        if self.ammo[self.gun] < 5.0 {
            self.ammo[self.gun] += [50.0, 20.0, 100.0, 100.0][self.gun];
        }
        self.shoot(
            tg,
            self.gun,
            1.0,
            self.pos + if self.tall { 16.0 } else { 8.0 },
        );
    }
    fn shoot(&mut self, tg: Target, gun: usize, k: f64, muzzle: f64) {
        if gun == PLASMA {
            self.missiles.push(Missile {
                kind: BALL_PLASMA,
                x: muzzle,
                u: 0.12,
                h: 5.0,
                v: 7.0,
                t: 0.0,
            });
            return;
        }
        let Target::Foe(i) = tg else {
            if let Target::Barrel(i) = tg {
                self.explode(i);
            }
            return;
        };
        self.hit(i, DAMAGE[gun] * k);
        if gun == SHOTGUN {
            let (x, u) = (self.foes[i].x, self.foes[i].u);
            for _ in 0..2 {
                let x = x + (self.rng.f() - 0.5) * 6.0;
                let h = 2.0 + self.rng.f() * 6.0;
                self.puff(BLOOD, x, u, h, 5.0);
            }
            for j in 0..self.foes.len() {
                let f = &self.foes[j];
                if j != i && f.st < DIE && (f.x - x).abs() < 10.0 {
                    self.hit(j, k);
                }
            }
        }
    }
    fn hit(&mut self, i: usize, dmg: f64) {
        let f = &self.foes[i];
        let k = FOES[f.kind];
        let x = f.x + (self.rng.f() - 0.5) * 3.0;
        let u = f.u;
        let h = k.float + 2.0 + self.rng.f() * 5.0;
        self.puff(BLOOD, x, u, h, 5.0);
        let f = &mut self.foes[i];
        f.hp -= dmg;
        if f.hp <= 0.0 {
            f.st = DIE;
            f.t = 0.0;
            f.gib = dmg >= 8.0;
        } else if f.st == WALK && self.rng.f() < 0.45 {
            f.st = PAIN;
            f.t = 3.0;
        }
    }
    fn explode(&mut self, i: usize) {
        let b = self.barrels[i].clone();
        if b.fuse < 0.0 {
            return;
        }
        self.barrels[i].fuse = -1.0;
        self.puff(BOOM, b.x + 2.0, b.u, 3.0, 9.0);
        for j in 0..self.foes.len() {
            let f = &self.foes[j];
            if f.st < DIE && (f.x - b.x).abs() < 22.0 {
                self.hit(j, 12.0);
            }
        }
        for o in &mut self.barrels {
            if o.fuse == 0.0 && (o.x - b.x).abs() < 20.0 {
                o.fuse = 3.0;
            }
        }
    }
    fn puff(&mut self, kind: u8, x: f64, u: f64, h: f64, life: f64) {
        if self.puffs.len() < 80 {
            self.puffs.push(Puff {
                kind,
                x,
                u,
                h,
                t: 0.0,
                life,
                seed: (self.rng.int() & 0xffff) as f64,
            });
        }
    }
    fn update_foes(&mut self, l: f64) {
        let ahead = self.ahead();
        let up = self.foes.iter().filter(|f| f.st < DIE).count();
        if !self.hold && l > 0.0 {
            self.spawn_t -= 1.0;
            let most = 1.0 + round(l * 0.75);
            if self.spawn_t <= 0.0 && (up as f64) < most {
                self.spawn(l, ahead);
                self.spawn_t = round(
                    (230.0 - 218.0 * clamp01(l / 10.0).powf(0.6)) * (0.6 + 0.8 * self.rng.f()),
                );
            }
        }
        let pace = 0.7 + 0.06 * l;
        for i in 0..self.foes.len() {
            let mut f = self.foes[i].clone();
            let kind = FOES[f.kind];
            f.flash = (f.flash - 1.0).max(0.0);
            if f.st == DIE {
                f.t += 1.0;
                if f.t >= DIE_T {
                    f.st = DEAD;
                }
                self.foes[i] = f;
                continue;
            }
            if f.st == DEAD {
                self.foes[i] = f;
                continue;
            }
            if self.hold {
                f.st = FLEE;
            } else if f.st == FLEE {
                f.st = WALK;
            }
            let dist = f.x - self.pos;
            if f.st == FLEE {
                f.x += kind.speed * pace + 0.25;
                f.walk += kind.speed * pace + 0.25;
                self.foes[i] = f;
                continue;
            }
            if f.st == PAIN {
                f.t -= 1.0;
                if f.t <= 0.0 {
                    f.st = WALK;
                }
                self.foes[i] = f;
                continue;
            }
            if f.st == ATTACK {
                f.t += 1.0;
                let attack = f.t == 4.0;
                self.foes[i] = f;
                if attack {
                    self.attack(i);
                }
                let f = &mut self.foes[i];
                if f.t >= 9.0 {
                    f.st = WALK;
                    f.cool = round((50.0 + 60.0 * self.rng.f()) * (1.3 - 0.07 * l));
                }
                continue;
            }
            let stand = if self.tall { kind.near } else { kind.stand };
            if dist > stand {
                let step = (kind.speed * pace).min(dist - stand);
                f.x -= step;
                f.walk += step;
            }
            f.cool -= 1.0;
            if f.cool <= 0.0
                && dist < ahead - 4.0
                && !self.blocked(f.x)
                && (f.kind != PINKY || dist < stand + 4.0)
            {
                f.st = ATTACK;
                f.t = 0.0;
            }
            self.foes[i] = f;
        }
    }
    fn spawn(&mut self, l: f64, ahead: f64) {
        let w = [
            1.0,
            if l >= 2.0 { 0.9 } else { 0.0 },
            if l >= 4.0 { 0.45 + 0.05 * l } else { 0.0 },
            if l >= 6.0 { 0.25 + 0.05 * l } else { 0.0 },
        ];
        let mut r = self.rng.f() * (w[0] + w[1] + w[2] + w[3]);
        let mut kind = 0;
        while kind < 3 && r >= w[kind] {
            r -= w[kind];
            kind += 1;
        }
        let k = FOES[kind];
        self.foes.push(Foe {
            kind,
            x: self.pos + ahead + 6.0 + self.rng.f() * 14.0,
            u: (self.rng.f() * 2.0 - 1.0) * 0.5,
            hp: k.hp,
            st: WALK,
            t: 0.0,
            walk: self.rng.f() * 8.0,
            cool: 20.0 + (self.rng.f() * 40.0).floor(),
            flash: 0.0,
            gib: false,
            seed: (self.rng.int() & 0xffff) as f64,
        });
    }
    fn attack(&mut self, i: usize) {
        let f = self.foes[i].clone();
        let dist = f.x - self.pos;
        if f.kind == ZOMBIE {
            self.foes[i].flash = 2.0;
            if self.rng.f() < 0.25 {
                let n = 2.0 + self.rng.f() * 4.0;
                self.hurt(n);
            }
        } else if f.kind == IMP {
            self.missiles.push(Missile {
                kind: BALL_FIRE,
                x: f.x - 2.0,
                u: f.u,
                h: 6.0,
                v: 1.5,
                t: 0.0,
            });
        } else if f.kind == CACO {
            self.missiles.push(Missile {
                kind: BALL_CACO,
                x: f.x - 3.0,
                u: f.u,
                h: FOES[CACO].float + 4.0,
                v: 1.15,
                t: 0.0,
            });
        } else if dist < FOES[PINKY].stand + 6.0 {
            let n = 3.0 + self.rng.f() * 5.0;
            self.hurt(n);
        }
    }
    fn hurt(&mut self, n: f64) {
        self.hp = (self.hp - n).max(12.0);
        self.ouch = 8.0;
    }
    fn update_missiles(&mut self) {
        for i in 0..self.missiles.len() {
            let mut m = self.missiles[i].clone();
            m.t += 1.0;
            if m.kind == BALL_PLASMA {
                m.x += m.v;
                for j in 0..self.foes.len() {
                    let f = &self.foes[j];
                    if f.st < DIE && m.x >= f.x - 2.0 && m.x - m.v <= f.x + 6.0 {
                        self.hit(j, DAMAGE[PLASMA]);
                        m.t = -1.0;
                        break;
                    }
                }
                if m.x - self.pos > self.range() + 20.0 || self.blocked(m.x) {
                    m.t = -1.0;
                }
                self.missiles[i] = m;
                continue;
            }
            if self.hold {
                self.puff(SPARK, m.x, m.u, m.h, 4.0);
                m.t = -1.0;
                self.missiles[i] = m;
                continue;
            }
            m.x -= m.v;
            let dist = m.x - self.pos;
            let k = m.v / m.v.max(dist);
            m.u += (0.0 - m.u) * k;
            m.h += (5.0 - m.h) * k;
            if dist < 4.0 {
                self.puff(SPARK, self.pos + 4.0, 0.0, 5.0, 5.0);
                let n = 4.0 + self.rng.f() * 6.0;
                self.hurt(n);
                m.t = -1.0;
            }
            self.missiles[i] = m;
        }
        self.missiles.retain(|m| m.t >= 0.0);
    }
    fn update_doors(&mut self) {
        for dr in &mut self.doors {
            let mut want = dr.x - self.pos < 3.5 * UNIT;
            for f in &self.foes {
                if f.st != DEAD && (f.x - dr.x).abs() < 1.2 * UNIT {
                    want = true;
                }
            }
            dr.open = clamp01(dr.open + if want { 0.045 } else { -0.03 });
        }
        self.doors.retain(|dr| dr.x > self.pos - 2.0 * UNIT);
    }
    fn squad_place(&self, m: &Mate) -> f64 {
        let rest = m.busy < 0.5 && !m.leaving;
        if self.tall {
            [24.0, 32.0, 46.0, 54.0][m.slot] * if rest { 0.7 } else { 1.0 }
        } else {
            -(11.0 + m.slot as f64 * 11.0) - if rest { 5.0 } else { 0.0 }
        }
    }
    fn update_squad(&mut self, l: f64, d: PixelDials) {
        self.crew.room = self.room();
        self.crew.update(&self.base.dials.agents, d.boost);
        for i in 0..self.crew.mates.len() {
            let mut m = self.crew.mates[i].clone();
            if m.x.is_nan() {
                m.x = if self.tall { -14.0 } else { -(self.mx + 10.0) };
                m.y = 10.0 + (m.seed * 20.0).floor();
            }
            if failed(Some(&m)) {
                m.x -= self.v;
                self.crew.mates[i] = m;
                continue;
            }
            if finished(Some(&m)) {
                self.crew.mates[i] = m;
                continue;
            }
            m.x = ease_to(
                m.x,
                self.squad_place(&m),
                if m.age < ARRIVE * 2.0 { 0.07 } else { 0.05 },
            );
            m.y -= 1.0;
            if self.hold || m.busy < 0.5 || m.here < 0.6 || self.switch_t > 0.0 {
                self.crew.mates[i] = m;
                continue;
            }
            let at = self.pos + m.x;
            let tg = self.target(at, false);
            if let Some(tg) = tg {
                if m.y <= 0.0 {
                    m.y = round(rate_of(self.gun, l) * (1.4 + m.seed * 0.6));
                    self.shoot(tg, self.gun, 0.6, at + 8.0);
                }
            }
            self.crew.mates[i] = m;
        }
    }
    fn update_face(&mut self, d: PixelDials) {
        self.ouch = (self.ouch - 1.0).max(0.0);
        self.grin = (self.grin - 1.0).max(0.0);
        if d.t % 12.0 == 0.0 {
            self.hp = (self.hp + 1.0).min(100.0);
        }
        self.look_t -= 1.0;
        if self.look_t <= 0.0 {
            let r = self.rng.f();
            self.look = if self.hold {
                LOOK_AHEAD
            } else if r < 0.3 {
                LOOK_LEFT
            } else if r < 0.6 {
                LOOK_RIGHT
            } else {
                LOOK_AHEAD
            };
            self.look_t = 18.0 + (self.rng.f() * 30.0).floor();
        }
    }
}
impl Scene for Doom {
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
        if self.base.dials.strength <= 0.0 {
            return self.base.blank();
        }
        let d = self.base.frame_dials();
        let mut px = self.base.begin_paint();
        self.paint(&mut px, d);
        self.base.finish_grid(px)
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn doom_frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/doom.json"), 0.0);
    }
    #[test]
    fn doom3d_frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/doom3d.json"), 0.0);
    }
}
