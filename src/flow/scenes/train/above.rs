use super::*;
impl Train {
    pub(super) fn paint_above(&mut self, px: &mut Painter, d: PixelDials, P: Pal) {
        let W = px.w as i32;
        let H = px.h as i32;
        let tx = self.tx;
        let ny = self.nose_y;
        let av = (self.pos / 3.).floor() as i32;
        let kn = q(d.night, 8.);
        let light = self.lights(d);
        let glow = Self::wait_glow(d);
        let beam = light.max(glow * 0.8);
        let t = d.t;
        let sleeper = mix(P.sleeper, P.ballast, q((self.v / 3. - 0.3) / 0.5, 4.));
        let still = 1. - q((self.v / 3. - 0.8) / 1.5, 2.);
        let joint = mix(P.rail, P.ballast, 0.5 * still);
        for y in 0..H {
            self.row_v[y as usize] = av + ny - y;
        }
        for y in 0..H {
            let v = self.row_v[y as usize];
            let s = (v * 3) as f64;
            let z = (s / ZONE).floor();
            let zone = zone_of(z);
            let local = s - z * ZONE;
            let road = crosses(zone) && (local - CROSS).abs() < ROAD;
            let bridge = zone == RIVER && local > BRIDGE0 && local < BRIDGE1;
            let plat = zone == STATION && local > PLAT0 && local < PLAT1;
            let canopy = zone == STATION && local > CANOPY0 && local < CANOPY1;
            for x in 0..W {
                let dx = x - tx;
                let mut c;
                if dx >= -3 && dx <= 9 {
                    let rail = dx == -2 || dx == 1 || dx == 5 || dx == 8;
                    if road {
                        c = if rail { P.rail } else { P.road };
                    } else if rail {
                        c = if v % 8 == 0 { joint } else { P.rail };
                    } else if dx != 3 && (v & 1) == 0 {
                        c = if bridge {
                            mix(sleeper, P.steel, 0.5)
                        } else {
                            sleeper
                        };
                    } else {
                        c = if bridge {
                            P.steel
                        } else if hash(x as f64, v as f64, 5.) < 0.3 {
                            P.ballast2
                        } else {
                            P.ballast
                        };
                    }
                    if bridge && (dx == -3 || dx == 9) {
                        c = P.post;
                    }
                } else if plat && ((dx >= -6 && dx <= -4) || (dx >= 10 && dx <= 12)) {
                    c = if dx == -4 || dx == 10 {
                        P.platformEdge
                    } else {
                        P.platform
                    };
                    if canopy && dx != -4 && dx != 10 {
                        c = if (v & 3) == 0 {
                            mix(P.canopy, P.valance, 0.25)
                        } else {
                            P.canopy
                        };
                    }
                    if kn > 0.3 && (dx == -5 || dx == 11) && v % 6 == 0 {
                        c = P.spill;
                    }
                } else if road {
                    c = P.road;
                    if kn > 0.3 && (local - CROSS + ROAD).floor() == 0. && x % 10 == 3 {
                        c = P.spill;
                    }
                } else {
                    c = self.ground_above(x, v, zone, local, P, t, still);
                }
                px.px[(y * W + x) as usize] = c;
            }
        }
        self.trees_above(px, P);
        self.houses_above(px, P, kn);
        let mut sigs = self.extras.clone();
        let top_s = ((av + ny + 2) * 3) as f64;
        let bot_s = ((av + ny - H - 2) * 3) as f64;
        let mut k = ((bot_s - SIGNAL0) / SIGNAL).floor();
        while k * SIGNAL + SIGNAL0 <= top_s {
            sigs.push(k * SIGNAL + SIGNAL0);
            k += 1.;
        }
        for sg in sigs {
            let y = ny - ((sg / 3.).floor() as i32 - av);
            if y < 0 || y >= H {
                continue;
            }
            self.signal_lamp(px, tx - 4, y, sg, glow);
            px.set((tx - 4) as f64, (y + 1) as f64, P.post, false);
        }
        for (z, cars) in &self.roads {
            let vc = ((*z as f64 * ZONE + CROSS) / 3.).floor();
            let y = ny - (vc as i32 - av);
            if self.gate_shut(vc) {
                for yy in [y - 1, y, y + 1] {
                    if yy < 0 || yy >= H {
                        continue;
                    }
                    px.set(
                        (tx - 4) as f64,
                        yy as f64,
                        if (yy & 1) == 0 { 0xe03020 } else { 0xf0f0f0 },
                        true,
                    );
                    px.set(
                        (tx + 10) as f64,
                        yy as f64,
                        if (yy & 1) == 1 { 0xe03020 } else { 0xf0f0f0 },
                        true,
                    );
                }
            }
            for c in cars {
                let yy = if c.dir > 0. { y } else { y - 1 };
                if yy < 0 || yy >= H {
                    continue;
                }
                let x0 = round(c.x);
                let body = mix(c.color, NIGHT_SHADE, 0.6 * kn);
                px.set(
                    x0,
                    yy as f64,
                    if c.dir > 0. {
                        mix(body, 0xff3020, 0.6 * light)
                    } else {
                        mix(body, 0xfff6d8, 0.7 * light)
                    },
                    true,
                );
                px.set(
                    x0 + 1.,
                    yy as f64,
                    if c.dir > 0. {
                        mix(body, 0xfff6d8, 0.7 * light)
                    } else {
                        mix(body, 0xff3020, 0.6 * light)
                    },
                    true,
                );
            }
        }
        let run = 3.0_f64.min(round((self.v / 3.) * 0.7)) as i32;
        if run >= 1 {
            streak(&mut px.px, W as usize, H as usize, run);
        }
        for m in &self.crew.mates {
            if m.x.is_nan() {
                continue;
            }
            let top = ny - round(m.x / 3.) as i32;
            let livery = mix(LIVERY[m.slot % 4], NIGHT_SHADE, 0.55 * kn);
            let cab = Self::cab(m, d.t);
            for r in 0..A_UNIT {
                let y = top + r;
                if y < 0 || y >= H {
                    continue;
                }
                for i in 0..4 {
                    if let Some(c) = Self::unit_above(r, i, livery, P, cab) {
                        let at = (y * W + tx + 5 + i) as usize;
                        if at < px.px.len() {
                            px.px[at] = c;
                            self.solid[at] = 1;
                        }
                    }
                }
                if kn > 0.3 && r != A_CAR {
                    self.spill(
                        px,
                        P,
                        tx + 4,
                        tx + 9,
                        y,
                        if r > A_CAR { r - A_CAR - A_GAP } else { r },
                        kn,
                    );
                }
            }
        }
        let len = A_LOCO + CARS * (A_GAP + A_CAR);
        for r in 0..len {
            let y = ny + r;
            if y >= H {
                break;
            }
            for i in 0..4 {
                if let Some(c) = Self::train_above(r, i, P) {
                    let at = (y * W + tx - 2 + i) as usize;
                    if at < px.px.len() {
                        px.px[at] = c;
                        self.solid[at] = 1;
                    }
                }
            }
            if kn > 0.3 && r >= A_LOCO + A_GAP {
                self.spill(
                    px,
                    P,
                    tx - 3,
                    tx + 2,
                    y,
                    (r - A_LOCO - A_GAP) % (A_CAR + A_GAP),
                    kn,
                );
            }
        }
        let foot = ((PLAT0 + 90.) / 3.).floor();
        for y in 0..H {
            let s = (self.row_v[y as usize] * 3) as f64;
            let z = (s / ZONE).floor();
            if zone_of(z) != STATION || ((s - z * ZONE) / 3.).floor() != foot {
                continue;
            }
            for x in 0.max(tx - 6)..=(W - 1).min(tx + 12) {
                px.px[(y * W + x) as usize] = P.slate;
            }
        }
        if beam > 0. {
            for dy in 1..=16 {
                let y = ny - dy;
                if y < 0 {
                    break;
                }
                let half = 1. + dy as f64 * 0.2;
                let k = (1. - dy as f64 / 17.).powf(1.5) * 0.32 * beam;
                for x in
                    (tx as f64 - 0.5 - half).floor() as i32..=(tx as f64 - 0.5 + half).ceil() as i32
                {
                    if x < 0 || x >= W {
                        continue;
                    }
                    let a = q(
                        k * clamp01(half + 0.5 - (x as f64 + 0.5 - tx as f64).abs()),
                        5.,
                    );
                    if a > 0. {
                        let at = (y * W + x) as usize;
                        px.px[at] = mix(px.px[at], P.beam, a);
                    }
                }
            }
        }
        for p in &self.puffs {
            let r = p.r * (0.8 + p.up * 0.12);
            self.puff(
                px,
                P,
                p,
                self.tx as f64 - 0.5 + p.side * 2.,
                self.nose_y as f64 + (self.pos - p.s) / 3.,
                r,
                r * 0.55,
            );
        }
        self.rain(px, d, P);
    }
    fn ground_above(
        &self,
        x: i32,
        v: i32,
        zone: i32,
        local: f64,
        P: Pal,
        t: f64,
        still: f64,
    ) -> u32 {
        if zone == RIVER {
            let l = local
                + 9. * (x as f64 * 0.13 + v as f64 * 0.002).sin()
                + 4. * (x as f64 * 0.31 + 1.7).sin();
            if l > WATER0 && l < WATER1 {
                return if hash(x as f64, (v + (i32_of(t) >> 3)) as f64, 9.) < 0.06 * still {
                    P.waterHi
                } else {
                    P.water
                };
            }
            if l > WATER0 - 8. && l < WATER1 + 8. {
                return if hash(x as f64, v as f64, 10.) < 0.5 {
                    P.reed
                } else {
                    P.grass2
                };
            }
        }
        if zone == WOODS {
            return if hash(x as f64, v as f64, 11.) < 0.3 * still {
                P.treeShade
            } else {
                P.floor
            };
        }
        if (zone == VILLAGE || zone == STATION) && (x - self.tx - 3).abs() > 12 {
            let gv = (v as f64 / 4.).floor();
            let ox = (hash(gv, 0., 41.) * 6.).floor() as i32;
            let gx = ((x + ox) as f64 / 6.).floor();
            if ((x + ox) % 6 == 0 && hash(gx, gv, 42.) < 0.55)
                || (v % 4 == 0 && hash(gx, gv, 43.) < 0.45)
            {
                return P.hedge;
            }
            let k = hash(gx, gv, 12.);
            return if k < 0.4 {
                P.grass
            } else if k < 0.78 {
                P.grass2
            } else if k < 0.9 {
                if (v & 1) == 0 {
                    P.plough
                } else {
                    P.plough2
                }
            } else {
                P.wheat2
            };
        }
        let side = if x < self.tx { 0 } else { 1 };
        let vv = v + side * 11;
        let fr = (vv as f64 / 24.).floor();
        let lo = if side == 0 { 0 } else { self.tx + 13 };
        let hi = if side == 0 { self.tx - 7 } else { self.w - 1 };
        let xs = lo + round((hi - lo) as f64 * (0.25 + 0.5 * hash(fr, side as f64, 13.))) as i32;
        if vv as f64 - fr * 24. == 0. || x == xs {
            return if hash(x as f64, v as f64, 14.) < 0.12 {
                P.grass2
            } else {
                P.hedge
            };
        }
        let kind = hash(fr * 2. + if x < xs { 0. } else { 1. }, side as f64, 15.);
        if kind < 0.3 {
            return if hash(x as f64, v as f64, 16.) < 0.15 * still {
                P.grass2
            } else {
                P.grass
            };
        }
        if kind < 0.45 {
            return if hash(x as f64, v as f64, 17.) < 0.02 * still {
                P.sheep
            } else {
                P.grass2
            };
        }
        if kind < 0.65 {
            return if (x & 1) == 0 { P.wheat } else { P.wheat2 };
        }
        if kind < 0.84 {
            return if still < 1. {
                mix(P.plough, P.plough2, 0.5)
            } else if (v & 1) == 0 {
                P.plough
            } else {
                P.plough2
            };
        }
        if kind < 0.9 {
            return if hash(x as f64, v as f64, 18.) < 0.12 * still {
                P.wheat
            } else {
                P.rape
            };
        }
        P.grass
    }
    fn trees_above(&mut self, px: &mut Painter, P: Pal) {
        let W = px.w as i32;
        let H = px.h as i32;
        self.canopy.fill(0);
        let av = self.row_v[self.nose_y as usize];
        for gv in (self.row_v[(H - 1) as usize] as f64 / 5.).floor() as i32 - 1
            ..=(self.row_v[0] as f64 / 5.).floor() as i32 + 1
        {
            for gx in -1..=(W as f64 / 6.).floor() as i32 + 1 {
                let cx = (gx * 6 + 1) as f64 + hash(gx as f64, gv as f64, 22.) * 4.;
                let cvv = (gv * 5 + 1) as f64 + hash(gx as f64, gv as f64, 23.) * 3.;
                let s = cvv * 3.;
                let z = (s / ZONE).floor();
                let zone = zone_of(z);
                let local = s - z * ZONE;
                let p = if zone == WOODS {
                    0.97
                } else if zone == RIVER {
                    0.3
                } else if zone == VILLAGE {
                    0.22
                } else if zone == STATION {
                    0.08
                } else {
                    0.13
                };
                if hash(gx as f64, gv as f64, 21.) >= p {
                    continue;
                }
                if (cx - self.tx as f64 - 3.).abs() < 11. {
                    continue;
                }
                if crosses(zone) && (local - CROSS).abs() < 12. {
                    continue;
                }
                if zone == RIVER && local > WATER0 - 30. && local < WATER1 + 30. {
                    continue;
                }
                let rx = if zone == WOODS {
                    2.3
                } else {
                    1.4 + hash(gx as f64, gv as f64, 24.) * 0.8
                };
                let ry = rx * 0.55;
                let y0 = self.nose_y as f64 - (cvv - av as f64);
                for y in (y0 - ry - 1.).floor() as i32..=(y0 + ry + 1.).ceil() as i32 {
                    if y < 0 || y >= H {
                        continue;
                    }
                    for x in (cx - rx - 1.).floor() as i32..=(cx + rx + 1.).ceil() as i32 {
                        if x < 0 || x >= W {
                            continue;
                        }
                        let ex = (x as f64 + 0.5 - cx) / rx;
                        let ey = (y as f64 + 0.5 - y0) / ry;
                        let i = (y * W + x) as usize;
                        if ex * ex + ey * ey <= 1. {
                            self.canopy[i] = if ex + ey < -0.5 { 3 } else { 2 };
                        } else {
                            let sx = (x as f64 - 0.5 - cx) / rx;
                            let sy = (y as f64 - 0.5 - y0) / ry;
                            if sx * sx + sy * sy <= 1. && self.canopy[i] == 0 {
                                self.canopy[i] = 1;
                            }
                        }
                    }
                }
            }
        }
        for (i, &k) in self.canopy.iter().enumerate() {
            if k != 0 {
                px.px[i] = if k == 3 {
                    P.treeLit
                } else if k == 2 {
                    P.tree
                } else {
                    mix(px.px[i], P.treeShade, 0.6)
                };
            }
        }
    }
    fn houses_above(&self, px: &mut Painter, P: Pal, kn: f64) {
        let W = px.w as i32;
        let H = px.h as i32;
        let av = self.row_v[self.nose_y as usize];
        for gv in (self.row_v[(H - 1) as usize] as f64 / 4.).floor() as i32 - 1
            ..=(self.row_v[0] as f64 / 4.).floor() as i32 + 1
        {
            let s = (gv * 4 * 3) as f64;
            let z = (s / ZONE).floor();
            let zone = zone_of(z);
            if zone != VILLAGE && zone != STATION {
                continue;
            }
            if crosses(zone) && (s - z * ZONE - CROSS).abs() < 12. {
                continue;
            }
            let y0 = self.nose_y - (gv * 4 + 2 - av);
            for gx in 0..=(W as f64 / 5.).floor() as i32 {
                let x0 = gx * 5 + 1;
                if (x0 + 1 - self.tx - 3).abs() < 13 {
                    continue;
                }
                if hash(gx as f64, gv as f64, 31.) > if zone == STATION { 0.6 } else { 0.45 } {
                    continue;
                }
                let slate = hash(gx as f64, gv as f64, 32.) < 0.35;
                let lit = if slate {
                    mix(P.slate, P.wall, 0.25)
                } else {
                    P.roof
                };
                let ridge = if slate {
                    P.slate
                } else {
                    mix(P.roof, P.roofDark, 0.5)
                };
                let dark = if slate {
                    mix(P.slate, 0, 0.3)
                } else {
                    P.roofDark
                };
                for dy in 0..2 {
                    let y = y0 + dy;
                    if y < 0 || y >= H {
                        continue;
                    }
                    for i in 0..3 {
                        if x0 + i < W {
                            px.px[(y * W + x0 + i) as usize] = if i == 0 {
                                lit
                            } else if i == 1 {
                                ridge
                            } else {
                                dark
                            };
                        }
                    }
                }
                if kn > 0.3
                    && hash(gx as f64, gv as f64, 33.) < 0.6
                    && y0 + 1 >= 0
                    && y0 + 1 < H
                    && x0 + 3 < W
                {
                    px.set((x0 + 3) as f64, (y0 + 1) as f64, P.houseWin, true);
                }
            }
        }
    }
    fn train_above(r: i32, i: i32, P: Pal) -> Option<u32> {
        let edge = i == 0 || i == 3;
        if r < A_LOCO {
            return Some(if r == 0 {
                if edge {
                    P.lamp
                } else {
                    P.yellow
                }
            } else if r == 1 {
                P.yellow
            } else if r == 2 {
                P.glass
            } else if edge {
                if r == 3 {
                    P.glass
                } else {
                    P.loco
                }
            } else if r == 7 && i == 1 {
                P.grille
            } else if r == 5 || r == 6 || r == 8 || r == 9 {
                P.vent
            } else {
                P.carRoof
            });
        }
        let rr = r - A_LOCO - A_GAP;
        if rr < 0 {
            return if i == 1 || i == 2 {
                Some(P.bogie)
            } else {
                None
            };
        }
        let cr = rr % (A_CAR + A_GAP);
        if cr >= A_CAR {
            return if i == 1 || i == 2 {
                Some(P.bogie)
            } else {
                None
            };
        }
        if edge {
            return Some(if cr == 0 || cr == A_CAR - 1 {
                P.cream
            } else if (cr & 1) == 1 {
                P.sideGlass
            } else {
                P.cream
            });
        }
        Some(if cr == 3 || cr == 6 {
            P.vent
        } else {
            P.carRoof
        })
    }
    fn unit_above(r: i32, i: i32, livery: u32, P: Pal, cab: i32) -> Option<u32> {
        let edge = i == 0 || i == 3;
        if r == A_CAR {
            return if i == 1 || i == 2 {
                Some(P.bogie)
            } else {
                None
            };
        }
        let cr = if r < A_CAR { r } else { r - A_CAR - A_GAP };
        if cab == 2 && r < 2 {
            return Some(FLASH);
        }
        if r < A_CAR && cr == 0 {
            return Some(if edge && cab != 0 { P.lamp } else { P.yellow });
        }
        if r < A_CAR && cr == 1 {
            return Some(P.glass);
        }
        if edge {
            return Some(if cr == 0 || cr == A_CAR - 1 {
                livery
            } else if (cr & 1) == 1 {
                mix(livery, P.sideGlass, 0.7)
            } else {
                livery
            });
        }
        Some(if cr == 3 || cr == 6 {
            P.vent
        } else {
            P.carRoof
        })
    }
    fn spill(&self, px: &mut Painter, P: Pal, xl: i32, xr: i32, y: i32, cr: i32, kn: f64) {
        if cr <= 0 || cr >= A_CAR - 1 || (cr & 1) == 0 {
            return;
        }
        let a = q(0.45 * kn, 4.);
        for x in [xl, xr] {
            if x >= 0 && x < px.w as i32 {
                let i = y as usize * px.w + x as usize;
                if self.solid[i] == 0 {
                    px.px[i] = mix(px.px[i], P.spill, a);
                }
            }
        }
    }
}
