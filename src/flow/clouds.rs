//! The layered cloud painter the balloon and the rockets fly through
//! (flow's `hooks/clouds/layered.ts`): soft cumulus low, broken banks in the
//! middle, cirrus streaks high. Pure in (x, y): the world scrolls past, so a
//! cell looks the same every time it's asked about the same place. Every
//! cell is two half cells (▀: the top half in fg, the bottom in bg), each
//! composited back to front as the sky blended toward each layer's colour.

use std::cell::RefCell;
use std::collections::HashMap;

use super::js::round;
use super::pixels::{clamp01, hash, mix};

const TOP_HALF: u32 = 0x2580; // ▀

/// One cell of cloud: a glyph, its colour, and its background.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct CloudCell {
    pub glyph: u32,
    pub fg: u32,
    pub bg: u32,
}

fn smooth(k: f64) -> f64 {
    k * k * (3.0 - 2.0 * k)
}

fn smoothstep(a: f64, b: f64, v: f64) -> f64 {
    smooth(clamp01((v - a) / (b - a)))
}

/// Smooth value noise in [0, 1) at a real (u, v).
pub fn noise(u: f64, v: f64, salt: f64) -> f64 {
    let iu = u.floor();
    let iv = v.floor();
    let fu = smooth(u - iu);
    let fv = smooth(v - iv);
    let a = hash(iu, iv, salt);
    let b = hash(iu + 1.0, iv, salt);
    let c = hash(iu, iv + 1.0, salt);
    let d = hash(iu + 1.0, iv + 1.0, salt);
    let top = a + (b - a) * fu;
    top + (c + (d - c) * fu - top) * fv
}

// ── Low: feathered cumulus heaps ──────────────────────────────────────────

const SLOT_W: f64 = 38.0;
const SLOT_H: f64 = 4.6;
const SLOT_FROM: f64 = 6.6;
const SLOT_ROWS: f64 = 3.0;
const MAX_PUFFS: usize = 12;
const FEATHER: f64 = 1.7;

const DEEP: u32 = 0x7d8fb0;
const SHADOW: u32 = 0xa3b2cb;
const MID: u32 = 0xd2dbe8;
const LIGHT: u32 = 0xf0f3f8;
const SUN: u32 = 0xffffff;

const LX: f64 = -0.45;
const LY: f64 = 0.82;
const LZ: f64 = 0.45;

#[derive(Clone)]
struct Heap {
    n: usize,
    base: f64,
    top: f64,
    x0: f64,
    x1: f64,
    px: [f64; MAX_PUFFS],
    py: [f64; MAX_PUFFS],
    pr: [f64; MAX_PUFFS],
}

thread_local! {
    static HEAPS: RefCell<HashMap<i64, Heap>> = RefCell::new(HashMap::new());
    static ELEMENTS: RefCell<HashMap<i64, [f64; 5]>> = RefCell::new(HashMap::new());
}

fn heap_at(sx: f64, sy: f64) -> Heap {
    let key = (sx * 8.0 + sy) as i64;
    HEAPS.with(|heaps| {
        let mut heaps = heaps.borrow_mut();
        if let Some(c) = heaps.get(&key) {
            return c.clone();
        }
        if heaps.len() > 4096 {
            heaps.clear();
        }
        let c = build(sx, sy);
        heaps.insert(key, c.clone());
        c
    })
}

fn build(sx: f64, sy: f64) -> Heap {
    let mut c = Heap {
        n: 0,
        base: 0.0,
        top: 0.0,
        x0: 0.0,
        x1: 0.0,
        px: [0.0; MAX_PUFFS],
        py: [0.0; MAX_PUFFS],
        pr: [0.0; MAX_PUFFS],
    };
    if !(0.0..SLOT_ROWS).contains(&sy) {
        return c;
    }
    let odds = if sy == 0.0 {
        0.58
    } else if sy == 1.0 {
        0.54
    } else {
        0.4
    };
    if hash(sx, sy, 110.0) > odds {
        return c;
    }
    let base_row = SLOT_FROM + sy * SLOT_H + hash(sx, sy, 111.0) * (SLOT_H - 1.2);
    let s = hash(sx, sy, 112.0);
    let size = (if s < 0.4 {
        0.35 + s
    } else if s < 0.85 {
        0.75 + (s - 0.4) * 0.5
    } else {
        1.0 + (s - 0.85) * 3.0
    }) * if sy == 2.0 { 0.7 } else { 1.0 };
    let width = 8.0 + size * 20.0;
    let height = 3.0 + size * 4.5 + (size - 1.0).max(0.0) * 5.0;
    let cx = sx * SLOT_W + 7.0 + hash(sx, sy, 113.0) * (SLOT_W - 14.0);
    let base = base_row * 2.0;
    let (px, py, pr) = (&mut c.px, &mut c.py, &mut c.pr);

    let n0 = round(width / 6.0).clamp(2.0, 7.0) as usize;
    let r_main = height / 1.2;
    let mut k = 0;
    let mut run = 0.0;
    for i in 0..n0 {
        let fi = i as f64;
        let u = (fi / (n0 as f64 - 1.0)) * 2.0 - 1.0;
        let r =
            r_main * (0.6 + 0.4 * (1.0 - u * u)) * (0.85 + 0.3 * hash(sx, sy * 7.0 + fi, 116.0));
        if i > 0 {
            run += (pr[i - 1] + r) * (0.55 + 0.2 * hash(sx * 31.0 + fi, sy, 115.0));
        }
        px[k] = run;
        py[k] = base + r * (0.12 + 0.25 * hash(sx + fi, sy, 117.0));
        pr[k] = r;
        k += 1;
    }
    for p in px.iter_mut().take(n0) {
        *p = *p - run / 2.0 + cx;
    }
    let n1 = (MAX_PUFFS - n0).min(1 + (size * 2.5 + hash(sx, sy, 118.0) * 2.0).floor() as usize);
    for j in 0..n1 {
        let fj = j as f64;
        let i = (hash(sx, sy + fj, 119.0) * n0 as f64 * 0.6 + n0 as f64 * 0.2).floor();
        let host = i.max(0.0).min(n0 as f64 - 1.0) as usize;
        let a = (hash(sx, sy + fj, 120.0) - 0.5) * 1.6;
        let hr = pr[host];
        let r = (hr * (0.5 + 0.25 * hash(sx + fj, sy, 121.0))).max(1.8);
        px[k] = px[host] + a.sin() * hr * 0.75;
        py[k] = py[host] + a.cos() * hr * 0.55 + r * 0.25;
        pr[k] = r;
        k += 1;
    }
    // Back to front: higher puffs farther back.
    for i in 1..k {
        let mut j = i;
        while j > 0 && py[j - 1] + pr[j - 1] * 0.3 < py[j] + pr[j] * 0.3 {
            px.swap(j, j - 1);
            py.swap(j, j - 1);
            pr.swap(j, j - 1);
            j -= 1;
        }
    }
    let (mut top, mut x0, mut x1) = (base + 1.0, f64::INFINITY, f64::NEG_INFINITY);
    for i in 0..k {
        top = top.max(py[i] + pr[i]);
        x0 = x0.min(px[i] - pr[i]);
        x1 = x1.max(px[i] + pr[i]);
    }
    if top > 46.0 {
        return c;
    }
    c.n = k;
    c.base = base;
    c.top = top;
    c.x0 = x0 - FEATHER;
    c.x1 = x1 + FEATHER;
    c
}

fn shade(l: f64) -> u32 {
    if l < 0.2 {
        mix(DEEP, SHADOW, l / 0.2)
    } else if l < 0.5 {
        mix(SHADOW, MID, (l - 0.2) / 0.3)
    } else if l < 0.78 {
        mix(MID, LIGHT, (l - 0.5) / 0.28)
    } else {
        mix(LIGHT, SUN, ((l - 0.78) / 0.22).min(1.0))
    }
}

/// Cumulus coverage and colour at column wx, half row qy.
fn cumulus(wx: f64, qy: f64) -> (f64, u32) {
    let (mut out_a, mut out_c) = (0.0, 0);
    let sxc = (wx / SLOT_W).floor();
    let y = qy / 2.0;
    let syc = ((y - SLOT_FROM) / SLOT_H).floor();
    for dy in -2..=0 {
        for dx in -1..=1 {
            let c = heap_at(sxc + f64::from(dx), syc + f64::from(dy));
            if c.n == 0 || qy < c.base - 1.0 || qy > c.top + FEATHER || wx < c.x0 || wx > c.x1 {
                continue;
            }
            let mut sd = f64::NEG_INFINITY;
            let (mut wsum, mut nx, mut ny) = (0.0, 0.0, 0.0);
            for i in 0..c.n {
                let ox = wx - c.px[i];
                let oy = qy - c.py[i];
                let r = c.pr[i];
                let s = r - (ox * ox + oy * oy).sqrt();
                if s > sd {
                    sd = s;
                }
                if s < -FEATHER {
                    continue;
                }
                let mut w = (s.min(4.0) + FEATHER) * (1.0 + i as f64 * 0.12);
                w *= w;
                w *= w;
                w *= w;
                wsum += w;
                nx += (ox / r) * w;
                ny += (oy / r) * w;
            }
            if wsum == 0.0 {
                continue;
            }
            let rough = (noise(wx / 2.3, qy / 2.1, 131.0) - 0.5) * 1.8
                + (noise(wx / 1.1, qy / 1.2, 132.0) - 0.5) * 0.7;
            let mut a = smoothstep(-FEATHER, 1.4, sd + rough);
            a *= smoothstep(c.base - 1.8, c.base + 1.6, qy + rough * 0.5);
            if a <= out_a {
                continue;
            }
            nx /= wsum;
            ny /= wsum;
            let l2 = nx * nx + ny * ny;
            if l2 > 1.0 {
                let l = l2.sqrt();
                nx /= l;
                ny /= l;
            }
            let nz = (1.0 - nx * nx - ny * ny).max(0.0).sqrt();
            let dot = nx * LX + ny * LY + nz * LZ;
            let h = (qy - c.base) / (c.top - c.base);
            let mut lit = 0.42 + dot * 0.72 + (h - 0.4) * 0.6;
            let under = smoothstep(-0.5, (0.35 * (c.top - c.base)).min(4.0), qy - c.base);
            lit = lit.min(0.06 + under * 1.1);
            out_a = a * 0.98;
            out_c = shade(clamp01(lit));
        }
    }
    (out_a, out_c)
}

// ── Middle: stratocumulus and altocumulus ─────────────────────────────────

const BANK_LIT: u32 = 0xeef2f8;
const BANK_SHADE: u32 = 0x8b9bb8;
const ALTO_LIT: u32 = 0xd6dfee;
const ALTO_SHADE: u32 = 0x7385a8;

const DECK_Y: [f64; 2] = [16.8, 22.8];
const DECK_ROWS: [f64; 2] = [3.0, 4.0];
const CELL_W: [f64; 2] = [15.0, 8.0];
const CELL_H: [f64; 2] = [2.1, 1.9];
const RX: [f64; 2] = [8.5, 3.8];
const RY: [f64; 2] = [1.7, 1.05];
const DECK_ALPHA: [f64; 2] = [0.94, 0.86];
/// An element's lumps start somewhere in this span: flow's own literal, not quite TAU.
#[allow(clippy::approx_constant)]
const LUMP_PHASES: f64 = 6.28;

/// Element (ex, ey, rx, ry, phase; rx = 0 for none) in grid cell (i, j) of deck k.
fn element_at(k: usize, i: f64, j: f64) -> [f64; 5] {
    let key = (i * 16.0 + k as f64 * 8.0 + j) as i64;
    ELEMENTS.with(|elements| {
        let mut elements = elements.borrow_mut();
        if let Some(el) = elements.get(&key) {
            return *el;
        }
        if elements.len() > 4096 {
            elements.clear();
        }
        let mut el = [0.0; 5];
        let kf = k as f64;
        let cw = CELL_W[k];
        let crowd = noise(
            (i * cw) / if k == 0 { 75.0 } else { 55.0 } + kf * 11.3,
            j * 0.35 + kf * 4.1,
            173.0,
        );
        let odds = (if k == 0 {
            smoothstep(0.32, 0.72, crowd)
        } else {
            smoothstep(0.24, 0.62, crowd)
        }) * if j == DECK_ROWS[k] - 1.0 { 0.5 } else { 0.85 };
        if hash(i, j, 175.0 + kf) <= odds {
            let size = 0.55 + 0.7 * hash(i, j, 177.0 + kf) + 0.25 * crowd;
            el[0] = (i + 0.5 + (hash(i, j, 179.0 + kf) - 0.5) * 0.9) * cw;
            el[1] = DECK_Y[k] + (j + 0.5 + (hash(i, j, 181.0 + kf) - 0.5) * 0.85) * CELL_H[k];
            el[2] = RX[k] * size * if crowd > 0.6 { 1.25 } else { 1.0 };
            el[3] = RY[k] * size;
            el[4] = hash(i, j, 183.0) * LUMP_PHASES;
        }
        elements.insert(key, el);
        el
    })
}

/// Coverage and colour of deck k at column x, real row y.
fn bank(k: usize, x: f64, y: f64) -> (f64, u32) {
    let y0 = DECK_Y[k];
    let cw = CELL_W[k];
    let ch = CELL_H[k];
    if y < y0 - 1.6 || y > y0 + DECK_ROWS[k] * ch + 2.0 {
        return (0.0, 0);
    }
    let ci = (x / cw).floor();
    let cj = ((y - y0) / ch).floor();
    let mut rough: Option<f64> = None;
    let (mut best, mut best_lit) = (0.0, 0.0);
    for dj in -1..=1 {
        let j = cj + f64::from(dj);
        if j < 0.0 || j >= DECK_ROWS[k] {
            continue;
        }
        for di in -1..=1 {
            let el = element_at(k, ci + f64::from(di), j);
            let rx = el[2];
            if rx == 0.0 {
                continue;
            }
            let ox = (x - el[0]) / rx;
            if !(-1.4..=1.4).contains(&ox) {
                continue;
            }
            let oy_rows = y - el[1];
            let ry = el[3] * if oy_rows < 0.0 { 0.5 } else { 1.28 };
            if oy_rows > ry * 1.3 || oy_rows < -ry * 1.6 {
                continue;
            }
            let lump = if oy_rows > 0.0 {
                (1.0 + 0.28 * (ox * 4.2 + el[4]).sin()) / 1.28
            } else {
                1.0
            };
            let oy = oy_rows / (ry * lump);
            let rough = *rough
                .get_or_insert_with(|| (noise(x / 1.9, y / 0.75, 171.0 + k as f64) - 0.5) * 0.5);
            let e = (ox * ox + oy * oy).sqrt() + rough;
            let a = 1.0 - smoothstep(0.5, 1.15, e);
            if a <= best {
                continue;
            }
            best = a;
            best_lit = clamp01(
                0.5 + 0.38 * oy
                    + 0.2 * (1.0 - e)
                    + if k == 0 { 0.05 } else { 0.0 }
                    + (rough - 0.1) * 0.7,
            );
        }
    }
    if best <= 0.01 {
        return (0.0, 0);
    }
    let c = if k == 0 {
        mix(BANK_SHADE, BANK_LIT, best_lit)
    } else {
        mix(ALTO_SHADE, ALTO_LIT, best_lit)
    };
    (best * DECK_ALPHA[k], c)
}

// ── High: cirrus streaks ──────────────────────────────────────────────────

const CIRRUS_DIM: u32 = 0x8a9dc0;
const CIRRUS_BRIGHT: u32 = 0xdbe4f2;

fn cirrus(x: f64, y: f64) -> (f64, u32) {
    if !(27.5..=44.6).contains(&y) {
        return (0.0, 0);
    }
    let patch = smoothstep(0.42, 0.7, noise(x / 64.0 + 4.2, y / 7.0 + 1.1, 161.0));
    if patch <= 0.0 {
        return (0.0, 0);
    }
    let f =
        noise(x / 34.0 + y * 0.12, y / 1.15, 163.0) * 0.75 + noise(x / 11.0, y / 0.8, 165.0) * 0.25;
    let ridge = 1.0 - (f - 0.5).abs() * 2.0;
    let mut a = smoothstep(0.72, 0.97, ridge);
    if a <= 0.0 {
        return (0.0, 0);
    }
    a *= 0.55 + 0.45 * noise(x / 2.2, y * 3.1, 167.0);
    a *= patch;
    a *= smoothstep(27.5, 32.0, y) * (1.0 - smoothstep(42.5, 44.8, y));
    if a <= 0.01 {
        return (0.0, 0);
    }
    let c = mix(
        CIRRUS_DIM,
        CIRRUS_BRIGHT,
        clamp01(ridge * 1.2 - 0.2) * (1.0 - smoothstep(30.0, 44.0, y) * 0.5),
    );
    (a * 0.62, c)
}

/// The colour of a half cell at column x, real row y: sky, then each layer over it.
fn half(x: f64, y: f64, sky: u32) -> u32 {
    let mut col = sky;
    let (a, c) = cirrus(x + 0.5, y);
    if a > 0.0 {
        col = mix(col, c, a);
    }
    for k in [1, 0] {
        let (a, c) = bank(k, x + 0.5, y);
        if a > 0.0 {
            col = mix(col, mix(c, sky, smoothstep(22.0, 32.0, y) * 0.25), a);
        }
    }
    if y < 24.5 {
        let (a, c) = cumulus(x + 0.5, y * 2.0);
        if a > 0.0 {
            col = mix(col, c, a);
        }
    }
    col
}

const Q_DAY: f64 = 12.0;
const Q_DARK: f64 = 8.0;

/// A cloud colour snapped to steps away from the sky behind it (the sky
/// itself within half a step), so a frame keeps few distinct colours.
pub fn snap(c: u32, sky: u32) -> u32 {
    let dark = ((sky >> 16) & 255) < 64 && ((sky >> 8) & 255) < 64 && (sky & 255) < 64;
    let q = if dark { Q_DARK } else { Q_DAY };
    let ch = |sh: u32| {
        let s = f64::from((sky >> sh) & 255);
        let v = f64::from((c >> sh) & 255);
        (s + round((v - s) / q) * q).clamp(0.0, 255.0) as u32
    };
    (ch(16) << 16) | (ch(8) << 8) | ch(0)
}

/// The cloud cell at world (x, y), or `None` for open sky.
pub fn layered(x: f64, y: f64, sky: u32) -> Option<CloudCell> {
    let top = half(x, y + 0.25, sky);
    let bot = half(x, y - 0.25, sky);
    if top == sky && bot == sky {
        return None;
    }
    Some(CloudCell {
        glyph: TOP_HALF,
        fg: top,
        bg: bot,
    })
}
