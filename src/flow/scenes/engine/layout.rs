//! Where everything sits, in dots: the band runs sideways (the mill engine,
//! then a line shaft driving a run of machines), the spine stands up (a
//! vertical engine with gear trains packed round it).

use std::f64::consts::{PI, TAU};

use super::{
    c, Belt, Emitter, Engine, Gauge, Gear, Kind, Module, Pulley, Shaft, PITCH, SHAFT_RATIO,
};
use crate::flow::cells::{is_tall, Rng};
use crate::flow::js::i32_of;

impl Engine {
    fn add_pulley(&mut self, x: f64, y: f64, r: f64, s: f64, color: u32, spokes: f64) {
        self.pulleys.push(Pulley {
            x,
            y,
            r,
            phase: 0.0,
            s,
            color,
            spokes,
        });
    }

    /// A belt from pulley 1 to pulley 2; returns pulley 2's speed ratio.
    #[allow(clippy::too_many_arguments)]
    fn add_belt(
        &mut self,
        x1: f64,
        y1: f64,
        r1: f64,
        s1: f64,
        x2: f64,
        y2: f64,
        r2: f64,
        crossed: bool,
    ) -> f64 {
        self.belts.push(Belt {
            x1,
            y1,
            r1,
            x2,
            y2,
            r2,
            v: s1 * r1,
            crossed,
        });
        ((if crossed { -1.0 } else { 1.0 }) * s1 * r1) / r2
    }

    /// A new gear; returns its index.
    fn add_gear(&mut self, x: f64, y: f64, n: f64, s: f64, phase: f64, color: u32) -> usize {
        self.gears.push(Gear {
            x,
            y,
            n,
            r: n * PITCH,
            phase,
            s,
            color,
            rim: false,
        });
        self.gears.len() - 1
    }

    /// A gear meshing with gear `a`, centred at height y (dx from the pitch
    /// circles); turns the other way.
    fn mesh_gear(&mut self, a: usize, n: f64, y: f64, side: f64, color: u32) -> usize {
        let g = self.gears[a];
        let r = n * PITCH;
        let dist = g.r + r + 0.5;
        let dy = (-dist * 0.85).max((dist * 0.85).min(y - g.y));
        let dx = side * (dist * dist - dy * dy).sqrt();
        self.mesh_at(a, n, dx, dy, color)
    }

    /// A gear meshing with gear `a`, its centre offset (dx, dy) along the line of centres.
    fn mesh_at(&mut self, a: usize, n: f64, dx: f64, dy: f64, color: u32) -> usize {
        let g = self.gears[a];
        let beta = dy.atan2(dx);
        // Keep a tooth of `a` meeting a gap of the new gear at the contact point.
        let phase = beta + PI - (PI - g.n * (beta - g.phase)) / n;
        self.add_gear(g.x + dx, g.y + dy, n, (-g.s * g.n) / n, phase, color)
    }

    pub(super) fn layout(&mut self) {
        let wd = (self.columns * 2) as f64;
        let hd = (self.rows * 4) as f64;
        self.vertical = is_tall(self.columns, self.rows);
        self.gears.clear();
        self.pulleys.clear();
        self.belts.clear();
        self.modules.clear();
        self.gauges.clear();
        self.emitters.clear();
        self.hangers.clear();
        self.shafts.clear();
        self.wall = vec![0; self.columns * self.rows];
        self.riser_x = -1.0;
        self.shaft2_y = -1.0;
        if !self.vertical {
            let ex = if wd >= 100.0 { 2.0 } else { 0.0 };
            let ey = hd - 20.0;
            self.ey = ey;
            self.radius = 9.6;
            self.cx = ex + 20.0;
            self.cy = ey + 10.0;
            self.ux = 1.0;
            self.uy = 0.0;
            self.vx = 0.0;
            self.vy = -1.0;
            self.crank = 5.2;
            self.rod_len = 20.5;
            self.half_t = 4.0;
            self.chim_x = ex + 80.0;
            self.chim_y = ey + 2.0;
            self.valve_x = ex + 73.5;
            self.valve_y = ey + 4.0;
            self.gov_x = ex + 41.0;
            self.gov_y = ey + 0.5;
            self.gov_arm = 5.2;
            self.gov_base = self.cy - 2.5;
        } else {
            let mid = (wd / 2.0).floor();
            self.ey = 0.0;
            self.radius = (mid - 4.0).clamp(8.0, 12.0);
            self.cx = mid;
            self.cy = hd - 3.0 - self.radius;
            self.ux = 0.0;
            self.uy = -1.0;
            self.vx = 1.0;
            self.vy = 0.0;
            self.crank = self.radius * 0.52;
            self.rod_len = self.crank * 2.9;
            self.half_t = 4.5;
        }
        // The cylinder sits past the crosshead's travel; the piston runs its length.
        let near = self.rod_len - self.crank;
        self.cyl0 = self.rod_len + self.crank + if self.vertical { 5.0 } else { 4.5 };
        self.piston_offset = self.cyl0 + 3.0 - near;
        self.cyl1 = self.cyl0 + 3.0 + self.crank * 2.0 + 3.0;
        // The engine's own chimney (placed below).
        self.emitters.push(Emitter {
            x: 0.0,
            y: 0.0,
            kind: 0,
        });
        if self.vertical {
            self.layout_spine(wd);
        } else {
            self.layout_band(wd);
        }
        self.emitters[0].x = self.chim_x;
        self.emitters[0].y = self.chim_y;
    }

    fn layout_spine(&mut self, wd: f64) {
        let hd = (self.rows * 4) as f64;
        let cyl_top = self.cy - self.cyl1;
        // Boiler above the cylinder with a second line shaft between them;
        // chimney on the boiler; steam room above that.
        self.boiler_bottom = ((cyl_top - 7.0) / 2.0).floor();
        self.boiler_top = self.boiler_bottom - 7.0;
        self.chim_x = self.cx + 3.0;
        self.chim_y = (self.boiler_top - 3.0) * 2.0;
        self.valve_x = self.cx - 5.5;
        self.valve_y = self.boiler_top * 2.0 - 1.0;
        let avail = self.cx - self.half_t - 1.5;
        self.gov_x = (avail / 2.0).max(2.0);
        self.gov_arm = (avail / 2.0 - 0.3).clamp(2.5, 5.2);
        self.gov_y = cyl_top + 2.5;
        self.gov_base = self.gov_y + self.gov_arm + 2.0;
        let wall_row = ((self.boiler_top * 2.0) / 4.0).floor().max(0.0) as usize;
        for cell in self.wall.iter_mut().skip(wall_row * self.columns) {
            *cell = 1;
        }

        // Overhead shaft; a belt at the right steps down to the second shaft.
        self.shaft_y = 2.0;
        self.shafts.push(Shaft {
            y: 2.0,
            x0: 0.0,
            x1: wd,
            s: SHAFT_RATIO,
        });
        self.hangers.push((wd * 0.3).floor());
        self.hangers.push((wd * 0.7).floor());
        let s2y = self.boiler_bottom * 2.0 + 3.0;
        self.shaft2_y = s2y;
        let r_x = wd - 3.5;
        self.add_pulley(r_x, 2.5, 2.0, SHAFT_RATIO, c::IRON_LT, 3.0);
        let s2 = self.add_belt(r_x, 2.5, 2.0, SHAFT_RATIO, r_x, s2y + 0.5, 2.6, false);
        self.add_pulley(r_x, s2y + 0.5, 2.6, s2, c::BRASS, 3.0);
        self.shafts.push(Shaft {
            y: s2y,
            x0: 0.0,
            x1: wd,
            s: s2,
        });

        // In the steam room: a cam lifting a striker against a bell (left), and a
        // gauge cluster on a riser from the boiler (right, clear of the plume).
        let cam_x = (self.cx * 0.4).clamp(3.5, 6.0);
        let cam_y = self.boiler_top * 2.0 - 6.0;
        if cam_y - 15.0 > 4.0 {
            self.add_pulley(cam_x + 3.5, 2.5, 2.0, SHAFT_RATIO, c::IRON_LT, 3.0);
            let sc = self.add_belt(cam_x + 3.5, 2.5, 2.0, SHAFT_RATIO, cam_x, cam_y, 1.1, false);
            self.modules.push(Module {
                kind: Kind::Bell,
                x: cam_x,
                y: cam_y,
                w: 0.0,
                s: sc,
                phase: 0.0,
                prev: 0.0,
            });
        }
        let gc = self.columns as f64 - 5.0;
        let gr = ((self.boiler_top * 2.0) / 4.0).floor() - 2.0;
        if gc * 2.0 >= self.chim_x + 3.0 && gr >= 2.0 {
            let two = gr - 2.0 >= 1.0;
            self.gauge_at(gc, gr, 0.1);
            if two {
                self.gauge_at(gc, gr - 2.0, -0.15);
            }
            self.riser_x = gc * 2.0 + 1.0;
            self.riser_y0 = (if two { gr - 2.0 } else { gr }) * 4.0 + 2.0;
            self.riser_y1 = self.boiler_top * 2.0;
        }
        self.gauge_at(
            (self.cx / 2.0).floor() - 2.0,
            ((self.boiler_top * 2.0 + 5.0) / 4.0).floor(),
            0.0,
        );

        // Gear trains packed down both sides to the floor: the left one off the
        // governor's spindle, the right one belted from the second shaft.
        let left = self.start_gear(
            self.gov_x,
            self.gov_base,
            s2 * 1.25,
            0.4,
            c::BRASS,
            false,
            wd,
            hd,
        );
        if let Some(left) = left {
            self.gov_base = self.gears[left].y;
            self.pack_chain(left, 0, wd, hd);
        }
        let right = self.start_gear(wd - 4.0, s2y + 5.0, 0.0, 1.3, c::COPPER, true, wd, hd);
        if let Some(right) = right {
            let (gx, gy) = (self.gears[right].x, self.gears[right].y);
            let s = self.add_belt(r_x, s2y + 0.5, 1.5, s2, gx, gy, 1.4, false);
            self.gears[right].s = s;
            self.add_pulley(r_x, s2y + 0.5, 1.5, s2, c::BRASS_HI, 0.0);
            self.add_pulley(gx, gy, 1.4, s, c::BRASS_HI, 0.0);
            self.pack_chain(right, 1, wd, hd);
        }
        // Teeth cut on the flywheel's rim drive pinions tucked round it. In the
        // spine's frame (u up, v right) the wheel turns +1 on screen with the crank.
        let wn = ((self.radius + 0.4) / PITCH).ceil();
        let wheel = self.add_gear(self.cx, self.cy, wn, 1.0, PI / 2.0, c::IRON_LT);
        self.gears[wheel].rim = true;
        const ANGLES: [f64; 8] = [-0.6, -2.54, -0.25, -2.89, 0.15, -3.29, 0.5, -3.64];
        for (k, &angle) in ANGLES.iter().enumerate() {
            for n in (6..=10).rev() {
                let n = f64::from(n);
                let r = n * PITCH;
                let dist = self.gears[wheel].r + r + 0.5;
                let dx = angle.cos() * dist;
                let dy = angle.sin() * dist;
                if !self.gear_fits(self.cx + dx, self.cy + dy, r, Some(wheel), wd, hd, true) {
                    continue;
                }
                let color = if k & 1 == 1 { c::BRASS } else { c::COPPER };
                let p = self.mesh_at(wheel, n, dx, dy, color);
                self.pack_chain(p, 2 + k, wd, hd);
                break;
            }
        }
        // Then pack idler gears into whatever room is left, each meshing with
        // exactly one gear already placed (and clear of all the others).
        const COLORS: [u32; 4] = [c::BRASS, c::COPPER, c::IRON_LT, c::BRASS_HI];
        for pass in 0..4 {
            let count = self.gears.len();
            let mut added = 0;
            for i in 0..count {
                let o = self.gears[i];
                for k in 0..16 {
                    let ang = (f64::from(k) / 16.0) * TAU + f64::from(pass) * 0.2;
                    for n in (6..=10).rev() {
                        let n = f64::from(n);
                        let r = n * PITCH;
                        let dist = o.r + r + 0.5;
                        let dx = ang.cos() * dist;
                        let dy = ang.sin() * dist;
                        if !self.gear_fits(o.x + dx, o.y + dy, r, Some(i), wd, hd, i == wheel) {
                            continue;
                        }
                        self.mesh_at(i, n, dx, dy, COLORS[(i + k as usize) & 3]);
                        added += 1;
                        break;
                    }
                }
            }
            if added == 0 {
                break;
            }
        }
    }

    /// Room for a gear (pitch radius r) at (x, y) in the spine, clear of the
    /// engine and of every gear but its parent.
    #[allow(clippy::too_many_arguments)]
    fn gear_fits(
        &self,
        x: f64,
        y: f64,
        r: f64,
        parent: Option<usize>,
        wd: f64,
        hd: f64,
        on_wheel: bool,
    ) -> bool {
        let tip = r + 1.0;
        if x - tip < 0.0 || x + tip > wd || y + tip > hd - 2.5 {
            return false;
        }
        if y - tip < self.shaft2_y + 2.0 {
            return false;
        }
        let wx = x - self.cx;
        let wy = y - self.cy;
        if !on_wheel && (wx * wx + wy * wy).sqrt() < self.radius + tip + 2.2 {
            return false;
        }
        let cyl_top = self.cy - self.cyl1;
        let cyl_bottom = self.cy - self.cyl0;
        if y + tip > cyl_top - 1.0 && y - tip < self.cy - self.radius + 3.0 {
            let core = if y - tip < cyl_bottom + 1.0 {
                if x > self.cx {
                    self.half_t + 3.6
                } else {
                    self.half_t + 1.4
                }
            } else {
                6.2
            };
            if wx.abs() < core + tip {
                return false;
            }
        }
        if x < self.cx
            && x - tip < self.gov_x + self.gov_arm + 1.0
            && y - tip < self.gov_y + self.gov_arm + 1.5
        {
            return false;
        }
        for (i, o) in self.gears.iter().enumerate() {
            if Some(i) == parent {
                continue;
            }
            let dx = o.x - x;
            let dy = o.y - y;
            if (dx * dx + dy * dy).sqrt() < o.r + r + 2.4 {
                return false;
            }
        }
        true
    }

    /// The biggest gear that fits near (x, y), searching down the side.
    #[allow(clippy::too_many_arguments)]
    fn start_gear(
        &mut self,
        x: f64,
        y: f64,
        s: f64,
        phase: f64,
        color: u32,
        hug_right: bool,
        wd: f64,
        hd: f64,
    ) -> Option<usize> {
        let bottom = (self.rows * 4) as f64;
        for n in (6..=12).rev() {
            let n = f64::from(n);
            let r = n * PITCH;
            let gx = if hug_right {
                wd - r - 1.2
            } else {
                (r + 1.1).max(x)
            };
            let mut gy = y + r;
            while gy < bottom {
                if self.gear_fits(gx, gy, r, None, wd, hd, false) {
                    return Some(self.add_gear(gx, gy, n, s, phase, color));
                }
                gy += 1.0;
            }
        }
        None
    }

    /// Mesh gear after gear downward from gear `g`, zigzagging, while they fit.
    fn pack_chain(&mut self, mut g: usize, seed: usize, wd: f64, hd: f64) {
        const SIZES: [f64; 8] = [9.0, 12.0, 7.0, 10.0, 6.0, 11.0, 8.0, 13.0];
        const STEEPS: [f64; 6] = [0.75, 0.95, 0.5, 0.3, 0.1, -0.2];
        const COLORS: [u32; 4] = [c::COPPER, c::BRASS, c::IRON_LT, c::BRASS_HI];
        let mut side = if seed != 0 { -1.0 } else { 1.0 };
        for guard in 0..24 {
            let mut placed = None;
            let at = self.gears[g];
            'search: for k in 0..SIZES.len() {
                let n = SIZES[(seed * 3 + guard + k) % SIZES.len()];
                let r = n * PITCH;
                let dist = at.r + r + 0.5;
                for &steep in &STEEPS {
                    for sd in 0..2 {
                        let sgn = if sd == 0 { side } else { -side };
                        let dy = steep * dist;
                        let dx = sgn * (dist * dist - dy * dy).sqrt();
                        if self.gear_fits(at.x + dx, at.y + dy, r, Some(g), wd, hd, false) {
                            placed = Some(self.mesh_at(
                                g,
                                n,
                                dx,
                                dy,
                                COLORS[(guard + seed) % COLORS.len()],
                            ));
                            side = -sgn;
                            break 'search;
                        }
                    }
                }
            }
            let Some(next) = placed else {
                return;
            };
            g = next;
        }
    }

    fn gauge_at(&mut self, col: f64, row: f64, bias: f64) {
        self.gauges.push(Gauge {
            col,
            row,
            wc: 2.0,
            bias,
        });
    }

    fn layout_band(&mut self, wd: f64) {
        let ey = self.ey;
        let ex = self.cx - 20.0;
        let mut lay = Rng::new(f64::from(
            (self.seed as i32) ^ i32_of((self.columns * 7919) as f64),
        ));
        self.shaft_y = ey + 2.0;
        self.boiler_top = (ey / 2.0).floor() + 3.0;
        self.gauge_at(
            (self.chim_x / 2.0).floor() - 4.0,
            (ey / 4.0).floor() + 2.0,
            0.0,
        );
        let mut x = ex + 88.0;
        self.shafts.push(Shaft {
            y: self.shaft_y,
            x0: x - 2.0,
            x1: wd,
            s: SHAFT_RATIO,
        });
        // Fixed-width machines, shuffled, with gear trains between them.
        let mut pool = [
            Kind::Hammer,
            Kind::Stack,
            Kind::Beam,
            Kind::Panel,
            Kind::Grind,
        ];
        let mut pi = pool.len();
        let mut i = 0;
        while x < wd {
            let left = wd - x;
            let mut kind = if i % 2 == 0 {
                Kind::Train
            } else {
                if pi >= pool.len() {
                    for k in (1..pool.len()).rev() {
                        let j = (lay.f() * (k + 1) as f64).floor() as usize;
                        pool.swap(k, j);
                    }
                    pi = 0;
                }
                pi += 1;
                pool[pi - 1]
            };
            i += 1;
            let mut w = if kind == Kind::Train {
                left.min(26.0 + (lay.f() * 30.0).floor())
            } else {
                kind.width()
            };
            if w > left || left - w < 10.0 {
                // The last stretch: a gear train fills it, or a pipe run if it's narrow.
                kind = if left >= 20.0 {
                    Kind::Train
                } else {
                    Kind::Pipe
                };
                w = left;
            }
            if kind == Kind::Train {
                w = self.layout_train(x, w, &mut lay);
            } else {
                self.layout_module(kind, x, w);
            }
            self.hangers.push((x + 1.0).floor());
            x += w;
        }
    }

    /// A belt-driven gear train in [x, x+w); returns the width it took.
    fn layout_train(&mut self, x: f64, w: f64, lay: &mut Rng) -> f64 {
        const COLORS: [u32; 4] = [c::BRASS, c::COPPER, c::IRON_LT, c::BRASS_HI];
        const SIZES: [f64; 10] = [8.0, 17.0, 9.0, 13.0, 7.0, 15.0, 10.0, 11.0, 16.0, 8.0];
        let ey = self.ey;
        let mut ci = (lay.f() * 4.0).floor() as usize;
        let n0 = 12.0 + (lay.f() * 4.0).floor();
        let r0 = n0 * PITCH;
        let gx = x + r0 + 1.6;
        let gy = ey + 11.5;
        self.add_pulley(gx, self.shaft_y + 0.5, 2.0, SHAFT_RATIO, c::IRON_LT, 3.0);
        let crossed = lay.f() < 0.3;
        let s = self.add_belt(
            gx,
            self.shaft_y + 0.5,
            2.0,
            SHAFT_RATIO,
            gx,
            gy,
            2.2,
            crossed,
        );
        let phase = lay.f() * TAU;
        let mut g = self.add_gear(gx, gy, n0, s, phase, COLORS[ci % 4]);
        ci += 1;
        self.add_pulley(gx, gy, 2.2, s, c::BRASS_HI, 0.0);
        let si0 = (lay.f() * SIZES.len() as f64).floor() as usize;
        let mut right = gx + r0 + 1.2;
        for si in si0..si0 + 20 {
            let n = SIZES[si % SIZES.len()];
            let r = n * PITCH;
            // Keep the teeth inside the band (y 4.5..18.5 above the bed).
            let lo = ey + 4.6 + r + 1.1;
            let hi = ey + 18.4 - r - 1.1;
            let y = if lo >= hi {
                ey + 11.5
            } else {
                lo + lay.f() * (hi - lo)
            };
            let at = self.gears[g];
            let dist = at.r + r + 0.5;
            let dy = (-dist * 0.85).max((dist * 0.85).min(y - at.y));
            let nx = at.x + (dist * dist - dy * dy).sqrt();
            if nx + r + 1.2 > x + w {
                break;
            }
            g = self.mesh_gear(g, n, y, 1.0, COLORS[ci % 4]);
            ci += 1;
            right = self.gears[g].x + self.gears[g].r + 1.2;
        }
        w.min(14.0f64.max((right - x + 2.0).ceil()))
    }

    fn layout_module(&mut self, kind: Kind, x: f64, w: f64) {
        let ey = self.ey;
        let sy = self.shaft_y + 0.5;
        let mut m = Module {
            kind,
            x,
            y: 0.0,
            w,
            s: 0.0,
            phase: 0.0,
            prev: 0.0,
        };
        match kind {
            Kind::Hammer => {
                self.add_pulley(x + 16.0, sy, 2.0, SHAFT_RATIO, c::IRON_LT, 3.0);
                m.s = self.add_belt(
                    x + 16.0,
                    sy,
                    2.0,
                    SHAFT_RATIO,
                    x + 16.0,
                    ey + 13.1,
                    1.1,
                    false,
                );
            }
            Kind::Beam => {
                self.add_pulley(x + 6.0, sy, 2.0, SHAFT_RATIO, c::IRON_LT, 3.0);
                m.s = self.add_belt(
                    x + 6.0,
                    sy,
                    2.0,
                    SHAFT_RATIO,
                    x + 6.0,
                    ey + 13.0,
                    1.2,
                    false,
                );
                self.add_pulley(x + 6.0, ey + 13.0, 3.6, m.s, c::IRON_LT, 4.0);
                self.add_pulley(x + 6.0, ey + 13.0, 1.2, m.s, c::BRASS, 0.0);
            }
            Kind::Grind => {
                self.add_pulley(x + 4.0, sy, 2.0, SHAFT_RATIO, c::IRON_LT, 3.0);
                m.s = self.add_belt(
                    x + 4.0,
                    sy,
                    2.0,
                    SHAFT_RATIO,
                    x + 13.0,
                    ey + 11.0,
                    2.0,
                    true,
                );
            }
            Kind::Stack => {
                self.emitters.push(Emitter {
                    x: x + 6.0,
                    y: ey + 1.5,
                    kind: 1,
                });
            }
            Kind::Panel => {
                self.gauge_at(((x + 5.0) / 2.0).floor(), (ey / 4.0).floor() + 1.0, -0.12);
                if w >= 20.0 {
                    self.gauge_at(((x + 15.0) / 2.0).floor(), (ey / 4.0).floor() + 2.0, 0.08);
                }
                self.emitters.push(Emitter {
                    x: x + 21.0,
                    y: ey + 9.0,
                    kind: 2,
                });
            }
            Kind::Train | Kind::Pipe | Kind::Bell => {}
        }
        self.modules.push(m);
    }
}
