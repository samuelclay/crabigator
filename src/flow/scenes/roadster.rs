//! Roadster: city parking, traffic and freeway jumps, ported from Flow.
//! See ../FORK.md for the pinned reference and retained animation timing.
mod draw;
use crate::flow::cells::{Cells, Rng};
use crate::flow::crew::{failed, finished, Crew, Mate};
use crate::flow::js::{i32_of, imul, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixel_scene::{Painter, PixelDials, PixelScene};
use crate::flow::pixels::{clamp01, hash1, smooth};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use std::collections::HashMap;

pub const DEF: SceneDef = SceneDef {
    name: "roadster",
    hue: SceneHue {
        key: 0.0,
        range: 0.0,
        spread: None,
    },
    figure: false,
    make: |seed| Box::new(Roadster::new(seed)),
};
const SPEED: [f64; 11] = [0.0, 0.0, 0.55, 0.85, 1.2, 1.8, 2.5, 3.6, 4.6, 5.6, 7.0];
const IDLE: f64 = 1.25;
const BRAKE: f64 = 0.02;
const APPROACH_MIN: f64 = 0.35;
const APPROACH_MAX: f64 = 1.4;
const DEPART: f64 = 8.0;
const FLIP_LEVEL: f64 = 9.3;
const LOT: i32 = 0;
const CITY: i32 = 0;
const TOWN: i32 = 1;
const SUBURB: i32 = 2;
const FREEWAY: i32 = 3;
const F_FAR: f64 = 0.08;
const F_MID: f64 = 0.55;
fn accel(v: f64) -> f64 {
    0.03 + 0.012 * v
}
fn decel(v: f64) -> f64 {
    0.05 + 0.016 * v
}
fn speed_for(level: f64) -> f64 {
    let l = level.clamp(0.0, 10.0);
    let i = (l.floor() as usize).min(9);
    SPEED[i] + (SPEED[i + 1] - SPEED[i]) * (l - i as f64)
}
fn level_for(v: f64) -> f64 {
    if v <= 0.0 {
        return 0.0;
    }
    for i in 1..10 {
        if v < SPEED[i + 1] {
            return i as f64 + (v - SPEED[i]) / (SPEED[i + 1] - SPEED[i]);
        }
    }
    10.0
}
fn road_zone(level: f64) -> i32 {
    if level < 4.5 {
        TOWN
    } else if level < 6.5 {
        SUBURB
    } else {
        FREEWAY
    }
}
fn side_zone(level: f64) -> i32 {
    if level < 2.5 {
        CITY
    } else {
        road_zone(level)
    }
}
#[derive(Clone, Copy)]
struct Seg {
    a: f64,
    b: f64,
    kind: i32,
    seed: f64,
}
#[derive(Clone, Copy)]
struct Prop {
    a: f64,
    w: f64,
    kind: i32,
    form: i32,
    h: f64,
    c: usize,
    seed: f64,
}
#[derive(Clone, Copy)]
struct Car {
    x: f64,
    lane: i32,
    v: f64,
    c: u32,
    truck: bool,
}
#[derive(Clone, Copy)]
struct Jump {
    wait: f64,
    t: f64,
    len: f64,
    hgt: f64,
    dir: f64,
    roll: bool,
    checked: bool,
}
#[derive(Clone, Copy)]
struct Puff {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    r: f64,
    life: f64,
    fade: f64,
    c: u32,
}
struct Own {
    jump: Option<Jump>,
    next: f64,
    waved: bool,
    last: f64,
}
#[derive(Clone, Copy)]
struct Sprite {
    w: f64,
    h: f64,
    rows: &'static [&'static [u8]],
}
const ROADSTER: Sprite = Sprite {
    w: 14.0,
    h: 3.0,
    rows: &[b"...bBBBb......", b"TBBGGGGBBBBBbL", b".WW.......WW.."],
};
const ROADSTER_MID: Sprite = Sprite {
    w: 18.0,
    h: 4.0,
    rows: &[
        b".....bBBBBb.......",
        b".bBBBGGGGGGBBbb...",
        b"TBBBBBBBBBBBBBBBBL",
        b"..WW.........WW...",
    ],
};
const ROADSTER_BIG: Sprite = Sprite {
    w: 24.0,
    h: 5.0,
    rows: &[
        b"........bBBBBb..........",
        b"....bBBBGGGGGGGBbb......",
        b"bBBBBBBBBBBBBBBBBBBBBBb.",
        b"TBBBWRRWddddddddWRRWBBBL",
        b"....WWWW........WWWW....",
    ],
};
const SEDAN: Sprite = Sprite {
    w: 14.0,
    h: 3.0,
    rows: &[b"...bBBBBBb....", b"TBBGGBGGGBBBBL", b".WW.......WW.."],
};
const SEDAN_MID: Sprite = Sprite {
    w: 16.0,
    h: 4.0,
    rows: &[
        b"...bBBBBBBb.....",
        b"..BGGGBGGGGBBb..",
        b"TBBBBBBBBBBBBBBL",
        b"..WW........WW..",
    ],
};
const SEDAN_BIG: Sprite = Sprite {
    w: 20.0,
    h: 5.0,
    rows: &[
        b".....bBBBBBBBb......",
        b"...bBGGGGBGGGGBb....",
        b"bBBBBBBBBBBBBBBBBBBb",
        b"TBBBBBBBBBBBBBBBBBBL",
        b"...WRW.......WRW....",
    ],
};
const TRUCK: Sprite = Sprite {
    w: 22.0,
    h: 4.0,
    rows: &[
        b"XXXXXXXXXXXXXXXX......",
        b"XXXXXXXXXXXXXXXX.BGGb.",
        b"TXXXXXXXXXXXXXXXdBBBBL",
        b".WW.WW..........WW..W.",
    ],
};

use draw::{Pal, DAY};
pub struct Roadster {
    base: PixelScene,
    crew: Crew,
    own: HashMap<String, Own>,
    pos: f64,
    v: f64,
    started: bool,
    stop_at: Option<f64>,
    in_lot: f64,
    depart: f64,
    taken: Vec<f64>,
    jump: Option<Jump>,
    segs: Vec<Seg>,
    mids: Vec<Prop>,
    fars: Vec<Prop>,
    cars: Vec<Car>,
    puffs: Vec<Puff>,
    k_smoke: f64,
    k_rain: f64,
    rng: Rng,
    w: f64,
    h: f64,
    tall: bool,
    k: f64,
    car_x: f64,
    far_walk: f64,
    far_wheel: f64,
    dash_y: f64,
    near_wheel: f64,
    kerb: f64,
    lot_wheel: f64,
    stations: Vec<f64>,
    pal: Pal,
    bay_w: f64,
}
impl Roadster {
    fn new(seed: f64) -> Self {
        Self {
            base: PixelScene::new(seed),
            crew: Crew::new(6, 40.0, 50.0),
            own: HashMap::new(),
            pos: 0.0,
            v: 0.0,
            started: false,
            stop_at: None,
            in_lot: 0.0,
            depart: DEPART,
            taken: Vec::new(),
            jump: None,
            segs: Vec::new(),
            mids: Vec::new(),
            fars: Vec::new(),
            cars: Vec::new(),
            puffs: Vec::new(),
            k_smoke: 0.0,
            k_rain: 0.0,
            rng: Rng::new(f64::from(imul(i32_of(seed), 2654435761u32 as i32))),
            w: 0.0,
            h: 0.0,
            tall: false,
            k: 1.0,
            car_x: 0.0,
            far_walk: 0.0,
            far_wheel: 0.0,
            dash_y: 0.0,
            near_wheel: 0.0,
            kerb: 0.0,
            lot_wheel: 0.0,
            stations: Vec::new(),
            pal: DAY,
            bay_w: 16.0,
        }
    }
    fn main(&self) -> Sprite {
        if self.tall {
            ROADSTER_BIG
        } else {
            ROADSTER_MID
        }
    }
    fn near(&self) -> Sprite {
        if self.tall {
            SEDAN_BIG
        } else {
            SEDAN_MID
        }
    }
    fn layout(&mut self, d: PixelDials) {
        self.w = (d.columns * 2) as f64;
        self.h = (d.rows * 2) as f64;
        self.tall = d.tall;
        let h = self.h;
        if self.tall {
            self.k = 1.4;
            self.car_x = f64::from(i32_of((self.w - ROADSTER_BIG.w - 2.0).clamp(0.0, 4.0)) & !1);
            self.far_walk = h - 30.0;
            self.far_wheel = h - 24.0;
            self.dash_y = h - 21.0;
            self.near_wheel = h - 14.0;
            self.kerb = h - 13.0;
            self.lot_wheel = h - 4.0;
            self.bay_w = 24.0;
        } else {
            self.k = 1.0;
            self.car_x = f64::from(i32_of(round(self.w * 0.28).max(8.0)) & !1);
            self.far_walk = h - 6.0;
            self.far_wheel = h - 5.0;
            self.dash_y = h - 4.0;
            self.near_wheel = h - 1.0;
            self.kerb = h;
            self.lot_wheel = if h >= 10.0 { h - 6.0 } else { self.near_wheel };
            self.bay_w = 18.0;
        }
        self.stations = [22.0, -22.0, 44.0, -44.0, 66.0, -66.0, 0.0, 88.0, -88.0]
            .into_iter()
            .filter(|o| self.car_x + o >= 0.0 && self.car_x + o + ROADSTER.w <= self.w)
            .take(6)
            .collect();
    }
    fn lay_road(&mut self, until: f64, level: f64, idle: bool) {
        let k = self.k;
        if idle
            && self.stop_at.is_none()
            && !self
                .segs
                .iter()
                .any(|s| s.kind == LOT && s.b > self.pos + self.stopping_distance() + self.bay_w)
        {
            let cut = round(
                (self.pos + self.stopping_distance()).max(self.pos - self.car_x + self.w + 8.0 * k),
            );
            self.segs.retain(|s| s.a < cut);
            if let Some(last) = self.segs.last_mut() {
                if last.b > cut {
                    last.b = cut;
                }
            }
            self.push_seg(LOT);
        }
        while self.segs.last().is_none_or(|s| s.b < until) {
            self.push_seg(road_zone(level));
        }
        let behind = self.pos - self.car_x - 200.0 * k;
        self.segs.retain(|s| s.b > behind);
    }
    fn push_seg(&mut self, kind: i32) {
        let k = self.k;
        let a = self
            .segs
            .last()
            .map_or(self.pos - self.car_x - 40.0, |s| s.b);
        let len = match kind {
            LOT => 24.0 * k + (5.0 + (self.rng.f() * 3.0).floor()) * self.bay_w,
            TOWN => (90.0 + self.rng.f() * 70.0) * k,
            SUBURB => (140.0 + self.rng.f() * 90.0) * k,
            _ => (260.0 + self.rng.f() * 160.0) * k,
        };
        self.segs.push(Seg {
            a,
            b: round(a + len),
            kind,
            seed: f64::from(self.rng.int() & 0xffff),
        });
    }
    fn lay_props(&mut self, list: &mut Vec<Prop>, until: f64, start: f64, level: f64, far: bool) {
        let r = &mut self.rng;
        let k = self.k;
        let tall = self.tall;
        while list.last().is_none_or(|p| p.a + p.w < until) {
            let mut a = list.last().map_or(start, |p| p.a + p.w);
            let zone = side_zone(level);
            let (w, h);
            let mut form = 0;
            let even = |v: f64| f64::from(i32_of(round(v)) & !1);
            if far {
                if zone <= TOWN {
                    w = 3.0 + (r.f() * 6.0).floor();
                    h = if tall {
                        16.0 + r.f() * 44.0
                    } else {
                        1.0 + r.f() * 3.0
                    };
                    a += (r.f() * 2.0).floor();
                } else {
                    form = if zone == FREEWAY { 1 } else { 0 };
                    w = if tall { 120.0 } else { 60.0 } + (r.f() * 80.0).floor();
                    h = if tall {
                        6.0 + r.f() * 10.0
                    } else {
                        1.5 + r.f() * 1.5
                    };
                }
            } else if zone == CITY {
                w = even((10.0 + r.f() * 12.0) * k);
                h = if tall {
                    24.0 + r.f() * 30.0
                } else {
                    3.0 + (r.f() * 2.0).floor()
                };
                a += round(r.f() * 2.0) * 2.0;
            } else if zone == TOWN {
                w = even((10.0 + r.f() * 8.0) * k);
                h = if tall {
                    10.0 + r.f() * 12.0
                } else {
                    2.0 + (r.f() * 2.0).floor()
                };
            } else if zone == SUBURB {
                form = if r.f() < 0.4 { 1 } else { 0 };
                w = if form != 0 {
                    even((4.0 + r.f() * 3.0) * k)
                } else {
                    even((12.0 + r.f() * 4.0) * k)
                };
                h = if form != 0 {
                    if tall {
                        8.0 + r.f() * 8.0
                    } else {
                        2.0 + (r.f() * 2.0).floor()
                    }
                } else if tall {
                    9.0 + r.f() * 4.0
                } else {
                    3.0
                };
                a += even((2.0 + r.f() * 6.0) * k);
            } else {
                let p = r.f();
                form = if p < 0.5 {
                    0
                } else if p < 0.85 {
                    1
                } else {
                    2
                };
                w = if form == 0 {
                    even((30.0 + r.f() * 40.0) * k)
                } else if form == 1 {
                    even((4.0 + r.f() * 4.0) * k)
                } else {
                    even(12.0 * k)
                };
                h = if form == 0 {
                    if tall {
                        5.0
                    } else {
                        1.0
                    }
                } else if form == 1 {
                    if tall {
                        8.0 + r.f() * 6.0
                    } else {
                        2.0
                    }
                } else if tall {
                    18.0
                } else {
                    4.0
                };
                a += if form == 0 {
                    0.0
                } else {
                    even((4.0 + r.f() * 10.0) * k)
                };
            }
            list.push(Prop {
                a,
                w,
                kind: zone,
                form,
                h,
                c: (r.int() & 0xff) as usize,
                seed: f64::from(r.int() & 0xffff),
            });
        }
    }
    fn seg_at(&self, x: f64) -> Option<Seg> {
        self.segs
            .iter()
            .find(|s| x < s.b)
            .or(self.segs.last())
            .copied()
    }
    fn bays(&self, s: Seg) -> Vec<f64> {
        let mut out = Vec::new();
        let mut x = s.a + round(12.0 * self.k);
        while x + self.bay_w <= s.b - round(12.0 * self.k) + 1.0 {
            out.push(x + 1.0);
            x += self.bay_w;
        }
        out
    }
    fn bay_full(&self, x: f64) -> bool {
        self.lot_wheel != self.near_wheel
            && !self.taken.iter().any(|t| (t - x).abs() < 2.0)
            && hash1(x * 7.0 + 3.0) < 0.62
    }
    fn stopping_distance(&self) -> f64 {
        let mut d = 0.0;
        let mut u = self.v;
        while u > APPROACH_MAX * self.k {
            d += u;
            u -= decel(u);
        }
        let a = self.v.min(APPROACH_MAX * self.k);
        d + a * a / (2.0 * BRAKE * self.k) + 4.0
    }
    fn bay_ahead(&self, from: f64) -> Option<f64> {
        for &s in &self.segs {
            if s.kind != LOT || s.b < from {
                continue;
            }
            for b in self.bays(s) {
                if b >= from && !self.bay_full(b) {
                    return Some(b);
                }
            }
        }
        None
    }
    fn update(&mut self, d: PixelDials) {
        self.layout(d);
        if self.w == 0.0 || self.h == 0.0 {
            return;
        }
        let k = self.k;
        // Keep upstream's sound lead: it also controls visible speed and takeoff.
        let frame_ms = if self.base.dials.strength <= 1.0 && self.base.dials.tint == Tint::Normal {
            125.0
        } else {
            70.0
        };
        let lead = 380.0 / frame_ms;
        let dt = 380.0 / lead / 70.0;
        let idle = self.base.dials.strength <= IDLE || self.base.dials.waiting;
        let level = d.level;
        if !self.started {
            self.started = true;
            self.pos = round(self.pos);
            if idle {
                let a = self.pos - round(12.0 * k) - 2.0 * self.bay_w - 1.0;
                self.segs.push(Seg {
                    a: a - self.car_x - 60.0,
                    b: a,
                    kind: TOWN,
                    seed: 7.0,
                });
                self.segs.push(Seg {
                    a,
                    b: a + round(24.0 * k) + 6.0 * self.bay_w,
                    kind: LOT,
                    seed: 11.0,
                });
                self.stop_at = Some(self.pos);
                self.taken.push(self.pos);
                self.in_lot = 1.0;
                self.v = 0.0;
            } else {
                self.v = speed_for(level) * k;
            }
        }
        self.lay_road(self.pos - self.car_x + self.w + 60.0 * k, level, idle);
        let mut mids = std::mem::take(&mut self.mids);
        self.lay_props(
            &mut mids,
            self.pos * F_MID + self.w + 40.0,
            self.pos * F_MID - 40.0,
            level,
            false,
        );
        self.mids = mids;
        let mut fars = std::mem::take(&mut self.fars);
        self.lay_props(
            &mut fars,
            self.pos * F_FAR + self.w + 40.0,
            self.pos * F_FAR - 40.0,
            level,
            true,
        );
        self.fars = fars;
        self.mids.retain(|p| p.a + p.w > self.pos * F_MID - 60.0);
        self.fars.retain(|p| p.a + p.w > self.pos * F_FAR - 60.0);
        let vt;
        if !idle {
            self.stop_at = None;
            if self.v == 0.0 && self.in_lot > 0.0 && self.depart < DEPART {
                self.depart += dt;
                vt = 0.0;
            } else {
                vt = if self.in_lot > 0.0 {
                    speed_for(level).min(0.5) * k
                } else {
                    speed_for(level) * k
                };
            }
            if self.v > 0.0 && self.in_lot > 0.0 {
                self.in_lot = (self.in_lot - 0.07 * dt).max(0.0);
            }
        } else {
            if self.stop_at.is_none() && self.v > 0.0 {
                if let Some(at) = self.bay_ahead(self.pos + self.stopping_distance()) {
                    self.stop_at = Some(at);
                    self.taken.push(at);
                }
            }
            if let Some(at) = self.stop_at {
                let left = (at - self.pos).max(0.0);
                vt = if self.v == 0.0 {
                    0.0
                } else {
                    (APPROACH_MAX * k).min((2.0 * BRAKE * k * left).sqrt().max(if left > 1.0 {
                        APPROACH_MIN * k
                    } else {
                        0.0
                    }))
                };
                let turn = 3.0 * self.bay_w;
                if left < turn {
                    self.in_lot = self.in_lot.max(1.0 - left / turn);
                }
            } else {
                vt = if self.v == 0.0 { 0.0 } else { APPROACH_MAX * k };
            }
            if self.v == 0.0 {
                self.in_lot = (self.in_lot + 0.1 * dt).min(1.0);
            }
        }
        let dv = vt - self.v;
        if self.jump.is_none() {
            self.v += if dv > 0.0 {
                dv.min(accel(self.v / k) * k * dt)
            } else {
                dv.max(-decel(self.v / k) * k * dt)
            };
        }
        if let Some(ahead) = self.car_ahead(1, self.pos + self.main().w) {
            if self.jump.is_none() && ahead.x - (self.pos + self.main().w) < 4.0 * k {
                self.v = self.v.min(ahead.v);
            }
        }
        let was = self.pos;
        self.pos += self.v * dt;
        if let Some(at) = self.stop_at {
            if idle && (self.pos >= at - 0.05 || (self.v < 0.02 && at - self.pos < 0.6)) {
                self.pos = at;
                self.v = 0.0;
            }
        }
        if self.v == 0.0 && idle {
            self.depart = 0.0;
        }
        let back = self.pos - self.car_x - 400.0 * k;
        self.taken.retain(|t| *t > back);
        self.move_traffic(d, dt);
        self.move_jump(dt, lead);
        self.move_companions(d, dt, lead);
        self.move_puffs(dt, was);
        self.k_smoke = clamp01(
            self.k_smoke
                + if self.base.dials.tint == Tint::Smoke {
                    0.2
                } else {
                    -0.035
                } * dt,
        );
        self.k_rain = clamp01(
            self.k_rain
                + if self.base.dials.tint == Tint::Blue {
                    0.05
                } else {
                    -0.04
                } * dt,
        );
    }
    fn car_ahead(&self, lane: i32, x: f64) -> Option<Car> {
        self.cars
            .iter()
            .filter(|c| c.lane == lane && c.x >= x - 2.0)
            .min_by(|a, b| a.x.total_cmp(&b.x))
            .copied()
    }
    fn move_traffic(&mut self, d: PixelDials, dt: f64) {
        let k = self.k;
        if self.jump.is_none() {
            for c in &mut self.cars {
                c.v = if c.lane == 0 {
                    if self.v >= 2.6 * k {
                        (2.2 * k).min(self.v - 2.0 * k)
                    } else {
                        self.v + 1.2 * k
                    }
                } else if c.x < self.pos || d.level >= FLIP_LEVEL {
                    (4.2 * k).min((self.v - 2.5 * k).max(0.0))
                } else {
                    self.v + 1.2 * k
                };
            }
        }
        for c in &mut self.cars {
            c.x += c.v * dt;
        }
        let left = self.pos - self.car_x;
        self.cars
            .retain(|c| c.x > left - 60.0 * k && c.x < left + self.w + 200.0 * k);
        let spawn = left + self.w + 20.0 * k;
        if self.seg_at(spawn).is_none_or(|s| s.kind != FREEWAY) || self.v < 2.0 * k {
            return;
        }
        let (mut far, mut near) = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for c in &self.cars {
            if c.lane == 0 {
                far = far.max(c.x)
            } else {
                near = near.max(c.x)
            }
        }
        if spawn - far > (70.0 + self.rng.f() * 40.0) * k && self.rng.f() < 0.05 * dt {
            let truck = !self.tall && self.rng.f() < 0.25;
            self.cars.push(Car {
                x: spawn,
                lane: 0,
                v: 2.2 * k,
                c: draw::TRAFFIC[(self.rng.f() * 7.0).floor() as usize],
                truck,
            });
        }
        if d.level >= FLIP_LEVEL
            && spawn - near > (60.0 + self.rng.f() * 60.0) * k
            && self.rng.f() < 0.08 * dt
        {
            self.cars.push(Car {
                x: spawn,
                lane: 1,
                v: 4.2 * k,
                c: draw::TRAFFIC[(self.rng.f() * 7.0).floor() as usize],
                truck: false,
            });
        }
    }
    fn twist(&mut self) -> (f64, bool) {
        let roll = self.rng.f() < 0.7;
        (
            if self.rng.f() < if roll { 0.5 } else { 0.6 } {
                -1.0
            } else {
                1.0
            },
            roll,
        )
    }
    fn jump_over(&mut self, gap: f64, l: f64, own: f64, rv: f64, hgt: f64, wait: f64) -> Jump {
        let len = ((gap + l + own + 6.0 * self.k) / rv.max(0.1)).clamp(10.0, 48.0);
        let (dir, roll) = self.twist();
        Jump {
            wait,
            t: 0.0,
            len,
            hgt,
            dir,
            roll,
            checked: false,
        }
    }
    fn hgt(&self, main: bool) -> f64 {
        if self.tall {
            if main {
                14.0
            } else {
                8.0
            }
        } else if main {
            5.0
        } else {
            3.0
        }
    }
    fn move_jump(&mut self, dt: f64, lead: f64) {
        let l = self.main().w;
        let Some(mut j) = self.jump.take() else {
            let Some(c) = self.car_ahead(1, self.pos + l) else {
                return;
            };
            if self.v == 0.0 || self.in_lot > 0.0 {
                return;
            }
            let rv = self.v - c.v;
            let gap = c.x - (self.pos + l);
            if rv > 0.4 && gap <= l + rv * dt * lead {
                self.jump = Some(self.jump_over(
                    gap - rv * dt * lead,
                    self.near().w,
                    l,
                    rv,
                    self.hgt(true),
                    round(lead),
                ));
            }
            return;
        };
        if j.wait > 0.0 {
            j.wait -= 1.0;
            self.jump = Some(j);
            return;
        }
        self.flight(&mut j, self.pos, l, 1, self.v, true, dt);
        if j.t < j.len {
            self.jump = Some(j);
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn flight(&mut self, j: &mut Jump, x: f64, l: f64, lane: i32, speed: f64, main: bool, dt: f64) {
        let wheel = if lane == 1 {
            self.near_wheel
        } else {
            self.far_wheel
        };
        j.t += dt;
        if !j.checked && j.t >= j.len / 2.0 {
            j.checked = true;
            for _ in 0..3 {
                let rem = j.len - j.t;
                let at = x + speed * rem;
                let mut more: f64 = 0.0;
                for c in &self.cars {
                    if c.lane != lane {
                        continue;
                    }
                    let lc = if c.truck {
                        TRUCK.w
                    } else if lane == 1 {
                        self.near().w
                    } else {
                        SEDAN.w
                    };
                    let cx = c.x + c.v * rem;
                    if cx < at + l + 2.0 && cx + lc + 2.0 > at && speed - c.v > 0.3 {
                        more = more.max((cx + lc + 2.0 - at) / (speed - c.v));
                    }
                }
                if more <= 0.0 {
                    break;
                }
                j.len = (j.t + 60.0).min(j.len + more);
            }
        }
        let u = j.t / j.len;
        let lift = Self::lift(Some(j));
        if u < 0.3 {
            for _ in 0..if main { 2 } else { 1 } {
                self.puffs.push(Puff {
                    x: x + 2.0 + self.rng.f() * (l - 4.0),
                    y: wheel - lift + 1.0,
                    vx: (self.rng.f() - 0.5) * 0.2,
                    vy: 0.35 + self.rng.f() * 0.3,
                    r: 0.6 + self.rng.f() * 0.6,
                    life: 0.7,
                    fade: 0.06,
                    c: draw::VAPOUR,
                });
            }
        }
        if j.t >= j.len {
            for c in &self.cars {
                if c.lane != lane {
                    continue;
                }
                let lc = if c.truck {
                    TRUCK.w
                } else if lane == 1 {
                    self.near().w
                } else {
                    SEDAN.w
                };
                let close = (speed - c.v).abs();
                if c.x < x + l + 1.0 && c.x + lc + 1.0 > x && close > 0.3 {
                    j.len = j.len.max(
                        j.t + 30.0_f64.min(
                            (if speed > c.v {
                                c.x + lc + 2.0 - x
                            } else {
                                x + l + 2.0 - c.x
                            }) / close,
                        ),
                    );
                }
            }
        }
        if j.t >= j.len {
            for i in 0..4 {
                self.puffs.push(Puff {
                    x: x + if i < 2 { 1.0 } else { l - 3.0 } + self.rng.f() * 2.0,
                    y: wheel,
                    vx: -0.1 - self.rng.f() * 0.2,
                    vy: -0.05,
                    r: 0.6,
                    life: 0.5,
                    fade: 0.05,
                    c: draw::DUST,
                });
            }
        }
    }
    fn lift(j: Option<&Jump>) -> f64 {
        let Some(j) = j.filter(|j| j.wait <= 0.0) else {
            return 0.0;
        };
        let u = clamp01(j.t / j.len);
        round(4.0 * u * (1.0 - u) * j.hgt)
    }
    fn turn(j: Option<&Jump>) -> f64 {
        let Some(j) = j.filter(|j| j.wait <= 0.0) else {
            return 0.0;
        };
        j.dir * 2.0 * std::f64::consts::PI * smooth(clamp01((j.t / j.len - 0.08) / 0.84))
    }
    // Upstream uses 6.283 for companion sway; keep its exact phase.
    #[allow(clippy::approx_constant)]
    fn move_companions(&mut self, d: PixelDials, dt: f64, lead: f64) {
        let k = self.k;
        self.crew.room = self.stations.len();
        self.crew.update(&self.base.dials.agents, d.boost);
        let out_l = -(self.car_x + 24.0 * k);
        let out_r = self.w - self.car_x + 24.0 * k;
        let moving = clamp01(self.v / k);
        let mut mates = std::mem::take(&mut self.crew.mates);
        for m in &mut mates {
            let mut o = self.own.remove(&m.id).unwrap_or_else(|| Own {
                jump: None,
                next: 30.0 + self.rng.f() * 60.0,
                waved: false,
                last: f64::NAN,
            });
            let ph = m.seed * 6.283;
            let station = self.stations[m.slot] * k;
            let rest = station - 10.0 * k;
            let sway = ((d.t * 0.031 + ph).sin() * 3.0 + (d.t * 0.013 + ph * 2.3).sin() * 2.0)
                * k
                * m.busy
                * moving;
            let at = rest + (station - rest) * m.busy + sway;
            let out = if m.leaving && m.ok { out_r } else { out_l };
            let l = ROADSTER.w;
            let want = out + (at - out) * m.here;
            let w_want = self.pos + want;
            let speed = if o.last.is_nan() {
                self.v
            } else {
                (w_want - o.last) / dt
            };
            o.last = w_want;
            let mut x = want;
            if o.jump.is_some_and(|j| j.wait <= 0.0) {
                let j = o.jump.as_mut().unwrap();
                self.flight(j, w_want, l, 0, speed, false, dt);
                if j.t >= j.len {
                    o.jump = None;
                }
            } else {
                if let Some(j) = o.jump.as_mut() {
                    j.wait -= 1.0;
                }
                if o.jump.is_some_and(|j| j.wait > 0.0)
                    && self.cars.iter().any(|c| {
                        c.lane == 0
                            && c.x < w_want + l + 1.0 + (speed - c.v).abs() * dt
                            && c.x
                                + if c.truck { TRUCK.w } else { SEDAN.w }
                                + 1.0
                                + (speed - c.v).abs() * dt
                                > w_want
                    })
                {
                    o.jump.as_mut().unwrap().wait = 0.0;
                }
                for ci in 0..self.cars.len() {
                    let c = self.cars[ci];
                    if o.jump.is_some_and(|j| j.wait == 0.0) {
                        break;
                    }
                    if c.lane != 0 {
                        continue;
                    }
                    let lc = if c.truck { TRUCK.w } else { SEDAN.w };
                    let ahead = c.x + lc / 2.0 > w_want + l / 2.0;
                    let gap = if ahead {
                        c.x - (w_want + l)
                    } else {
                        w_want - (c.x + lc)
                    };
                    let rv = if ahead { speed - c.v } else { c.v - speed };
                    if o.jump.is_none() && rv > 0.6 && gap > -l / 2.0 && gap <= l + rv * dt * lead {
                        o.jump = Some(self.jump_over(
                            (gap - rv * dt * lead).max(0.0),
                            lc,
                            l,
                            rv,
                            self.hgt(false) + if c.truck { 1.0 } else { 0.0 },
                            if gap < rv * dt * 2.0 {
                                0.0
                            } else {
                                round(lead)
                            },
                        ));
                        if o.jump.is_some_and(|j| j.wait == 0.0) {
                            break;
                        }
                    }
                    let wx = self.pos + x;
                    if o.jump.is_some() || !(c.x < wx + l + 1.0 && c.x + lc + 1.0 > wx) {
                        continue;
                    }
                    let close = (speed - c.v).abs();
                    if close > 0.3 {
                        let need = if ahead {
                            c.x + lc + 2.0 - wx
                        } else {
                            wx + l + 2.0 - c.x
                        };
                        let (dir, roll) = self.twist();
                        o.jump = Some(Jump {
                            wait: 0.0,
                            t: 0.0,
                            len: (need / close).clamp(8.0, 60.0),
                            hgt: self.hgt(false),
                            dir,
                            roll,
                            checked: false,
                        });
                    } else {
                        x = if ahead {
                            c.x - l - 1.0 - self.pos
                        } else {
                            c.x + lc + 1.0 - self.pos
                        };
                    }
                }
                let flip = finished(Some(m)) && !o.waved && moving >= 1.0;
                let fast = level_for(self.v / k) >= 7.0;
                o.next -= dt
                    * if fast && m.busy > 0.5 && !m.leaving && !m.waiting {
                        1.0
                    } else {
                        0.0
                    };
                if o.jump.is_none() && (flip || o.next <= 0.0) {
                    o.waved |= flip;
                    o.next = (50.0 + self.rng.f() * 110.0) * (1.4 - clamp01(d.level / 10.0));
                    let len = 14.0 + self.rng.f() * 6.0;
                    let hgt = self.hgt(false) * (0.8 + self.rng.f() * 0.4);
                    let (dir, roll) = self.twist();
                    o.jump = Some(Jump {
                        wait: round(lead),
                        t: 0.0,
                        len,
                        hgt,
                        dir,
                        roll,
                        checked: false,
                    });
                }
            }
            m.x = x;
            m.y = self.far_wheel;
            let wx = self.pos + x;
            if failed(Some(m)) && self.rng.f() < 0.6 * dt {
                self.puffs.push(Puff {
                    x: wx + 1.0,
                    y: self.far_wheel - 2.0,
                    vx: 0.0,
                    vy: -0.06 - self.rng.f() * 0.05,
                    r: 0.7,
                    life: 1.0,
                    fade: 0.016,
                    c: draw::SOOT,
                });
            }
            self.own.insert(m.id.clone(), o);
        }
        self.own.retain(|id, _| mates.iter().any(|m| m.id == *id));
        self.crew.mates = mates;
    }
    fn move_puffs(&mut self, dt: f64, was: f64) {
        let k = self.k;
        if self.k_smoke > 0.3 && self.base.dials.tint == Tint::Smoke {
            let run = self.pos - was;
            let n = ((2.0 + run * 0.4) * dt).ceil() as usize;
            let wheel = self.wheel_y();
            for _ in 0..n {
                self.puffs.push(Puff {
                    x: was
                        + run * self.rng.f()
                        + 1.0
                        + self.rng.f() * if self.tall { 5.0 } else { 3.0 }
                        - if self.v == 0.0 {
                            2.0 * self.rng.f()
                        } else {
                            0.0
                        },
                    y: wheel - self.rng.f(),
                    vx: -0.12 * self.rng.f() * k,
                    vy: -(0.12 + self.rng.f() * 0.25) * k,
                    r: 0.8 + self.rng.f() * 0.5,
                    life: 1.0,
                    fade: 0.012 + self.rng.f() * 0.008,
                    c: draw::SMOKE,
                });
            }
        }
        for p in &mut self.puffs {
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            p.vy *= 0.94;
            p.r = (if self.tall { 5.0_f64 } else { 3.0 }).min(p.r + 0.035 * dt * k);
            p.life -= p.fade * dt;
        }
        self.puffs.retain(|p| p.life > 0.0 && p.y > -6.0);
        if self.puffs.len() > 360 {
            self.puffs.drain(..self.puffs.len() - 360);
        }
    }
    fn wheel_y(&self) -> f64 {
        round(self.near_wheel + (self.lot_wheel - self.near_wheel) * smooth(clamp01(self.in_lot)))
    }
}
impl Scene for Roadster {
    fn dials(&mut self) -> &mut Dials {
        &mut self.base.dials
    }
    fn ensure(&mut self, columns: usize, rows: usize) {
        if self.base.ensure(columns, rows) {
            self.layout(self.base.frame_dials());
            self.segs.clear();
            self.mids.clear();
            self.fars.clear();
            self.cars.clear();
            self.puffs.clear();
            self.taken.clear();
            self.stop_at = None;
            self.jump = None;
            self.started = false;
            self.v = 0.0;
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
        let mut p = self.base.begin_paint();
        self.paint(&mut p, d);
        self.base.finish_grid(p)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::agents::{AgentDial, AgentState};

    #[test]
    fn parks_waits_and_jumps_clear_of_traffic() {
        for (columns, rows) in [(15, 4), (40, 12), (120, 5), (22, 60)] {
            for seed in [3.0, 8.0] {
                let mut road = Roadster::new(seed);
                road.ensure(columns, rows);
                road.base.dials.agents = ["a", "b", "c"]
                    .into_iter()
                    .enumerate()
                    .map(|(i, id)| AgentDial {
                        id: id.into(),
                        state: if i == 1 {
                            AgentState::Idle
                        } else {
                            AgentState::Working
                        },
                        ok: true,
                        task: String::new(),
                        kind: String::new(),
                        ms: 0.0,
                    })
                    .collect();
                road.base.dials.strength = 1.0;
                for _ in 0..300 {
                    road.step();
                    assert!(
                        road.own.values().all(|o| o.jump.is_none()),
                        "parked companions stay on their wheels"
                    );
                }
                assert!(road.v == 0.0 && road.in_lot >= 1.0);
                let mut airborne = false;
                for (level, frames) in [(10.0, 1500), (8.0, 200), (5.0, 200), (3.0, 200)] {
                    road.base.dials.strength = level;
                    for _ in 0..frames {
                        road.step();
                        let flying = road.jump.is_some_and(|j| j.wait <= 0.0);
                        airborne |= flying;
                        if !flying {
                            assert!(
                                !road.cars.iter().any(|c| c.lane == 1
                                    && c.x < road.pos + road.main().w
                                    && c.x + road.near().w > road.pos),
                                "main car crossed traffic at {columns}x{rows}, seed {seed}"
                            );
                        }
                        for m in &road.crew.mates {
                            if m.here < 1.0
                                || road
                                    .own
                                    .get(&m.id)
                                    .is_some_and(|o| o.jump.is_some_and(|j| j.wait <= 0.0))
                            {
                                continue;
                            }
                            let x = road.pos + m.x;
                            assert!(
                                !road.cars.iter().any(|c| c.lane == 0
                                    && c.x < x + 14.0
                                    && c.x + if c.truck { 22.0 } else { 14.0 } > x),
                                "companion crossed traffic at {columns}x{rows}, seed {seed}"
                            );
                        }
                    }
                }
                assert!(airborne, "freeway traffic triggers a jump");
                road.base.dials.strength = 2.0;
                road.base.dials.waiting = true;
                for _ in 0..600 {
                    road.step();
                }
                assert!(
                    road.v == 0.0 && road.in_lot >= 1.0,
                    "waiting parks in a lot"
                );
                road.base.dials.strength = 5.0;
                road.base.dials.waiting = false;
                for _ in 0..80 {
                    road.step();
                }
                assert!(
                    road.v > 0.0 && road.in_lot < 1.0,
                    "answering leaves the lot"
                );
            }
        }
    }

    #[test]
    fn frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/roadster.json"), 0.0);
    }
}
