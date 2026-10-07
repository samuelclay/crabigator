//! The band: a side view of the run. The slope crosses the band under a
//! winter-blue sky, far peaks and near hills scrolling behind at their own
//! pace; the skiers weave toward and away from you, their tracks trailing
//! across the slope's face.

use crate::flow::js::{i32_of, max, min, round};
use crate::flow::night::moon_pixel;
use crate::flow::pixels::{clamp, hash_murmur as hash, mix};

use super::{noise, Ski, View, B_CARVE, B_DOWN, B_SKI, B_STAND, B_TUCK, POLE, SPEED, TRK};

impl Ski {
    pub(super) fn draw_band(&mut self) {
        let pal = self.pal();
        let (w, h) = (self.pw, self.ph);
        let (wf, hf) = (w as f64, h as f64);
        let face_h = max(2.0, round(hf * 0.3));
        let g0 = hf - face_h;
        let cam = self.d;
        let sx0 = self.anchor_x(0);
        let tilt = self.steep * min(5.0, hf * 0.45);
        let kick = if self.jump_at >= 0.0 {
            self.jump_at + sx0
        } else {
            -1e9
        };
        let kick2 = self.last_kick + sx0;
        // Night: how dark, how much of the sky goes black (dusk keeps its glow), the moon.
        let night = self.k_night;
        let band_black = 1.0 - self.k_dusk;
        let (mx, my) = moon_pixel(self.columns as f64, self.rows as f64);

        // The slope's surface per column: tilted with the steepness, moguls, kickers.
        for x in 0..w {
            let xf = x as f64;
            let wx = cam + xf;
            let mut s = g0 + (tilt * (xf - sx0)) / wf;
            if self.moguls > 0.01 {
                let m = (wx * 0.42 + 3.0 * noise(wx * 0.03, 11.0)).sin();
                s -= self.moguls * 1.1 * m * m;
            }
            for k in [kick, kick2] {
                let u = k - wx;
                if (0.0..12.0).contains(&u) {
                    s -= (2.4 * (12.0 - u)) / 12.0;
                }
            }
            self.ground[x] = s as f32;
        }

        // Sky, far peaks, the near hills and their pines, then the slope.
        let far_x = cam * 0.06;
        let hill_x = cam * 0.3;
        for x in 0..w {
            let xf = x as f64;
            let fx = far_x + xf;
            let ridge = noise(fx / 26.0, 1.0) * 0.62 + noise(fx / 9.0, 2.0) * 0.38;
            let peak = hf * 0.06 + hf * 0.62 * (1.0 - ridge);
            let ridge_r =
                noise((fx + 1.0) / 26.0, 1.0) * 0.62 + noise((fx + 1.0) / 9.0, 2.0) * 0.38;
            let lit = ridge_r <= ridge;
            let cap = hf * (0.1 + 0.12 * noise(fx / 7.0, 3.0));
            let hx = hill_x + xf;
            let hill = g0 - 0.5 - hf * 0.22 * noise(hx / 33.0, 4.0);
            // A pine on the near hills every few pixels.
            let slot = (hx / 4.0).floor();
            let pc = slot * 4.0 + 2.0;
            let has_pine = hash(slot, 0.0, 5.0) < 0.4;
            let pine_h = 1.5 + hash(slot, 1.0, 5.0) * 1.8;
            let pine_base = g0 - 0.5 - hf * 0.22 * noise(pc / 33.0, 4.0);
            let surf = f64::from(self.ground[x]);
            for y in 0..h {
                let yf = y as f64;
                let k = y * w + x;
                if yf >= surf {
                    // The slope's face: lit at the lip, bluer below, mogul shadows on the far sides.
                    let depth = (yf - surf) / face_h;
                    let mut c = mix(pal.snow, pal.snow_shade, clamp(depth * 0.6, 0.0, 1.0));
                    if self.moguls > 0.01 {
                        let wx = cam + xf;
                        let m = (wx * 0.42 + 3.0 * noise(wx * 0.03, 11.0)).cos();
                        if m > 0.4 && yf - surf < 1.5 {
                            c = mix(c, pal.snow_deep, self.moguls * 0.5);
                        }
                    }
                    if yf - surf < 1.0 && yf + 1.0 > surf {
                        c = mix(pal.snow, c, 0.5);
                    }
                    self.pix[k] = c;
                    continue;
                }
                let mut c = mix(pal.sky_top, pal.sky_low, yf / max(1.0, g0));
                // By night the band's sky is plain black but for the moon and the stars
                // (as surf's): in so few rows a glow reads as stripes.
                if night > 0.0 && yf < peak {
                    let sky = self.night_sky(mix(c, 0, band_black), xf, yf, fx.floor(), mx, my);
                    c = mix(c, sky, night);
                }
                if yf >= peak {
                    let in_cap = yf < peak + cap;
                    c = match (in_cap, lit) {
                        (true, true) => pal.cap,
                        (true, false) => pal.cap_shade,
                        (false, true) => pal.peak,
                        (false, false) => pal.peak_shade,
                    };
                }
                if yf >= hill {
                    c = mix(pal.hill, pal.snow_shade, clamp((yf - hill) / 4.0, 0.0, 0.5));
                }
                if has_pine && yf < pine_base + 0.5 && yf >= pine_base - pine_h {
                    let tw = ((yf - (pine_base - pine_h)) / pine_h) * 1.3 + 0.2;
                    if (hx - pc).abs() <= tw {
                        c = pal.hill_pine;
                    }
                }
                // The surface's own row: a soft lip.
                if yf + 1.0 > surf {
                    c = mix(c, pal.snow, yf + 1.0 - surf);
                }
                self.pix[k] = c;
            }
        }

        // The lift hut at the top of the run (where the skier starts).
        self.draw_hut(sx0 - 16.0 - cam);

        // Back row: pines along the slope's lip, gates, rocks.
        self.draw_band_scenery(false);
        self.draw_kicker(kick - cam);
        self.draw_kicker(kick2 - cam);

        // Tracks across the face, then the skiers (back to front by depth), then the snow they throw.
        let n = self.extras();
        for i in 0..=n {
            self.band_track(i, face_h);
        }
        self.draw_particles(View::Band { cam });
        let mut order: Vec<usize> = (0..=n).collect();
        order.sort_by(|&a, &b| {
            let (a, b) = (self.skiers[a].lat, self.skiers[b].lat);
            a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal)
        });
        // Front row: a few big pines whipping past at speed. Drawn before the
        // skiers, so they pass behind them: crossing in front, a near-black pine
        // at night blinks the skier out and back again and again.
        self.draw_band_scenery(true);
        for i in order {
            self.band_skier(i, face_h);
        }
    }

    /// The row of a skier's feet on the slope's face: `lat` -1 at its lip, 1 at its foot.
    fn feet_y(&self, x: f64, lat: f64, face_h: f64) -> f64 {
        let sxp = clamp(round(x), 0.0, (self.pw - 1) as f64);
        f64::from(self.ground[sxp as usize]) + 0.3 + ((lat + 1.0) / 2.0) * max(0.5, face_h - 1.4)
    }

    fn band_track(&mut self, i: usize, face_h: f64) {
        let cam = self.d;
        let c = self.pal().track;
        let (head, len) = (self.skiers[i].head, self.skiers[i].len);
        let (mut px, mut py) = (f64::NAN, f64::NAN);
        for k in 1..=len {
            let j = (head + TRK - k) % TRK;
            let wx = f64::from(self.skiers[i].track[j * 2]);
            let lat = f64::from(self.skiers[i].track[j * 2 + 1]);
            if wx.is_nan() {
                px = f64::NAN;
                continue;
            }
            let x = wx - cam - 1.0;
            if x < -2.0 {
                break;
            }
            if x >= self.pw as f64 {
                continue;
            }
            let y = self.feet_y(x, lat, face_h) * 2.0 + 1.0;
            if !px.is_nan() {
                self.dot_line(px, py, x, y, c);
            }
            px = x;
            py = y;
        }
    }

    fn band_skier(&mut self, i: usize, face_h: f64) {
        let (sfx, lat, off, kit) = {
            let s = &self.skiers[i];
            (s.fx, s.lat, s.off, s.kit)
        };
        let pal = self.pal();
        let x = 2.0 * round((sfx - self.d) / 2.0);
        let hero = i == 0;
        let y = self.feet_y(x, lat, face_h);
        let air = if hero { self.air } else { 0.0 };
        if air > 0.3 {
            // A little shadow on the snow below.
            for k in -2..=3 {
                let k = f64::from(k);
                self.blend(x + k, y, pal.shadow, 0.55 - (k - 0.5).abs() * 0.08);
            }
        }
        // Whole cells (two pixels) both ways, like x: a sprite straddling a cell
        // boundary gets its colors re-paired in every two-color cell each time it
        // bobs a pixel, and the kit seems to swap places (worst on the dark night snow).
        let fy = 2.0 * round((y - air) / 2.0);
        if hero && self.fall == 1 && self.fall_t > 2 {
            self.sprite(&B_DOWN, x, fy, kit, 0.0);
            return;
        }
        let fast = self.v > SPEED[8] * 0.92;
        let standing = self.v < 0.12 || (hero && self.fall == 2);
        let mut spr = if standing {
            &B_STAND
        } else if fast {
            &B_TUCK
        } else {
            &B_SKI
        };
        // Leaning on the edge mid-turn.
        if !standing && !fast && (self.phi + off).cos().abs() > 0.8 {
            spr = &B_CARVE;
        }
        self.sprite(spr, x, fy, kit, 0.0);
        // Deep powder buries the skis.
        if self.steep > 0.75 && air <= 0.3 {
            for k in -2..=4 {
                let a = clamp((self.steep - 0.75) * 3.0, 0.0, 0.75);
                self.blend(x + f64::from(k), fy, pal.spray, a);
            }
        }
    }

    /// The scenery along the run: behind the skiers, pines along the slope's
    /// lip, slalom gates and rocks; in `front`, big near pines at speed.
    fn draw_band_scenery(&mut self, front: bool) {
        let pal = self.pal();
        let (wf, hf) = (self.pw as f64, self.ph as f64);
        let l = self.level() as f64;
        if front {
            if l < 8.0 {
                return;
            }
            // Big near pines (closer, so faster) at the highest levels.
            let par = 1.5;
            let cam = self.d * par;
            let span = 70.0;
            let mut slot = (cam / span).floor() - 1.0;
            while slot <= ((cam + wf) / span).floor() + 1.0 {
                if hash(slot, 9.0, 21.0) <= 0.12 * (l - 7.0) {
                    let x = slot * span + hash(slot, 3.0, 21.0) * span - cam;
                    self.pine(x, hf + 1.0, hf * 0.85, true);
                }
                slot += 1.0;
            }
            return;
        }
        let cam = self.d;
        let span = 13.0;
        let dens = 0.22 + self.steep * 0.4;
        let mut slot = (cam / span).floor() - 1.0;
        while slot <= ((cam + wf) / span).floor() + 1.0 {
            let here = slot;
            slot += 1.0;
            let h1 = hash(here, 0.0, 31.0);
            let x = here * span + (hash(here, 1.0, 31.0) * (span - 4.0)).floor() - cam;
            let xi = round(x);
            if xi < -8.0 || xi > wf + 8.0 {
                continue;
            }
            let base = f64::from(self.ground[clamp(xi, 0.0, wf - 1.0) as usize]);
            // Near the hut at the start the run is kept clear.
            if here * span < self.anchor_x(0) + 4.0 && here * span > self.anchor_x(0) - 30.0 {
                continue;
            }
            if h1 < dens {
                let ht = min(hf * 0.75, 3.5 + hash(here, 2.0, 31.0) * (hf * 0.45));
                self.pine(x, base + 0.6, ht, false);
            } else if h1 < dens + self.gates * 0.5 {
                // A slalom gate: a pole and its flag, standing in the face.
                let red = i32_of(here) & 1 != 0;
                let y = round(base + 1.0 + hash(here, 4.0, 31.0));
                for k in 0..3 {
                    self.put(xi, y - f64::from(k), if red { 0xe23b3b } else { 0x2f6fe0 });
                }
                let flag = if red { 0xff4d4d } else { 0x3d82ff };
                self.put(xi + 1.0, y - 2.0, flag);
                self.put(xi + 1.0, y - 1.0, flag);
            } else if h1 > 0.93 {
                // A rock poking out of the snow.
                let y = round(base + 1.0 + hash(here, 4.0, 31.0));
                self.put(xi, y, pal.rock);
                self.put(xi + 1.0, y, pal.rock);
                self.put(xi + 2.0, y, mix(pal.rock, pal.snow_shade, 0.4));
                self.put(xi + 1.0, y - 1.0, pal.snow);
            }
        }
    }

    /// A side-view pine: tiers of boughs, a lit left edge, snow on the tips, a trunk.
    fn pine(&mut self, cx: f64, base: f64, ht: f64, dark: bool) {
        let pal = self.pal();
        let top = base - ht;
        let yb = base.floor();
        let pine = if dark {
            mix(pal.pine, 0x000000, 0.3)
        } else {
            pal.pine
        };
        let mut y = top.floor();
        while y < yb {
            let r = (y - top) / ht;
            let tier = ((y - top) % 2.5) / 2.5;
            let hw = r * ht * 0.42 + tier * 0.7;
            let xa = round(cx - hw);
            let xb = round(cx + hw);
            let mut x = xa;
            while x <= xb {
                let c = if x == xa {
                    if tier > 0.5 {
                        pal.pine_snow
                    } else {
                        pal.pine_lit
                    }
                } else if x - xa == 1.0 && !dark {
                    pal.pine_lit
                } else {
                    pine
                };
                self.put(x, y, c);
                x += 1.0;
            }
            y += 1.0;
        }
        self.put(round(cx), yb, pal.trunk);
    }

    /// The lift hut: walls with a lit window, a snowy roof, a flag.
    fn draw_hut(&mut self, x: f64) {
        if x < -10.0 || x > self.pw as f64 {
            return;
        }
        let pal = self.pal();
        let xi = round(x);
        let base =
            f64::from(self.ground[clamp(xi + 3.0, 0.0, (self.pw - 1) as f64) as usize]).floor();
        for k in 0..7 {
            let k = f64::from(k);
            self.put(xi + k, base, pal.hut);
            self.put(
                xi + k,
                base - 1.0,
                if k == 2.0 || k == 4.0 {
                    pal.window
                } else {
                    pal.hut
                },
            );
        }
        for k in -1..8 {
            self.put(xi + f64::from(k), base - 2.0, pal.roof);
        }
        for k in 1..6 {
            self.put(xi + f64::from(k), base - 3.0, pal.roof);
        }
        for k in 3..6 {
            self.put(xi + 8.0, base - f64::from(k), POLE);
        }
        self.put(
            xi + 9.0,
            base - 5.0,
            if (self.t >> 2) & 1 != 0 {
                0xe8302c
            } else {
                0xff5a3c
            },
        );
    }

    /// A snow kicker: a ramp up to its lip, shadowed on the drop side.
    fn draw_kicker(&mut self, x: f64) {
        if x < -14.0 || x > self.pw as f64 + 2.0 {
            return;
        }
        let pal = self.pal();
        let xi = round(x);
        let w = self.pw as f64;
        // The ramp's lit face up to the lip, then the drop in shadow behind it.
        for u in 0..12 {
            let cx = xi - f64::from(u);
            if cx < 0.0 || cx >= w {
                continue;
            }
            let top = f64::from(self.ground[cx as usize]).floor();
            self.blend(cx, top, 0xffffff, 0.9);
            self.blend(cx, top + 1.0, pal.snow_shade, 0.35);
        }
        let lip_top = f64::from(self.ground[clamp(xi, 0.0, w - 1.0) as usize]);
        let below = f64::from(self.ground[clamp(xi + 3.0, 0.0, w - 1.0) as usize]);
        let mut y = lip_top.floor() + 1.0;
        while y <= below.ceil() + 1.0 {
            self.blend(xi + 1.0, y, pal.snow_deep, 0.8);
            self.blend(xi + 2.0, y, pal.snow_shade, 0.5);
            y += 1.0;
        }
    }
}
