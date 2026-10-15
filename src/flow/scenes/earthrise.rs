//! Earthrise: a rotating Earth over a shadowed lunar height field, following
//! Flow's cell renderer. The original artwork's MIT notice is retained in
//! ../THIRD-PARTY-NOTICES. Kitty image rendering and sound are host features.
// The nested pixel callbacks and coordinate predicates follow the reference renderer.
#![allow(clippy::type_complexity, clippy::manual_range_contains)]
use crate::flow::cells::{Cells, Rng};
use crate::flow::js::{hypot, round};
use crate::flow::palette::SceneHue;
use crate::flow::pixel_scene::{Painter, PixelDials, PixelScene, CLEAR};
use crate::flow::pixels::{
    clamp, clamp01, grey, hash, hash1, luma, mix, ramp_at, smoothstep as smooth,
};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use std::f64::consts::PI;
mod world;
use world::{earth_at, fbm, Layout};
pub const DEF: SceneDef = SceneDef {
    name: "earthrise",
    hue: SceneHue {
        // Keep land, signals and neutral details; turn the blue hues gently.
        key: 250.,
        range: 45.,
        spread: Some(45.),
    },
    figure: false,
    make: |seed| Box::new(Earthrise::new(seed)),
};
const GROUND: [u32; 13] = [
    0x0b0b0d, 0x18181b, 0x232326, 0x303033, 0x414143, 0x555556, 0x6b6a69, 0x83817d, 0x9c9993,
    0xb6b2aa, 0xcfcac1, 0xe6e1d8, 0xf7f4ee,
];
fn lerp(a: f64, b: f64, k: f64) -> f64 {
    a + (b - a) * k
}
fn light_color(r: f64, g: f64, b: f64) -> u32 {
    let c = |v: f64| {
        (255.0_f64.min(round((clamp(v, 0., 1.2) / 1.2).powf(0.72) * 255. * 1.22)) as u32) & 0xf8
    };
    (c(r) << 16) | (c(g) << 8) | c(b)
}
struct Meteor {
    x: f64,
    y: f64,
    ux: f64,
    uy: f64,
    speed: f64,
    age: f64,
    life: f64,
    len: f64,
    b: f64,
}
struct Comet {
    x: f64,
    y: f64,
    vx: f64,
    vy: f64,
    age: f64,
    life: f64,
    len: f64,
    b: f64,
    curl: f64,
}
struct View {
    col0: f64,
    row0: f64,
    fx: f64,
    fy: f64,
    w: usize,
    h: usize,
}
pub struct Earthrise {
    base: PixelScene,
    cells: Layout,
    specks: Vec<[f32; 3]>,
    spin: f64,
    drift: f64,
    alt: f64,
    k_smoke: f64,
    k_blue: f64,
    sky_off: f64,
    meteors: Vec<Meteor>,
    comets: Vec<Comet>,
    rng: Rng,
}
impl Earthrise {
    pub fn new(seed: f64) -> Self {
        Self {
            base: PixelScene::new(seed),
            cells: Layout::new(0, 0, false),
            specks: vec![],
            spin: hash1(seed) * PI * 2.,
            drift: 0.,
            alt: f64::NAN,
            k_smoke: 0.,
            k_blue: 0.,
            sky_off: hash1(seed + 7.) * 1000.,
            meteors: vec![],
            comets: vec![],
            rng: Rng::new((hash1(seed + 13.) * 0x7fffffff as f64).floor()),
        }
    }
    fn resize(&mut self, d: PixelDials) {
        self.cells = Layout::new(d.columns * 2, d.rows * 2, d.tall);
        self.cells.build();
        self.specks = vec![[0.; 3]; d.columns * 2 * d.rows * 4];
    }
    fn update(&mut self, d: PixelDials) {
        let busy = clamp01((d.level - 1.) / 9.);
        if self.alt.is_nan() {
            self.alt = busy;
        } else if d.wait < 0.5 {
            self.alt += clamp(busy - self.alt, -0.007, 0.007);
        }
        let still = 1. - d.wait;
        let rate = 0.05 * (1.6_f64 / 0.05).powf(busy) * still;
        let dt = 0.07;
        self.spin += rate * dt;
        self.drift += rate * 0.27 * dt;
        self.k_smoke = clamp01(
            self.k_smoke
                + clamp(
                    (if d.tint == Tint::Smoke { 1. } else { 0. }) - self.k_smoke,
                    -0.05,
                    0.05,
                ),
        );
        self.k_blue = clamp01(
            self.k_blue
                + clamp(
                    (if d.tint == Tint::Blue { 1. } else { 0. }) - self.k_blue,
                    -0.04,
                    0.04,
                ),
        );
        self.move_sky(d, dt, still);
    }
    fn move_sky(&mut self, d: PixelDials, dt: f64, still: f64) {
        let busy = self.alt;
        self.sky_off += (0.03 + 5. * busy.powf(1.7)) * still * dt;
        let l = &self.cells;
        if l.pw == 0 {
            return;
        }
        let c0 = l.col0;
        let c1 = l.col0 + l.pw as f64 * l.s;
        let r0 = l.row0;
        let r1 = (r0 + 4.).max(l.horizon);
        for m in &mut self.meteors {
            m.age += dt;
            m.x += m.ux * m.speed * dt;
            m.y += m.uy * m.speed * dt;
        }
        self.meteors.retain(|m| m.age < m.life);
        let meteors = (0.015 + 0.9 * busy * busy) * (1. + d.boost / 30.) * still;
        let rng = &mut self.rng;
        if self.meteors.len() < 6 && rng.f() < meteors * dt {
            let a = 0.2 + 0.55 * rng.f();
            let dir = if rng.f() < 0.65 { -1. } else { 1. };
            self.meteors.push(Meteor {
                x: c0 + (c1 - c0) * rng.f(),
                y: r0 + (r1 - r0) * 0.6 * rng.f(),
                ux: dir * a.cos(),
                uy: a.sin(),
                speed: 45. + 60. * rng.f(),
                age: 0.,
                life: 0.3 + 0.5 * rng.f(),
                len: 6. + 9. * rng.f(),
                b: 0.7 + 0.6 * rng.f(),
            });
        }
        for c in &mut self.comets {
            c.age += dt * still;
            c.x += c.vx * dt * still;
            c.y += c.vy * dt * still;
        }
        self.comets.retain(|c| c.age < c.life);
        let comets = 0.055 * smooth(0.2, 1., busy) * still;
        let room = if busy > 0.55 { 2 } else { 1 };
        if self.comets.len() < room && rng.f() < comets * dt {
            let a = (rng.f() - 0.5) * 0.7 + (if rng.f() < 0.5 { 0. } else { PI });
            let v = 1.5 + 2.5 * rng.f();
            self.comets.push(Comet {
                x: c0 + (c1 - c0) * (0.12 + 0.76 * rng.f()),
                y: r0 + (r1 - r0) * (0.12 + 0.4 * rng.f()),
                vx: a.cos() * v,
                vy: a.sin() * v * 0.4,
                age: 0.,
                life: 18. + 16. * rng.f(),
                len: 16. + 20. * rng.f(),
                b: 0.75 + 0.35 * rng.f(),
                curl: (if rng.f() < 0.5 { -1. } else { 1. }) * (0.002 + 0.003 * rng.f()),
            });
        }
    }
    fn sky(&self, v: &View, out: &mut [[f32; 3]], t_sec: f64) {
        let busy = self.alt;
        let dim = 1. - 0.6 * self.k_smoke;
        let mut add = |col: f64, row: f64, k: f64, tr: f64, tg: f64, tb: f64| {
            let x = round((col - v.col0) / v.fx - 0.5);
            let y = round((row - v.row0) / v.fy - 0.5);
            if x < 0. || y < 0. || x >= v.w as f64 || y >= v.h as f64 {
                return;
            }
            let o = y as usize * v.w + x as usize;
            out[o][0] = (out[o][0] as f64 + k * tr) as f32;
            out[o][1] = (out[o][1] as f64 + k * tg) as f32;
            out[o][2] = (out[o][2] as f64 + k * tb) as f32;
        };
        for [cell, dens, more, speed, lo, hi, salt] in [
            [4., 0.0032, 2.6, 0.35, 0.17, 0.45, 101.],
            [6., 0.0016, 2.2, 0.7, 0.24, 0.7, 211.],
            [12., 0.0005, 1.4, 1.4, 0.45, 1.05, 307.],
        ] {
            let ox = self.sky_off * speed;
            let oy = ox * 0.12;
            let u0 = v.col0 + ox;
            let v0 = v.row0 + oy;
            let p0 = dens * cell * cell * (1. + more * busy);
            let twinkling = 0.12 + 0.35 * busy;
            for j in (v0 / cell).floor() as i32..=((v0 + v.h as f64 * v.fy) / cell).floor() as i32 {
                let j = j as f64;
                for i in
                    (u0 / cell).floor() as i32..=((u0 + v.w as f64 * v.fx) / cell).floor() as i32
                {
                    let i = i as f64;
                    let h0 = hash(i, j, salt);
                    let mut p = p0;
                    if salt == 101. {
                        let mut q = j * cell - 0.32 * i * cell - 4.;
                        q -= 170. * round(q / 170.);
                        p *= 1.
                            + 3. * (-(q / 10.5).powi(2)).exp()
                                * smooth(0.35, 0.65, fbm(i * cell * 0.05, j * cell * 0.08, 3, 0.));
                    }
                    if h0 >= p {
                        continue;
                    }
                    let fade = clamp01((p - h0) / (0.3 * p0));
                    let col = (i + hash(i, j, salt + 1.)) * cell - ox;
                    let row = (j + hash(i, j, salt + 2.)) * cell - oy;
                    let mut k = (lo + (hi - lo) * hash(i, j, salt + 3.).powi(3))
                        * fade
                        * dim
                        * (1. + 0.35 * busy);
                    let tw = hash(i, j, salt + 4.);
                    if tw > 1.0 - twinkling {
                        k *= 0.5
                            + 0.5
                                * (((t_sec * (1.0 + 1.5 * busy))
                                    / (2.0 + 3.0 * hash(i, j, salt + 5.0)))
                                    * PI
                                    * 2.0
                                    + tw * 50.0)
                                    .sin();
                    }
                    let tint = hash(i, j, salt + 6.);
                    let (tr, tg, tb) = if tint < 0.3 {
                        (0.84, 0.9, 1.)
                    } else if tint > 0.88 {
                        (1., 0.93, 0.84)
                    } else {
                        (0.96, 0.96, 0.98)
                    };
                    add(col, row, k, tr, tg, tb);
                }
            }
        }
        let mut field = |x0: f64,
                         x1: f64,
                         y0: f64,
                         y1: f64,
                         at: &mut dyn FnMut(
            f64,
            f64,
            &mut dyn FnMut(f64, f64, f64, f64, f64, f64),
        )| {
            let xa = 0.0_f64.max(((x0 - v.col0) / v.fx).floor()) as i32;
            let xb = (v.w as f64 - 1.).min(((x1 - v.col0) / v.fx).ceil()) as i32;
            let ya = 0.0_f64.max(((y0 - v.row0) / v.fy).floor()) as i32;
            let yb = (v.h as f64 - 1.).min(((y1 - v.row0) / v.fy).ceil()) as i32;
            for y in ya..=yb {
                for x in xa..=xb {
                    at(
                        v.col0 + (x as f64 + 0.5) * v.fx,
                        v.row0 + (y as f64 + 0.5) * v.fy,
                        &mut add,
                    );
                }
            }
        };
        for m in &self.meteors {
            let tl = 0.5_f64.max(m.len.min(m.speed * m.age));
            let wd = 0.45_f64.max(0.6 * v.fx.min(v.fy));
            let env =
                1.0_f64.min(m.age / 0.08) * (1. - smooth(0.55 * m.life, m.life, m.age)) * m.b * dim;
            let tx = m.x - m.ux * tl;
            let ty = m.y - m.uy * tl;
            field(
                m.x.min(tx) - 2.,
                m.x.max(tx) + 2.,
                m.y.min(ty) - 2.,
                m.y.max(ty) + 2.,
                &mut |col, row, add| {
                    let back = -((col - m.x) * m.ux + (row - m.y) * m.uy);
                    if back < -wd * 2. || back > tl + wd {
                        return;
                    }
                    let perp = (col - m.x) * m.uy - (row - m.y) * m.ux;
                    let along = if back < 0. {
                        (-(back / wd).powi(2)).exp()
                    } else {
                        (1. - clamp01(back / tl)).powf(1.6)
                    };
                    let k = env * along * (-(perp / wd).powi(2)).exp();
                    if k < 0.01 {
                        return;
                    }
                    add(col, row, k, 0.92, 0.96, 1.);
                },
            );
        }
        let (sx, sy) = world::sun_screen();
        for c in &self.comets {
            let env = smooth(0., 3., c.age) * (1. - smooth(c.life - 4., c.life, c.age)) * c.b * dim;
            if env <= 0. {
                continue;
            }
            let reach = c.len * 1.1 + 4.;
            let ux = -sx;
            let uy = -sy;
            field(
                c.x - reach,
                c.x + reach,
                c.y - reach * 0.5,
                c.y + reach * 0.5,
                &mut |col, row, add| {
                    let dx = col - c.x;
                    let dy = row - c.y;
                    let r2 = dx * dx + dy * dy;
                    let mut kr = 1.3 * (-r2 / 0.35).exp() + 0.6 * (-r2 / 5.).exp();
                    let mut kg = kr;
                    let mut kb = kr;
                    let a = dx * ux + dy * uy;
                    if a > 0. {
                        let p = dx * uy - dy * ux;
                        let wi = 0.5 + 0.022 * a;
                        let ion = 0.85
                            * (-a / (0.6 * c.len)).exp()
                            * (-(p / wi).powi(2)).exp()
                            * smooth(0., 2., a);
                        let pd = p - c.curl * a * a;
                        let wd = 0.8 + 0.1 * a;
                        let dust = 0.8 * (-a / (0.4 * c.len)).exp() * (-(pd / wd).powi(2)).exp();
                        kr += ion * 0.62 + dust * 1.;
                        kg += ion * 0.82 + dust * 0.95;
                        kb += ion * 1.05 + dust * 0.82;
                    }
                    let m = kr.max(kg).max(kb) * env;
                    if m < 0.015 {
                        return;
                    }
                    add(col, row, env, kr, kg, kb);
                },
            );
        }
    }
    fn tinted(&self, mut c: u32) -> u32 {
        if self.k_smoke > 0. {
            c = mix(
                grey(c, 0.9),
                0x8a8682,
                self.k_smoke * 0.5 * (1. - luma(c) / 255.),
            );
        }
        if self.k_blue > 0. {
            let v = luma(c) / 255.;
            let cold = (((round(v * 40.) as u32) & 255) << 16)
                | (((round(14. + v * 100.) as u32) & 255) << 8)
                | round(60. + v * 195.) as u32;
            c = mix(c, cold, self.k_blue * 0.85);
        }
        c
    }
    fn dust(&self, x: usize, y: usize, sky: usize, t_sec: f64) -> f64 {
        let l = &self.cells;
        let up = (sky - y) as f64 / 4.0_f64.max(l.ph as f64 * 0.5);
        let n = fbm(
            (l.col0 + x as f64 * l.s) * 0.033 + t_sec * 0.4,
            (l.row0 + y as f64 * l.sy) * 0.0625,
            2,
            0.,
        );
        self.k_smoke * clamp01(1.3 - up) * (0.55 + 0.45 * n)
    }
    fn paint(&mut self, px: &mut Painter, d: PixelDials) {
        let l = &self.cells;
        let t_sec = d.t * 0.07;
        let ey = lerp(
            l.ey_low,
            l.ey_high,
            if self.alt.is_nan() {
                clamp01((d.level - 1.) / 9.)
            } else {
                self.alt
            },
        );
        for i in 0..l.pw * l.ph {
            let b = l.ground[i] as f64;
            if b < 0. {
                continue;
            }
            let q = round(clamp01(b).powf(0.85) * 28.) / 28.;
            px.px[i] = self.tinted(ramp_at(&GROUND, q));
        }
        if self.k_smoke > 0. {
            for x in 0..l.pw {
                let mut sky = 0;
                while sky < l.ph && l.ground[sky * l.pw + x] < 0. {
                    sky += 1;
                }
                for y in 0..sky {
                    let k = self.dust(x, y, sky, t_sec);
                    if k <= 0.12 {
                        continue;
                    }
                    px.px[y * l.pw + x] = mix(0x1c1b1a, 0x77746f, round(k * 6.) / 6.);
                }
            }
        }
        let nx = clamp(round(l.s), 1., 2.) as usize;
        let ny = clamp(round(l.sy), 1., 3.) as usize;
        let sw = l.s / nx as f64;
        let shh = l.sy / ny as f64;
        let pall = self.k_smoke;
        for y in 0..l.ph {
            for x in 0..l.pw {
                let i = y * l.pw + x;
                if l.near[i] == 0 {
                    continue;
                }
                let (mut r, mut g, mut b, mut a) = (0., 0., 0., 0.);
                for jy in 0..ny {
                    let dy = l.row0 + y as f64 * l.sy + (jy as f64 + 0.5) * shh - ey;
                    for jx in 0..nx {
                        let dx = l.col0 + x as f64 * l.s + (jx as f64 + 0.5) * sw - l.ex;
                        let out = earth_at(dx, dy, l.er, self.spin, self.drift);
                        r += out[0] as f64;
                        g += out[1] as f64;
                        b += out[2] as f64;
                        a += out[3] as f64;
                    }
                }
                let m = (nx * ny) as f64;
                a /= m;
                if a < 0.3 {
                    continue;
                }
                let c = self.tinted(light_color(r / m / a, g / m / a, b / m / a));
                let under = px.px[i];
                px.px[i] = if under == CLEAR {
                    c
                } else {
                    mix(under, c, 1. - pall * 0.5)
                };
            }
        }
        let view = View {
            col0: l.col0,
            row0: l.row0,
            fx: l.s,
            fy: l.sy / 2.,
            w: px.dw,
            h: px.dh,
        };
        let pw = l.pw;
        let mut specks = std::mem::take(&mut self.specks);
        if specks.len() != px.dw * px.dh {
            self.specks = specks;
            return;
        }
        specks.fill([0.; 3]);
        self.sky(&view, &mut specks, t_sec);
        for y in 0..px.dh {
            let pyy = y >> 1;
            for x in 0..px.dw {
                let [r, g, b] = specks[y * px.dw + x].map(|x| x as f64);
                let v = r.max(g).max(b);
                if v < 0.16 {
                    continue;
                }
                let i = pyy * pw + x;
                if px.px[i] != CLEAR {
                    continue;
                }
                let cell = (pyy >> 1) * (pw >> 1) + (x >> 1);
                let c0 = (pyy & !1) * pw + (x & !1);
                if px.px[c0] != CLEAR
                    || px.px[c0 + 1] != CLEAR
                    || px.px[c0 + pw] != CLEAR
                    || px.px[c0 + pw + 1] != CLEAR
                {
                    continue;
                }
                let star = self.tinted(light_color(r, g, b) | 0x101010);
                let prev = if px.dots[cell] != 0 {
                    Some(px.dot_color[cell])
                } else {
                    None
                };
                px.dot(x as f64, y as f64, star);
                if let Some(prev) = prev {
                    if luma(prev) > luma(star) {
                        px.dot_color[cell] = prev;
                    }
                }
            }
        }
        self.specks = specks;
    }
}
impl Scene for Earthrise {
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
        crate::flow::reference::check(include_str!("../testdata/earthrise.json"), 0.0);
    }
}
