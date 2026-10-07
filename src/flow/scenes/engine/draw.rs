//! The canvases and everything drawn on them: solids on the quadrant grid,
//! moving parts as braille dots, steam as a dithered density field, then
//! all of it folded into cells.

use std::f64::consts::{PI, TAU};

use super::{
    c, fire_color, frac, level_of, Belt, Engine, Gauge, Gear, Kind, Module, Pulley, BAYER,
    BLUE_FIRE, BRAILLE_BASE, FIRE, MAX_SPEED, SOLID, SPOKES,
};
use crate::flow::cells::DEFAULT_COLOR;
use crate::flow::js::{i32_of, round};
use crate::flow::pixels::{clamp01, dist, mix, BRAILLE, QUAD};
use crate::flow::scene::Tint;

/// Whether ordered dither at (x, y) lights a dot of density p.
fn dither(x: i32, y: i32, p: f64) -> bool {
    p > (BAYER[(((y & 3) << 2) | (x & 3)) as usize] + 0.5) / 16.0
}

impl Engine {
    // ---- canvases -------------------------------------------------------------

    /// A solid quadrant pixel (qx in dots, qy in half-rows).
    fn q(&mut self, qx: f64, qy: f64, color: u32) {
        let x = qx.floor();
        let y = qy.floor();
        let qw = (self.columns * 2) as f64;
        if !(x >= 0.0 && y >= 0.0 && x < qw && y < (self.rows * 2) as f64) {
            return;
        }
        self.quad[(y * qw + x) as usize] = (color & 0xff_ffff) + SOLID;
    }

    fn qrect(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: u32) {
        let mut y = y0.floor();
        while y <= y1 {
            let mut x = x0.floor();
            while x <= x1 {
                self.q(x, y, color);
                x += 1.0;
            }
            y += 1.0;
        }
    }

    /// A braille dot (x, y in dots); the cell takes the color of its highest-priority dot.
    fn dot(&mut self, x: f64, y: f64, color: u32, pri: u8) {
        let dx = x.floor();
        let dy = y.floor();
        if !(dx >= 0.0
            && dy >= 0.0
            && dx < (self.columns * 2) as f64
            && dy < (self.rows * 4) as f64)
        {
            return;
        }
        let (dx, dy) = (dx as usize, dy as usize);
        let c = (dy >> 2) * self.columns + (dx >> 1);
        self.bits[c] |= BRAILLE[dx & 1][dy & 3] as u8;
        if pri >= self.bpri[c] {
            self.bpri[c] = pri;
            self.bcol[c] = color;
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn line(&mut self, x0: f64, y0: f64, x1: f64, y1: f64, color: u32, pri: u8) {
        let n = ((x1 - x0).abs().max((y1 - y0).abs()) * 1.5).ceil() + 1.0;
        let mut k = 0.0;
        while k <= n {
            let f = k / n;
            self.dot(x0 + (x1 - x0) * f, y0 + (y1 - y0) * f, color, pri);
            k += 1.0;
        }
    }

    /// Map a point in the engine's frame (s along the stroke, t across it) to dots.
    fn fx(&self, s: f64, t: f64) -> f64 {
        self.cx + s * self.ux + t * self.vx
    }

    fn fy(&self, s: f64, t: f64) -> f64 {
        self.cy + s * self.uy + t * self.vy
    }

    // ---- drawing ------------------------------------------------------------------

    pub(super) fn draw(&mut self) {
        let n = self.columns * self.rows;
        let level = level_of(self.dials.strength);
        if level <= 0.0 || n == 0 {
            for i in 0..n {
                self.out.blank(i);
            }
            return;
        }
        self.quad.fill(0);
        self.bits.fill(0);
        self.bpri.fill(0);
        self.spark.fill(0);
        self.steam.fill(0.0);

        self.draw_steam();
        self.draw_shaft();
        for i in 0..self.gears.len() {
            let g = self.gears[i];
            self.draw_gear(&g);
        }
        for i in 0..self.pulleys.len() {
            let p = self.pulleys[i];
            self.draw_pulley(&p);
        }
        for i in 0..self.modules.len() {
            let m = self.modules[i];
            self.draw_module(&m);
        }
        for i in 0..self.belts.len() {
            let b = self.belts[i];
            self.draw_belt(&b);
        }
        self.draw_wheel();
        self.draw_linkage();
        self.draw_governor();
        if self.vertical {
            self.draw_boiler_spine();
        } else {
            self.draw_boiler_band();
        }
        self.draw_bed();
        self.draw_sparks();
        self.compose();
        for i in 0..self.gauges.len() {
            let g = self.gauges[i];
            self.draw_gauge(&g);
        }
    }

    /// The line shaft: a twisted stripe that races along it as it turns.
    fn draw_shaft(&mut self) {
        for i in 0..self.shafts.len() {
            let sh = self.shafts[i];
            let sy = sh.y.floor();
            let v = (sh.s * self.omega * 1.2).abs();
            let off = (sh.s * self.angle * 1.2).floor();
            let blur = v > 1.3;
            let mut x = sh.x0.floor();
            while x < sh.x1 {
                if blur {
                    self.dot(x, sy, c::STEEL, 1);
                    self.dot(x, sy + 1.0, c::IRON_LT, 1);
                } else {
                    let y = if (i32_of(x + off) & 3) < 2 {
                        sy
                    } else {
                        sy + 1.0
                    };
                    self.dot(x, y, c::STEEL, 1);
                }
                x += 1.0;
            }
        }
        let y = self.shaft_y.floor();
        for i in 0..self.hangers.len() {
            let hx = self.hangers[i];
            self.line(hx, y - 2.0, hx, y - 0.5, c::IRON, 1);
            self.dot(hx - 1.0, y - 2.0, c::IRON, 1);
            self.dot(hx + 1.0, y - 2.0, c::IRON, 1);
        }
    }

    fn draw_gear(&mut self, g: &Gear) {
        let a = g.phase + g.s * self.angle;
        let w = (g.s * self.omega).abs();
        let blur_t = clamp01(((w * g.n) / TAU - 0.4) / 0.5);
        let blur_s = clamp01((w - 0.3) / 0.4);
        let tip = g.r + 0.95;
        let spokes: f64 = if g.r >= 4.0 { 3.0 } else { 0.0 };
        let spacing = TAU / spokes.max(1.0);
        let dark = mix(g.color, 0x202020, 0.35);
        let x0 = i32_of((g.x - tip).floor());
        let x1 = i32_of((g.x + tip).ceil());
        let y0 = i32_of((g.y - tip).floor());
        let y1 = i32_of((g.y + tip).ceil());
        for y in y0..=y1 {
            let fy = f64::from(y);
            let dy = fy + 0.5 - g.y;
            for x in x0..=x1 {
                let fx = f64::from(x);
                let dx = fx + 0.5 - g.x;
                let d = (dx * dx + dy * dy).sqrt();
                if d > tip || (g.rim && d <= g.r) {
                    continue;
                }
                if d <= 0.9 {
                    self.dot(fx, fy, c::BRASS_HI, 3);
                    continue;
                }
                if d > g.r {
                    // Teeth: narrow, so the gaps read and the neighbours' teeth slot in.
                    let m = frac((g.n * (dy.atan2(dx) - a)) / TAU + 0.2);
                    let p = (if m < 0.4 { 1.0 } else { 0.0 }) * (1.0 - blur_t) + 0.4 * blur_t;
                    if dither(x, y, p) {
                        self.dot(fx, fy, g.color, 2);
                    }
                    continue;
                }
                if d > g.r - 1.0 {
                    self.dot(fx, fy, g.color, 2);
                    continue;
                }
                if spokes == 0.0 {
                    continue;
                }
                let mut m = (dy.atan2(dx) - a) % spacing;
                if m < 0.0 {
                    m += spacing;
                }
                let near = m.min(spacing - m) * d;
                let p = (if near < 0.5 { 1.0 } else { 0.0 }) * (1.0 - blur_s) + 0.3 * blur_s;
                if dither(x, y, p) {
                    self.dot(fx, fy, dark, 1);
                }
            }
        }
    }

    fn draw_pulley(&mut self, p: &Pulley) {
        let a = p.phase + p.s * self.angle;
        let w = (p.s * self.omega).abs();
        let blur = clamp01((w - 0.3) / 0.4);
        let spacing = TAU / p.spokes.max(1.0);
        let x0 = i32_of((p.x - p.r).floor());
        let x1 = i32_of((p.x + p.r).ceil());
        let y0 = i32_of((p.y - p.r).floor());
        let y1 = i32_of((p.y + p.r).ceil());
        for y in y0..=y1 {
            let fy = f64::from(y);
            let dy = fy + 0.5 - p.y;
            for x in x0..=x1 {
                let fx = f64::from(x);
                let dx = fx + 0.5 - p.x;
                let d = (dx * dx + dy * dy).sqrt();
                if d > p.r {
                    continue;
                }
                if d > p.r - 0.95 || d <= 0.75 {
                    self.dot(fx, fy, p.color, 3);
                    continue;
                }
                if p.spokes == 0.0 {
                    continue;
                }
                let mut m = (dy.atan2(dx) - a) % spacing;
                if m < 0.0 {
                    m += spacing;
                }
                let near = m.min(spacing - m) * d;
                let pr = (if near < 0.6 { 1.0 } else { 0.0 }) * (1.0 - blur) + 0.35 * blur;
                if dither(x, y, pr) {
                    self.dot(fx, fy, c::IRON, 2);
                }
            }
        }
        // A single bright mark on small pulleys so they visibly turn.
        if p.spokes == 0.0 && p.r >= 1.4 && w < 0.5 {
            self.dot(
                p.x + a.cos() * (p.r - 0.5),
                p.y + a.sin() * (p.r - 0.5),
                c::STEEL,
                4,
            );
        }
    }

    /// Two runs of leather with a stitch pattern travelling along them.
    fn draw_belt(&mut self, b: &Belt) {
        let ddx = b.x2 - b.x1;
        let ddy = b.y2 - b.y1;
        let len = (ddx * ddx + ddy * ddy).sqrt();
        if len < 1.0 {
            return;
        }
        let dx = ddx / len;
        let dy = ddy / len;
        let nx = -dy;
        let ny = dx;
        let travel = b.v * self.angle;
        let speed = (b.v * self.omega).abs();
        let blur = speed > 1.5;
        for side in [-1.0, 1.0] {
            let ax = b.x1 + nx * b.r1 * side;
            let ay = b.y1 + ny * b.r1 * side;
            let s2 = if b.crossed { -side } else { side };
            let bx = b.x2 + nx * b.r2 * s2;
            let by = b.y2 + ny * b.r2 * s2;
            let lx = bx - ax;
            let ly = by - ay;
            let l = (lx * lx + ly * ly).sqrt();
            let steps = l.ceil();
            // The +n run moves toward pulley 1, the −n run away from it.
            let off = if side > 0.0 { travel } else { -travel };
            let mut k = 0.0;
            while k <= steps {
                let f = k / steps.max(1.0);
                let ph = (((k + off) % 5.0) + 5.0) % 5.0;
                let x = ax + lx * f;
                let y = ay + ly * f;
                if blur {
                    let color = if i32_of(k) & 1 == 0 {
                        c::LEATHER_HI
                    } else {
                        c::LEATHER
                    };
                    self.dot(x, y, color, 3);
                } else if ph < 3.4 {
                    self.dot(x, y, if ph < 1.0 { c::LEATHER_HI } else { c::LEATHER }, 3);
                }
                k += 1.0;
            }
        }
    }

    fn draw_module(&mut self, m: &Module) {
        let x = m.x;
        let ey = self.ey;
        let qy0 = (ey / 2.0).floor();
        match m.kind {
            Kind::Bell => {
                // A snail cam lifts a striker rod; at the top of each lift it rings the bell.
                let a = m.phase + m.s * self.angle;
                let mut yy = (m.y - 3.0).floor();
                while yy <= m.y + 3.0 {
                    let mut xx = (m.x - 3.0).floor();
                    while xx <= m.x + 3.0 {
                        let dx = xx + 0.5 - m.x;
                        let dy = yy + 0.5 - m.y;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d <= 2.9 {
                            let rr = 1.5 + 1.3 * (1.0 - frac((2.0 * (dy.atan2(dx) - a)) / TAU));
                            if d <= rr {
                                if d < 1.0 {
                                    self.dot(xx, yy, c::BRASS_HI, 4);
                                } else {
                                    self.dot(xx, yy, c::IRON, 2);
                                }
                            }
                        }
                        xx += 1.0;
                    }
                    yy += 1.0;
                }
                let lift = self.cam_lift(m) * (1.3 / 1.8);
                let top = m.y - 1.5 - lift - 9.0;
                self.line(m.x, m.y - 2.0 - lift, m.x, top, c::STEEL, 4);
                self.dot(m.x - 1.0, top, c::IRON, 4);
                self.dot(m.x + 1.0, top, c::IRON, 4);
                // Guide bracket from the pillar.
                self.line(1.0, m.y - 6.0, m.x - 1.0, m.y - 6.0, c::IRON, 1);
                let ring = lift > 1.05 && self.omega > 0.02;
                let bq = ((m.y - 1.5 - 1.3 - 9.0 - 3.0) / 2.0).floor();
                let bx = m.x.floor();
                self.qrect(
                    bx - 1.0,
                    bq - 1.0,
                    bx,
                    bq - 1.0,
                    if ring { c::BRASS_HI } else { c::BRASS },
                );
                self.qrect(
                    bx - 2.0,
                    bq,
                    bx + 1.0,
                    bq,
                    if ring { c::BRASS_HI } else { c::BRASS_DK },
                );
                self.line(bx - 0.5, bq * 2.0 - 2.0, 1.0, bq * 2.0 - 5.0, c::IRON, 1);
                if ring {
                    self.dot(bx - 4.0, bq * 2.0 + 1.0, c::BRASS_HI, 3);
                    self.dot(bx + 3.0, bq * 2.0 + 1.0, c::BRASS_HI, 3);
                    self.dot(bx - 5.0, bq * 2.0, c::BRASS_DK, 3);
                    self.dot(bx + 4.0, bq * 2.0, c::BRASS_DK, 3);
                }
            }
            Kind::Hammer => {
                // A snail cam lifts the helve, then lets the hammer fall on the anvil.
                let a = m.phase + m.s * self.angle;
                let ccx = x + 16.0;
                let ccy = ey + 13.1;
                let mut yy = (ccy - 4.0).floor();
                while yy <= ccy + 4.0 {
                    let mut xx = (ccx - 4.0).floor();
                    while xx <= ccx + 4.0 {
                        let dx = xx + 0.5 - ccx;
                        let dy = yy + 0.5 - ccy;
                        let d = (dx * dx + dy * dy).sqrt();
                        if d <= 3.9 {
                            let rr = 2.0 + 1.8 * (1.0 - frac((2.0 * (dy.atan2(dx) - a)) / TAU));
                            if d <= rr {
                                if d < 1.1 {
                                    self.dot(xx, yy, c::BRASS_HI, 4);
                                } else {
                                    self.dot(xx, yy, c::IRON, 2);
                                }
                            }
                        }
                        xx += 1.0;
                    }
                    yy += 1.0;
                }
                let lift = self.cam_lift(m);
                let pxv = x + 6.0;
                let pyv = ey + 9.0;
                let contact_y = ccy - 2.0 - lift - 0.5;
                let k = (contact_y - pyv) / 10.0;
                let hx = x + 28.0;
                let hy = pyv + k * 22.0;
                self.line(pxv, pyv, hx, hy, c::WOOD, 3);
                self.line(pxv, pyv + 1.0, hx, hy + 1.0, c::WOOD, 3);
                // Post, hammer head, anvil.
                self.qrect(x + 5.0, qy0 + 5.0, x + 6.0, qy0 + 8.0, c::IRON_DK);
                self.q(x + 5.0, qy0 + 4.0, c::BRASS);
                let hq = ((hy + 1.0) / 2.0).floor();
                self.qrect(hx - 2.0, hq, hx + 1.0, hq + 1.0, c::IRON);
                self.qrect(x + 25.0, qy0 + 8.0, x + 31.0, qy0 + 8.0, c::IRON_DK);
            }
            Kind::Stack => {
                let mut qy = qy0 + 1.0;
                while qy <= qy0 + 8.0 {
                    let mut qx = x + 3.0;
                    while qx <= x + 8.0 {
                        let brick = if ((i32_of(qx) + (i32_of(qy) & 1) * 2) & 3) == 0 {
                            c::BRICK_DK
                        } else {
                            c::BRICK
                        };
                        self.q(qx, qy, if qy == qy0 + 1.0 { c::BRASS } else { brick });
                        qx += 1.0;
                    }
                    qy += 1.0;
                }
                let fire = self.fire_at(x + 5.0, 7.0);
                self.qrect(x + 5.0, qy0 + 7.0, x + 6.0, qy0 + 8.0, fire);
            }
            Kind::Beam => {
                // Crank disc → rod → rocking beam on its column → pump rod.
                let a = m.phase + m.s * self.angle;
                let pin_x = x + 6.0 + a.cos() * 2.4;
                let pin_y = ey + 13.0 + a.sin() * 2.4;
                let piv_x = x + 22.0;
                let piv_y = ey + 5.0;
                let h = 15.0;
                let end_x = piv_x - h;
                let rod = 7.0;
                let ddx = pin_x - end_x;
                let y_l = pin_y - (rod * rod - ddx * ddx).max(0.0).sqrt();
                let sn = ((y_l - piv_y) / h).clamp(-0.6, 0.6);
                let cs = (1.0 - sn * sn).sqrt();
                let lx = piv_x - h * cs;
                let ly = piv_y + h * sn;
                let rx = piv_x + h * cs;
                let ry = piv_y - h * sn;
                self.line(lx, ly, rx, ry, c::IRON, 3);
                self.line(lx, ly + 1.0, rx, ry + 1.0, c::IRON_LT, 3);
                self.line(pin_x, pin_y, lx, ly + 1.0, c::STEEL, 4);
                self.dot(pin_x, pin_y, c::BRASS_HI, 5);
                self.line(rx, ry + 1.0, rx, ey + 12.0, c::STEEL, 4);
                self.qrect(x + 21.0, qy0 + 3.0, x + 22.0, qy0 + 8.0, c::IRON);
                self.q(x + 21.0, qy0 + 2.0, c::BRASS);
                self.q(x + 22.0, qy0 + 2.0, c::BRASS);
                let pc = rx.floor();
                self.qrect(pc - 2.0, qy0 + 6.0, pc + 2.0, qy0 + 8.0, c::COPPER);
                self.qrect(pc - 3.0, qy0 + 6.0, pc + 3.0, qy0 + 6.0, c::BRASS_DK);
                self.qrect(pc + 3.0, qy0 + 7.0, x + m.w - 1.0, qy0 + 7.0, c::COPPER_DK);
            }
            Kind::Grind => {
                let a = m.phase + m.s * self.angle;
                let gx = x + 13.0;
                let gy = ey + 11.0;
                let blur = clamp01(((m.s * self.omega).abs() - 0.25) / 0.4);
                let mut yy = (gy - 6.0).floor();
                while yy <= gy + 6.0 {
                    let mut xx = (gx - 6.0).floor();
                    while xx <= gx + 6.0 {
                        let dx = xx + 0.5 - gx;
                        let dy = yy + 0.5 - gy;
                        let d = (dx * dx + dy * dy).sqrt();
                        if (2.4..=5.8).contains(&d) {
                            let ring = (d * 0.9).floor()
                                + (((dy.atan2(dx) - a) / TAU) * 14.0 + 14.0).floor();
                            let grain = (i32_of(ring) & 1) == 0;
                            let p = if d > 5.0 {
                                1.0
                            } else {
                                (if grain { 0.9 } else { 0.15 }) * (1.0 - blur) + 0.5 * blur
                            };
                            if dither(i32_of(xx), i32_of(yy), p) {
                                self.dot(xx, yy, if d > 5.0 { c::STONE } else { c::STONE_DK }, 2);
                            }
                        }
                        xx += 1.0;
                    }
                    yy += 1.0;
                }
                self.draw_pulley_at(gx, gy, 2.0, m.s);
                self.qrect(x + 7.0, qy0 + 8.0, x + 19.0, qy0 + 8.0, c::WOOD);
                self.qrect(x + 19.0, qy0 + 5.0, x + 20.0, qy0 + 5.0, c::IRON_LT);
                self.qrect(x + 20.0, qy0 + 6.0, x + 20.0, qy0 + 8.0, c::IRON_DK);
            }
            Kind::Panel | Kind::Pipe => {
                // Pipework: a manifold with valves, handwheels and (on panels) gauges.
                let x1 = x + m.w - 1.0;
                self.qrect(x, qy0 + 6.0, x1, qy0 + 6.0, c::COPPER);
                if m.kind == Kind::Panel {
                    self.qrect(x + 2.0, qy0 + 2.0, x + 3.0, qy0 + 5.0, c::COPPER_DK);
                    self.qrect(
                        x + m.w - 5.0,
                        qy0 + 3.0,
                        x + m.w - 4.0,
                        qy0 + 5.0,
                        c::COPPER_DK,
                    );
                    self.qrect(x + 1.0, qy0 + 2.0, x + 4.0, qy0 + 2.0, c::BRASS_DK);
                    self.draw_handwheel(x + 11.0, ey + 9.5);
                    self.qrect(x + 10.0, qy0 + 6.0, x + 12.0, qy0 + 6.0, c::BRASS);
                } else if m.w >= 6.0 {
                    let half = (m.w / 2.0).floor();
                    self.draw_handwheel(x + half, ey + 9.5);
                    self.qrect(
                        x + half - 1.0,
                        qy0 + 6.0,
                        x + half + 1.0,
                        qy0 + 6.0,
                        c::BRASS,
                    );
                }
            }
            Kind::Train => {}
        }
    }

    fn draw_pulley_at(&mut self, x: f64, y: f64, r: f64, s: f64) {
        let a = s * self.angle;
        for k in 0..10 {
            let t = (f64::from(k) / 10.0) * TAU;
            self.dot(
                x + t.cos() * (r - 0.4),
                y + t.sin() * (r - 0.4),
                c::BRASS,
                3,
            );
        }
        self.dot(x, y, c::BRASS_HI, 4);
        self.dot(x + a.cos() * 1.2, y + a.sin() * 1.2, c::STEEL, 4);
    }

    /// A valve handwheel: five spokes, nudged now and then by an invisible hand.
    fn draw_handwheel(&mut self, x: f64, y: f64) {
        let a = (self.t * 0.01 + x).sin() * 0.6;
        for k in 0..12 {
            let t = (f64::from(k) / 12.0) * TAU;
            self.dot(x + t.cos() * 2.4, y + t.sin() * 2.4, c::BRASS, 3);
        }
        for k in 0..5 {
            let t = a + (f64::from(k) / 5.0) * TAU;
            self.line(x, y, x + t.cos() * 1.8, y + t.sin() * 1.8, c::BRASS_DK, 2);
        }
        self.dot(x, y, c::BRASS_HI, 4);
    }

    fn draw_wheel(&mut self) {
        let big_r = self.radius;
        let rim = big_r - 1.7;
        let hub = 1.9;
        let spacing = TAU / SPOKES;
        let blur = clamp01((self.omega - 0.17) / 0.24);
        let smear = blur * spacing * 0.95;
        let x0 = i32_of((self.cx - big_r).floor());
        let x1 = i32_of((self.cx + big_r).ceil());
        let y0 = i32_of((self.cy - big_r).floor());
        let y1 = i32_of((self.cy + big_r).ceil());
        let spin_phase = i32_of(self.t * 5.0) & 15;
        let rim_color = mix(c::IRON_LT, c::BRASS_DK, 0.15);
        for y in y0..=y1 {
            let fy = f64::from(y);
            for x in x0..=x1 {
                let fx = f64::from(x);
                let ddx = fx + 0.5 - self.cx;
                let ddy = fy + 0.5 - self.cy;
                let d = (ddx * ddx + ddy * ddy).sqrt();
                if d > big_r {
                    continue;
                }
                if d >= rim {
                    self.dot(fx, fy, rim_color, 2);
                    continue;
                }
                if d <= hub {
                    self.dot(fx, fy, c::BRASS, 4);
                    continue;
                }
                // Angle in the engine's frame, the same one the crank turns in.
                let s = ddx * self.ux + ddy * self.uy;
                let t = ddx * self.vx + ddy * self.vy;
                let a = t.atan2(s);
                let mut m = (a - self.angle) % spacing;
                if m < 0.0 {
                    m += spacing;
                }
                let behind = spacing - m;
                let near = m.min(behind) * d;
                if near < 0.75 {
                    self.dot(fx, fy, if blur > 0.5 { c::IRON_LT } else { c::IRON }, 1);
                    continue;
                }
                // Motion blur: a dithered smear trailing each spoke.
                if smear > 0.0 && behind < smear {
                    let k = 1.0 - behind / smear;
                    if dither(x + spin_phase, y, k * (0.35 + 0.5 * blur)) {
                        self.dot(fx, fy, c::IRON_DK, 0);
                    }
                }
            }
        }
    }

    fn draw_linkage(&mut self) {
        let th = self.angle;
        let rc = self.crank;
        let big_l = self.rod_len;
        let sin_t = th.sin();
        let cos_t = th.cos();
        // Crank pin, and the crosshead the rod drags along the stroke.
        let pin_s = rc * cos_t;
        let pin_t = rc * sin_t;
        let s_h = rc * cos_t + (big_l * big_l - rc * rc * sin_t * sin_t).sqrt();
        let pin_x = self.fx(pin_s, pin_t);
        let pin_y = self.fy(pin_s, pin_t);
        let hx = self.fx(s_h, 0.0);
        let hy = self.fy(s_h, 0.0);
        let ht = self.half_t;
        let near = big_l - rc;

        // Slide bars either side of the crosshead's travel.
        let gt = if self.vertical { 3.5 } else { 2.5 };
        let (cyl0, cyl1) = (self.cyl0, self.cyl1);
        self.line(
            self.fx(near - 3.0, gt),
            self.fy(near - 3.0, gt),
            self.fx(cyl0, gt),
            self.fy(cyl0, gt),
            c::IRON,
            1,
        );
        self.line(
            self.fx(near - 3.0, -gt),
            self.fy(near - 3.0, -gt),
            self.fx(cyl0, -gt),
            self.fy(cyl0, -gt),
            c::IRON,
            1,
        );
        // Piston rod from crosshead into the cylinder.
        self.line(
            hx,
            hy,
            self.fx(cyl0 + 1.0, 0.0),
            self.fy(cyl0 + 1.0, 0.0),
            c::STEEL,
            3,
        );
        // Valve gear: an eccentric a quarter-turn ahead of the crank, its rod to the valve chest.
        let e_s = 1.6 * (th + PI / 2.0).cos();
        let e_t = 1.6 * (th + PI / 2.0).sin();
        let v_s = cyl0 + 2.0 + e_s * 0.6;
        self.line(
            self.fx(e_s, e_t),
            self.fy(e_s, e_t),
            self.fx(v_s, ht + 1.6),
            self.fy(v_s, ht + 1.6),
            c::BRASS_DK,
            2,
        );
        // The connecting rod, pin to crosshead, and the crank web from the hub.
        self.line(self.cx, self.cy, pin_x, pin_y, c::BRASS_DK, 3);
        self.line(pin_x, pin_y, hx, hy, c::STEEL, 4);
        for oy in [-1.0, 0.0] {
            for ox in [-1.0, 0.0] {
                self.dot(pin_x + ox + 0.5, pin_y + oy + 0.5, c::BRASS_HI, 5);
            }
        }

        // Solids in the engine frame: crosshead, cylinder (cut away), valve chest.
        let s_p = s_h + self.piston_offset;
        let s0 = (near - 4.0).min(cyl0);
        let s1 = cyl1 + 0.5;
        let t_max = ht + 4.0;
        let qw = (self.columns * 2) as f64;
        let qh = (self.rows * 2) as f64;
        // The frame is axis-aligned (u and v are unit axes), so two corners bound it.
        let xa = self.fx(s0, -t_max);
        let xb = self.fx(s1, t_max);
        let ya = self.fy(s0, -t_max);
        let yb = self.fy(s1, t_max);
        let qx0 = i32_of(xa.min(xb).floor().max(0.0));
        let qx1 = i32_of(xa.max(xb).ceil().min(qw - 1.0));
        let qy0 = i32_of((ya.min(yb) / 2.0).floor().max(0.0));
        let qy1 = i32_of((ya.max(yb) / 2.0).ceil().min(qh - 1.0));
        let chest0 = cyl0 + 3.5;
        let chest1 = cyl1 - 3.5;
        let steam_inside = mix(c::CAVITY, 0x3a4048, clamp01(self.heat * 1.2));
        for qy in qy0..=qy1 {
            let fqy = f64::from(qy);
            for qx in qx0..=qx1 {
                let fqx = f64::from(qx);
                let px = fqx + 0.5 - self.cx;
                let py = fqy * 2.0 + 1.0 - self.cy;
                let s = px * self.ux + py * self.uy;
                let t = px * self.vx + py * self.vy;
                let at = t.abs();
                if (s - s_h).abs() <= 1.5 && at <= 2.0 {
                    self.q(fqx, fqy, c::BRASS);
                    continue;
                }
                if s >= cyl0 && s <= cyl1 {
                    let capped = s < cyl0 + 2.0 || s > cyl1 - 2.0;
                    if capped && at <= ht + 1.2 {
                        self.q(fqx, fqy, c::IRON);
                        continue;
                    }
                    if at <= ht - 1.4 {
                        let color = if (s - s_p).abs() <= 1.1 {
                            c::BRASS_HI
                        } else if s < s_p {
                            c::CAVITY
                        } else {
                            steam_inside
                        };
                        self.q(fqx, fqy, color);
                        continue;
                    }
                    if at <= ht {
                        self.q(fqx, fqy, c::COPPER);
                        continue;
                    }
                    if t > ht && t <= ht + 3.0 && s >= chest0 && s <= chest1 {
                        let edge = t > ht + 1.5 && (s < chest0 + 1.0 || s > chest1 - 1.0);
                        self.q(fqx, fqy, if edge { c::BRASS_DK } else { c::IRON_LT });
                        continue;
                    }
                }
            }
        }
    }

    fn draw_governor(&mut self) {
        let spin = clamp01(self.omega / MAX_SPEED);
        let phi = 0.22 + 1.05 * spin.sqrt();
        let big_l = self.gov_arm;
        let x = self.gov_x;
        let y = self.gov_y;
        // In the spine the spindle is bevel-driven off the second shaft above.
        let from = if self.shaft2_y >= 0.0 {
            self.shaft2_y + 1.0
        } else {
            y - 0.5
        };
        self.line(x, from, x, self.gov_base, c::IRON, 1);
        // The balls swing round the spindle: show that as a slight in/out wobble.
        let wob = if spin > 0.05 {
            0.12 * (self.angle * 1.7).sin() * spin
        } else {
            0.0
        };
        let sin_p = phi.sin() * (1.0 - wob.abs());
        let cos_p = phi.cos();
        let sleeve_y = y + big_l * cos_p * 1.05;
        for side in [-1.0, 1.0] {
            let bx = x + side * big_l * sin_p;
            let by = y + big_l * cos_p;
            self.line(x, y, bx, by, c::BRASS_DK, 2);
            self.line(
                x + side * big_l * sin_p * 0.5,
                y + big_l * cos_p * 0.5,
                x,
                sleeve_y,
                c::BRASS_DK,
                2,
            );
            for oy in [-1.0, 0.0] {
                for ox in [-1.0, 0.0] {
                    self.dot(bx + ox + 0.5, by + oy + 0.5, c::BRASS_HI, 5);
                }
            }
        }
        self.dot(x, sleeve_y, c::BRASS, 3);
        self.dot(x, y - 1.0, c::BRASS, 3);
    }

    /// The firebox's color at a quadrant pixel: it flickers (a draw from the rng).
    fn fire_at(&mut self, qx: f64, qy: f64) -> u32 {
        let flick = self.rng.f() * 0.18 - 0.06 + (self.t * 0.7 + qx * 1.3).sin() * 0.04;
        let odd = if i32_of(qy) & 1 == 1 { 1.0 } else { 0.88 };
        let h = self.heat * odd + flick * (0.4 + self.heat);
        // Blue: a gas flame, never cold-dark, so it reads at every level.
        if self.dials.tint == Tint::Blue {
            return fire_color(0.3 + h * 0.75, &BLUE_FIRE);
        }
        fire_color(if self.sputter > 0.0 { h * 0.7 } else { h }, &FIRE)
    }

    fn warning_lamp(&mut self, qx: f64, qy: f64) {
        let blue = self.dials.tint == Tint::Blue;
        let on = blue && (i32_of(self.t) >> 3) % 3 != 0;
        let color = if on {
            c::BLUE
        } else if blue {
            0x24508c
        } else {
            c::BLUE_OFF
        };
        let x = f64::from(i32_of(qx.floor()) & !1);
        self.q(x, qy, color);
        self.q(x + 1.0, qy, color);
    }

    fn draw_boiler_band(&mut self) {
        let ex = self.cx - 20.0;
        let qy0 = ((self.cy - 10.0) / 2.0).floor();
        let row = |k: f64| qy0 + k;
        let b0 = ex + 70.0;
        let b1 = ex + 83.0;
        for k in 3..=8 {
            let k = f64::from(k);
            let inset = if k == 3.0 { 1.0 } else { 0.0 };
            let color = if k == 5.0 {
                c::BRASS
            } else if k == 3.0 {
                c::COPPER_DK
            } else {
                c::COPPER
            };
            self.qrect(b0 + inset, row(k), b1 - inset, row(k), color);
        }
        for k in 7..=8 {
            let k = f64::from(k);
            let mut x = b0 + 2.0;
            while x <= b1 - 4.0 {
                let frame = x == b0 + 2.0 || x == b1 - 4.0;
                let color = if frame {
                    c::BRASS_DK
                } else {
                    self.fire_at(x, k)
                };
                self.q(x, row(k), color);
                x += 1.0;
            }
        }
        let chim = self.chim_x.floor();
        self.qrect(chim - 2.0, row(1.0), chim + 1.0, row(1.0), c::BRASS);
        self.qrect(chim - 1.0, row(2.0), chim, row(2.0), c::IRON_DK);
        self.q(self.valve_x.floor(), row(2.0), c::BRASS);
        self.warning_lamp(b0 + 6.0, row(2.0));
        let chest_end = self.fx(self.cyl1 - 4.0, 0.0).floor();
        self.qrect(chest_end, row(1.0), b0 + 1.0, row(1.0), c::COPPER_DK);
        self.q(chest_end, row(2.0), c::COPPER_DK);
        self.qrect(b0 + 1.0, row(2.0), b0 + 2.0, row(2.0), c::COPPER_DK);
        let guide = self.rod_len - self.crank;
        let (g0, g1) = (
            self.fx(guide - 2.0, 0.0).floor(),
            self.fx(guide - 1.0, 0.0).floor(),
        );
        self.qrect(g0, row(7.0), g1, row(8.0), c::IRON_DK);
        let (c0, c1) = (
            self.fx(self.cyl0, 0.0).floor(),
            self.fx(self.cyl0 + 1.0, 0.0).floor(),
        );
        self.qrect(c0, row(7.0), c1, row(8.0), c::IRON_DK);
        let (e0, e1) = (
            self.fx(self.cyl1 - 1.0, 0.0).floor(),
            self.fx(self.cyl1, 0.0).floor(),
        );
        self.qrect(e0, row(7.0), e1, row(8.0), c::IRON_DK);
    }

    fn draw_boiler_spine(&mut self) {
        let wd = (self.columns * 2) as f64;
        let mid = self.cx;
        let half = ((wd / 2.0).floor() - 3.0).min(15.0);
        let bottom = self.boiler_bottom;
        let top = self.boiler_top;
        let b0 = mid - half;
        let b1 = mid + half - 1.0;
        let mut k = top;
        while k <= bottom {
            let inset = if k == top {
                2.0
            } else if k == top + 1.0 {
                1.0
            } else {
                0.0
            };
            let color = if k == top {
                c::COPPER_DK
            } else if k == top + 4.0 {
                c::BRASS
            } else {
                c::COPPER
            };
            self.qrect(b0 + inset, k, b1 - inset, k, color);
            k += 1.0;
        }
        let mut k = bottom - 2.0;
        while k <= bottom - 1.0 {
            let mut x = b0 + 3.0;
            while x <= b1 - 3.0 {
                let frame = x == b0 + 3.0 || x == b1 - 3.0;
                let color = if frame {
                    c::BRASS_DK
                } else {
                    self.fire_at(x, k)
                };
                self.q(x, k, color);
                x += 1.0;
            }
            k += 1.0;
        }
        self.qrect(b0 - 1.0, bottom, b1 + 1.0, bottom, c::IRON_DK);
        let chim = self.chim_x.floor();
        let chim_top = (self.chim_y / 2.0).floor();
        self.qrect(chim - 2.0, chim_top, chim + 1.0, chim_top, c::BRASS);
        self.qrect(chim - 1.0, chim_top + 1.0, chim, top - 1.0, c::IRON_DK);
        self.q(self.valve_x.floor(), top - 1.0, c::BRASS);
        self.q(self.valve_x.floor(), top, c::BRASS);
        self.warning_lamp(b1 - 3.0, top + 1.0);
        // Steam pipe down the side to the valve chest.
        let px = self.fx(0.0, self.half_t + 2.0).floor();
        let chest_top = (self.fy(self.cyl1 - 4.0, 0.0) / 2.0).floor();
        let mut k = bottom + 1.0;
        while k <= chest_top {
            self.q(px, k, c::COPPER_DK);
            k += 1.0;
        }
        // Guide frame feet beside the crosshead.
        let g0 = (self.fy(self.rod_len - self.crank - 3.0, 0.0) / 2.0).floor();
        self.qrect(
            (self.cx - 6.0).floor(),
            g0,
            (self.cx - 5.0).floor(),
            g0,
            c::IRON_DK,
        );
        self.qrect(
            (self.cx + 4.0).floor(),
            g0,
            (self.cx + 5.0).floor(),
            g0,
            c::IRON_DK,
        );
        // Rivets along the boiler's brass band.
        let mut x = b0 + 1.0;
        while x <= b1 - 1.0 {
            self.q(x, top + 4.0, c::BRASS_HI);
            x += 3.0;
        }
        // Iron pillars holding up the overhead shaft, riveted, down to the boiler.
        let mut k = 2.0;
        while k < top {
            let rivet = k % 4.0 == 0.0;
            let color = if rivet { c::BRASS_DK } else { c::IRON };
            self.q(0.0, k, color);
            self.q(wd - 1.0, k, color);
            k += 1.0;
        }
        // Gauge riser from the boiler crown, with a handwheel valve partway.
        if self.riser_x >= 0.0 {
            let rq0 = (self.riser_y0 / 2.0).floor();
            let rq1 = (self.riser_y1 / 2.0).floor();
            self.qrect(self.riser_x, rq0, self.riser_x + 1.0, rq1, c::COPPER_DK);
            self.qrect(
                self.riser_x - 1.0,
                rq1 - 1.0,
                self.riser_x + 2.0,
                rq1 - 1.0,
                c::BRASS,
            );
        }
    }

    /// The bed plate along the floor, with a lamp lit for each bit of subagent load.
    fn draw_bed(&mut self) {
        let wd = (self.columns * 2) as f64;
        let qy = (self.rows * 2) as f64 - 1.0;
        let mut x0 = 0.0;
        let x1 = wd - 1.0;
        if !self.vertical {
            x0 = (self.cx + self.radius).ceil() + 1.0;
        }
        self.qrect(x0, qy, x1, qy, c::IRON_DK);
        let (most, per) = if self.vertical {
            (4.0, 12.0)
        } else {
            (16.0, 4.0)
        };
        let lit = round(self.dials.coverage_boost / per).min(most);
        if lit <= 0.0 {
            return;
        }
        let step = f64::from(i32_of(((x1 - x0) / (lit + 1.0)).floor()) & !1).max(4.0);
        let lamp = match self.dials.tint {
            Tint::Smoke => c::LAMP_LOW,
            Tint::Blue => c::LAMP_BLUE,
            Tint::Normal => c::LAMP_ON,
        };
        let mut k = 0.0;
        while k < lit {
            let x = f64::from(i32_of(x0 + step * (k + 1.0)) & !1);
            if x + 1.0 > x1 {
                break;
            }
            let blink = (f64::from(i32_of(self.t) >> 3) + k * 5.0) % 13.0 == 0.0;
            let color = if blink { c::BRASS_DK } else { lamp };
            self.q(x, qy, color);
            self.q(x + 1.0, qy, color);
            k += 1.0;
        }
    }

    fn draw_steam(&mut self) {
        let wd = self.columns * 2;
        let hd = self.rows * 4;
        let p = &self.puffs;
        let st = &mut self.steam;
        for i in 0..p.n {
            let r = f64::from(p.r[i]);
            let life = f64::from(p.age[i]) / f64::from(p.life[i]);
            let amp = f64::from(p.amp[i]) * (1.0 - life) * (1.0 - life * 0.3);
            let x = f64::from(p.x[i]);
            let y = f64::from(p.y[i]);
            let x0 = i32_of((x - r).floor().max(0.0));
            let x1 = i32_of((x + r).ceil().min(wd as f64 - 1.0));
            let y0 = i32_of((y - r).floor().max(0.0));
            let y1 = i32_of((y + r).ceil().min(hd as f64 - 1.0));
            let inv = 1.0 / (r * r);
            for yy in y0..=y1 {
                let dy = f64::from(yy) + 0.5 - y;
                for xx in x0..=x1 {
                    let dx = f64::from(xx) + 0.5 - x;
                    let f = 1.0 - (dx * dx + dy * dy) * inv;
                    if f > 0.0 {
                        let k = yy as usize * wd + xx as usize;
                        st[k] = (f64::from(st[k]) + amp * f) as f32;
                    }
                }
            }
        }
    }

    fn draw_sparks(&mut self) {
        let s = &self.sparks;
        for i in 0..s.n {
            let x = f64::from(s.x[i]).floor();
            let y = f64::from(s.y[i]).floor();
            if !(x >= 0.0
                && y >= 0.0
                && x < (self.columns * 2) as f64
                && y < (self.rows * 4) as f64)
            {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            let c = (y >> 2) * self.columns + (x >> 1);
            self.spark[c] |= BRAILLE[x & 1][y & 3] as u8;
        }
    }

    fn steam_color(&self, d: f64) -> u32 {
        // Smoke: sooty black-brown; blue: steam lit blue-white.
        match self.dials.tint {
            Tint::Smoke => mix(0x2a2826, 0x6e6a66, d),
            Tint::Blue => mix(0x34507e, 0xa8ccff, d),
            Tint::Normal => mix(0x5d666e, 0xe8ecef, d),
        }
    }

    /// Turn the canvases into cells: solids, then sparks, then linkage, then steam.
    fn compose(&mut self) {
        let w = self.columns;
        let qw = w * 2;
        let wd = w * 2;
        for row in 0..self.rows {
            for col in 0..w {
                let i = row * w + col;
                let qa = row * 2 * qw + col * 2;
                let q = [
                    self.quad[qa],
                    self.quad[qa + 1],
                    self.quad[qa + qw],
                    self.quad[qa + qw + 1],
                ];
                if q.iter().any(|&v| v != 0) {
                    self.solid_cell(i, q);
                    continue;
                }
                let mut sbits = 0;
                let mut sum = 0.0;
                let dx0 = col * 2;
                let dy0 = row * 4;
                for dy in 0..4 {
                    let base = (dy0 + dy) * wd + dx0;
                    for dx in 0..2 {
                        let v = f64::from(self.steam[base + dx]);
                        sum += if v > 1.0 { 1.0 } else { v };
                        if v > (BAYER[(((dy0 + dy) & 3) << 2) | ((dx0 + dx) & 3)] + 0.5) / 16.0 {
                            sbits |= BRAILLE[dx][dy];
                        }
                    }
                }
                let avg = sum / 8.0;
                let b = u32::from(self.bits[i]);
                let sp = u32::from(self.spark[i]);
                // In the spine's engine house, a dim brick wall stands behind the machinery.
                let wall = self.wall[i] == 1;
                let bg = if wall { c::WALL_MORTAR } else { DEFAULT_COLOR };
                if sp != 0 {
                    self.out.set(i, BRAILLE_BASE | sp | b, c::SPARK, bg);
                } else if b != 0 {
                    let steam = if avg > 0.3 { sbits } else { 0 };
                    self.out.set(i, BRAILLE_BASE | b | steam, self.bcol[i], bg);
                } else if sbits != 0 {
                    let color = self.steam_color(avg * 1.4);
                    self.out.set(i, BRAILLE_BASE | sbits, color, bg);
                } else if wall {
                    let glyph = if ((col + (row & 1) * 2) & 3) == 0 {
                        0x2597
                    } else {
                        0x2584
                    };
                    self.out.set(i, glyph, c::WALL_BRICK, c::WALL_MORTAR);
                } else {
                    self.out.blank(i);
                }
            }
        }
    }

    /// Up to four quadrant colors → one quadrant glyph with a fg and (maybe) a bg.
    fn solid_cell(&mut self, i: usize, qs: [u32; 4]) {
        let mut cols = [0u32; 4];
        let mut counts = [0u8; 4];
        let mut n = 0;
        let mut empty = false;
        for &q in &qs {
            if q == 0 {
                empty = true;
                continue;
            }
            let mut j = 0;
            while j < n && cols[j] != q {
                j += 1;
            }
            if j == n {
                cols[n] = q;
                counts[n] = 0;
                n += 1;
            }
            counts[j] += 1;
        }
        let mut a = 0;
        for j in 1..n {
            if counts[j] > counts[a] {
                a = j;
            }
        }
        let mut b: Option<usize> = None;
        if !empty {
            for j in 0..n {
                if j != a && b.is_none_or(|b| counts[j] > counts[b]) {
                    b = Some(j);
                }
            }
        }
        let fg = cols[a] - SOLID;
        let bg = b.map(|b| cols[b] - SOLID);
        let mut mask = 0;
        for (k, &q) in qs.iter().enumerate() {
            if q == 0 {
                continue;
            }
            let c = q - SOLID;
            let fore = match bg {
                None => true,
                Some(bg) => c == fg || (c != bg && dist(c, fg) <= dist(c, bg)),
            };
            if fore {
                mask |= 1 << k;
            }
        }
        match bg {
            Some(_) if mask == 15 => self.out.set_fg(i, 0x2588, fg),
            Some(bg) => self.out.set(i, QUAD[mask], fg, bg),
            None => {
                let behind = if self.wall[i] == 1 {
                    c::WALL_MORTAR
                } else {
                    DEFAULT_COLOR
                };
                self.out.set(i, QUAD[mask], fg, behind);
            }
        }
    }

    /// A pressure gauge: a cream face (a deliberate background) and a needle that climbs.
    fn draw_gauge(&mut self, g: &Gauge) {
        let (col, row, wc) = (g.col, g.row, g.wc);
        let hc = 1.0;
        if col < 0.0 || row < 0.0 || col + wc > self.columns as f64 || row + hc > self.rows as f64 {
            return;
        }
        let gw = wc * 2.0;
        let gh = hc * 4.0;
        let base = clamp01(self.pressure + g.bias * self.pressure);
        let wobble = if base > 0.7 {
            (self.t * 1.9 + g.col).sin() * 0.025
        } else {
            0.0
        };
        let p = clamp01(base + wobble);
        let a = (-0.75 + 1.5 * p) * PI;
        let ox = gw / 2.0;
        let oy = gh * 0.7;
        let len = (gw / 2.0).min(oy) + 0.2;
        let dx = a.sin();
        let dy = -a.cos();
        let mut b0 = 0;
        let mut b1 = 0;
        for k in 0..=8 {
            let f = (f64::from(k) / 8.0) * len;
            let x = (ox + dx * f).floor();
            let y = (oy - 0.5 + dy * f + 0.5).floor();
            if x < 0.0 || y < 0.0 || x >= gw || y >= gh {
                continue;
            }
            let (x, y) = (x as usize, y as usize);
            let bit = BRAILLE[x & 1][y & 3];
            if x >> 1 == 0 {
                b0 |= bit;
            } else {
                b1 |= bit;
            }
        }
        let color = if p > 0.85 { c::NEEDLE_HOT } else { c::NEEDLE };
        let face = match self.dials.tint {
            Tint::Blue => mix(c::FACE, 0x8cbcff, 0.6),
            Tint::Smoke => mix(c::FACE, 0x5a5650, 0.45),
            Tint::Normal => c::FACE,
        };
        let i = (row * self.columns as f64 + col) as usize;
        self.out.set(i, BRAILLE_BASE | b0, color, face);
        self.out.set(i + 1, BRAILLE_BASE | b1, color, face);
    }
}
