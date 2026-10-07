//! The spine: a three-quarter view down the fall line. The skiers carve
//! S-turns down the column, the slope scrolling up past them, forest along
//! both edges and the tracks winding away behind; above the summit, the sky
//! and the far peaks.

use std::f64::consts::PI;

use crate::flow::js::{i32_of, max, round};
use crate::flow::night::moon_pixel;
use crate::flow::pixels::{clamp, hash_murmur as hash, mix};

use super::{
    noise, noise2, Kit, Ski, View, POLE, SPEED, SPINE, SPINE_ANCHORS, S_BODY, S_DOWN, S_TUCK, TRK,
};

impl Ski {
    pub(super) fn draw_spine(&mut self) {
        let pal = self.pal();
        let (w, h) = (self.pw, self.ph);
        let (wf, hf) = (w as f64, h as f64);
        let feet0 = round(hf * SPINE_ANCHORS[0]);
        let top = self.d * SPINE - feet0; // the world row at the grid's top
        let ew = self.edge_w();
        let deep = clamp((self.steep - 0.6) / 0.4, 0.0, 1.0);
        let mg = self.moguls;

        // Sky above the summit, snow below, shaded by moguls and the lie of the land.
        for y in 0..h {
            let wy = top + y as f64;
            if wy < -1.0 {
                self.summit_row(y, wy);
                continue;
            }
            let el = ew + 2.2 * noise(wy / 17.0, 41.0) - 1.0;
            let er = wf - ew - 2.2 * noise(wy / 19.0, 42.0) + 1.0;
            for x in 0..w {
                let xf = x as f64;
                let mut c = mix(pal.snow, pal.snow_shade, 0.15);
                // The snow's lie, drawn out down the fall line as the speed blurs it.
                let n = noise2(xf / 7.0, wy / (10.0 + self.v * 9.0), 43.0);
                c = mix(c, pal.snow_shade, n * (0.35 + deep * 0.25));
                if xf < el || xf >= er {
                    c = mix(c, pal.snow_shade, 0.35);
                }
                if mg > 0.01 {
                    // Moguls on a staggered grid: lit on the uphill-left, shadowed downhill-right.
                    let row = (wy / 6.0).floor();
                    let stagger = f64::from(i32_of(row) & 1) * 3.5;
                    let mx = ((xf + stagger) % 7.0) - 3.5;
                    let my = (wy % 6.0) - 3.0;
                    let jit = hash(((xf + stagger) / 7.0).floor(), row, 44.0);
                    if jit < 0.75 {
                        let r2 = (mx * mx) / 9.0 + (my * my) / 6.0;
                        if r2 < 1.0 {
                            let s = clamp((mx * 0.35 + my * 0.55) * (1.0 - r2) * 1.4, -1.0, 1.0);
                            c = if s > 0.0 {
                                mix(c, pal.snow_shade, mg * s * 0.9)
                            } else {
                                mix(c, 0xffffff, -s * mg * 0.7)
                            };
                        }
                    }
                }
                if wy < 0.5 {
                    c = mix(c, pal.snow_shade, 0.35);
                }
                self.pix[y * w + x] = c;
            }
        }

        // Tracks, the scenery lying flat (gates, kickers), shadows; then upright things in depth order.
        let n = self.extras();
        for i in 0..=n {
            self.spine_track(i, top);
        }
        self.spine_flat(top, ew);
        self.spine_kicker(top);
        self.draw_particles(View::Spine { top });

        // Trees and skiers drawn top to bottom, so lower ones stand in front.
        let span = 4.0;
        let s0 = ((top - 4.0) / span).floor();
        let s1 = ((top + hf + 12.0) / span).floor();
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|&a, &b| {
            let (a, b) = (self.skiers[a].fy, self.skiers[b].fy);
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        });
        let mut next = 0;
        let mut slot = s0;
        while slot <= s1 {
            let wy_base = slot * span;
            while next < order.len() && self.skiers[order[next]].fy < wy_base {
                self.spine_skier(order[next], top);
                next += 1;
            }
            if wy_base >= 1.0 {
                for side in 0..2 {
                    let sf = f64::from(side);
                    let h0 = hash(slot, sf, 51.0);
                    // The forest thickens toward the run at speed: trees whipping by.
                    let reach = ew + 1.0 + self.steep * 2.0;
                    let bx = if side == 0 {
                        hash(slot, sf + 4.0, 51.0) * reach - 1.0
                    } else {
                        wf - hash(slot, sf + 4.0, 51.0) * reach + 1.0
                    };
                    if h0 < 0.85 {
                        let base = wy_base + hash(slot, sf + 2.0, 51.0) * span - top;
                        self.spine_pine(bx, base, 5.0 + hash(slot, sf + 6.0, 51.0) * 5.0);
                    }
                    // A lone pine further out on the run now and then.
                    if h0 > 0.97 {
                        let x = if side == 0 { ew + 3.0 } else { wf - ew - 3.0 };
                        self.spine_pine(x, wy_base - top, 6.0);
                    }
                }
            }
            slot += 1.0;
        }
        while next < order.len() {
            self.spine_skier(order[next], top);
            next += 1;
        }
    }

    /// The view above the summit: sky and the far peaks.
    fn summit_row(&mut self, y: usize, wy: f64) {
        let pal = self.pal();
        let w = self.pw;
        let yf = y as f64;
        let sky = mix(
            pal.sky_top,
            pal.sky_low,
            clamp((wy + 18.0) / 17.0, 0.0, 1.0),
        );
        let night = self.k_night;
        let mx = moon_pixel(self.columns as f64, self.rows as f64).0;
        for x in 0..w {
            let xf = x as f64;
            let ridge = noise(xf / 11.0, 61.0) * 0.6 + noise(xf / 4.0, 62.0) * 0.4;
            let peak = -2.0 - 11.0 * ridge;
            let ridge_r =
                noise((xf + 1.0) / 11.0, 61.0) * 0.6 + noise((xf + 1.0) / 4.0, 62.0) * 0.4;
            let mut c = sky;
            if night > 0.0 && wy < peak {
                c = mix(c, self.night_sky(c, xf, yf, xf, mx, yf - wy - 14.0), night);
            }
            if wy >= peak {
                let capped = wy < peak + 2.5 + 2.0 * noise(xf / 5.0, 63.0);
                let lit = ridge_r <= ridge;
                c = match (capped, lit) {
                    (true, true) => pal.cap,
                    (true, false) => pal.cap_shade,
                    (false, true) => pal.peak,
                    (false, false) => pal.peak_shade,
                };
            }
            self.pix[y * w + x] = c;
        }
    }

    /// A skier's tracks: two lines of dots (one per ski) winding up behind them.
    fn spine_track(&mut self, i: usize, top: f64) {
        let c = self.pal().track;
        let (head, len) = (self.skiers[i].head, self.skiers[i].len);
        let (mut ax, mut ay) = (f64::NAN, f64::NAN);
        for k in 1..=len {
            let j = (head + TRK - k) % TRK;
            let x = f64::from(self.skiers[i].track[j * 2]);
            let wy = f64::from(self.skiers[i].track[j * 2 + 1]);
            if x.is_nan() {
                ax = f64::NAN;
                continue;
            }
            let y = (wy - top) * 2.0;
            if y < -4.0 {
                break;
            }
            if !ax.is_nan() {
                self.dot_line(ax - 1.0, ay, x - 1.0, y, c);
                self.dot_line(ax + 1.0, ay, x + 1.0, y, c);
            }
            ax = x;
            ay = y;
        }
    }

    /// Slalom gates and piste markers.
    fn spine_flat(&mut self, top: f64, ew: f64) {
        let (wf, hf) = (self.pw as f64, self.ph as f64);
        let span = 16.0;
        let mut slot = (top / span).floor() - 1.0;
        while slot <= ((top + hf) / span).floor() + 1.0 {
            let here = slot;
            slot += 1.0;
            if here < 1.0 {
                continue;
            }
            let y = round(here * span - top);
            // Orange piste markers down both edges.
            if here % 2.0 == 0.0 {
                for x in [ew + 1.0, wf - ew - 2.0] {
                    self.put(x, y, 0xff8a1c);
                    self.put(x, y - 1.0, 0xff8a1c);
                }
            }
            if hash(here, 0.0, 71.0) < self.gates * 1.6 {
                // A gate: two poles and a flag between, red or blue.
                let red = i32_of(here) & 1 != 0;
                let cx = round(wf / 2.0 + (hash(here, 1.0, 71.0) - 0.5) * self.room() * 1.2);
                let col = if red { 0xe23b3b } else { 0x2f6fe0 };
                let lite = if red { 0xff6a5a } else { 0x5a9bff };
                for x in [cx - 2.0, cx + 2.0] {
                    self.put(x, y, col);
                    self.put(x, y - 1.0, col);
                    self.put(x, y - 2.0, col);
                }
                for dx in -1..=1 {
                    self.put(cx + f64::from(dx), y - 2.0, lite);
                }
                let shadow = self.pal().shadow;
                self.blend(cx + 3.0, y + 1.0, shadow, 0.4);
            }
        }
    }

    /// The kickers: a ramp lit and rising toward its lip, its drop in shadow below.
    fn spine_kicker(&mut self, top: f64) {
        let pal = self.pal();
        for k in [self.jump_at, self.last_kick] {
            if k < 0.0 {
                continue;
            }
            let lip = k * SPINE - top;
            if lip < -2.0 || lip > self.ph as f64 + 8.0 {
                continue;
            }
            let cx = self.pw as f64 / 2.0 + self.kick_x * self.room();
            for r in 0..6 {
                let r = f64::from(r);
                let y = round(lip) - r;
                let hw = 3.0 - r * 0.15;
                let mut x = round(cx - hw);
                while x <= round(cx + hw) {
                    let c = if r == 0.0 {
                        0xffffff
                    } else {
                        mix(pal.snow, 0xffffff, 0.3 + (5.0 - r) * 0.1)
                    };
                    self.put(x, y, c);
                    x += 1.0;
                }
            }
            let mut x = round(cx - 3.0);
            while x <= round(cx + 3.0) {
                self.blend(x, round(lip) + 1.0, pal.shadow, 0.7);
                self.blend(x + 1.0, round(lip) + 2.0, pal.shadow, 0.35);
                x += 1.0;
            }
        }
    }

    /// A pine seen from up the slope: a triangle of tiers with snow on it, a shadow downhill.
    fn spine_pine(&mut self, cx: f64, base: f64, ht: f64) {
        let pal = self.pal();
        if base < -1.0 || base - ht > self.ph as f64 {
            return;
        }
        let yb = round(base);
        // Shadow down and to the right.
        for k in 0..4 {
            let k = f64::from(k);
            self.blend(cx + 1.0 + k, yb + 1.0, pal.shadow, 0.45 - k * 0.08);
        }
        let top = base - ht;
        let mut y = top.floor();
        while y < yb {
            let r = (y - top) / ht;
            let tier = ((y - top) % 3.0) / 3.0;
            let hw = r * ht * 0.3 + tier * 0.8;
            let xa = round(cx - hw);
            let xb = round(cx + hw);
            let mut x = xa;
            while x <= xb {
                let c = if x == xa {
                    if tier < 0.34 {
                        pal.pine_snow
                    } else {
                        pal.pine_lit
                    }
                } else if x == xb {
                    mix(pal.pine, 0, 0.25)
                } else if tier < 0.34 && hash(x, y, 81.0) < 0.35 {
                    pal.pine_snow
                } else {
                    pal.pine
                };
                self.put(x, y, c);
                x += 1.0;
            }
            y += 1.0;
        }
        self.put(round(cx), yb, pal.trunk);
    }

    fn spine_skier(&mut self, i: usize, top: f64) {
        let (x, sfy, lat, off, kit): (f64, f64, f64, f64, Kit) = {
            let s = &self.skiers[i];
            (s.fx, s.fy, s.lat, s.off, s.kit)
        };
        let pal = self.pal();
        let hero = i == 0;
        let yg = sfy - top; // feet on the snow
        let air = if hero { self.air } else { 0.0 };
        let y = yg - air;
        if y < -8.0 || y > self.ph as f64 + 6.0 {
            return;
        }
        // Nobody stands in the sky above the summit.
        if !hero && sfy < 3.0 {
            return;
        }
        if air > 0.3 {
            for k in -2..=2 {
                let k = f64::from(k);
                self.blend(
                    x + k + round(air * 0.4),
                    yg,
                    pal.shadow,
                    0.6 - k.abs() * 0.12,
                );
            }
        }
        if hero && self.fall == 1 && self.fall_t > 2 {
            // A yard sale: lying in the snow, skis crossed, a pole flung aside.
            self.sprite(&S_DOWN, x, y, kit, 0.0);
            self.ski_line(x - 1.0, y + 1.0, 0.9, kit.skis, 5.0);
            self.ski_line(x + 1.0, y + 1.0, -0.9, kit.skis, 5.0);
            for k in 0..3 {
                self.put(x + 4.0 + f64::from(k), y - 2.0 + f64::from(k >> 1), POLE);
            }
            return;
        }
        let standing = self.v < 0.12 || (hero && self.fall == 2);
        let fast = self.v > SPEED[8] * 0.92;
        // Heading: across the hill when stopped, else down the fall line swinging with the turns.
        let ph = self.phi + off;
        let lat_v = self.amp * ph.cos() * self.rate * self.room();
        let down_v = max(0.01, self.v * SPINE * 2.0);
        let head = if standing {
            PI / 2.0
        } else {
            lat_v.atan2(down_v)
        };
        let dxs = head.sin();
        let dys = head.cos();
        let buried = self.steep > 0.75 && air <= 0.3;
        // Skis: two parallel lines through the feet along the heading (half buried in powder).
        for side in [-1.0, 1.0] {
            let gap = if standing { 0.5 } else { 1.1 };
            let ox = dys * side * gap;
            let oy = -dxs * side * gap;
            let mut t = if standing { -2.5 } else { -1.5 };
            while t <= 2.5 {
                let u = x + ox + dxs * t;
                let v = (y + 0.5) * 2.0 + oy + dys * t;
                if !(buried && t > -1.0) {
                    self.put(u, v / 2.0, kit.skis);
                }
                t += 0.5;
            }
        }
        // Poles: planted beside them when stopped, trailing behind when skiing.
        if standing {
            for k in 1..4 {
                let k = f64::from(k);
                self.put(x - 2.0, y - k, POLE);
                self.put(x + 2.0, y - k, POLE);
            }
        } else if !fast {
            for side in [-1.0, 1.0] {
                let mut t = 0.0;
                while t < 3.5 {
                    self.put(
                        x + side * 2.0 - dxs * t * 0.6 + side * t * 0.2,
                        y - 2.0 - (dys * t) / 2.0,
                        POLE,
                    );
                    t += 0.5;
                }
            }
        }
        let lean = if standing { 0.0 } else { round(-lat * 1.2) };
        self.sprite(
            if fast { &S_TUCK } else { &S_BODY },
            x,
            y - 1.0,
            kit,
            clamp(lean, -1.0, 1.0),
        );
    }

    /// A ski lying in the snow: `len` pixels through (x, y) at `slope`.
    fn ski_line(&mut self, x: f64, y: f64, slope: f64, c: u32, len: f64) {
        let mut t = -len / 2.0;
        while t <= len / 2.0 {
            self.put(x + t, y + t * slope * 0.5, c);
            t += 0.5;
        }
    }
}
