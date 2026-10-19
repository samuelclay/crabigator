use super::*;
use crate::flow::clouds;
use crate::flow::crew::beckon;
use crate::flow::js::hypot;
use crate::flow::night::{
    moon_cover, moon_pixel, moon_radius, MOON, NIGHT_HORIZON, NIGHT_ZENITH, STAR, STAR_DIM,
};
use crate::flow::pixels::{grey, hash, mix, noise1};
use std::f64::consts::PI;
const GLASS: u32 = 0x161a22;
const TYRE: u32 = 0x0e0e10;
const RIM: u32 = 0x8a9098;
const TAIL: u32 = 0xff2a2a;
const HEAD: u32 = 0xfff0c0;
const HAZARD: u32 = 0xffa020;
const PEARL: u32 = 0xffffff;
const RAIN: u32 = 0x9ab8e0;
pub(super) const SMOKE: u32 = 0xd0d0cc;
pub(super) const SOOT: u32 = 0x4a4a4a;
pub(super) const VAPOUR: u32 = 0xf4f8ff;
pub(super) const DUST: u32 = 0xb8a888;
const LIVERY: [u32; 6] = [0xe0303a, 0x2f6fe0, 0xf0c020, 0x30b050, 0xf07a20, 0x9a50d0];
pub(super) const TRAFFIC: [u32; 7] = [
    0x8a2a2a, 0x2a4a7a, 0x74787e, 0xc8c4bc, 0x3a5a3a, 0x5a4a6a, 0xa08060,
];
const TOWER: [u32; 6] = [0x8a94a0, 0x6f7f92, 0xa89c8a, 0x9a6a5a, 0x5f6b78, 0xb4b8bc];
const TOWER_N: [u32; 6] = [0x1e2228, 0x1a2030, 0x262420, 0x2a1c1a, 0x181c22, 0x2a2c30];
const SHOP: [u32; 5] = [0xc8a070, 0xb06050, 0x7090a0, 0xd0c090, 0x90a070];
const SHOP_N: [u32; 5] = [0x2c2418, 0x281614, 0x182228, 0x2c2a1e, 0x202418];
const AWNING: [u32; 4] = [0xd03030, 0x2f6fd0, 0x30a050, 0xe0a020];
const HOUSE: [u32; 4] = [0xe8dcc0, 0xc8d8e0, 0xf0e0d0, 0xd0c0a0];
const HOUSE_N: [u32; 4] = [0x2a2822, 0x22262a, 0x2c2824, 0x26221c];
const ROOF: [u32; 3] = [0x8a3a2a, 0x4a4a52, 0x6a4a3a];
const ROOF_N: [u32; 3] = [0x1e0e0a, 0x121216, 0x1a120e];
const BILLBOARD: [u32; 4] = [0xe04040, 0x3a8ae0, 0xf0c020, 0x40c070];
const HAZE: [u32; 2] = [0xb8b6b2, 0x7a7874];
const STORM_CAST: [u32; 2] = [0x1e2c4c, 0x2a4078];
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
pub(super) struct Pal {
    skyTop: u32,
    skyLow: u32,
    haze: u32,
    hazeLit: u32,
    hill: u32,
    hillFar: u32,
    walk: u32,
    kerb: u32,
    asphalt: u32,
    dash: u32,
    yellow: u32,
    grass: u32,
    grassDark: u32,
    barrier: u32,
    barrierTop: u32,
    pole: u32,
    lampHead: u32,
    lamp: u32,
    pool: u32,
    sign: u32,
    signText: u32,
    pBlue: u32,
    lot: u32,
    lotLine: u32,
    glassWin: u32,
    lit: u32,
    shopWin: u32,
    shopLit: u32,
    trunk: u32,
    tree: u32,
    treeDark: u32,
    wall: u32,
    wallLine: u32,
    beam: u32,
    green: u32,
}
pub(super) const DAY: Pal = Pal {
    skyTop: 0x5a98de,
    skyLow: 0xa9cff0,
    haze: 0x8faac4,
    hazeLit: 0x8faac4,
    hill: 0x7a9e74,
    hillFar: 0x8eaab0,
    walk: 0xb8b4ac,
    kerb: 0x8c8a86,
    asphalt: 0x45484d,
    dash: 0xe8e8e0,
    yellow: 0xe8c040,
    grass: 0x5f9a46,
    grassDark: 0x4a7e38,
    barrier: 0xbab8b0,
    barrierTop: 0x8e8c84,
    pole: 0x5a5e64,
    lampHead: 0x34363a,
    lamp: 0x34363a,
    pool: 0xb8b4ac,
    sign: 0x1f6a3a,
    signText: 0xe8f0e8,
    pBlue: 0x2a5cc8,
    lot: 0x6a6c70,
    lotLine: 0xe0e0d8,
    glassWin: 0x3a5470,
    lit: 0x3a5470,
    shopWin: 0x2e4658,
    shopLit: 0x2e4658,
    trunk: 0x6a4a30,
    tree: 0x3f8a3a,
    treeDark: 0x2f6a2c,
    wall: 0xa8a49a,
    wallLine: 0x8c887e,
    beam: 0x45484d,
    green: 0x30e070,
};
const NIGHT: Pal = Pal {
    skyTop: NIGHT_ZENITH,
    skyLow: NIGHT_HORIZON,
    haze: 0x161c28,
    hazeLit: 0xb89a5a,
    hill: 0x0e1610,
    hillFar: 0x121820,
    walk: 0x3c3b38,
    kerb: 0x2c2b29,
    asphalt: 0x1c1d20,
    dash: 0x8c8c84,
    yellow: 0x8a7428,
    grass: 0x14240f,
    grassDark: 0x0e1a0a,
    barrier: 0x3e3e3c,
    barrierTop: 0x2a2a28,
    pole: 0x2a2c30,
    lampHead: 0x2a2c30,
    lamp: 0xffe2a0,
    pool: 0x5e5644,
    sign: 0x0e3a20,
    signText: 0xc8dcc8,
    pBlue: 0x1a3c8a,
    lot: 0x262729,
    lotLine: 0x7a7a74,
    glassWin: 0x10141a,
    lit: 0xffd27a,
    shopWin: 0x12161c,
    shopLit: 0xffc870,
    trunk: 0x1a120c,
    tree: 0x10200e,
    treeDark: 0x0a1608,
    wall: 0x34332f,
    wallLine: 0x26251f,
    beam: 0x4a4a46,
    green: 0x30e070,
};
#[derive(Clone, Copy)]
#[allow(non_snake_case)]
struct Livery {
    B: u32,
    b: u32,
    G: u32,
    T: u32,
    L: u32,
    W: u32,
    R: u32,
    X: u32,
    d: u32,
}
impl Livery {
    fn color(self, ch: u8) -> Option<u32> {
        Some(match ch {
            b'B' => self.B,
            b'b' => self.b,
            b'G' => self.G,
            b'T' => self.T,
            b'L' => self.L,
            b'W' => self.W,
            b'R' => self.R,
            b'X' => self.X,
            b'd' => self.d,
            _ => return None,
        })
    }
}
fn q(k: f64, n: f64) -> f64 {
    round(clamp01(k) * n) / n
}
impl Roadster {
    fn palette(&mut self, d: PixelDials) {
        let n = q(d.night, 8.0);
        self.pal = Pal {
            skyTop: self.tone(mix(DAY.skyTop, NIGHT.skyTop, n), d),
            skyLow: self.tone(mix(DAY.skyLow, NIGHT.skyLow, n), d),
            haze: self.tone(mix(DAY.haze, NIGHT.haze, n), d),
            hazeLit: self.tone(mix(DAY.hazeLit, NIGHT.hazeLit, n), d),
            hill: self.tone(mix(DAY.hill, NIGHT.hill, n), d),
            hillFar: self.tone(mix(DAY.hillFar, NIGHT.hillFar, n), d),
            walk: self.tone(mix(DAY.walk, NIGHT.walk, n), d),
            kerb: self.tone(mix(DAY.kerb, NIGHT.kerb, n), d),
            asphalt: self.tone(mix(DAY.asphalt, NIGHT.asphalt, n), d),
            dash: self.tone(mix(DAY.dash, NIGHT.dash, n), d),
            yellow: self.tone(mix(DAY.yellow, NIGHT.yellow, n), d),
            grass: self.tone(mix(DAY.grass, NIGHT.grass, n), d),
            grassDark: self.tone(mix(DAY.grassDark, NIGHT.grassDark, n), d),
            barrier: self.tone(mix(DAY.barrier, NIGHT.barrier, n), d),
            barrierTop: self.tone(mix(DAY.barrierTop, NIGHT.barrierTop, n), d),
            pole: self.tone(mix(DAY.pole, NIGHT.pole, n), d),
            lampHead: self.tone(mix(DAY.lampHead, NIGHT.lampHead, n), d),
            lamp: self.tone(mix(DAY.lamp, NIGHT.lamp, n), d),
            pool: self.tone(mix(DAY.pool, NIGHT.pool, n), d),
            sign: self.tone(mix(DAY.sign, NIGHT.sign, n), d),
            signText: self.tone(mix(DAY.signText, NIGHT.signText, n), d),
            pBlue: self.tone(mix(DAY.pBlue, NIGHT.pBlue, n), d),
            lot: self.tone(mix(DAY.lot, NIGHT.lot, n), d),
            lotLine: self.tone(mix(DAY.lotLine, NIGHT.lotLine, n), d),
            glassWin: self.tone(mix(DAY.glassWin, NIGHT.glassWin, n), d),
            lit: self.tone(mix(DAY.lit, NIGHT.lit, n), d),
            shopWin: self.tone(mix(DAY.shopWin, NIGHT.shopWin, n), d),
            shopLit: self.tone(mix(DAY.shopLit, NIGHT.shopLit, n), d),
            trunk: self.tone(mix(DAY.trunk, NIGHT.trunk, n), d),
            tree: self.tone(mix(DAY.tree, NIGHT.tree, n), d),
            treeDark: self.tone(mix(DAY.treeDark, NIGHT.treeDark, n), d),
            wall: self.tone(mix(DAY.wall, NIGHT.wall, n), d),
            wallLine: self.tone(mix(DAY.wallLine, NIGHT.wallLine, n), d),
            beam: self.tone(mix(DAY.beam, NIGHT.beam, n), d),
            green: self.tone(mix(DAY.green, NIGHT.green, n), d),
        };
    }
}

fn span(start: f64, end: f64) -> impl Iterator<Item = f64> {
    std::iter::successors(Some(start), |v| Some(v + 1.0)).take_while(move |v| *v < end)
}
fn inclusive(start: f64, end: f64) -> impl Iterator<Item = f64> {
    std::iter::successors(Some(start), |v| Some(v + 1.0)).take_while(move |v| *v <= end)
}
fn even(v: f64) -> f64 {
    f64::from(i32_of(v) & !1)
}
impl Roadster {
    fn tone(&self, c: u32, d: PixelDials) -> u32 {
        let nk = q(d.night, 4.0);
        let mut out = c;
        if self.k_smoke > 0.0 {
            out = mix(
                grey(c, 0.6 * self.k_smoke),
                mix(HAZE[0], HAZE[1], nk),
                q(0.35 * self.k_smoke, 6.0),
            );
        }
        if self.k_rain > 0.0 {
            out = mix(
                out,
                mix(STORM_CAST[0], STORM_CAST[1], nk),
                q(0.45 * self.k_rain, 6.0),
            );
        }
        out
    }
    fn sky(&self, y: f64, d: PixelDials) -> u32 {
        let n = usize::from(d.night > 0.5);
        let nk = q(d.night, 8.0);
        let yy = f64::from((i32_of(y) >> 1) * 2);
        let f = q(yy / (self.far_walk - 1.0).max(1.0), 6.0);
        let mut c = mix(
            mix(DAY.skyTop, NIGHT.skyTop, nk),
            mix(DAY.skyLow, NIGHT.skyLow, nk),
            f,
        );
        if self.k_smoke > 0.0 {
            c = mix(
                c,
                mix([0xb0b0ae, 0x4a4a4a][n], [0xc8c6c2, 0x6a6864][n], f),
                q(self.k_smoke, 6.0),
            );
        }
        if self.k_rain > 0.0 {
            c = mix(
                c,
                mix([0x1b2540, 0x16264c][n], [0x3e4f70, 0x2a4274][n], f),
                q(self.k_rain, 6.0),
            );
        }
        c
    }
    pub(super) fn paint(&mut self, px: &mut Painter, d: PixelDials) {
        if self.w == 0.0 || self.segs.is_empty() {
            return;
        }
        self.palette(d);
        for y in span(0.0, self.far_walk) {
            let c = self.sky(y, d);
            for x in span(0.0, self.w) {
                px.set(x, y, c, false);
            }
        }
        self.paint_moon(px, d);
        self.paint_clouds(px, d);
        self.paint_far(px, d);
        self.paint_mid(px, d);
        self.paint_road(px, d);
        self.paint_props(px, d);
        for c in &self.cars {
            if c.lane == 0 {
                self.paint_traffic(px, *c, d);
            }
        }
        for m in &self.crew.mates {
            let o = self.own.get(&m.id);
            if !o.is_some_and(|o| o.jump.is_some_and(|j| j.wait <= 0.0)) {
                self.paint_mate(px, m, o, d);
            }
        }
        if self.tall {
            self.paint_lot_cars(px, d);
        }
        if !self.jump.is_some_and(|j| j.wait <= 0.0) {
            self.paint_main(px, d);
        }
        for c in &self.cars {
            if c.lane == 1 {
                self.paint_traffic(px, *c, d);
            }
        }
        for m in &self.crew.mates {
            let o = self.own.get(&m.id);
            if o.is_some_and(|o| o.jump.is_some_and(|j| j.wait <= 0.0)) {
                self.paint_mate(px, m, o, d);
            }
        }
        if self.jump.is_some_and(|j| j.wait <= 0.0) {
            self.paint_main(px, d);
        }
        self.paint_puffs(px, d);
        self.paint_stars(px, d);
        if self.k_rain > 0.05 {
            self.paint_rain(px, d);
        }
    }
    fn paint_moon(&self, px: &mut Painter, d: PixelDials) {
        if d.night < 0.5 || self.k_rain > 0.5 {
            return;
        }
        let (mx, my) = moon_pixel(d.columns as f64, d.rows as f64);
        let r = moon_radius(self.tall);
        for y in inclusive((my - r).floor().max(0.0), my + r).take_while(|y| *y < self.far_walk) {
            for x in inclusive((mx - r * 2.0).floor(), mx + r * 2.0) {
                let k = moon_cover(x + 0.5, y + 0.5, mx, my, r);
                if k > 0.5 {
                    px.set(
                        x,
                        y,
                        mix(
                            self.sky(y, d),
                            MOON,
                            q(d.night, 4.0) * (1.0 - 0.5 * self.k_smoke),
                        ),
                        false,
                    );
                }
            }
        }
    }
    fn paint_clouds(&self, px: &mut Painter, d: PixelDials) {
        if !self.tall {
            return;
        }
        let k = q((1.0 - self.k_smoke.max(self.k_rain)) * (1.0 - d.night), 4.0);
        if k <= 0.0 {
            return;
        }
        let horizon = f64::from(i32_of(self.far_walk) >> 1);
        let drift = self.pos * 0.03 * 0.5 + d.t * 0.03;
        for r in span(0.0, horizon) {
            let y = (horizon - r) / 1.5 + 4.0;
            let top = self.sky(2.0 * r, d);
            let bot = self.sky(2.0 * r + 1.0, d);
            for col in span(0.0, f64::from(i32_of(self.w) >> 1)) {
                let x = (col + drift).floor() / 1.5;
                let Some(c) = clouds::layered(x, y, top) else {
                    continue;
                };
                let fg = clouds::snap(mix(top, c.fg, k), top);
                let bg = clouds::snap(mix(bot, c.bg, k), bot);
                px.set(2.0 * col, 2.0 * r, fg, false);
                px.set(2.0 * col + 1.0, 2.0 * r, fg, false);
                px.set(2.0 * col, 2.0 * r + 1.0, bg, false);
                px.set(2.0 * col + 1.0, 2.0 * r + 1.0, bg, false);
            }
        }
    }
    fn paint_far(&self, px: &mut Painter, d: PixelDials) {
        let pal = self.pal;
        let off = self.pos * F_FAR;
        let base = self.far_walk;
        for p in &self.fars {
            let x0 = round(p.a - off);
            if x0 > self.w || x0 + p.w < 0.0 {
                continue;
            }
            if p.kind <= TOWN {
                let top = round(base - p.h).max(0.0);
                let c = mix(pal.haze, self.sky(top, d), 0.3);
                for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                    for y in span(top, base) {
                        let lit = d.night > 0.5
                            && (i32_of(x) & 1) == 0
                            && (i32_of(y) & 1) == 1
                            && hash(p.seed + x - x0, y, 31.0) < 0.18;
                        px.set(x, y, if lit { mix(c, pal.hazeLit, 0.7) } else { c }, false);
                    }
                }
            } else {
                let c = if p.form != 0 {
                    pal.hillFar
                } else {
                    mix(pal.hill, pal.hillFar, 0.4)
                };
                for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                    let u = (x - x0) / p.w;
                    let h = p.h * (PI * u).sin() * (0.75 + 0.5 * noise1((x - x0) * 0.08, p.seed));
                    let top = even(round(base - h)).max(0.0);
                    for y in span(top, base) {
                        px.set(x, y, c, false);
                    }
                }
            }
        }
    }
    fn paint_mid(&self, px: &mut Painter, d: PixelDials) {
        let pal = self.pal;
        let off = round(self.pos * F_MID);
        let base = self.far_walk;
        let night = d.night > 0.5;
        let nk = q(d.night, 8.0);
        let tall = self.tall;
        for p in &self.mids {
            let x0 = p.a - off;
            if x0 > self.w || x0 + p.w < 0.0 {
                continue;
            }
            let top = even(round(base - p.h)).max(0.0);
            if p.kind == CITY || (p.kind == TOWN && !tall && p.h >= 3.0) {
                let i = p.c % TOWER.len();
                let body = self.tone(mix(TOWER[i], TOWER_N[i], nk), d);
                for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                    for y in span(top, base) {
                        let win = (x - x0) % 2.0 == 1.0
                            && y % 2.0 == 1.0
                            && x - x0 < p.w - 1.0
                            && y > top;
                        let lit =
                            win && hash(p.seed + x - x0, y, 7.0) < if night { 0.45 } else { 0.0 };
                        px.set(
                            x,
                            y,
                            if win {
                                if lit {
                                    pal.lit
                                } else {
                                    pal.glassWin
                                }
                            } else {
                                body
                            },
                            false,
                        );
                    }
                }
            } else if p.kind == TOWN {
                let i = p.c % SHOP.len();
                let wall = self.tone(mix(SHOP[i], SHOP_N[i], nk), d);
                let awn = self.tone(AWNING[p.c % AWNING.len()], d);
                let shop_top = base - if tall { 6.0 } else { 2.0 };
                for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                    for y in span(top, base) {
                        let u = x - x0;
                        let mut c = wall;
                        if y >= shop_top + if tall { 2.0 } else { 1.0 } && u > 0.0 && u < p.w - 1.0
                        {
                            c = if night && hash(p.seed, f64::from(i32_of(u) >> 2), 3.0) < 0.7 {
                                pal.shopLit
                            } else {
                                pal.shopWin
                            };
                        } else if y >= shop_top && y < shop_top + if tall { 2.0 } else { 1.0 } {
                            c = if tall && (i32_of(u) >> 1) % 2 != 0 {
                                mix(awn, 0xffffff, 0.6)
                            } else {
                                awn
                            };
                        } else if u % 3.0 == 1.0 && y % 2.0 == 1.0 && y > top && u < p.w - 1.0 {
                            c = if hash(p.seed + u, y, 9.0) < if night { 0.5 } else { 0.0 } {
                                pal.lit
                            } else {
                                pal.glassWin
                            };
                        }
                        px.set(x, y, c, false);
                    }
                }
            } else if p.kind == SUBURB {
                if p.form != 0 {
                    self.paint_tree(px, x0, p.w, base, p.h, pal);
                } else {
                    let i = p.c % HOUSE.len();
                    let wall = self.tone(mix(HOUSE[i], HOUSE_N[i], nk), d);
                    let roof =
                        self.tone(mix(ROOF[p.c % ROOF.len()], ROOF_N[p.c % ROOF.len()], nk), d);
                    let eave = base - round(p.h * 0.5);
                    let eave_y = even(eave);
                    for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                        let u = x - x0;
                        let edge = u.min(p.w - 1.0 - u);
                        let ridge = top.max(
                            eave_y
                                - (if tall { 6.0_f64 } else { 2.0 })
                                    .min((edge * if tall { 0.8 } else { 0.5 }).floor()),
                        );
                        for y in span(ridge, eave_y) {
                            px.set(x, y, roof, false);
                        }
                        for y in span(eave_y, base) {
                            let win = if tall {
                                u % 6.0 >= 2.0
                                    && u % 6.0 <= 3.0
                                    && y >= eave_y + 2.0
                                    && y < eave_y + 4.0
                            } else {
                                (u == 3.0 || u == p.w - 4.0) && y == eave_y
                            };
                            px.set(
                                x,
                                y,
                                if win {
                                    if night && hash(p.seed, f64::from(i32_of(u) >> 2), 5.0) < 0.6 {
                                        pal.lit
                                    } else {
                                        pal.glassWin
                                    }
                                } else {
                                    wall
                                },
                                false,
                            );
                        }
                    }
                }
            } else if p.form == 0 {
                for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                    for y in span(base - round(p.h).max(1.0), base) {
                        px.set(
                            x,
                            y,
                            if (x - x0) % if tall { 10.0 } else { 8.0 } == 0.0 {
                                pal.wallLine
                            } else {
                                pal.wall
                            },
                            false,
                        );
                    }
                }
            } else if p.form == 1 {
                self.paint_tree(px, x0, p.w, base, p.h, pal);
            } else {
                let face = self.tone(BILLBOARD[p.c % BILLBOARD.len()], d);
                let face_h = if tall { 8.0 } else { 2.0 };
                for x in span(x0.max(0.0), self.w.min(x0 + p.w)) {
                    for y in span(top, top + face_h) {
                        px.set(x, y, if night { mix(face, 0, 0.4) } else { face }, false);
                    }
                    if x - x0 == 2.0 || x - x0 == p.w - 3.0 {
                        for y in span(top + face_h, base) {
                            px.set(x, y, pal.pole, false);
                        }
                    }
                }
            }
        }
    }
    fn paint_tree(&self, px: &mut Painter, x0: f64, w: f64, base: f64, h: f64, pal: Pal) {
        let tall = self.tall;
        let trunk_h = if tall { 3.0 } else { 0.0 };
        let cy = base - trunk_h - h / 2.0;
        for x in span(x0.max(0.0), self.w.min(x0 + w)) {
            let u = (x - x0 + 0.5) / w - 0.5;
            let half = h / 2.0 * (1.0 - 4.0 * u * u).max(0.0).sqrt();
            let top = even(round(cy - half)).max(0.0);
            let bot = base.min(round(cy + half));
            for y in span(top, bot) {
                px.set(x, y, if u > 0.15 { pal.treeDark } else { pal.tree }, false);
            }
            if tall && u.abs() < 0.15 {
                for y in span(bot, base) {
                    px.set(x, y, pal.trunk, false);
                }
            }
        }
    }
    fn col(&self, x: f64) -> f64 {
        round(x - self.pos) + self.car_x
    }
    fn paint_road(&self, px: &mut Painter, d: PixelDials) {
        let pal = self.pal;
        let h = self.h;
        let tall = self.tall;
        let run = self.v;
        let left = self.pos - self.car_x;
        let mut si = 0;
        for x in span(0.0, self.w) {
            let wx = round(left) + x;
            while si < self.segs.len() - 1 && wx >= self.segs[si].b {
                si += 1;
            }
            let kind = self.segs[si].kind;
            for y in span(self.far_walk, self.far_wheel - if tall { 4.0 } else { 0.0 }) {
                let mut c = if kind == FREEWAY {
                    if y == self.far_walk {
                        pal.barrierTop
                    } else {
                        pal.barrier
                    }
                } else if kind == SUBURB {
                    if y == self.far_walk && tall {
                        pal.grass
                    } else {
                        pal.walk
                    }
                } else {
                    pal.walk
                };
                if kind == LOT && !tall {
                    c = pal.lot;
                }
                px.set(x, y, c, false);
            }
            for y in inclusive(
                self.far_wheel - if tall { 4.0 } else { 0.0 },
                self.near_wheel,
            ) {
                px.set(x, y, pal.asphalt, false);
            }
            let per = if kind == FREEWAY { 16.0 } else { 12.0 } * self.k;
            let on = per / 2.0;
            let mut cover = 0.0;
            let n = if run > 1.0 { 4 } else { 1 };
            for i in 0..n {
                if ((wx + i as f64 * run / n as f64) % per + per) % per < on {
                    cover += 1.0;
                }
            }
            let line = if kind == SUBURB { pal.yellow } else { pal.dash };
            let lc = mix(pal.asphalt, line, q(cover / n as f64, 3.0));
            for y in span(self.dash_y, self.dash_y + if tall { 2.0 } else { 1.0 }) {
                px.set(x, y, lc, false);
            }
            for y in span(self.kerb, if tall { self.kerb + 2.0 } else { h }) {
                px.set(
                    x,
                    y,
                    if kind == FREEWAY {
                        if y == self.kerb {
                            pal.dash
                        } else {
                            pal.asphalt
                        }
                    } else {
                        pal.kerb
                    },
                    false,
                );
            }
            if tall {
                for y in span(self.kerb + 2.0, h) {
                    let mut c = if kind == TOWN {
                        pal.walk
                    } else if kind == LOT {
                        pal.lot
                    } else {
                        pal.grass
                    };
                    if kind != TOWN && kind != LOT && (y + f64::from(i32_of(wx) >> 3)) % 7.0 == 0.0
                    {
                        c = pal.grassDark;
                    }
                    px.set(x, y, c, false);
                }
            }
        }
        for &s in &self.segs {
            if s.kind != LOT {
                continue;
            }
            let x0 = self.col(s.a);
            let x1 = self.col(s.b);
            if x1 < 0.0 || x0 > self.w {
                continue;
            }
            let top = if tall {
                self.kerb + 2.0
            } else {
                self.lot_wheel
            };
            let bot = if tall { h } else { self.far_walk + 1.0 };
            for x in span(x0.max(0.0), self.w.min(x1)) {
                for y in span(top, bot) {
                    px.set(x, y, pal.lot, false);
                }
            }
            for b in self.bays(s) {
                let lx = self.col(b - 1.0);
                for y in inclusive(top + if tall { 2.0 } else { 0.0 }, self.lot_wheel) {
                    px.set(lx, y, pal.lotLine, false);
                    if tall {
                        px.set(lx + self.bay_w, y, pal.lotLine, false);
                    }
                }
            }
        }
        if !tall {
            self.paint_lot_cars(px, d);
        }
        let beam = clamp01((d.night - 0.3) / 0.4).max(self.k_rain);
        if beam > 0.0 && self.in_lot < 0.5 && self.jump.is_none() {
            let nose = self.car_x + self.main().w;
            let len = 30.0 * self.k;
            let rows = if tall { 4.0 } else { 1.0 };
            for i in span(0.0, len) {
                let k = q(beam * 0.45 * (1.0 - i / len).powi(2), 4.0);
                for r in span(0.0, rows).take_while(|r| *r <= i * rows / (len * 0.4)) {
                    px.set(
                        nose + i,
                        self.near_wheel - r,
                        mix(px.get(nose + i, self.near_wheel - r), 0xfff0c8, k),
                        false,
                    );
                }
            }
        }
    }
    fn paint_lot_cars(&self, px: &mut Painter, d: PixelDials) {
        for &s in &self.segs {
            if s.kind != LOT {
                continue;
            }
            for b in self.bays(s) {
                if !self.bay_full(b) {
                    continue;
                }
                let x = self.col(b) + 1.0;
                if x > self.w || x + 24.0 < 0.0 {
                    continue;
                }
                let spr = self.near();
                let c = self.tone(
                    mix(
                        TRAFFIC[(hash1(b) * TRAFFIC.len() as f64).floor() as usize],
                        0x101014,
                        q(d.night, 4.0) * 0.5,
                    ),
                    d,
                );
                self.draw_car(
                    px,
                    spr,
                    x,
                    self.lot_wheel - spr.h + 1.0,
                    0.0,
                    self.livery(c, d, 0, 0.45),
                    false,
                    false,
                );
            }
        }
    }
    fn paint_props(&self, px: &mut Painter, d: PixelDials) {
        let pal = self.pal;
        let tall = self.tall;
        let k = self.k;
        let night = d.night > 0.5 || self.k_rain > 0.5;
        let base = self.far_walk;
        let smear = if self.v > 2.2 * k {
            round(self.v / k / 1.6).min(5.0)
        } else {
            0.0
        };
        let post = |px: &mut Painter, x: f64, top: f64, c: u32| {
            for y in span(top, base + if tall { 1.0 } else { 0.0 }) {
                px.set(x, y, c, false);
                for i in inclusive(1.0, smear) {
                    px.set(
                        x + i,
                        y,
                        mix(px.get(x + i, y), c, q(0.6 / (i + 0.5), 3.0)),
                        false,
                    );
                }
            }
        };
        for s in &self.segs {
            let x0 = self.col(s.a);
            let x1 = self.col(s.b);
            if x1 < -20.0 || x0 > self.w + 20.0 {
                continue;
            }
            if s.kind == TOWN || s.kind == LOT {
                let mut wx = s.a + 10.0 * k;
                while wx < s.b - 4.0 {
                    let x = even(self.col(wx));
                    let top = if tall { base - 26.0 } else { 1.0 };
                    post(px, x, top, pal.pole);
                    px.set(x + 1.0, top, pal.pole, false);
                    px.set(
                        x + 2.0,
                        top,
                        if night { pal.lamp } else { pal.lampHead },
                        false,
                    );
                    if night {
                        if d.wait > 0.0 {
                            px.lamp(x + 2.0, top, pal.lamp);
                        }
                        for y in span(base, self.far_wheel - if tall { 4.0 } else { 0.0 }) {
                            for i in inclusive(-1.0, 4.0) {
                                px.set(x + i, y, pal.pool, false);
                            }
                        }
                    }
                    wx += 40.0 * k;
                }
                if s.kind == TOWN {
                    let x = even(self.col(s.a + 2.0 * k));
                    let top = if tall { base - 20.0 } else { 2.0 };
                    post(px, x, top + if tall { 4.0 } else { 1.0 }, pal.pole);
                    for y in span(top, top + if tall { 4.0 } else { 1.0 }) {
                        px.set(x, y, 0x1a1c1e, false);
                    }
                    px.set(x, top + if tall { 3.0 } else { 0.0 }, pal.green, false);
                } else {
                    let x = even(self.col(s.a + 4.0 * k));
                    let top = if tall { base - 16.0 } else { 0.0 };
                    let sz = if tall { 4.0 } else { 2.0 };
                    post(px, x + 1.0, top + sz, pal.pole);
                    for y in span(top, top + sz) {
                        for i in span(0.0, sz) {
                            px.set(x + i, y, pal.pBlue, false);
                        }
                    }
                    if tall {
                        px.set(x + 1.0, top + 1.0, 0xffffff, false);
                    }
                }
            } else if s.kind == SUBURB {
                let gap = 56.0 * k;
                let mut wx = s.a + 8.0 * k;
                while wx < s.b - 4.0 {
                    let x = even(self.col(wx));
                    let top = if tall { base - 30.0 } else { 0.0 };
                    post(px, x, top, pal.trunk);
                    px.set(x - 1.0, top + 1.0, pal.trunk, false);
                    px.set(x + 1.0, top + 1.0, pal.trunk, false);
                    if tall {
                        for i in span(1.0, gap) {
                            let u = i / gap;
                            px.set(
                                x + i,
                                top + 1.0 + round(4.0 * u * (1.0 - u) * 3.0),
                                mix(px.get(x + i, top + 2.0), 0x202022, 0.7),
                                false,
                            );
                        }
                    }
                    wx += gap;
                }
            } else if s.kind == FREEWAY {
                let mut wx = s.a + 20.0 * k;
                while wx < s.b - 4.0 {
                    let x = even(self.col(wx));
                    let top = if tall { base - 34.0 } else { 0.0 };
                    post(px, x, top, pal.pole);
                    for i in inclusive(1.0, if tall { 6.0 } else { 3.0 }) {
                        px.set(x + i, top, pal.pole, false);
                    }
                    px.set(
                        x + if tall { 6.0 } else { 3.0 },
                        top + if tall { 1.0 } else { 0.0 },
                        if night { pal.lamp } else { pal.lampHead },
                        false,
                    );
                    wx += 72.0 * k;
                }
                if s.b - s.a > 300.0 * k {
                    let x = even(self.col(s.a + 120.0 * k));
                    let sw = even(round(18.0 * k));
                    let top = if tall { base - 40.0 } else { 0.0 };
                    let sh = if tall { 8.0 } else { 2.0 };
                    post(px, x, top, pal.pole);
                    post(px, x + sw - 1.0, top, pal.pole);
                    for i in span(0.0, sw) {
                        for y in span(top, top + sh) {
                            let text = if tall {
                                y > top + 1.0
                                    && y < top + sh - 2.0
                                    && i > 1.0
                                    && i < sw - 2.0
                                    && hash(i, y, s.seed) < 0.5
                            } else {
                                y == top && i > 1.0 && i < sw - 2.0 && i % 3.0 != 0.0
                            };
                            px.set(x + i, y, if text { pal.signText } else { pal.sign }, false);
                        }
                    }
                }
            }
        }
    }
    fn livery(&self, c: u32, d: PixelDials, lights: i32, dusk: f64) -> Livery {
        let n = q(d.night, 4.0);
        let body = self.tone(mix(c, mix(c, 0x0a0c12, dusk), n), d);
        let lit = lights > 0;
        Livery {
            B: body,
            b: mix(body, 0, 0.22),
            G: GLASS,
            T: if lights == 2 {
                HAZARD
            } else if lit || n > 0.5 {
                TAIL
            } else {
                mix(TAIL, body, 0.4)
            },
            L: if lights == 2 {
                HAZARD
            } else if lit {
                HEAD
            } else {
                mix(0xd8dce0, body, 0.3)
            },
            W: TYRE,
            R: if self.tall { RIM } else { TYRE },
            X: body,
            d: mix(body, 0, 0.4),
        }
    }
    #[allow(clippy::too_many_arguments)]
    fn draw_car(
        &self,
        px: &mut Painter,
        s: Sprite,
        x: f64,
        top: f64,
        ang: f64,
        l: Livery,
        lamps: bool,
        roll: bool,
    ) {
        if ang == 0.0 {
            for (j, row) in s.rows.iter().enumerate() {
                for (i, &ch) in row.iter().enumerate() {
                    let Some(c) = l.color(ch) else { continue };
                    if lamps && (ch == b'T' || ch == b'L') {
                        px.lamp(x + i as f64, top + j as f64, c);
                    } else {
                        px.set(x + i as f64, top + j as f64, c, true);
                    }
                }
            }
            return;
        }
        if roll {
            self.draw_roll(px, s, x, top, ang, l);
            return;
        }
        let cx = x + s.w / 2.0;
        let cy = top + s.h / 2.0;
        let cos = ang.cos();
        let sin = ang.sin();
        let r = hypot(s.w / 2.0, s.h);
        for sy in inclusive((cy - r / 2.0 - 1.0).floor(), cy + r / 2.0 + 1.0) {
            for sx in inclusive((cx - r - 1.0).floor(), cx + r + 1.0) {
                let x1 = sx + 0.5 - cx;
                let y1 = (sy + 0.5 - cy) * 2.0;
                let xx = x1 * cos + y1 * sin;
                let yy = -x1 * sin + y1 * cos;
                let i = (xx + s.w / 2.0).floor();
                let j = (yy / 2.0 + s.h / 2.0).floor();
                if i < 0.0 || j < 0.0 || i >= s.w || j >= s.h {
                    continue;
                }
                if let Some(c) = l.color(s.rows[j as usize][i as usize]) {
                    px.set(sx, sy, c, true);
                }
            }
        }
    }
    fn draw_roll(&self, px: &mut Painter, s: Sprite, x: f64, top: f64, ang: f64, l: Livery) {
        let h = s.h;
        let depth = h;
        let cy = top + h / 2.0;
        let cos = ang.cos();
        let sin = ang.sin();
        let r = (hypot(h, depth) / 2.0).ceil() + 1.0;
        for i in 0..s.w as usize {
            let (mut jt, mut jb) = (-1i32, -1i32);
            let mut tyre = false;
            for j in 0..h as usize {
                let ch = s.rows[j][i];
                if l.color(ch).is_none() {
                    continue;
                }
                if jt < 0 {
                    jt = j as i32;
                }
                jb = j as i32;
                tyre |= ch == b'W';
            }
            if jt < 0 {
                continue;
            }
            let yt = h / 2.0 - f64::from(jt);
            let yb = h / 2.0 - f64::from(jb) - 1.0;
            let roof = l.color(s.rows[jt as usize][i]).unwrap();
            for sy in inclusive((cy - r).floor(), cy + r) {
                let y = cy - (sy + 0.5);
                let (mut lo, mut hi) = (f64::NEG_INFINITY, f64::INFINITY);
                let mut face = 0;
                if sin.abs() > 1e-6 {
                    let a = (yb - y * cos) / sin;
                    let b = (yt - y * cos) / sin;
                    lo = a.min(b);
                    hi = a.max(b);
                    face = 1;
                } else if y * cos < yb || y * cos > yt {
                    continue;
                }
                if cos.abs() > 1e-6 {
                    let a = (-depth / 2.0 + y * sin) / cos;
                    let b = (depth / 2.0 + y * sin) / cos;
                    lo = lo.max(a.min(b));
                    if a.max(b) < hi {
                        hi = a.max(b);
                        face = 2;
                    }
                } else if (y * sin).abs() > depth / 2.0 {
                    continue;
                }
                if lo > hi {
                    continue;
                }
                let c = if face == 2 {
                    let yy = y * cos + hi * sin;
                    let j = (h / 2.0 - yy).floor().clamp(f64::from(jt), f64::from(jb)) as usize;
                    l.color(s.rows[j][i]).unwrap_or(l.b)
                } else if y * cos + hi * sin > (yt + yb) / 2.0 {
                    roof
                } else if tyre && (-y * sin + hi * cos).abs() > depth / 2.0 - 1.0 {
                    l.W
                } else {
                    l.d
                };
                px.set(x + i as f64, sy, c, true);
            }
        }
    }
    fn paint_main(&self, px: &mut Painter, d: PixelDials) {
        let s = self.main();
        let top = self.wheel_y() - s.h + 1.0 - Self::lift(self.jump.as_ref());
        let lights = i32::from(d.night > 0.3 || self.k_rain > 0.3 || d.wait > 0.0);
        self.draw_car(
            px,
            s,
            self.car_x,
            top,
            Self::turn(self.jump.as_ref()),
            self.livery(PEARL, d, lights, 0.12),
            d.wait > 0.0,
            self.jump.is_some_and(|j| j.roll),
        );
    }
    fn paint_mate(&self, px: &mut Painter, m: &Mate, o: Option<&Own>, d: PixelDials) {
        if m.x.is_nan() {
            return;
        }
        let s = ROADSTER;
        let x = round(self.car_x + m.x);
        if x > self.w || x + s.w < 0.0 {
            return;
        }
        let jump = o.and_then(|o| o.jump.as_ref());
        let top = self.far_wheel - s.h + 1.0 - Self::lift(jump);
        let lights = if m.waiting && !m.leaving {
            if beckon(m, d.t) {
                2
            } else {
                0
            }
        } else if m.busy >= 0.5 || m.leaving || m.here < 1.0 {
            1
        } else {
            0
        };
        self.draw_car(
            px,
            s,
            x,
            top,
            Self::turn(jump),
            self.livery(LIVERY[m.slot % LIVERY.len()], d, lights, 0.45),
            false,
            jump.is_some_and(|j| j.roll),
        );
    }
    fn paint_traffic(&self, px: &mut Painter, c: Car, d: PixelDials) {
        let near = c.lane == 1;
        let s = if c.truck {
            TRUCK
        } else if near {
            self.near()
        } else {
            SEDAN
        };
        let x = self.col(c.x);
        if x > self.w || x + s.w < 0.0 {
            return;
        }
        let wheel = if near {
            self.near_wheel
        } else {
            self.far_wheel
        };
        let lit = i32::from(d.night > 0.3 || self.k_rain > 0.3);
        let mut l = self.livery(c.c, d, lit, 0.45);
        if c.truck {
            l.X = self.tone(mix(0xd8d8d4, 0x2a2a2a, q(d.night, 4.0) * 0.6), d);
        }
        self.draw_car(px, s, x, wheel - s.h + 1.0, 0.0, l, false, false);
    }
    fn paint_puffs(&self, px: &mut Painter, d: PixelDials) {
        for p in &self.puffs {
            let x = self.col(p.x);
            let r = p.r;
            let a = q(p.life * if p.c == VAPOUR { 0.9 } else { 0.75 }, 4.0);
            if a <= 0.0 {
                continue;
            }
            let c = if p.c == SMOKE {
                self.tone(mix(SMOKE, 0x5a5a5e, q(d.night, 2.0) * 0.5), d)
            } else {
                p.c
            };
            for y in inclusive((p.y - r / 2.0).floor(), p.y + r / 2.0) {
                for xx in inclusive((x - r).floor(), x + r) {
                    let dd =
                        ((xx - x) / r.max(0.5)).powi(2) + ((y - p.y) / (r / 2.0).max(0.5)).powi(2);
                    if dd > 1.0 {
                        continue;
                    }
                    px.set(xx, y, mix(px.get(xx, y), c, a), false);
                }
            }
        }
    }
    fn paint_stars(&self, px: &mut Painter, d: PixelDials) {
        if d.night < 0.5 || self.k_smoke > 0.3 || self.k_rain > 0.3 {
            return;
        }
        for r in 0..i32_of(self.far_walk) >> 1 {
            for c in 0..d.columns {
                let (r, c) = (f64::from(r), c as f64);
                let h = hash(c, r, 1977.0);
                if h > 0.06 {
                    continue;
                }
                let sky = self.sky(r * 2.0, d);
                if px.get(c * 2.0, r * 2.0) != sky
                    || px.get(c * 2.0 + 1.0, r * 2.0 + 1.0) != sky
                    || px.get(c * 2.0 + 1.0, r * 2.0) != sky
                {
                    continue;
                }
                px.dot(
                    c * 2.0 + if h < 0.03 { 0.0 } else { 1.0 },
                    r * 4.0 + (h * 60.0).floor() % 4.0,
                    if h < 0.02 { STAR } else { STAR_DIM },
                );
            }
        }
    }
    fn paint_rain(&self, px: &mut Painter, d: PixelDials) {
        let n = round(px.dw as f64 * px.dh as f64 * 0.025 * self.k_rain) as usize;
        let slant = 1.0 + self.v / self.k / 3.0;
        for i in 0..n {
            let x0 = hash1((i * 3 + 1) as f64) * px.dw as f64;
            let y0 = hash1((i * 3 + 2) as f64) * px.dh as f64;
            let fall = d.t * 3.0;
            let y = (y0 + fall) % px.dh as f64;
            let x = ((x0 - fall * slant * 0.5) % px.dw as f64 + px.dw as f64) % px.dw as f64;
            px.dot(x, y, self.tone(RAIN, d));
        }
    }
}
