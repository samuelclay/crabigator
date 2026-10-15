//! Hosted companion rockets: each runs the same flight state machine, with
//! an immutable snapshot of its host's camera and formation for each frame.
use super::*;
#[derive(Clone)]
pub(super) struct Host {
    pub site_x: [f64; 3],
    pub alt: f64,
    pub top_row: f64,
    pub view_x: f64,
    pub scroll: f64,
    pub lz: f64,
    pub body_px: f64,
    pub shore: f64,
    pub sea_x: f64,
    pub ox: f64,
    pub orbit_frame: (f64, f64, f64),
    pub state: State,
}
impl LaunchSite {
    fn host_frame(&self) -> Host {
        Host {
            site_x: [self.site_x(0), self.site_x(1), self.site_x(2)],
            alt: self.world.alt,
            top_row: self.top_row(),
            view_x: self.view_x(),
            scroll: self.scroll(),
            lz: self.lz,
            body_px: self.body_px,
            shore: self.shore(),
            sea_x: self.sea_x(),
            ox: self.ox,
            orbit_frame: self.orbit_frame(),
            state: self.state,
        }
    }
    pub(super) fn site_x(&self, slot: usize) -> f64 {
        let m = &(match self.rocket {
            Rocket::Falcon => sprites::mini_falcon_specs(),
            Rocket::Starship => sprites::mini_starship_specs(),
        })[usize::from(self.tall)];
        let s = self.spec();
        let pw = self.columns() * 2.0;
        let gap = if pw < 32.0 {
            0.0
        } else if self.tall {
            3.0
        } else {
            4.0
        };
        let l = -m.body_l;
        let r = ceil_even(l + m.fly.wf()) + m.tower_w;
        let mut right = self.tx + s.tower_w + gap;
        let mut left = self.ox
            - if self.look().catches && !self.tall {
                s.fly.wf() + 3.0
            } else {
                0.0
            }
            - gap;
        let mut x = 0.0;
        for k in 0..=slot {
            if k % 2 == 0 {
                x = right - l;
                right = x + r + gap;
            } else {
                x = left - r;
                left = x + l - gap;
            }
        }
        if x >= 0.0 && x + m.body_w <= pw {
            return x;
        }
        if self.tall && slot == 2 {
            let behind =
                (ceil_even((self.ox + self.spec().fly.wf() + self.tx) / 2.0) - m.body_w).floor();
            if behind >= 0.0 {
                return behind;
            }
        }
        f64::NAN
    }
    pub(super) fn sea_off(&self, h: &Host) -> f64 {
        (self.body_px - h.body_px)
            .min(2.0 * h.shore - (h.ox + h.sea_x) - self.spec().fly.wf() - 2.0)
    }
    pub(super) fn add_mini(&mut self, mate: Option<Mate>, slot: usize, seed: f64) -> usize {
        let mut r = LaunchSite::new(self.rocket, seed);
        r.host = Some(self.host_frame());
        r.mate = mate;
        r.slot = slot;
        r.lag = 3 + 5 * slot + (seed % 4.0) as usize;
        r.world.dials.accent = self.world.dials.accent;
        r.ensure(self.world.columns, self.world.rows);
        self.minis.push(r);
        self.minis.len() - 1
    }
    pub(super) fn step_minis(&mut self) {
        self.minis.retain(|r| {
            r.mate
                .as_ref()
                .is_none_or(|m| self.crew.mates.iter().any(|x| x.id == m.id))
        });
        for m in self.crew.mates.clone() {
            if !self
                .minis
                .iter()
                .any(|r| r.mate.as_ref().is_some_and(|x| x.id == m.id))
            {
                let slot = m.slot;
                let seed = (m.seed * 1e6).floor();
                self.add_mini(Some(m), slot, seed);
            }
        }
        let host = self.host_frame();
        for r in &mut self.minis {
            if let Some(m) = &r.mate {
                r.mate = self.crew.mates.iter().find(|x| x.id == m.id).cloned();
            }
            r.host = Some(host.clone());
            let idle = r.mate.as_ref().is_some_and(|m| m.busy < 0.5)
                && !r.fresh
                && r.state == State::Rest
                && r.part == Part::Full;
            r.world.dials.strength = if idle {
                1.0
            } else {
                self.log[(self.log_at + 32 - r.lag) & 31] as f64
            };
            r.world.dials.tint = if failed(r.mate.as_ref()) {
                Tint::Smoke
            } else {
                self.world.dials.tint
            };
            r.world.dials.accent = self.world.dials.accent;
            r.step();
        }
    }
    pub(super) fn shift(&mut self, rows: f64) {
        self.world.alt += rows;
        self.parts.shift(2.0 * rows);
        for r in &mut self.minis {
            if r.state == State::Fly {
                r.shift(rows);
            }
        }
    }
    pub(super) fn formation(&mut self, thr_goal: f64, homeward: bool) -> f64 {
        let h = self.host.as_ref().unwrap();
        let m = self.mate.as_ref();
        if self.state != State::Fly || h.state != State::Fly {
            return thr_goal;
        }
        let quiet = m.map_or(0.0, |m| 1.0 - m.busy);
        let mut gap = if self.tall {
            [8.0, 14.0, 20.0]
        } else {
            [1.0, 2.0, 3.0]
        }[self.slot % 3]
            + quiet * if self.tall { 10.0 } else { 3.0 };
        gap += (noise1(self.world.t * 0.02, self.slot as f64 * 31.0 + 7.0) - 0.5)
            * if self.tall { 3.0 } else { 1.0 };
        if let Some(m) = m {
            let away = 1.0 - m.here;
            let run = self.ph as f64 + 20.0;
            gap += if finished(Some(m)) {
                -away * away * run * 1.5
            } else {
                away * run
            };
        }
        let goal = h.alt + (h.top_row - self.top_row() - gap) / 2.0;
        self.world.alt += (goal - self.world.alt) * 0.2;
        if finished(m) {
            return 1.0;
        }
        if homeward {
            thr_goal
        } else {
            thr_goal * (1.0 - 0.85 * quiet)
        }
    }
    pub(super) fn tumble(&self) -> f64 {
        if failed(self.mate.as_ref()) {
            (1.0 - self.mate.as_ref().unwrap().here)
                * 3.0
                * if self.slot & 1 != 0 { -1.0 } else { 1.0 }
        } else {
            0.0
        }
    }
    pub(super) fn big(&self) -> bool {
        self.tall && self.host.is_none()
    }
    pub(super) fn orbit_frame(&self) -> (f64, f64, f64) {
        let ocx = self.pw as f64 / 2.0 + if self.tall { 0.0 } else { -2.0 };
        let ocy = (self.ph as f64
            - if self.split_tall {
                2.0 * self.split_w
            } else {
                0.0
            })
            * if self.tall { 0.42 } else { 0.36 };
        let room = self.pw as f64
            - if self.split_tall {
                0.0
            } else {
                2.0 * self.split_w
            };
        let full = if self.tall {
            0.85_f64.min((room - 7.0) / (2.0 * self.spec().fly.hf() * 0.3_f64.max(self.tilt.sin())))
        } else {
            1.0
        };
        (ocx, ocy, full)
    }
    pub(super) fn station(&self) -> (f64, f64, f64) {
        let (hx, hy, full) = self.host.as_ref().unwrap().orbit_frame;
        let (along0, across) = if self.tall {
            [(-4.0, 18.0), (4.0, -18.0), (-14.0, 30.0)]
        } else {
            [(-16.0, 4.0), (14.0, -4.0), (-30.0, -4.0)]
        }[self.slot % 3];
        let m = self.mate.as_ref();
        let sc = if self.tall { full } else { 1.0 };
        let sn = self.tilt.sin();
        let cs = self.tilt.cos();
        let along = along0 - m.map_or(0.0, |m| (1.0 - m.busy) * 6.0);
        let mut x = clamp(
            hx + (along * sn + across * cs) * sc,
            3.0,
            self.pw as f64 - 3.0,
        );
        let mut y = clamp(
            hy + ((-along * cs + across * sn) / 2.0) * sc,
            2.0,
            self.ph as f64 - 3.0,
        );
        if let Some(m) = m {
            let away = 1.0 - m.here;
            let run = (self.pw + self.ph) as f64;
            if !m.leaving {
                x -= sn * away * run;
                y += (cs * away * run) / 2.0;
            } else if m.ok {
                x += sn * away * away * run * 1.5;
                y -= cs * away * away * run * 0.75;
            } else {
                y += away * (self.ph as f64 + 10.0);
            }
        }
        (x, y, full)
    }
    pub(super) fn step_split_view(&mut self) {
        if self.host.is_some() || self.booster_only {
            return;
        }
        let n = round(self.split_w);
        let mut goal = 0.0;
        if self.split_tall && n > 0.0 {
            let shown = self.rows() - n - 1.0;
            let mid = self.focus().1 - self.chute_room() / 4.0;
            goal = clamp(round(mid - shown / 2.0), 0.0, n);
        }
        self.split_off += clamp((goal - self.split_off) * 0.35, -1.5, 1.5);
        if self.split_off > n {
            self.split_off = n;
        }
    }
    pub(super) fn shift_split_view(&mut self, out: &mut Cells) {
        let off = round(self.split_off);
        let n = round(self.split_w);
        if off <= 0.0 || !self.split_tall || n <= 0.0 {
            return;
        }
        let w = self.world.columns;
        let shown = self.world.rows - n as usize;
        for r in 0..shown {
            for x in 0..w {
                let j = (r + off as usize) * w + x;
                out.set(
                    r * w + x,
                    out.code_point(j),
                    out.foreground(j),
                    out.background(j),
                );
            }
        }
        let mut kept = 0;
        for l in 0..self.n_lamps {
            let r = (self.lamp_cell[l] / w) as f64 - off;
            if r < 0.0 || r >= shown as f64 - 1.0 {
                continue;
            }
            self.lamp_cell[kept] = r as usize * w + self.lamp_cell[l] % w;
            self.lamp_color[kept] = self.lamp_color[l];
            kept += 1;
        }
        self.n_lamps = kept;
    }
    pub(super) fn homeward(&mut self) {
        self.booster_only = true;
        self.geo();
        self.fresh = false;
        self.part = Part::Booster;
        self.returning = true;
        self.state = State::Fly;
        self.fly_time = 120.0;
        self.staged = 7.0;
        self.since_stage = 0.0;
        self.log.fill(7);
        self.layer = LAYER[7];
        self.world.alt = self.host.as_ref().map_or(self.layer + TILE, |h| h.alt);
        self.v = -0.6;
        self.service = 0.0;
    }
    /// Transfer the shared pixel layer while drawing each mini, retaining the
    /// exact painter order without cyclic references to the parent rocket.
    pub(super) fn draw_minis(&mut self, out: &mut Cells) {
        let host = self.host_frame();
        let mut minis = std::mem::take(&mut self.minis);
        for r in &mut minis {
            r.host = Some(host.clone());
            self.swap_layer(r);
            r.draw_site(out);
            self.swap_layer(r);
        }
        self.minis = minis;
    }
    fn swap_layer(&mut self, r: &mut LaunchSite) {
        std::mem::swap(&mut self.pc, &mut r.pc);
        std::mem::swap(&mut self.pa, &mut r.pa);
        std::mem::swap(&mut self.touched, &mut r.touched);
        std::mem::swap(&mut self.list, &mut r.list);
        std::mem::swap(&mut self.n_touched, &mut r.n_touched);
        std::mem::swap(&mut self.lamp_cell, &mut r.lamp_cell);
        std::mem::swap(&mut self.lamp_color, &mut r.lamp_color);
        std::mem::swap(&mut self.n_lamps, &mut r.n_lamps);
    }
}
