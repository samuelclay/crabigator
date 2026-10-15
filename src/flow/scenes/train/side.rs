use super::*;
impl Train {
    pub(super) fn paint_side(&mut self, px: &mut Painter, d: PixelDials, P: Pal) {
        let W = px.w as i32;
        let H = px.h as i32;
        let B = H - 1;
        let nose = self.pos;
        let nx = self.nose_x;
        let yRoof = B - 4;
        let yWin = B - 3;
        let yLow = B - 2;
        let yBog = B - 1;
        let yHz = B - 5;
        let t = d.t;
        let kn = q(d.night, 8.);
        let light = self.lights(d);
        let beam = light.max(Self::wait_glow(d) * 0.8);
        let at = |f: f64, x: i32| (nose * f + x as f64 - nx as f64 + 1.).floor();
        self.sky_row.resize(H as usize, 0);
        for y in 0..H {
            let c = mix(P.skyTop, P.skyHz, q(y as f64 / 1.max(yHz) as f64, 6.));
            self.sky_row[y as usize] = c;
            px.px[(y * W) as usize..((y + 1) * W) as usize].fill(c);
        }
        for x in 0..W {
            for layer in 0..2 {
                let u = (x as f64
                    + nose * F_CLOUD * (layer + 1) as f64
                    + t * 0.015 * (layer + 1) as f64)
                    * if layer != 0 { 0.06 } else { 0.04 };
                let th = (noise1(u, (31 + layer) as f64) - 0.56) * 7.
                    + self.k_smoke * 1.2
                    + self.k_storm * 2.;
                if th <= 0. {
                    continue;
                }
                let cy = 0.max(yHz - 4 + layer);
                let top = round(cy as f64 - th * 0.5) as i32;
                let bot = (yHz - 1).min(round(cy as f64 + th * 0.5) as i32);
                for y in 0.max(top)..=bot {
                    px.px[(y * W + x) as usize] = if y == bot && bot > top {
                        P.cloudShade
                    } else {
                        P.cloud
                    };
                }
            }
        }
        for x in 0..W {
            let far =
                round(yHz as f64 - 0.7 - 1.7 * noise1((x as f64 + nose * F_HILLFAR) * 0.02, 41.))
                    as i32;
            let near =
                round(yHz as f64 - 0.2 - 1.1 * noise1((x as f64 + nose * F_HILL) * 0.035, 43.))
                    as i32;
            for y in 0.max(far)..yHz {
                px.px[(y * W + x) as usize] = if y >= near { P.hill } else { P.hillFar };
            }
        }
        if kn > 0. {
            let (mx, my) = moon_pixel(d.columns as f64, d.rows as f64);
            let r = moon_radius(false);
            for y in 0.max((my - r).floor() as i32)..=(yHz - 1).min((my + r).ceil() as i32) {
                for x in
                    0.max((mx - r - 1.).floor() as i32)..=(W - 1).min((mx + r + 1.).ceil() as i32)
                {
                    let k = moon_cover(x as f64 + 0.5, y as f64 + 0.5, mx, my, r);
                    if k > 0. {
                        let i = (y * W + x) as usize;
                        px.px[i] = mix(px.px[i], MOON, q(k * kn, 4.));
                    }
                }
            }
        }
        if yHz >= 1 {
            for x in 0..W {
                let u = at(F_TREES, x);
                let zone = zone_of((u / F_TREES / ZONE).floor());
                let thin = if zone == WOODS {
                    0.12
                } else if zone == FIELDS {
                    0.42
                } else {
                    0.5
                };
                let j = (u / 3.).floor();
                let c = u - j * 3.;
                let lone = hash1(j * 31. + 7.) < 0.1;
                if noise1(u * 0.07, 61.) < thin && !lone {
                    continue;
                }
                let tall =
                    1 + i32::from(
                        c == 1. && hash1(j * 17. + 3.) < if zone == WOODS { 0.8 } else { 0.55 },
                    ) + i32::from(c == 1. && zone == WOODS && hash1(j * 13. + 5.) < 0.2);
                for dy in 0..tall {
                    if yHz - 1 - dy < 0 {
                        break;
                    }
                    px.px[((yHz - 1 - dy) * W + x) as usize] = if dy == tall - 1 && tall > 1 {
                        P.treeFarLit
                    } else {
                        P.treeFar
                    };
                }
            }
        }
        if kn > 0.3 {
            let n = round(W as f64 * 0.06 * kn) as i32;
            for i in 0..n {
                let x = (hash1((i * 977 + 3) as f64) * px.dw as f64).floor() as i32;
                let y = (hash1((i * 977 + 5) as f64) * 1.max(yHz - 1) as f64 * 2.).floor() as i32;
                let cell = ((y >> 1) * W + (x & !1)) as usize;
                if px.px[cell] != self.sky_row[(y >> 1) as usize]
                    || px.px[cell + 1] != self.sky_row[(y >> 1) as usize]
                {
                    continue;
                }
                let twinkle = hash1((i * 31 + (i32_of(t) >> 4)) as f64) < 0.15;
                px.dot(
                    x as f64,
                    y as f64,
                    if twinkle || hash1((i * 977 + 7) as f64) < 0.4 {
                        STAR_DIM
                    } else {
                        STAR
                    },
                );
            }
        }
        if yHz >= 0 {
            for x in 0..W {
                let u = at(F_MID, x);
                let S = u / F_MID;
                let z = (S / ZONE).floor();
                let zone = zone_of(z);
                let local = S - z * ZONE;
                let mut ground = Self::field((u / 23.).floor(), P);
                if zone == WOODS {
                    ground = P.floor;
                }
                if zone == RIVER && local > WATER0 - 30. && local < WATER1 + 30. {
                    ground = P.water;
                }
                if crosses(zone) && (local - CROSS).abs() < ROAD / F_MID {
                    ground = P.road;
                }
                px.px[(yHz * W + x) as usize] = ground;
                Self::far_things(px, x, u, yHz, P, kn);
            }
        }
        let (_, moon_y) = moon_pixel(d.columns as f64, d.rows as f64);
        for y in 0.max(yHz - 3)..=yHz {
            if kn == 0. || (y as f64 + 0.5 - moon_y).abs() > moon_radius(false) * 0.5 + 0.5 {
                smear(&mut px.px, W as usize, y, F_MID * self.v * 0.6);
            }
        }
        if yRoof >= 0 {
            for x in 0..W {
                let u = at(F_FIELD, x);
                let S = u / F_FIELD;
                let z = (S / ZONE).floor();
                let zone = zone_of(z);
                let local = S - z * ZONE;
                let mut c = Self::field((u / 31.).floor() + 977., P);
                if zone == WOODS {
                    c = if hash1((u / 3.).floor() * 13.) < 0.6 {
                        P.tree
                    } else {
                        P.treeShade
                    };
                } else if zone == RIVER && local > WATER0 && local < WATER1 {
                    c = if hash1((u / 2.).floor() + (i32_of(t) >> 3) as f64 * 7.) < 0.12 {
                        P.waterHi
                    } else {
                        P.water
                    };
                } else if zone == RIVER && local > WATER0 - 14. && local < WATER1 + 14. {
                    c = P.reed;
                } else if crosses(zone) && (local - CROSS).abs() < ROAD {
                    c = P.road;
                }
                let Sf = at(F_FARPLAT, x) / F_FARPLAT;
                let zf = (Sf / ZONE).floor();
                if zone_of(zf) == STATION && Sf - zf * ZONE > PLAT0 && Sf - zf * ZONE < PLAT1 {
                    c = P.platform;
                }
                px.px[(yRoof * W + x) as usize] = c;
            }
        }
        self.station_behind(px, d, P, yHz, &at);
        let far_grit = clamp01(1. - (self.v * F_FAR - 2.) / 3.);
        let far_joint = mix(P.railFar, P.ballastFar, q(far_grit, 3.));
        for x in 0..W {
            let S = at(F_FAR, x) / F_FAR;
            let z = (S / ZONE).floor();
            let zone = zone_of(z);
            let local = S - z * ZONE;
            let road = crosses(zone) && (local - CROSS).abs() < ROAD;
            let bridge = zone == RIVER && local > BRIDGE0 && local < BRIDGE1;
            if yWin >= 0 {
                px.px[(yWin * W + x) as usize] = if S.floor() % 24. == 0. {
                    if road {
                        P.road
                    } else if bridge {
                        P.steel
                    } else {
                        far_joint
                    }
                } else {
                    P.railFar
                };
            }
            if yLow >= 0 {
                px.px[(yLow * W + x) as usize] = if road {
                    P.road
                } else if bridge {
                    P.steel
                } else if hash1(S.floor() * 5. + 3.) < 0.3 * far_grit {
                    P.ballastFar
                } else {
                    P.verge
                };
            }
        }
        let sleeper = mix(P.sleeper, P.ballast, q((self.v - 0.6) / 0.8, 3.));
        let grit = clamp01(1. - (self.v - 2.) / 3.);
        let joint = mix(P.rail, P.ballast, q(grit, 3.));
        for x in 0..W {
            let S = nose + x as f64 - nx as f64 + 1.;
            let si = S.floor();
            let z = (S / ZONE).floor();
            let zone = zone_of(z);
            let local = S - z * ZONE;
            let road = crosses(zone) && (local - CROSS).abs() < ROAD;
            let bridge = zone == RIVER && local > BRIDGE0 && local < BRIDGE1;
            if yBog >= 0 {
                px.px[(yBog * W + x) as usize] = if road {
                    P.road
                } else if bridge {
                    P.steel
                } else if si % 3. == 0. {
                    sleeper
                } else if hash1(si * 7. + 11.) < 0.35 * grit {
                    P.ballast2
                } else {
                    P.ballast
                };
            }
            px.px[(B * W + x) as usize] = if si % 24. == 0. && !road {
                if bridge {
                    P.steel
                } else {
                    joint
                }
            } else {
                P.rail
            };
        }
        for (y, f) in [(yRoof, F_FIELD), (yWin, F_FAR), (yLow, F_FAR), (yBog, 1.)] {
            if y >= 0 {
                smear(&mut px.px, W as usize, y, f * self.v);
            }
        }
        for m in &self.crew.mates {
            if m.x.is_nan() {
                continue;
            }
            let front = nx - 1 + round(m.x) as i32;
            let livery = mix(LIVERY[m.slot % 4], NIGHT_SHADE, 0.55 * kn);
            let cab = Self::cab(m, d.t);
            for b in 0..UNIT {
                let x = front - b;
                if x < 0 || x >= W {
                    continue;
                }
                for r in 0..4 {
                    let y = yRoof - 3 + r;
                    if y < 0 {
                        continue;
                    }
                    if let Some(c) = Self::unit_side(b, r, livery, P, cab) {
                        let i = (y * W + x) as usize;
                        px.px[i] = c;
                        self.solid[i] = 1;
                    }
                }
            }
        }
        for b in 0..TRAIN {
            let x = nx - 1 - b;
            if x < 0 {
                break;
            }
            if x >= W {
                continue;
            }
            for r in 0..4 {
                let y = yRoof + r;
                if y < 0 {
                    continue;
                }
                if let Some(c) = Self::train_side(b, r, P) {
                    let i = (y * W + x) as usize;
                    px.px[i] = c;
                    self.solid[i] = 1;
                }
            }
        }
        self.in_front(px, d, P, &at, light);
        if beam > 0. {
            let len = 56.min(W - nx);
            for dx in 0..len {
                let x = nx + dx;
                let k = (1. - dx as f64 / len as f64).powf(1.6);
                for y in 0.max(yWin)..=B {
                    let spread = if y >= yBog {
                        1.
                    } else if y == yLow {
                        0.55
                    } else {
                        0.25
                    };
                    let a = q(k * spread * 0.75 * beam, 5.);
                    if a > 0. {
                        let i = (y * W + x) as usize;
                        px.px[i] = mix(px.px[i], P.beam, a);
                    }
                }
            }
        }
        self.birds(px, d, P);
        for p in &self.puffs {
            self.puff(
                px,
                P,
                p,
                self.nose_x as f64 - 1. + (p.s - self.pos),
                yRoof as f64 - 0.5 - p.up,
                p.r,
                p.r * 0.5 + 0.3,
            );
        }
        self.rain(px, d, P);
    }
    fn field(k: f64, P: Pal) -> u32 {
        let h = hash1(k * 4523. + 17.);
        if h < 0.34 {
            P.grass
        } else if h < 0.58 {
            P.grass2
        } else if h < 0.74 {
            P.wheat
        } else if h < 0.86 {
            P.plough
        } else if h < 0.93 {
            P.rape
        } else {
            P.grass
        }
    }
    fn far_things(px: &mut Painter, x: i32, u: f64, base: i32, P: Pal, kn: f64) {
        let slot = 7.;
        let j = (u / slot).floor();
        let sj = (j * slot + slot / 2.) / F_MID;
        let zj = (sj / ZONE).floor();
        let zone = zone_of(zj);
        let local = sj - zj * ZONE;
        let h = hash1(j * 6151. + 29.);
        let h2 = hash1(j * 6151. + 31.);
        let mut set = |dy: i32, c: u32, keep: bool| {
            let y = base - dy;
            if y < 0 {
                return;
            }
            if keep {
                px.set(x as f64, y as f64, c, true);
            } else {
                px.px[y as usize * px.w + x as usize] = mix(c, P.skyHz, 0.18);
            }
        };
        let wet = zone == RIVER && local > WATER0 - 60. && local < WATER1 + 60.;
        let mut p = if zone == WOODS {
            [0.9, 0.06, 0., 0.]
        } else if zone == VILLAGE || zone == STATION {
            [0.16, 0.04, 0.3, 0.03]
        } else if zone == RIVER {
            [0.3, 0.1, 0.03, 0.]
        } else {
            [0.15, 0.05, 0.05, 0.05]
        };
        if wet {
            p[2] = 0.;
            p[3] = 0.;
        }
        let mut kind = if h < p[0] {
            1
        } else if h < p[0] + p[1] {
            2
        } else if h < p[0] + p[1] + p[2] {
            3
        } else if h < p[0] + p[1] + p[2] + p[3] {
            4
        } else {
            0
        };
        if zone == VILLAGE && (local - (CROSS - 120.)).abs() < slot / F_MID / 2. {
            kind = 5;
        }
        if crosses(zone) && (local - CROSS).abs() < 20. / F_MID {
            kind = 0;
        }
        if kind == 0 {
            return;
        }
        let c = u - j * slot - (h2 * 3.).floor();
        if kind == 1 {
            let big = (zone == WOODS && h2 > 0.3) || h2 > 0.8;
            let w = if big { 4. } else { 3. };
            if c < 0. || c >= w {
                return;
            }
            let tall = if big { 3 } else { 2 };
            for dy in 0..tall {
                if dy == tall - 1 && (c == 0. || c == w - 1.) {
                    continue;
                }
                set(
                    dy,
                    if dy == tall - 1 || c == 0. {
                        P.treeLit
                    } else if c == w - 1. || dy == 0 {
                        P.treeShade
                    } else {
                        P.tree
                    },
                    false,
                );
            }
        } else if kind == 2 {
            if c != 1. {
                return;
            }
            for dy in 0..4 {
                set(dy, if dy == 3 { P.treeLit } else { P.tree }, false);
            }
        } else if kind == 3 {
            if c < 0. || c >= 4. {
                return;
            }
            let roof = if h2 > 0.6 { P.slate } else { P.roof };
            set(
                0,
                if c == 1. { P.houseWin } else { P.wall },
                c == 1. && kn > 0.3,
            );
            set(1, roof, false);
            if c >= 1. && c <= 2. {
                set(2, roof, false);
            }
        } else if kind == 4 {
            if c < 0. || c >= 5. {
                return;
            }
            set(0, P.brick, false);
            set(1, P.roofDark, false);
            if c >= 1. && c <= 3. {
                set(2, P.roofDark, false);
            }
        } else {
            if c < 0. || c >= 6. {
                return;
            }
            if c <= 1. {
                for dy in 0..4 {
                    let lit = dy == 2 && c == 0. && kn > 0.3;
                    set(dy, if lit { P.houseWin } else { P.wall }, lit);
                }
                set(4, P.slate, false);
                if c == 0. {
                    set(5, P.slate, false);
                }
            } else {
                set(
                    0,
                    if c == 3. { P.houseWin } else { P.wall },
                    c == 3. && kn > 0.3,
                );
                set(1, P.slate, false);
                if c >= 3. && c <= 4. {
                    set(2, P.slate, false);
                }
            }
        }
    }
    fn station_behind(
        &self,
        px: &mut Painter,
        d: PixelDials,
        P: Pal,
        yHz: i32,
        at: &dyn Fn(f64, i32) -> f64,
    ) {
        let W = px.w as i32;
        let kn = q(d.night, 8.);
        for x in 0..W {
            let s = at(F_FARPLAT, x) / F_FARPLAT;
            let z = (s / ZONE).floor();
            if zone_of(z) == STATION {
                let local = s - z * ZONE;
                if local > CANOPY0 && local < CANOPY1 {
                    if yHz - 2 >= 0 {
                        px.px[((yHz - 2) * W + x) as usize] = P.canopy;
                    }
                    if yHz > 0 {
                        px.px[((yHz - 1) * W + x) as usize] = P.valance;
                    }
                    if yHz >= 0 {
                        if local.floor() % 30. == 0. {
                            px.px[(yHz * W + x) as usize] = P.post;
                        } else if kn > 0.3 && local.floor() % 30. == 15. {
                            px.set(x as f64, yHz as f64, P.spill, true);
                        }
                    }
                }
            }
            let sb = at(F_STATION, x) / F_STATION;
            let zb = (sb / ZONE).floor();
            if zone_of(zb) != STATION {
                continue;
            }
            let lb = sb - zb * ZONE;
            let w0 = 420.;
            if lb < w0 || lb >= w0 + 22. / F_STATION {
                continue;
            }
            let c = ((lb - w0) * F_STATION).floor() as i32;
            for dy in 0..3 {
                let y = yHz - dy;
                if y < 0 {
                    continue;
                }
                let win = dy < 2 && c % 4 == 2;
                if win && kn > 0.3 {
                    px.set(x as f64, y as f64, P.houseWin, true);
                } else {
                    px.px[(y * W + x) as usize] = if win { P.houseWin } else { P.brick };
                }
            }
            if yHz - 3 >= 0 {
                px.px[((yHz - 3) * W + x) as usize] = P.slate;
            }
            if yHz - 4 >= 0 && c >= 16 && c <= 17 {
                px.px[((yHz - 4) * W + x) as usize] = P.brick;
            }
        }
    }
    fn in_front(
        &self,
        px: &mut Painter,
        d: PixelDials,
        P: Pal,
        at: &dyn Fn(f64, i32) -> f64,
        light: f64,
    ) {
        let W = px.w as i32;
        let H = px.h as i32;
        let B = H - 1;
        let nose = self.pos;
        let nx = self.nose_x;
        let t = d.t;
        let glow = Self::wait_glow(d);
        for x in 0..W {
            let S = nose + x as f64 - nx as f64 + 1.;
            let z = (S / ZONE).floor();
            let zone = zone_of(z);
            let local = S - z * ZONE;
            if zone == STATION && local > PLAT0 && local < PLAT1 {
                if B > 0 {
                    px.px[((B - 1) * W + x) as usize] = P.platform;
                }
                px.px[(B * W + x) as usize] = P.platformFace;
                if local.floor() % 48. == 24. && local > PLAT0 + 20. && local < PLAT1 - 20. {
                    for y in 0.max(B - 5)..=B - 2 {
                        px.px[(y * W + x) as usize] = P.post;
                    }
                    if B - 6 >= 0 {
                        px.lamp(
                            x as f64,
                            (B - 6) as f64,
                            mix(P.post, P.lamp, 0.35_f64.max(light).max(glow)),
                        );
                    }
                }
            }
            if crosses(zone) && local.floor() == CROSS + 8. {
                let c = S - 8.;
                let near = c >= nose - TRAIN as f64 - 10. && c <= nose + 260.;
                for y in 0.max(B - 5)..=B {
                    px.px[(y * W + x) as usize] = P.post;
                }
                let on = (i32_of(t) >> 3) % 2 == 0;
                if B - 6 >= 0 {
                    px.set(
                        x as f64,
                        (B - 6) as f64,
                        if near && on { RED } else { RED_OFF },
                        near && on,
                    );
                    px.set(
                        (x + 1) as f64,
                        (B - 6) as f64,
                        if near && !on { RED } else { RED_OFF },
                        near && !on,
                    );
                }
            }
            if zone == RIVER || zone_of(z + 1.) == RIVER || zone_of(z - 1.) == RIVER {
                let ut = at(F_TRUSS, x);
                let st = ut / F_TRUSS;
                let zt = (st / ZONE).floor();
                let lt = st - zt * ZONE;
                if zone_of(zt) == RIVER && lt > BRIDGE0 && lt < BRIDGE1 {
                    let top = 0.max(B - 6);
                    let p = ((ut % 16.) + 16.) % 16.;
                    let yd = top
                        + round((B - top) as f64 * if p < 8. { p / 8. } else { (16. - p) / 8. })
                            as i32;
                    let k = clamp01((self.v * F_TRUSS - 3.) / 4.);
                    let a = q(1. - 0.65 * k, 4.);
                    let haze = q(0.3 * k, 4.);
                    px.px[(top * W + x) as usize] = P.steel;
                    px.px[(B * W + x) as usize] = P.steel;
                    if haze > 0. {
                        for y in top + 1..B {
                            let i = (y * W + x) as usize;
                            px.px[i] = mix(px.px[i], P.steel, haze);
                        }
                    }
                    if yd > top && yd < B {
                        let i = (yd * W + x) as usize;
                        px.px[i] = mix(px.px[i], P.steel, a);
                    }
                    if p == 0. {
                        for y in top + 1..B {
                            let i = (y * W + x) as usize;
                            px.px[i] = mix(px.px[i], P.steel, a);
                        }
                    }
                }
            }
        }
        let mut sigs = self.extras.clone();
        let mut k = ((nose - nx as f64 - SIGNAL0) / SIGNAL).floor();
        while k * SIGNAL + SIGNAL0 < nose + (W - nx) as f64 + 2. {
            sigs.push(k * SIGNAL + SIGNAL0);
            k += 1.;
        }
        for sg in sigs {
            let x = round(nx as f64 - 1. + sg - nose) as i32;
            if x < 0 || x >= W {
                continue;
            }
            for y in 0.max(B - 5)..=B {
                px.px[(y * W + x) as usize] = P.post;
            }
            if B - 6 >= 0 {
                self.signal_lamp(px, x, B - 6, sg, glow);
            }
        }
        let blur = 1.0_f64.max(self.v * F_POLE * 0.7);
        for x in 0..W {
            let u = nose * F_POLE + x as f64 - nx as f64 + 1.;
            let p = ((u % 70.) + 70.) % 70.;
            if p >= blur {
                continue;
            }
            let s = u / F_POLE;
            let z = (s / ZONE).floor();
            let zone = zone_of(z);
            let local = s - z * ZONE;
            if (zone == STATION && local > PLAT0 - 40. && local < PLAT1 + 40.)
                || (zone == RIVER && local > BRIDGE0 - 20. && local < BRIDGE1 + 20.)
            {
                continue;
            }
            let a = q(0.25_f64.max(1.2 / blur), 4.);
            for y in 0.max(B - 8)..=B {
                let i = (y * W + x) as usize;
                px.px[i] = mix(px.px[i], P.pole, a);
            }
            if B - 8 >= 0 && blur < 2. {
                if x > 0 {
                    px.px[((B - 8) * W + x - 1) as usize] = P.pole;
                }
                if x + 1 < W {
                    px.px[((B - 8) * W + x + 1) as usize] = P.pole;
                }
            }
        }
    }
    fn train_side(b: i32, r: i32, P: Pal) -> Option<u32> {
        if b < LOCO {
            return match r {
                0 => {
                    if b < 4 {
                        None
                    } else {
                        Some(if b == 14 || b == 15 {
                            P.grille
                        } else {
                            P.carRoof
                        })
                    }
                }
                1 => {
                    if b < 2 {
                        None
                    } else {
                        Some(if b < 4 {
                            P.glass
                        } else if b < 6 {
                            P.yellow
                        } else if b < 8 {
                            P.glass
                        } else if b >= 12 && b < 22 && (b & 2) == 0 {
                            P.locoVent
                        } else if b >= LOCO - 2 {
                            P.yellow
                        } else {
                            P.loco
                        })
                    }
                }
                2 => Some(if b < 2 {
                    P.lamp
                } else if b < 4 || b >= LOCO - 2 {
                    P.yellow
                } else {
                    P.loco
                }),
                _ => {
                    if b < 4 || b >= 30 {
                        None
                    } else {
                        Some(if b < 10 || b >= 24 { P.bogie } else { P.grille })
                    }
                }
            };
        }
        let rb = b - LOCO - GAP;
        if rb < 0 {
            return if r == 2 { Some(P.bogie) } else { None };
        }
        if rb / (CAR + GAP) >= CARS {
            return None;
        }
        let cb = rb % (CAR + GAP);
        if cb >= CAR {
            return if r == 2 { Some(P.bogie) } else { None };
        }
        Some(Self::car_side(cb, r, P, P.crimson, P.cream))
    }
    fn car_side(cb: i32, r: i32, P: Pal, body: u32, band: u32) -> u32 {
        if r == 0 {
            return P.carRoof;
        }
        if r == 1 {
            if cb < 2 || cb >= CAR - 2 {
                return band;
            }
            if cb < 4 || cb >= CAR - 4 {
                return mix(band, P.vent, 0.35);
            }
            return if (cb & 2) == 0 { P.glass } else { band };
        }
        if r == 2 {
            return body;
        }
        if (cb >= 2 && cb < 8) || (cb >= CAR - 8 && cb < CAR - 2) {
            P.bogie
        } else {
            P.shadow
        }
    }
    fn unit_side(b: i32, r: i32, livery: u32, P: Pal, cab: i32) -> Option<u32> {
        if b < CAR {
            if cab == 2 && b < 4 && (r == 1 || r == 2) {
                return Some(FLASH);
            }
            if r == 0 {
                return if b < 4 { None } else { Some(P.carRoof) };
            }
            if r == 1 {
                return if b < 2 {
                    None
                } else {
                    Some(if b < 4 {
                        P.glass
                    } else if b < 8 {
                        livery
                    } else if (b & 2) == 0 {
                        P.glass
                    } else {
                        mix(livery, 0xffffff, 0.35)
                    })
                };
            }
            if r == 2 {
                return Some(if b < 2 {
                    if cab != 0 {
                        P.lamp
                    } else {
                        P.yellow
                    }
                } else if b < 4 {
                    P.yellow
                } else {
                    livery
                });
            }
            return Some(if (b >= 4 && b < 10) || (b >= CAR - 8 && b < CAR - 2) {
                P.bogie
            } else {
                P.shadow
            });
        }
        let cb = b - CAR - GAP;
        if cb < 0 {
            return if r == 2 { Some(P.bogie) } else { None };
        }
        Some(Self::car_side(
            cb,
            r,
            P,
            livery,
            mix(livery, 0xffffff, 0.35),
        ))
    }
    fn birds(&self, px: &mut Painter, d: PixelDials, P: Pal) {
        let Some(f) = &self.flock else {
            return;
        };
        let dir = if f.vx > 0. { 1. } else { -1. };
        for k in 0..f.n {
            let bx = round(f.x - dir * (k as f64 * 4. + hash1((k * 7 + 1) as f64) * 2.)) as i32;
            let by = round(f.y + (k % 2) as f64 * 2. + hash1((k * 7 + 2) as f64) * 1.5) as i32;
            let up = (((d.t + (k * 5) as f64 + f.ph) / 3.).floor() as i32 & 1) == 1;
            for (dx, dy) in [
                (-1, if up { -1 } else { 0 }),
                (0, 0),
                (1, if up { -1 } else { 0 }),
            ] {
                let x = bx + dx;
                let y = by + dy;
                if x < 0 || y < 0 || x >= px.dw as i32 || y >= px.dh as i32 {
                    continue;
                }
                let c = ((y as usize >> 2) * 2) * px.w + (x as usize & !1);
                if self.solid[c] != 0 || self.solid[c + 1] != 0 {
                    continue;
                }
                px.dot(x as f64, y as f64, P.bird);
            }
        }
    }
}
