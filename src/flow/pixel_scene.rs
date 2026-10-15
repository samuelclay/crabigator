//! Shared pixel painter and frame folding for the newer upstream scenes.
use super::cells::{is_tall, Cells, DEFAULT_COLOR};
use super::night::ease_night;
use super::pixels::{fit_quad, QuadFit, BITS, BRAILLE, NEAR, QUAD};
use super::scene::{Dials, Tint};
use super::waiting::{ease_wait, wait_tone, SEPIA_AMOUNT};
pub const CLEAR: u32 = DEFAULT_COLOR;
#[derive(Clone, Copy, Debug)]
pub struct PixelDials {
    pub level: f64,
    pub t: f64,
    pub night: f64,
    pub tint: Tint,
    pub boost: f64,
    pub wait: f64,
    pub columns: usize,
    pub rows: usize,
    pub tall: bool,
}
#[derive(Default, Debug)]
pub struct Painter {
    pub w: usize,
    pub h: usize,
    pub dw: usize,
    pub dh: usize,
    pub px: Vec<u32>,
    pub keep: Vec<u8>,
    pub lamps: Vec<i32>,
    pub dots: Vec<u8>,
    pub dot_color: Vec<u32>,
}
impl Painter {
    pub fn new(columns: usize, rows: usize) -> Self {
        Self {
            w: columns * 2,
            h: rows * 2,
            dw: columns * 2,
            dh: rows * 4,
            px: vec![0; columns * rows * 4],
            keep: vec![0; columns * rows * 4],
            lamps: vec![-1; columns * rows],
            dots: vec![0; columns * rows],
            dot_color: vec![0; columns * rows],
        }
    }
    pub fn clear(&mut self, color: u32) {
        self.px.fill(color);
        self.keep.fill(0);
        self.lamps.fill(-1);
        self.dots.fill(0);
    }
    pub fn set(&mut self, x: f64, y: f64, color: u32, keep: bool) {
        let x = super::js::i32_of(x);
        let y = super::js::i32_of(y);
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        let i = y as usize * self.w + x as usize;
        self.px[i] = color;
        if keep {
            self.keep[i] = 1;
        }
    }
    pub fn get(&self, x: f64, y: f64) -> u32 {
        let x = super::js::i32_of(x);
        let y = super::js::i32_of(y);
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            CLEAR
        } else {
            self.px[y as usize * self.w + x as usize]
        }
    }
    pub fn lamp(&mut self, x: f64, y: f64, color: u32) {
        let x = super::js::i32_of(x);
        let y = super::js::i32_of(y);
        if x < 0 || y < 0 || x as usize >= self.w || y as usize >= self.h {
            return;
        }
        self.set(x as f64, y as f64, color, true);
        self.lamps[(y as usize >> 1) * (self.w >> 1) + (x as usize >> 1)] = color as i32;
    }
    pub fn rect(&mut self, x: f64, y: f64, w: f64, h: f64, color: u32, keep: bool) {
        for yy in super::js::i32_of(y).max(0)..super::js::i32_of(y + h).min(self.h as i32) {
            for xx in super::js::i32_of(x).max(0)..super::js::i32_of(x + w).min(self.w as i32) {
                self.set(xx as f64, yy as f64, color, keep);
            }
        }
    }
    pub fn dot(&mut self, x: f64, y: f64, color: u32) {
        let x = super::js::i32_of(x);
        let y = super::js::i32_of(y);
        if x < 0 || y < 0 || x as usize >= self.dw || y as usize >= self.dh {
            return;
        }
        let c = (y as usize >> 2) * (self.dw >> 1) + (x as usize >> 1);
        self.dots[c] |= BRAILLE[x as usize & 1][y as usize & 3] as u8;
        self.dot_color[c] = color;
    }
}
#[derive(Debug)]
pub struct PixelScene {
    pub dials: Dials,
    pub level_ease: f64,
    pub painter: Painter,
    pub out: Cells,
    level: f64,
    k_night: f64,
    k_wait: f64,
    t: f64,
}
impl PixelScene {
    pub fn new(_seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            level_ease: 0.05,
            painter: Painter::default(),
            out: Cells::default(),
            level: f64::NAN,
            k_night: -1.0,
            k_wait: 0.0,
            t: 0.0,
        }
    }
    pub fn frame_dials(&self) -> PixelDials {
        PixelDials {
            level: if self.level.is_nan() {
                self.dials.strength.clamp(0.0, 10.0)
            } else {
                self.level
            },
            t: self.t,
            night: self.k_night.max(0.0),
            tint: self.dials.tint,
            boost: self.dials.coverage_boost,
            wait: self.k_wait,
            columns: self.out.columns,
            rows: self.out.rows,
            tall: is_tall(self.out.columns, self.out.rows),
        }
    }
    pub fn ensure(&mut self, columns: usize, rows: usize) -> bool {
        if self.out.columns == columns && self.out.rows == rows {
            return false;
        }
        self.out = Cells::new(columns, rows);
        self.painter = Painter::new(columns, rows);
        true
    }
    pub fn step(&mut self) -> PixelDials {
        self.t += 1.0;
        let want = self.dials.strength.clamp(0.0, 10.0);
        if self.level.is_nan() {
            self.level = want;
        }
        let dl = want - self.level;
        self.level += if dl.abs() < 0.01 {
            dl
        } else {
            dl * self.level_ease
        };
        self.k_night = ease_night(self.k_night, self.dials.night);
        self.k_wait = ease_wait(self.k_wait, self.dials.waiting);
        self.frame_dials()
    }
    pub fn begin_paint(&mut self) -> Painter {
        let mut painter = std::mem::take(&mut self.painter);
        painter.clear(CLEAR);
        painter
    }
    pub fn blank(&mut self) -> &Cells {
        for i in 0..self.out.len() {
            self.out.blank(i);
        }
        &self.out
    }
    pub fn finish_grid(&mut self, p: Painter) -> &Cells {
        for cell in 0..self.out.len() {
            let r = cell / self.out.columns;
            let c = cell - r * self.out.columns;
            let k0 = 2 * r * p.w + 2 * c;
            let ks = [k0, k0 + 1, k0 + p.w, k0 + p.w + 1];
            let q = ks.map(|k| p.px[k]);
            let mut clear = 0usize;
            let mut keep = -1;
            for j in 0..4 {
                if q[j] == CLEAR {
                    clear |= 1 << j;
                } else if keep < 0 && p.keep[ks[j]] != 0 {
                    keep = j as i32;
                }
            }
            let (mask, fg, bg) = if clear == 15 {
                (0, CLEAR, CLEAR)
            } else if clear != 0 {
                let (mut sr, mut sg, mut sb, mut count) = (0u32, 0u32, 0u32, 0u32);
                for (j, &color) in q.iter().enumerate() {
                    if clear & (1 << j) != 0 {
                        continue;
                    }
                    sr += (color >> 16) & 255;
                    sg += (color >> 8) & 255;
                    sb += color & 255;
                    count += 1;
                }
                let avg = |s| ((s as f64 / count as f64) + 0.5) as u32;
                (
                    15 ^ clear,
                    if keep >= 0 {
                        q[keep as usize]
                    } else {
                        (avg(sr) << 16) | (avg(sg) << 8) | avg(sb)
                    },
                    CLEAR,
                )
            } else {
                let mut fit = QuadFit::default();
                fit_quad(&q, &mut fit, NEAR, keep);
                if fit.spread == 0 {
                    (0, q[0], q[0])
                } else {
                    (fit.mask as usize, fit.fg, fit.bg)
                }
            };
            if p.dots[cell] != 0 {
                self.out.set(
                    cell,
                    0x2800 | u32::from(p.dots[cell]),
                    p.dot_color[cell],
                    if BITS[mask] > 2 { fg } else { bg },
                );
            } else if mask == 0 && bg == CLEAR {
                self.out.blank(cell);
            } else {
                self.out.set(cell, QUAD[mask], fg, bg);
            }
        }
        wait_tone(
            &mut self.out,
            self.k_wait,
            self.t,
            SEPIA_AMOUNT,
            Some(&p.lamps),
        );
        self.painter = p;
        &self.out
    }
}
