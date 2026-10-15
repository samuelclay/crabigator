//! Side-on and first-person rendering of the same combat simulation.
use super::*;
impl Doom {
    pub(super) fn paint(&mut self, px: &mut Painter, d: PixelDials) {
        if self.tall {
            self.paint_view(px, d);
        } else {
            self.paint_band(px, d);
        }
        let s = round(self.k_smk * 6.0) / 6.0;
        let b = round(self.k_blu * 6.0) / 6.0;
        if s <= 0.0 && b <= 0.0 {
            return;
        }
        for i in 0..px.px.len() {
            let k = if px.keep[i] != 0 { 0.5 } else { 1.0 };
            px.px[i] = tone(px.px[i], s * k, b * k);
        }
        for c in &mut px.lamps {
            if *c >= 0 {
                *c = tone(*c as u32, s * 0.5, b * 0.5) as i32;
            }
        }
    }
    fn blit(
        &mut self,
        px: &mut Painter,
        s: Sprite,
        pal: &[u32],
        x0: f64,
        y0: f64,
        flip: bool,
        scale: f64,
        rows: f64,
        fade: f64,
        seed: f64,
        z: f64,
        lamp: i32,
    ) {
        let dw = round(s.w as f64 * scale).max(1.0) as i32;
        let dh = round(rows * scale).max(1.0) as i32;
        let x0 = round(x0) as i32;
        let y0 = round(y0) as i32;
        let w = px.w;
        let y_max = if z >= 0.0 { self.hv } else { px.h };
        for j in 0..dh {
            let y = y0 + j;
            if y < 0 || y >= y_max as i32 {
                continue;
            }
            let sy = (((j as f64 + 0.5) / dh as f64 * s.h as f64).floor() as usize).min(s.h - 1);
            for i in 0..dw {
                let x = x0 + i;
                if x < 0 || x >= w as i32 {
                    continue;
                }
                let mut sx =
                    (((i as f64 + 0.5) / dw as f64 * s.w as f64).floor() as usize).min(s.w - 1);
                if flip {
                    sx = s.w - 1 - sx;
                }
                let c = s.p[sy * s.w + sx] as usize;
                if c == 0 {
                    continue;
                }
                if fade < 1.0 && hash(i as f64, j as f64, seed) >= fade {
                    continue;
                }
                if z >= 0.0 {
                    let k = y as usize * w + x as usize;
                    if self.zb[k] as f64 <= z {
                        continue;
                    }
                    self.zb[k] = z as f32;
                }
                if c as i32 == lamp {
                    px.lamp(x as f64, y as f64, pal[c]);
                } else {
                    px.set(x as f64, y as f64, pal[c], true);
                }
            }
        }
    }
    fn sprite(&mut self, px: &mut Painter, s: Sprite, pal: &[u32], x: f64, y: f64, flip: bool) {
        self.blit(px, s, pal, x, y, flip, 1.0, s.h as f64, 1.0, 0.0, -1.0, -1);
    }
    fn shade(pal: &[u32], l: f64) -> Vec<u32> {
        pal.iter().map(|&c| lit(c, l)).collect()
    }
    fn ready(&self) -> bool {
        !self.hold && self.since_sighted < 40.0
    }
    fn paint_band(&mut self, px: &mut Painter, d: PixelDials) {
        let w = px.w as f64;
        let h = px.h as f64;
        let cam_x = self.pos - self.mx;
        let ceil = if h >= 8.0 { 1.0 } else { 0.0 };
        let floor_rows = if h >= 8.0 { 2.0 } else { 1.0 };
        let wall_bot = h - floor_rows;
        let wall_rows = (wall_bot - ceil).max(1.0);
        let fx = self.mx + 9.0;
        let flashing = self.flash > 0.0;
        let smoke = self.k_smk;
        let mut tex = Tex::default();
        for x in 0..px.w {
            let x = x as f64;
            let a = (cam_x + x).floor();
            let seg = (a / SEG).floor();
            let lx = a - seg * SEG;
            let kind = seg_kind(seg, self.seed);
            let sl = seg_light(seg, d.t, self.seed);
            let boost = if flashing {
                0.35 * (1.0 - (x - fx).abs() / 14.0).max(0.0)
            } else {
                0.0
            };
            for y in 0..px.h {
                let y = y as f64;
                let mut c;
                if y < ceil {
                    c = tex.ceil(a, 0.0);
                } else if y < wall_bot {
                    let v = 1.0 - (y - ceil + 0.5) / wall_rows;
                    c = tex.wall(kind, lx, v, seg, d.t, self.seed);
                    if c == SKY {
                        c = tex.sky(x + cam_x * 0.15, (v - 0.25) / 0.56, self.seed);
                    }
                } else {
                    c = tex.floor(a, (y - wall_bot) * 8.0 - 4.0, d.t, self.seed);
                }
                let lamp = tex.lamp;
                let mut c = c as u32;
                if !lamp && !tex.glow {
                    c = lit(c, q8(sl + boost));
                }
                if smoke > 0.0 && y < wall_bot {
                    let n = noise2(a * 0.07 - d.t * 0.05, y * 0.45, self.seed + 11.0);
                    if n > 0.45 {
                        c = mix(c, 0x707070, round(smoke * (n - 0.45) * 2.2 * 4.0) / 4.0);
                    }
                }
                if lamp {
                    px.lamp(x, y, c);
                } else {
                    px.set(x, y, c, false);
                }
            }
        }
        let foot = h - 1.0
            + if h < 10.0 {
                f64::from(i32_of(10.0 - h) >> 1)
            } else {
                0.0
            };
        for b in self.barrels.clone() {
            let x = round(b.x - cam_x);
            if x > -6.0 && x < w + 2.0 {
                self.blit(
                    px,
                    BARREL,
                    BARREL_PAL,
                    x,
                    foot + 1.0 - BARREL.h as f64 - 1.0,
                    false,
                    1.0,
                    BARREL.h as f64,
                    1.0,
                    0.0,
                    -1.0,
                    3,
                );
            }
        }
        for round in 0..2 {
            for i in (0..self.foes.len()).rev() {
                let f = self.foes[i].clone();
                if (f.st == DEAD || f.st == DIE) != (round == 0) {
                    continue;
                }
                self.draw_foe_band(px, &f, crate::flow::js::round(f.x - cam_x), foot, d);
            }
        }
        for s in (0..SQUAD).rev() {
            if let Some(m) = self.crew.in_slot(s).cloned() {
                self.draw_mate_band(px, &m, round(self.pos + m.x - cam_x), foot, d);
            }
        }
        self.draw_marine_band(
            px,
            0,
            self.mx,
            foot,
            if self.ready() { 0 } else { 1 },
            self.walk,
            self.v > 0.02,
            self.flash > 0.0,
            1.0,
            0.0,
            -1.0,
        );
        for m in self.missiles.clone() {
            let x = round(m.x - cam_x);
            let y = foot - round(m.h);
            if m.kind == BALL_PLASMA {
                self.sprite(px, BOLT, BOLT_PAL, x, y - 1.0, false);
                px.set(x - 2.0, y, 0x3a6ac8, true);
            } else {
                self.sprite(
                    px,
                    BALL_SMALL,
                    if m.kind == BALL_FIRE {
                        FIRE_PAL
                    } else {
                        CACO_PAL
                    },
                    x - 1.0,
                    y - 1.0,
                    i32_of(m.t) & 2 == 0,
                );
                px.set(
                    x + 2.0,
                    y,
                    if m.kind == BALL_FIRE {
                        0xa02810
                    } else {
                        0x701880
                    },
                    true,
                );
            }
        }
        for p in &self.puffs {
            self.draw_puff_band(px, p, round(p.x - cam_x), foot);
        }
    }
    fn draw_foe_band(&mut self, px: &mut Painter, f: &Foe, x: f64, foot: f64, d: PixelDials) {
        let kind = FOES[f.kind];
        if x < -12.0 || x > px.w as f64 + 4.0 {
            return;
        }
        let frame = if f.st == ATTACK && f.t >= 2.0 && f.t < 7.0 {
            2
        } else {
            i32_of((f.walk / 3.0).floor()) as usize & 1
        };
        let s = kind.frames[frame];
        let bob = if kind.float != 0.0 {
            round((d.t * 0.12 + f.seed).sin() * 0.8)
        } else {
            0.0
        };
        if f.st == DIE || f.st == DEAD {
            let k = if f.st == DEAD { 1.0 } else { f.t / DIE_T };
            let rows = round(s.h as f64 - (s.h as f64 - 2.0) * k).max(2.0);
            let lift = round(kind.float * (1.0 - k));
            if k > 0.5 {
                for i in -1..s.w as i32 + 1 {
                    px.set(
                        x + i as f64,
                        foot,
                        if f.gib { 0x9a0c0c } else { 0x6a0808 },
                        true,
                    );
                }
            }
            let pal = if f.gib && k > 0.5 {
                GIB_PAL[f.kind]
            } else {
                kind.pal
            };
            self.blit(
                px,
                s,
                pal,
                x,
                foot + 1.0 - rows - lift - if k > 0.5 { 1.0 } else { 0.0 },
                false,
                1.0,
                rows,
                1.0,
                0.0,
                -1.0,
                -1,
            );
            return;
        }
        let top = foot + 1.0 - s.h as f64 - kind.float + bob;
        let pal = if f.st == PAIN {
            Self::shade(kind.pal, 1.5)
        } else {
            kind.pal.to_vec()
        };
        self.sprite(px, s, &pal, x, top, f.st == FLEE);
        if f.kind == IMP && f.st == ATTACK && f.t < 4.0 {
            self.sprite(px, BALL_SMALL, FIRE_PAL, x + 2.0, top - 2.0, false);
        }
        if f.flash > 0.0 {
            px.lamp(x - 1.0, top + 4.0, 0xffe070);
        }
    }
    fn draw_marine_band(
        &mut self,
        px: &mut Painter,
        i: usize,
        x: f64,
        foot: f64,
        pose: usize,
        walk: f64,
        moving: bool,
        firing: bool,
        fade: f64,
        seed: f64,
        rows: f64,
    ) {
        let leg = if moving {
            1 + (i32_of((walk / 3.0).floor()) as usize & 1)
        } else {
            0
        };
        let s = MARINE[pose][leg];
        let h = if rows < 0.0 { s.h as f64 } else { rows };
        let top = foot + 1.0 - h;
        self.blit(
            px,
            s,
            MARINE_PAL[i],
            x,
            top,
            false,
            1.0,
            h,
            fade,
            seed,
            -1.0,
            -1,
        );
        if rows >= 0.0 {
            return;
        }
        if self.switch_t > 2.0 && self.switch_t < SWITCH - 2.0 {
            return;
        }
        let g = GUNS[self.gun];
        let (hx, hy) = if pose == 0 { (7.0, 3.0) } else { (7.0, 4.0) };
        let gx = x + hx + 1.0 - g.ax;
        let gy = top + hy;
        self.blit(
            px,
            g.s,
            GUN_PAL,
            gx,
            gy,
            false,
            1.0,
            g.s.h as f64,
            fade,
            seed,
            -1.0,
            -1,
        );
        if firing && fade > 0.5 {
            self.blit(
                px,
                FLASH_SIDE,
                if self.gun == PLASMA {
                    PLASMA_FLASH_PAL
                } else {
                    FLASH_PAL
                },
                gx + g.s.w as f64,
                gy - 1.0,
                false,
                1.0,
                3.0,
                1.0,
                0.0,
                -1.0,
                3,
            );
        }
    }
    fn draw_mate_band(&mut self, px: &mut Painter, m: &Mate, x: f64, foot: f64, d: PixelDials) {
        let i = 1 + m.slot;
        let goal = self.squad_place(m);
        let running = (goal - m.x).abs() > 1.5 && !m.leaving;
        let pose = if m.busy > 0.5 && !self.hold && !m.leaving {
            0
        } else {
            1
        };
        let flash =
            pose == 0 && m.y > round(rate_of(self.gun, d.level) * (1.4 + m.seed * 0.6)) - 2.0;
        if failed(Some(m)) {
            let k = clamp01((1.0 - m.here) * 2.2);
            let rows = round(9.0 - 7.0 * k).max(2.0);
            if k > 0.6 {
                for j in -1..8 {
                    px.set(x + j as f64, foot, 0x6a0808, true);
                }
            }
            self.draw_marine_band(
                px,
                i,
                x,
                foot,
                1,
                0.0,
                false,
                false,
                clamp01(m.here * 2.0),
                m.seed * 9999.0,
                rows,
            );
        } else {
            let fade = if finished(Some(m)) { m.here } else { 1.0 };
            self.draw_marine_band(
                px,
                i,
                x,
                foot,
                pose,
                self.walk + m.seed * 6.0 + if running { m.age * 0.6 } else { 0.0 },
                self.v > 0.02 || running,
                flash,
                fade,
                m.seed * 9999.0 + m.age,
                -1.0,
            );
            if finished(Some(m)) {
                self.fog(px, x + 3.0, foot - 4.0, 1.0 - m.here, m.age, 4.0);
            }
        }
        if m.waiting && beckon(m, d.t) {
            px.lamp(x + 3.0, foot - 9.0, 0xffb020);
        }
    }
    fn fog(&self, px: &mut Painter, x: f64, y: f64, k: f64, t: f64, r: f64) {
        let n = round(10.0 * (std::f64::consts::PI * clamp01(k)).sin()) as i32;
        for i in 0..n {
            let a = hash1(t * 31.0 + i as f64) * std::f64::consts::PI * 2.0;
            let rr = r * (0.3 + 0.7 * hash1(t * 17.0 + i as f64 * 3.0));
            px.dot(
                x + a.cos() * rr,
                (y + a.sin() * rr * 1.6) * 2.0,
                if i & 1 != 0 { 0xa8ff98 } else { 0xe8ffe0 },
            );
        }
    }
    fn draw_puff_band(&self, px: &mut Painter, p: &Puff, x: f64, foot: f64) {
        let y = foot - round(p.h);
        let k = p.t / p.life;
        if p.kind == BLOOD {
            px.set(x, y + (p.t / 2.0).floor(), 0xc81818, true);
            if p.t > 1.0 {
                px.set(
                    x + if i32_of(p.seed) & 1 != 0 { 1.0 } else { -1.0 },
                    y + p.t - 1.0,
                    0x8a0a0a,
                    true,
                );
            }
        } else if p.kind == SPARK || p.kind == DUST {
            let c = if p.kind == SPARK {
                if k < 0.5 {
                    0xfff0a0
                } else {
                    0xff7020
                }
            } else {
                0xa0a0a0
            };
            px.set(x, y, c, true);
            if p.t < 3.0 {
                px.set(x - 1.0, y - 1.0, c, true);
                px.set(x + 1.0, y + 1.0, c, true);
            }
        } else {
            let r = 1.5 + 4.5 * k.sqrt();
            for dy in -6..=6 {
                for dx in -7..=7 {
                    let q = hypot(dx as f64, dy as f64 * 1.2) / r;
                    if q > 1.0 {
                        continue;
                    }
                    let heat = (1.0 - q) * (1.15 - k);
                    if heat < 0.08 {
                        continue;
                    }
                    px.set(x + dx as f64, y + dy as f64, fire_color(heat), true);
                }
            }
        }
    }
    fn paint_view(&mut self, px: &mut Painter, d: PixelDials) {
        let w = px.w;
        let hv = self.hv;
        let fy = self.fy;
        let bob_k = (self.v * 1.5).min(1.0);
        let hz = hv as f64 * 0.5 + (self.walk * 0.5).sin() * 0.6 * bob_k;
        self.hz = hz;
        let cam_z = self.pos / UNIT;
        let extra = if self.flash > 0.0 { 0.2 } else { 0.0 };
        let smoke = self.k_smk;
        let s0 = (self.pos / SEG).floor();
        for k in 0..self.lights.len() {
            self.lights[k] = seg_light(s0 + k as f64, d.t, self.seed) as f32;
        }
        let mut tex = Tex::default();
        for y in 0..hv {
            let dy = y as f64 + 0.5 - hz;
            let tp = if dy > 0.0 {
                fy * 0.62 / dy
            } else if dy < 0.0 {
                fy * 1.38 / -dy
            } else {
                1e9
            };
            for x in 0..w {
                let ux = self.ux[x] as f64;
                let uz = self.uz[x] as f64;
                let tw = if ux.abs() > 1e-6 { 1.0 / ux.abs() } else { 1e9 };
                let mut t = tw.min(tp);
                let mut c = 0;
                let mut surf = if tw < tp { 0 } else { 1 };
                for dr in &self.doors {
                    let td = (dr.x - self.pos) / UNIT / uz;
                    if td <= 0.05 || td >= t || dr.open >= 1.0 {
                        continue;
                    }
                    let h = (-dy * td) / fy;
                    if h < -0.62 + dr.open * (0.62 + 1.38) {
                        continue;
                    }
                    t = td;
                    surf = 2;
                    let lx = ((td * ux + 1.0) * 16.0).floor();
                    let v = (h + 0.62) / (0.62 + 1.38) - dr.open;
                    c = if lx <= 1.0 || lx >= 30.0 {
                        0x4c4c48
                    } else if lx == 15.0 || lx == 16.0 {
                        0x5a5852
                    } else if (v * 16.0).floor() % 3.0 == 0.0 {
                        0x6a6862
                    } else {
                        0x8c8a82
                    };
                    tex.lamp = false;
                    tex.glow = false;
                    break;
                }
                if surf == 0 {
                    let a = ((cam_z + tw * uz) * UNIT).floor();
                    let seg = (a / SEG).floor();
                    let side = if ux < 0.0 { 0.0 } else { 1.0 };
                    let v = clamp(((-dy * tw) / fy + 0.62) / (0.62 + 1.38), 0.0, 0.999);
                    let track = self.doors.iter().any(|dr| (a - dr.x).abs() <= 1.0);
                    if track {
                        c = 0x4c4c48;
                        tex.lamp = false;
                        tex.glow = false;
                    } else {
                        c = tex.wall(
                            seg_kind(seg * 2.0 + side, self.seed),
                            a - seg * SEG,
                            v,
                            seg,
                            d.t,
                            self.seed + side,
                        );
                        if c == SKY {
                            c = tex.sky(
                                x as f64 * 2.0 + side * 60.0 + d.t * 0.02,
                                (v - 0.25) / 0.56,
                                self.seed,
                            );
                        }
                    }
                } else if surf == 1 {
                    let a = ((cam_z + tp * uz) * UNIT).floor();
                    let lat = tp * ux * UNIT;
                    c = if dy > 0.0 {
                        tex.floor(a, lat, d.t, self.seed)
                    } else {
                        tex.ceil(a, lat)
                    };
                }
                let fog_l = if t < 9.75 {
                    (1.3 - t / 7.5).min(1.1)
                } else {
                    0.0
                };
                let k = (((cam_z + t * uz) * UNIT) / SEG).floor() - s0;
                let l = q8((if tex.glow {
                    (fog_l * 1.2).max(0.85)
                } else if fog_l > 0.0 && k < self.lights.len() as f64 {
                    self.lights.get(k as usize).copied().unwrap_or(f32::NAN) as f64 * fog_l
                } else {
                    0.0
                }) + if t < 3.0 { extra } else { 0.0 });
                let mut c = if l <= 0.06 {
                    0x0a0806
                } else {
                    lit(c as u32, l)
                };
                if smoke > 0.0 && dy < 0.0 {
                    let n = noise2(
                        x as f64 * 0.18 - d.t * 0.03,
                        y as f64 * 0.2 + d.t * 0.01,
                        self.seed + 11.0,
                    );
                    if n > 0.45 {
                        c = mix(c, 0x686868, round(smoke * (n - 0.45) * 2.0 * 4.0) / 4.0);
                    }
                }
                self.zb[y * w + x] = t as f32;
                if tex.lamp && l > 0.4 {
                    px.lamp(x as f64, y as f64, c);
                } else {
                    px.set(x as f64, y as f64, c, false);
                }
            }
        }
        for b in self.barrels.clone() {
            self.billboard(
                px,
                BARREL,
                BARREL_PAL,
                b.x,
                b.u,
                0.0,
                false,
                1.0,
                BARREL.h as f64,
                d,
                3,
                false,
                1.0,
                0.0,
                0.3,
            );
        }
        for f in self.foes.clone() {
            self.draw_foe_view(px, &f, d);
        }
        for m in self.crew.mates.clone() {
            self.draw_mate_view(px, &m, d);
        }
        for m in self.missiles.clone() {
            if m.kind == BALL_PLASMA {
                self.billboard(
                    px,
                    BOLT,
                    BOLT_PAL,
                    m.x,
                    m.u,
                    m.h,
                    false,
                    0.9,
                    BOLT.h as f64,
                    d,
                    -1,
                    true,
                    1.0,
                    0.0,
                    0.3,
                );
            } else {
                self.billboard(
                    px,
                    BALL,
                    if m.kind == BALL_FIRE {
                        FIRE_PAL
                    } else {
                        CACO_PAL
                    },
                    m.x,
                    m.u,
                    m.h,
                    i32_of(m.t) & 2 == 0,
                    1.0,
                    BALL.h as f64,
                    d,
                    -1,
                    true,
                    1.0,
                    0.0,
                    0.7,
                );
            }
        }
        for p in self.puffs.clone() {
            self.draw_puff_view(px, &p, d);
        }
        self.draw_gun(px, d);
        if self.sb > 0 {
            self.draw_status(px);
        } else if self.pano {
            self.draw_hud(px);
        }
    }
    fn draw_hud(&mut self, px: &mut Painter) {
        let hv = self.hv as f64;
        if hv < 20.0 {
            return;
        }
        let w = px.w as f64;
        let y = hv - 7.0;
        let ammo = self.ammo[self.gun];
        self.number(px, ammo, 2.0 + format!("{ammo:.0}").len() as f64 * 4.0, y);
        let mut right = w - 3.0;
        if hv >= 24.0 {
            let fx = w - 14.0;
            let fy = hv - 13.0;
            px.rect(fx, fy, 12.0, 12.0, 0x2e2a24, true);
            self.sprite(
                px,
                FACES[self.face_now()],
                FACE_PAL,
                fx + 1.0,
                fy + 1.0,
                false,
            );
            self.bloody(px, fx + 1.0, fy + 1.0);
            right = fx - 3.0;
        }
        self.number(px, round(self.hp), right, y);
    }
    fn face_now(&self) -> usize {
        if self.hold {
            return LOOK_AHEAD;
        }
        if self.ouch > 0.0 {
            return OUCH;
        }
        if self.grin > 0.0 {
            return GRIN;
        }
        if self.since_shot < 5.0 && self.gun >= CHAINGUN {
            RAGE
        } else {
            self.look
        }
    }
    fn bloody(&self, px: &mut Painter, x: f64, y: f64) {
        if self.k_smk <= 0.5 && self.hp >= 40.0 {
            return;
        }
        let n = if self.k_smk > 0.5 { BLOODY.len() } else { 3 };
        for [dx, dy] in BLOODY.iter().take(n) {
            px.set(x + *dx as f64, y + *dy as f64, 0xb01010, true);
        }
    }
    fn billboard(
        &mut self,
        px: &mut Painter,
        s: Sprite,
        pal: &[u32],
        x: f64,
        u: f64,
        h: f64,
        flip: bool,
        size: f64,
        rows: f64,
        d: PixelDials,
        lamp_idx: i32,
        glow: bool,
        fade: f64,
        seed: f64,
        nearest: f64,
    ) -> bool {
        if !self.on_view(x, u, nearest) {
            return false;
        }
        let z = self.pt;
        let scale = self.pk * SPX * size;
        let foot_y = self.hz + self.pk * (0.62 - h * SPX);
        let dw = s.w as f64 * scale;
        let dh = rows * scale;
        self.bx = self.ps - dw / 2.0;
        self.by = foot_y - dh;
        self.bw = dw;
        self.bh = dh;
        let seg = (x / SEG).floor();
        let l = if glow {
            1.0
        } else {
            q8(
                seg_light(seg, d.t, self.seed) * clamp(1.3 - z / 7.5, 0.0, 1.1)
                    + if self.flash > 0.0 && z < 3.0 {
                        0.2
                    } else {
                        0.0
                    },
            )
        };
        if l <= 0.06 {
            return false;
        }
        let pal = if l == 1.0 {
            pal.to_vec()
        } else {
            Self::shade(pal, l)
        };
        self.blit(
            px, s, &pal, self.bx, self.by, flip, scale, rows, fade, seed, z, lamp_idx,
        );
        true
    }
    fn draw_foe_view(&mut self, px: &mut Painter, fo: &Foe, d: PixelDials) {
        let kind = FOES[fo.kind];
        let frame = if fo.st == ATTACK && fo.t >= 2.0 && fo.t < 7.0 {
            2
        } else {
            i32_of((fo.walk / 3.0).floor()) as usize & 1
        };
        let s = kind.frames[frame];
        let flip = fo.st == FLEE;
        if fo.st == DIE || fo.st == DEAD {
            let k = if fo.st == DEAD { 1.0 } else { fo.t / DIE_T };
            let rows = round(s.h as f64 - (s.h as f64 - 2.0) * k).max(2.0);
            let pal = if fo.gib && k > 0.5 {
                GIB_PAL[fo.kind]
            } else {
                kind.pal
            };
            if k > 0.5 {
                self.billboard(
                    px,
                    POOL,
                    POOL_PAL,
                    fo.x + 0.5,
                    fo.u,
                    0.0,
                    false,
                    1.0,
                    1.0,
                    d,
                    -1,
                    false,
                    1.0,
                    0.0,
                    0.3,
                );
            }
            self.billboard(
                px,
                s,
                pal,
                fo.x,
                fo.u,
                kind.float * (1.0 - k),
                flip,
                1.0,
                rows,
                d,
                -1,
                false,
                1.0,
                0.0,
                0.3,
            );
            return;
        }
        let bob = if kind.float != 0.0 {
            (d.t * 0.12 + fo.seed).sin() * 0.8
        } else {
            0.0
        };
        let pal = if fo.st == PAIN {
            Self::shade(kind.pal, 1.5)
        } else {
            kind.pal.to_vec()
        };
        if !self.billboard(
            px,
            s,
            &pal,
            fo.x,
            fo.u,
            kind.float + bob,
            flip,
            1.0,
            s.h as f64,
            d,
            -1,
            false,
            1.0,
            0.0,
            0.3,
        ) {
            return;
        }
        if fo.flash > 0.0 {
            px.lamp(self.bx + self.bw * 0.1, self.by + self.bh * 0.45, 0xffe070);
        }
        if fo.kind == IMP && fo.st == ATTACK && fo.t < 4.0 {
            self.billboard(
                px,
                BALL,
                FIRE_PAL,
                fo.x - 0.5,
                fo.u,
                9.0,
                false,
                1.0,
                BALL.h as f64,
                d,
                -1,
                true,
                1.0,
                0.0,
                0.3,
            );
        }
    }
    fn draw_mate_view(&mut self, px: &mut Painter, m: &Mate, d: PixelDials) {
        let i = 1 + m.slot;
        let u = [-0.5, 0.5, -0.3, 0.3][m.slot];
        let x = self.pos + m.x;
        let pose = if m.busy > 0.5 && !self.hold && !m.leaving {
            0
        } else {
            1
        };
        let running = (self.squad_place(m) - m.x).abs() > 1.5 && !m.leaving;
        let moving = self.v > 0.02 || running;
        let leg = if moving {
            1 + (i32_of(
                ((self.walk + m.seed * 6.0 + if running { m.age * 0.6 } else { 0.0 }) / 3.0)
                    .floor(),
            ) as usize
                & 1)
        } else {
            0
        };
        let s = MARINE_BACK[pose][leg];
        let mut rows = s.h as f64;
        let mut fade = 1.0;
        if failed(Some(m)) {
            let k = clamp01((1.0 - m.here) * 2.2);
            rows = round(s.h as f64 - 7.0 * k).max(2.0);
            fade = clamp01(m.here * 2.0);
            if k > 0.6 {
                self.billboard(
                    px,
                    POOL,
                    POOL_PAL,
                    x + 0.5,
                    u,
                    0.0,
                    false,
                    1.0,
                    1.0,
                    d,
                    -1,
                    false,
                    1.0,
                    0.0,
                    0.3,
                );
            }
        } else if finished(Some(m)) {
            fade = m.here;
        }
        if !self.billboard(
            px,
            s,
            MARINE_PAL[i],
            x,
            u,
            0.0,
            false,
            1.0,
            rows,
            d,
            -1,
            false,
            fade,
            m.seed * 9999.0 + m.age,
            0.9,
        ) {
            return;
        }
        let (bx, by, bw, bh) = (self.bx, self.by, self.bw, self.bh);
        if finished(Some(m)) {
            self.fog(
                px,
                bx + bw / 2.0,
                by + bh / 2.0,
                1.0 - m.here,
                m.age,
                (bw * 0.8).max(3.0),
            );
        }
        if !failed(Some(m)) && pose == 0 {
            let flash = m.y > round(rate_of(self.gun, d.level) * (1.4 + m.seed * 0.6)) - 2.0;
            if flash {
                px.lamp(
                    bx + bw * 0.5,
                    by - 1.0,
                    if self.gun == PLASMA {
                        0x70c0ff
                    } else {
                        0xffd040
                    },
                );
            }
        }
        if m.waiting && beckon(m, d.t) {
            px.lamp(bx + bw / 2.0, by - (bh * 0.15).max(2.0), 0xffb020);
        }
    }
    fn draw_puff_view(&mut self, px: &mut Painter, p: &Puff, d: PixelDials) {
        if !self.on_view(p.x, p.u, 0.3) {
            return;
        }
        let z = self.pt;
        let k = p.t / p.life;
        if p.kind == BOOM {
            let r = (1.5 + 4.5 * k.sqrt()) * self.pk * SPX;
            let cx = self.ps;
            let cy = self.hz + self.pk * (0.62 - p.h * SPX);
            let bound = r.ceil() as i32;
            for dy in -bound..=bound {
                for dx in -bound..=bound {
                    let q = hypot(dx as f64, dy as f64) / r;
                    if q > 1.0 {
                        continue;
                    }
                    let heat = (1.0 - q) * (1.15 - k);
                    if heat < 0.08 {
                        continue;
                    }
                    let x = round(cx + dx as f64) as i32;
                    let y = round(cy + dy as f64) as i32;
                    if x < 0
                        || y < 0
                        || x >= px.w as i32
                        || y >= self.hv as i32
                        || (self.zb[y as usize * px.w + x as usize] as f64) < z - 0.3
                    {
                        continue;
                    }
                    px.set(x as f64, y as f64, fire_color(heat), true);
                }
            }
            return;
        }
        let s = if p.kind == BLOOD { BLOODS } else { SPARKS };
        let pal = if p.kind == BLOOD {
            BLOOD_PAL
        } else if p.kind == SPARK {
            SPARK_PAL
        } else {
            DUST_PAL
        };
        self.billboard(
            px,
            s,
            pal,
            p.x,
            p.u,
            p.h - p.t * if p.kind == BLOOD { 0.6 } else { 0.0 },
            i32_of(p.seed) & 1 == 1,
            1.0,
            s.h as f64,
            d,
            -1,
            p.kind == SPARK,
            1.0,
            0.0,
            0.3,
        );
    }
    fn draw_gun(&mut self, px: &mut Painter, d: PixelDials) {
        let w = px.w as f64;
        let hv = self.hv as f64;
        let scale = if self.pano {
            if hv < 26.0 {
                hv / 26.0
            } else {
                round(hv / 34.0).max(1.0)
            }
        } else {
            round((w / 40.0).min(hv / 50.0)).max(1.0)
        };
        let s = FP_GUNS[self.gun];
        let bob = (self.v * 2.4).min(3.0);
        let ph = self.walk * 0.32;
        let sw = if self.switch_t > 0.0 {
            (std::f64::consts::PI * self.switch_t / SWITCH).sin() * s.h as f64 * scale * 1.1
        } else {
            0.0
        };
        let down = if self.hold { 4.0 } else { 0.0 } * scale + self.recoil * 0.5 * scale + sw;
        let x0 = round(w / 2.0 - (s.w as f64 * scale) / 2.0 + 1.0 + ph.cos() * bob);
        let y0 = round(
            hv - s.h as f64 * scale
                + ph.sin().abs() * bob
                + down
                + if self.sb > 0 { 1.0 } else { 0.0 },
        );
        let light = q8(seg_light((self.pos / SEG).floor(), d.t, self.seed) * 0.95
            + if self.flash > 0.0 { 0.25 } else { 0.0 });
        let mut shaded = Self::shade(FP_PAL, light);
        shaded[8] = FP_PAL[8];
        shaded[9] = FP_PAL[9];
        self.blit(
            px,
            s,
            &shaded,
            x0,
            y0,
            false,
            scale,
            s.h as f64,
            1.0,
            0.0,
            -1.0,
            if self.gun == PLASMA { 8 } else { -1 },
        );
        if self.flash > 0.0 && self.switch_t == 0.0 {
            let fl = FP_FLASH;
            self.blit(
                px,
                fl,
                if self.gun == PLASMA {
                    PLASMA_FLASH_PAL
                } else {
                    FLASH_PAL
                },
                x0 + s.w as f64 * scale / 2.0 - fl.w as f64 * scale / 2.0 - 0.5,
                y0 - fl.h as f64 * scale + scale,
                false,
                scale,
                fl.h as f64,
                1.0,
                0.0,
                -1.0,
                3,
            );
        }
        if self.gun == CHAINGUN && self.since_shot < 6.0 && i32_of(d.t) & 1 != 0 {
            for i in 0..3 {
                px.set(x0 + (4 + i * 2) as f64 * scale, y0, 0x5c5c60, true);
            }
        }
    }
    fn draw_status(&mut self, px: &mut Painter) {
        let w = px.w as f64;
        let y0 = self.hv as f64;
        let sb = self.sb as f64;
        px.rect(0.0, y0, w, sb, 0x5a5248, false);
        px.rect(0.0, y0, w, 1.0, 0x8a8070, false);
        px.rect(0.0, y0 + sb - 1.0, w, 1.0, 0x38322a, false);
        let fx = round(w / 2.0 - 6.0);
        px.rect(fx, y0 + 1.0, 12.0, sb - 2.0, 0x2e2a24, false);
        let fy = y0 + round((sb - 10.0) / 2.0).max(1.0);
        self.sprite(px, FACES[self.face_now()], FACE_PAL, fx + 1.0, fy, false);
        self.bloody(px, fx + 1.0, fy);
        if fx >= 12.0 {
            let dy = y0 + round((sb - 5.0) / 2.0);
            self.number(px, self.ammo[self.gun], fx - 2.0, dy);
            self.number(px, round(self.hp), w - 2.0, dy);
        }
    }
    fn number(&self, px: &mut Painter, n: f64, right: f64, y: f64) {
        let s = format!("{:.0}", clamp(round(n), 0.0, 999.0));
        let mut x = right - s.len() as f64 * 4.0 + 1.0;
        for ch in s.bytes() {
            let g = DIGITS[(ch - b'0') as usize].as_bytes();
            for j in 0..5 {
                for i in 0..3 {
                    if g[j * 3 + i] == b'1' {
                        px.set(x + i as f64, y + j as f64, 0xd82818, true);
                    }
                }
            }
            x += 4.0;
        }
    }
}
fn fire_color(heat: f64) -> u32 {
    if heat > 0.75 {
        0xfff4c0
    } else if heat > 0.5 {
        0xffd040
    } else if heat > 0.28 {
        0xff7a18
    } else {
        0xb02a0a
    }
}
