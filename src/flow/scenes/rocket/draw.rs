//! The launch site drawn into its pixel layer, two pixels a cell across and
//! two down, then folded into quadrant glyphs over the sky: the tower and
//! the mount, Earth from orbit, the particles, the plume, the rocket (and a
//! parted stage, a waiting booster, the Ship on its transporter, Dragon's
//! parachutes), the catch arms and the tower's lights.

use std::f64::consts::PI;

use crate::flow::cells::{Cells, DEFAULT_COLOR};
use crate::flow::js::{i32_of, round};
use crate::flow::palette::{lightness, tint_to};
use crate::flow::pixels::{fit_quad, hash, lower_block, mix, QUAD};
use crate::flow::scene::Tint;
use crate::flow::sky::SkyScene;

use super::particles::PMAX;
use super::sprites::{ramp_color, Ramp, Sprite, BLUE, SMOKE};
use super::{LaunchSite, Part, State};

/// Earth's limb seen from orbit: the bright blue edge of the atmosphere.
const LIMB: u32 = 0x6cb4f2;
const OCEAN: u32 = 0x1d4e8e;

const LIGHT_RED: u32 = 0xff3b30;
const LIGHT_AMBER: u32 = 0xffb020;
const LIGHT_GREEN: u32 = 0x5cff7a;
const LIGHT_BLUE: u32 = 0x4aa8ff;

impl LaunchSite {
    /// The plume's colours: the rocket's own, or a tint's.
    fn ramp(&self) -> &'static Ramp {
        match self.world.dials.tint {
            Tint::Smoke => &SMOKE,
            Tint::Blue => &BLUE,
            Tint::Normal => &self.look().ramp,
        }
    }

    /// The plume's length this frame, flickering (and sputtering on a failed command).
    fn plume_len(&mut self) -> f64 {
        let p = self.part;
        // (No plume from the Ship while it's still turning upright.)
        if self.thr <= 0.0 || p == Part::Dragon || p == Part::Capsule || self.flop > 0.3 {
            return 0.0;
        }
        let mut len = self.spec().plume
            * self.thr
            * (0.85 + 0.3 * self.rng.f())
            * if self.part == Part::Upper { 0.7 } else { 1.0 };
        if self.world.dials.tint == Tint::Smoke
            && hash(f64::from(i32_of(self.world.t) >> 1), 3.0, 43.0) < 0.35
        {
            len *= 0.25;
        }
        len
    }

    /// Paint a pixel by world pixel column and world half-row.
    fn paint(&mut self, x: f64, yh: f64, color: u32, a: f64) {
        self.paint_p(x, self.base - yh, color, a);
    }

    /// Paint a pixel by its grid-pixel position (row py from the top).
    fn paint_p(&mut self, x: f64, py: f64, color: u32, a: f64) {
        // World pixels (whole ones: a fraction would index nothing), seen from where the camera is along the ground.
        let x = x.floor() - self.view_x();
        let (pw, ph) = (self.pw as f64, self.ph as f64);
        if x < 0.0 || x >= pw || py < 0.0 || py >= ph || a <= 0.03 {
            return;
        }
        let k = (py * pw + x) as usize;
        let old = f64::from(self.pa[k]);
        if a >= 1.0 {
            self.pc[k] = color;
            self.pa[k] = 1.0;
        } else if old == 0.0 {
            self.pc[k] = color;
            self.pa[k] = a as f32;
        } else {
            self.pc[k] = mix(self.pc[k], color, a);
            self.pa[k] = (old + a * (1.0 - old)) as f32;
        }
        let cell = (i32_of(py) >> 1) as usize * self.world.columns + (i32_of(x) >> 1) as usize;
        if self.touched[cell] == 0 {
            self.touched[cell] = 1;
            self.list[self.n_touched] = cell;
            self.n_touched += 1;
        }
    }

    /// The site over the sky: tower, mount, Earth, particles, plume, rocket, arms, lights.
    pub(super) fn draw_site(&mut self, out: &mut Cells) {
        self.geo();
        self.base = 2.0 * (self.scroll() + self.rows() - 2.0) + 1.0;
        if !self.site_hidden() {
            self.draw_tower();
            self.draw_mount();
        }
        self.draw_earth(out);
        self.draw_particles();
        if self.orbit > 0.02 {
            self.draw_tilted_plume();
        } else {
            self.draw_plume();
        }
        self.draw_rocket();
        if self.look().catches {
            self.draw_arms();
        }
        if !self.site_hidden() {
            self.draw_lights();
        }
        self.composite(out);
    }

    fn draw_tower(&mut self) {
        let s = self.spec();
        let tw = s.tower_w;
        let col = self.look().tower;
        let mz = self.look().mechazilla;
        let top = (s.tower_h - 1.0).min(self.base);
        let mut y = (self.base - self.ph as f64 + 1.0).max(0.0);
        while y <= top {
            let x0 = self.tx;
            self.paint(x0, y, col, 1.0);
            self.paint(x0 + tw - 1.0, y, col, 1.0);
            if mz && tw >= 6.0 {
                // X-bracing.
                let t = y % (tw - 2.0);
                self.paint(x0 + 1.0 + t, y, col, 1.0);
                self.paint(x0 + tw - 2.0 - t, y, col, 1.0);
            } else if !mz && y % 4.0 == 0.0 {
                let mut x = 1.0;
                while x < tw - 1.0 {
                    self.paint(x0 + x, y, col, 1.0);
                    x += 1.0;
                }
            } else {
                self.paint(x0 + 1.0 + (y % (tw - 2.0)), y, col, 0.85);
            }
            y += 1.0;
        }
        if self.tall {
            // A lightning rod on top.
            let rx = self.tx
                + if mz {
                    f64::from(i32_of(tw) >> 1)
                } else {
                    tw - 1.0
                };
            let mut y = s.tower_h;
            while y < s.tower_h + if mz { 4.0 } else { 2.0 } {
                self.paint(rx, y, 0x8a9099, 1.0);
                y += 1.0;
            }
            // The carriage the arms ride on.
            if mz {
                let ay = round(self.arm_y);
                let mut y = ay - 1.0;
                while y <= ay + s.arm_thick {
                    let mut x = 0.0;
                    while x < tw {
                        self.paint(self.tx + x, y, self.look().carriage, 1.0);
                        x += 1.0;
                    }
                    y += 1.0;
                }
            }
            // The service arm (crew access / ship quick-disconnect), swung away for flight.
            if let Some(service) = s.service {
                let y = s.mount + (s.fly.hf() - 1.0 - service);
                let full = self.tx - (self.body_px + s.body_w);
                let len = round(full * self.service).max(1.0);
                let mut x = 1.0;
                while x <= len {
                    self.paint(self.tx - x, y, col, 1.0);
                    x += 1.0;
                }
                self.paint(self.tx - 1.0, y - 1.0, col, 0.8);
            }
        }
    }

    /// Starship stands on the orbital launch mount: a table on legs.
    fn draw_mount(&mut self) {
        let m = self.spec().mount;
        if m <= 0.0 {
            return;
        }
        let l = self.body_px - 2.0;
        let r = self.body_px + self.spec().body_w + 1.0;
        let col = 0x80868f;
        let mut x = l;
        while x <= r {
            self.paint(x, m - 1.0, col, 1.0);
            x += 1.0;
        }
        let mut y = 0.0;
        while y < m - 1.0 {
            self.paint(l, y, col, 1.0);
            self.paint(r, y, col, 1.0);
            y += 1.0;
        }
    }

    fn draw_particles(&mut self) {
        for i in 0..PMAX {
            let life = f64::from(self.parts.life[i]);
            if life <= 0.0 {
                continue;
            }
            let f = 1.0 - life / f64::from(self.parts.max_life[i]);
            let (r0, r1) = (f64::from(self.parts.r0[i]), f64::from(self.parts.r1[i]));
            let rad = r0 + (r1 - r0) * f.sqrt();
            let a = f64::from(self.parts.a0[i]) * (1.0 - f);
            let cx = round(f64::from(self.parts.px[i]));
            let cy = round(f64::from(self.parts.py[i]));
            let color = self.parts.col[i];
            if rad < 0.8 {
                self.paint(cx, cy, color, a);
                continue;
            }
            let rx = rad.floor();
            let ry = (rad / 2.0).floor();
            let inv = 1.0 / (rad * rad);
            // Lit from above: the underside of a billow a little darker.
            let under = mix(color, 0x8e959e, 0.3);
            let mut dy = -ry;
            while dy <= ry {
                let mut dx = -rx;
                while dx <= rx {
                    let d2 = (dx * dx + 4.0 * dy * dy) * inv;
                    if d2 <= 1.0 {
                        self.paint(
                            cx + dx,
                            cy + dy,
                            if dy < 0.0 { under } else { color },
                            a * (1.0 - 0.6 * d2),
                        );
                    }
                    dx += 1.0;
                }
                dy += 1.0;
            }
        }
    }

    /// The plume on the climb: down from the nozzle, widening, splashing off the pad.
    fn draw_plume(&mut self) {
        let len = self.plume_len();
        if len < 0.5 {
            return;
        }
        let s = self.spec();
        let ramp = self.ramp();
        let (a0, cx, wk) = self.nozzle();
        // Thin air lets the plume balloon out.
        let spread = 0.08 + 0.45_f64.min(self.world.alt / 180.0);
        let t = self.world.t;
        let mut d = 0.0;
        while d < len {
            let y = a0 - 1.0 - d;
            let half = (s.body_w / 2.0) * wk + d * spread;
            if y < 0.0 {
                self.deflect(cx, half, len - d, ramp);
                break;
            }
            let k = d / len;
            // Shock diamonds in the core.
            let diamond = if self.tall && d % 5.0 == 2.0 && k < 0.6 {
                0.15
            } else {
                0.0
            };
            let xb = (cx + half).floor();
            let mut x = (cx - half).ceil();
            while x <= xb {
                let edge = (x - cx).abs() / (half + 0.5);
                let n = hash(x, d + t * 7.0, 47.0) - 0.5;
                let heat = 1.0 - 0.85 * k - 0.45 * edge * edge + diamond + n * 0.15;
                let a = ((1.0 - k) * 2.4 - edge * 0.35 + n * 0.4).min(1.0);
                self.paint(x, y, ramp_color(ramp, heat), a);
                x += 1.0;
            }
            d += 1.0;
        }
    }

    /// The plume hitting the pad, splashing sideways in fire and steam.
    fn deflect(&mut self, cx: f64, half: f64, rest: f64, ramp: &Ramp) {
        let reach = rest * 1.4 + 2.0;
        let t = self.world.t;
        for side in [-1.0, 1.0] {
            let mut s = 0.0;
            while s <= reach {
                let x = round(cx + side * (half + s));
                let k = s / reach;
                let n = hash(x, t, 53.0) - 0.5;
                let heat = 0.85 - 0.75 * k + n * 0.2;
                let a = 1.0 - k * 0.85 + n * 0.3;
                self.paint(x, 0.0, ramp_color(ramp, heat), a);
                if k > 0.3 {
                    self.paint(x, 1.0, ramp_color(ramp, heat - 0.25), a * 0.6);
                }
                s += 1.0;
            }
        }
    }

    /// The rocket as it flies now, a stage parting from it, and an upper stage being stacked back on.
    fn draw_rocket(&mut self) {
        let s = self.spec();
        let sp = if self.part == Part::Booster {
            self.booster_sprite()
        } else {
            &s.fly
        };
        let alpha = if self.part == Part::Booster {
            self.fade_in
        } else {
            1.0
        };
        self.draw_body(sp, self.part, 0.0, alpha, self.pos, false);
        if let Some(gh) = self.ghost {
            self.draw_body(&s.fly, gh.part, gh.d, gh.life / gh.max, self.pos, false);
        }
        let catches = self.look().catches;
        if self.state == State::Pan {
            // The next stack, already standing on the pad as the camera slides back to it.
            self.draw_body(&s.fly, Part::Full, s.mount - self.apx(), 1.0, 0.0, true);
        } else if catches && self.part == Part::Upper && self.booster_home && self.orbit < 0.01 {
            // Starship's booster, back on the mount, waiting for the Ship.
            self.draw_body(&s.fly, Part::Booster, s.mount - self.apx(), 1.0, 0.0, true);
        } else if catches && self.part == Part::Booster && self.state == State::Stack {
            // The Ship, on its transporter and then in the arms.
            self.draw_body(&s.fly, Part::Upper, self.stack_off, 1.0, self.ship_x, true);
        }
        self.draw_carrier();
        self.draw_chutes();
    }

    /// Dragon's parachutes over the capsule, on risers from its nose: two small
    /// drogues, then four striped mains, opening out (and collapsing on the water).
    fn draw_chutes(&mut self) {
        if self.chute == 0.0 || self.chute_open <= 0.0 {
            return;
        }
        let s = self.spec();
        let k = self.chute_open;
        let top = self.apx() + s.fly.hf() - 1.0;
        let cx = self.ox + self.pos + s.body_l + s.body_w / 2.0 - 0.5;
        let mains = self.chute == 2.0;
        let tall = self.tall;
        let size = if mains {
            if tall {
                9.0
            } else {
                3.0
            }
        } else if tall {
            5.0
        } else {
            2.0
        };
        let rise = round(size * (0.5 + 0.5 * k));
        let spread: &[f64] = if mains {
            if tall {
                &[-6.0, -2.0, 2.0, 6.0]
            } else {
                &[-1.5, 1.5]
            }
        } else if tall {
            &[-2.0, 2.0]
        } else {
            &[0.0]
        };
        let width = if mains {
            if tall {
                2.0
            } else {
                1.5
            }
        } else {
            1.0
        };
        let half = round(width * k).max(1.0);
        let riser = 0xc2c7cf;
        for &off in spread {
            let x = round(cx + off * k);
            let y = top + rise;
            // The risers, from the nose up to the canopy's edge.
            if tall {
                let mut i = 1.0;
                while i < rise {
                    let f = i / rise;
                    self.paint(round(cx + (x - cx) * f), top + i, riser, 0.7);
                    i += 1.0;
                }
            }
            // The canopy: a dome, its gores in white and orange (in the
            // session's hue, when there is one).
            let gore = match self.world.dials.accent {
                Some(hue) => tint_to(0xe8642a, hue, 0.0),
                None => 0xe8642a,
            };
            let mut dx = -half;
            while dx <= half {
                let c = if mains && (x + dx) % 2.0 != 0.0 {
                    gore
                } else {
                    0xf2f3f5
                };
                self.paint(x + dx, y, c, 1.0);
                if dx.abs() < half || half == 1.0 {
                    self.paint(x + dx, y + 1.0, c, 1.0);
                }
                dx += 1.0;
            }
        }
    }

    /// The Ship's ride home, two pixels tall under it: on the sea a barge (dark
    /// hull, deck, a wheelhouse at its stern), on land a transporter (a flatbed
    /// on wheels).
    fn draw_carrier(&mut self) {
        let cx = self.carrier_x;
        if !cx.is_finite() {
            return;
        }
        let s = self.spec();
        let x0 = round(self.ox + cx + s.body_l - 3.0);
        let x1 = x0 + s.body_w + 5.0;
        let sea = ((x0 + x1) / 4.0).floor() < self.shore();
        let mut x = x0;
        while x <= x1 {
            self.paint(x, 1.0, if sea { 0x6a6f78 } else { 0x7a7f88 }, 1.0);
            if sea {
                self.paint(x, 0.0, 0x2c3036, 1.0);
            } else if i32_of(x - x0) & 1 == 0 {
                self.paint(x, 0.0, 0x1a1c20, 1.0);
            }
            x += 1.0;
        }
        if sea {
            self.paint(x1, 2.0, 0x8a9099, 1.0);
            self.paint(x1, 3.0, 0x8a9099, 1.0);
            self.paint(x1 - 1.0, 2.0, 0x8a9099, 1.0);
        }
    }

    /// One part of the stack, posed with the rocket, `d` sprite rows further
    /// along its axis, faded to `alpha`, at `pos_x` along the ground.
    /// `upright`: something standing still on the ground (not turned with what's flying).
    fn draw_body(
        &mut self,
        sp: &Sprite,
        part: Part,
        d: f64,
        alpha: f64,
        pos_x: f64,
        upright: bool,
    ) {
        if alpha <= 0.03 {
            return;
        }
        let s = self.spec();
        let (r0, r1) = self.part_rows(part);
        let frost =
            self.look().mechazilla && (self.state == State::Rest || self.state == State::Ignite);
        let (cx0, cy0, th, sc) = self.pose(sp, pos_x, !upright);
        let cs = th.cos();
        let sn = th.sin();
        let cx = cx0 + 2.0 * d * sn * sc;
        let cy = cy0 - d * cs * sc;
        // Every grid pixel the rotated sprite might cover, sampled back into the
        // sprite (a pixel is twice as tall as it is wide).
        let (w, h) = (sp.wf(), sp.hf());
        let rx = ((cs.abs() * w / 2.0 + sn.abs() * h) * sc).ceil() + 1.0;
        let ry = ((sn.abs() * w / 4.0 + cs.abs() * h / 2.0) * sc).ceil() + 1.0;
        let inv = 1.0 / sc;
        // The engines glow while they burn, at the bottom of whatever is flying.
        let burning = d == 0.0 && part == self.part && self.thr > 0.0;
        // Dragon's capsule coming in: its heat shield glowing, then cooling under the parachutes.
        let heat = if (part == Part::Capsule || part == Part::Dragon)
            && self.burn > 20.0
            && self.burn < 95.0
        {
            (PI * (self.burn - 20.0) / 75.0).sin()
        } else {
            0.0
        };
        // Falcon's booster just before its entry burn: the engine end warming.
        let base_heat = if part == Part::Booster
            && self.booster_only
            && !self.look().catches
            && self.state == State::Fly
            && self.staged == 5.0
        {
            0.5
        } else {
            0.0
        };
        // The Ship coming in belly-first: its tiles glowing, the steel above them less.
        let ship_heat =
            if part == Part::Upper && self.look().catches && self.burn > 0.0 && self.burn < 90.0 {
                (PI * self.burn / 90.0).sin()
            } else {
                0.0
            };
        let tiles_to = s.body_l + (s.body_w / 2.0).floor();
        let blue = self.world.dials.tint == Tint::Blue;
        let ph = self.ph as f64;
        let py_end = (cy + ry).ceil();
        let px_end = (cx + rx).ceil();
        let mut py = (cy - ry).floor();
        while py <= py_end {
            if py < 0.0 || py >= ph {
                py += 1.0;
                continue;
            }
            let uy = (py + 0.5 - cy) * 2.0;
            let mut px = (cx - rx).floor();
            while px <= px_end {
                let ux = px + 0.5 - cx;
                let col = ((ux * cs + uy * sn) * inv + w / 2.0).floor();
                let row = (((-ux * sn + uy * cs) * inv) / 2.0 + h / 2.0).floor();
                if col < 0.0 || col >= w || row < r0 || row >= r1 {
                    px += 1.0;
                    continue;
                }
                let c0 = sp.c[(row * w + col) as usize];
                if c0 < 0 {
                    px += 1.0;
                    continue;
                }
                let mut c = c0 as u32;
                // The session's hue (crabigator's): a wash over the white and
                // stainless body, the trunk's cells in it outright.
                if let Some(hue) = self.world.dials.accent {
                    c = livery(c, hue);
                }
                // Burning up coming down: glowing hotter from the bottom, its leading end.
                if heat > 0.0 {
                    c = mix(
                        c,
                        0xff7a2a,
                        (heat * (0.4 + (0.6 * (row - r0)) / (r1 - r0).max(1.0))).min(0.9),
                    );
                }
                if ship_heat > 0.0 {
                    let tiles = col < tiles_to;
                    c = mix(
                        c,
                        if tiles { 0xff6a2a } else { 0xffb070 },
                        ship_heat * if tiles { 0.85 } else { 0.35 },
                    );
                }
                if base_heat > 0.0 && row >= r1 - 3.0 {
                    c = mix(c, 0xd8402a, base_heat * (1.0 - (r1 - 1.0 - row) / 3.0));
                }
                // Blue-white near a full context.
                if burning && row == r1 - 1.0 && col >= s.body_l && col < s.body_l + s.body_w {
                    c = mix(
                        c,
                        if blue { 0x8cc0ff } else { 0xffc46a },
                        (self.thr * 1.4).min(0.95),
                    );
                }
                // Super Heavy frosts over where the cold propellant sits.
                if frost && row > h * 0.6 && row < h - 2.0 && hash(col, row, 61.0) < 0.55 {
                    c = mix(c, 0xf4f8fc, 0.6);
                }
                self.paint_p(px, py, c, alpha);
                px += 1.0;
            }
            py += 1.0;
        }
    }

    /// The plume in orbit: a thin burn trailing back along the tilted axis.
    fn draw_tilted_plume(&mut self) {
        let len = self.plume_len();
        if len < 0.5 {
            return;
        }
        let s = self.spec();
        let sp = &s.fly;
        let (cx, cy, th, sc) = self.pose(sp, self.pos, true);
        let cs = th.cos();
        let sn = th.sin();
        let ramp = self.ramp();
        // The tail (the bottom of what's flying), in units (a pixel is 1 wide, 2 tall), and the axis pointing aft.
        let aft = 2.0 * self.part_rows(self.part).1 - sp.hf();
        let tx = cx - aft * sn * sc;
        let ty = cy * 2.0 + aft * cs * sc;
        let l = len * 2.0 * sc;
        let wk = self.nozzle().2;
        let fade = if self.world.dials.tint == Tint::Normal {
            0.55
        } else {
            0.85
        };
        let t = self.world.t;
        let mut d = 0.5;
        while d < l {
            let k = d / l;
            let half = (s.body_w / 2.0) * wk * sc + d * 0.12;
            let mut e = -half;
            while e <= half {
                let x = (tx - sn * d + cs * e).floor();
                let y = ((ty + cs * d + sn * e) / 2.0).floor();
                let edge = e.abs() / (half + 0.5);
                let n = hash(x, y + t * 7.0, 47.0) - 0.5;
                let heat = 1.0 - 0.85 * k - 0.45 * edge * edge + n * 0.15;
                let a = ((1.0 - k) * 1.6 - edge * 0.4 + n * 0.3).min(1.0) * fade;
                self.paint_p(x, y, ramp_color(ramp, heat), a);
                e += 0.75;
            }
            d += 1.0;
        }
    }

    /// Earth below the orbit: a curved limb of ocean and cloud sliding back.
    /// The limb's own cell in each column is drawn straight into the cells as a
    /// lower-block glyph filled to the curve's height (to an eighth of a cell,
    /// dithered a little per column), so the curve reads smoothly instead of in
    /// pixel-high steps; the ocean below it goes through the pixel layer.
    fn draw_earth(&mut self, out: &mut Cells) {
        let o = self.orbit;
        if o <= 0.01 {
            return;
        }
        let (w, h) = (self.world.columns, self.rows());
        let wf = w as f64;
        let ph = self.ph as f64;
        let h0 = if self.tall { 6.0 } else { 2.0 };
        let h1 = if self.tall { 2.5 } else { 0.4 };
        let ex = self.earth_x.floor();
        for c in 0..w {
            let cf = c as f64;
            // The limb's top in cells, at this column's centre.
            let rel = (cf + 0.5 - wf / 2.0) / (wf / 2.0);
            let top = (ph - (h0 - (h0 - h1) * rel * rel)) / 2.0;
            let rc = (h - 1.0).min(top.floor());
            let eighths = ((rc + 1.0 - top) * 8.0 + hash(cf, 0.0, 73.0))
                .floor()
                .clamp(0.0, 8.0);
            let i = (rc * wf) as usize + c;
            let behind = out.background(i);
            // A sliver is all bright limb; a fuller cell mostly the ocean under it.
            let limb = mix(LIMB, OCEAN, ((eighths - 2.0) / 6.0).max(0.0) * 0.65);
            out.set(i, lower_block(eighths), mix(behind, limb, o), behind);
            // The ocean (with cloud and land sliding by) in the whole cells below.
            let mut y = 2.0 * (rc + 1.0);
            while y < ph {
                for x in [2.0 * cf, 2.0 * cf + 1.0] {
                    let n = hash(f64::from(i32_of(x + ex) >> 1), y, 71.0);
                    let m = hash(
                        f64::from(i32_of(x + ex + 1.0) >> 2),
                        f64::from(i32_of(y) >> 1),
                        72.0,
                    );
                    let color = if n < 0.16 || m < 0.12 {
                        0xdde6f0
                    } else if m > 0.86 {
                        0x4c7b45
                    } else {
                        OCEAN
                    };
                    self.paint_p(x, y, color, o);
                }
                y += 1.0;
            }
        }
    }

    /// The catch arms: foreshortened stubs when swung open, across the rocket when shut.
    fn draw_arms(&mut self) {
        let s = self.spec();
        let ay = round(self.arm_y);
        let start = self.tx - 1.0;
        // Mechazilla's chopsticks reach well past the booster.
        let tip = self.body_px + self.arm_x
            - 1.0
            - if self.look().mechazilla && self.tall {
                2.0
            } else {
                0.0
            };
        let full = start - tip + 1.0;
        let len = s.arm_stub + round(self.reach * (full - s.arm_stub));
        let col = self.look().arm;
        let mut t = 0.0;
        while t < s.arm_thick {
            let mut x = 0.0;
            while x < len {
                self.paint(start - x, ay + t, col, 1.0);
                x += 1.0;
            }
            t += 1.0;
        }
        if self.reach >= 1.0 && self.tall {
            // The pincers' tips closing round the far side.
            self.paint(tip, ay - 1.0, col, 1.0);
            self.paint(tip, ay + s.arm_thick, col, 1.0);
        }
    }

    fn draw_lights(&mut self) {
        let s = self.spec();
        let blue = self.world.dials.tint == Tint::Blue;
        let mz = self.look().mechazilla;
        let top = if self.tall {
            s.tower_h + if mz { 4.0 } else { 2.0 }
        } else {
            s.tower_h - 1.0
        };
        let tx = if self.tall {
            self.tx
                + if mz {
                    f64::from(i32_of(s.tower_w) >> 1)
                } else {
                    s.tower_w - 1.0
                }
        } else {
            self.tx + s.tower_w - 1.0
        };
        let t = self.world.t;
        // A failed command: the lights burn low. Blue: blue lights all the way up the tower.
        let dim = if self.world.dials.tint == Tint::Smoke {
            0.45
        } else {
            1.0
        };
        if t % 24.0 < 6.0 {
            self.paint(tx, top, if blue { LIGHT_BLUE } else { LIGHT_RED }, dim);
        }
        if blue {
            let (mut y, mut k) = (2.0, 0.0);
            while y < top - 1.0 {
                if (t + k * 5.0) % 24.0 < 14.0 {
                    let side = if i32_of(k) & 1 != 0 {
                        0.0
                    } else {
                        s.tower_w - 1.0
                    };
                    self.paint(self.tx + side, y, LIGHT_BLUE, 1.0);
                }
                y += if self.tall { 6.0 } else { 3.0 };
                k += 1.0;
            }
        }
        // A light per few subagents, blinking out of step.
        let coverage = self.world.dials.coverage_boost;
        let extra = if coverage <= 0.0 {
            0
        } else if coverage < 30.0 {
            1
        } else {
            2
        };
        for k in 1..=extra {
            let kf = f64::from(k);
            if (t + kf * 8.0) % 24.0 >= 6.0 {
                continue;
            }
            let y = round((s.tower_h * (3.0 - kf)) / 4.0);
            let side = if k & 1 != 0 { 0.0 } else { s.tower_w - 1.0 };
            let color = if blue {
                LIGHT_BLUE
            } else if k == 1 {
                LIGHT_AMBER
            } else {
                LIGHT_GREEN
            };
            self.paint(self.tx + side, y, color, dim);
        }
    }

    /// Fold each touched cell's four pixels into the two colours that best fit them.
    fn composite(&mut self, out: &mut Cells) {
        let w = self.world.columns;
        let pw = self.pw;
        let mut q = [0u32; 4];
        for n in 0..self.n_touched {
            let cell = self.list[n];
            self.touched[cell] = 0;
            let r = cell / w;
            let c = cell - r * w;
            let behind = out.behind(cell);
            let k0 = 2 * r * pw + 2 * c;
            for (p, qp) in q.iter_mut().enumerate() {
                let k = k0 + if p & 2 != 0 { pw } else { 0 } + (p & 1);
                let a = f64::from(self.pa[k]);
                *qp = if a == 0.0 {
                    behind
                } else if a >= 1.0 {
                    self.pc[k]
                } else {
                    mix(behind, self.pc[k], a)
                };
                self.pa[k] = 0.0;
            }
            // The two colours that best fit them, each the plain average of its pixels.
            fit_quad(&q, &mut self.fit, i32::MAX, -1);
            if self.fit.spread == 0 {
                out.set(cell, 0x20, DEFAULT_COLOR, q[0]);
            } else {
                out.set(cell, QUAD[self.fit.mask as usize], self.fit.fg, self.fit.bg);
            }
        }
        self.n_touched = 0;
    }
}

/// The trunk's solar cells, which take the session's hue outright.
const TRUNK: u32 = 0x23315a;

/// A body colour in the session's hue: the trunk's cells outright, the white
/// and stainless body (anything light) washed with it, the dark parts as they are.
fn livery(c: u32, hue: f64) -> u32 {
    thread_local! {
        static MEMO: std::cell::RefCell<std::collections::HashMap<(u32, u64), u32>> =
            std::cell::RefCell::new(std::collections::HashMap::new());
    }
    MEMO.with(|memo| {
        let key = (c, hue.to_bits());
        if let Some(&hit) = memo.borrow().get(&key) {
            return hit;
        }
        let out = if c == TRUNK {
            tint_to(c, hue, 0.0)
        } else if lightness(c) >= 0.6 {
            tint_to(c, hue, 0.045)
        } else {
            c
        };
        let mut memo = memo.borrow_mut();
        if memo.len() > 256 {
            memo.clear();
        }
        memo.insert(key, out);
        out
    })
}
