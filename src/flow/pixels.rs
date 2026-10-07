//! Small pieces the scenes share: packed-RGB color math, hashes, glyph
//! tables, and the fit of a cell's four quadrant pixels to the two colors of
//! one quadrant glyph (flow's `hooks/pixels.ts`).

use super::js::{i32_of, imul};

/// A glyph's code point.
pub const fn g(ch: char) -> u32 {
    ch as u32
}

/// `v` held to lo..hi.
#[inline]
pub fn clamp(v: f64, lo: f64, hi: f64) -> f64 {
    if v < lo {
        lo
    } else if v > hi {
        hi
    } else {
        v
    }
}

#[inline]
pub fn clamp01(v: f64) -> f64 {
    clamp(v, 0.0, 1.0)
}

/// `a` moved `k` (0..1, held there) of the way to `b`, per channel of 0xRRGGBB.
pub fn mix(a: u32, b: u32, k: f64) -> u32 {
    if k <= 0.0 {
        return a;
    }
    if k >= 1.0 {
        return b;
    }
    let ch = |shift: u32| {
        let x = f64::from((a >> shift) & 255);
        let y = f64::from((b >> shift) & 255);
        i32_of(x + (y - x) * k + 0.5)
    };
    ((ch(16) << 16) | (ch(8) << 8) | ch(0)) as u32
}

/// Squared RGB distance between two 0xRRGGBB colors.
pub fn dist(a: u32, b: u32) -> i32 {
    let r = ((a >> 16) & 255) as i32 - ((b >> 16) & 255) as i32;
    let gg = ((a >> 8) & 255) as i32 - ((b >> 8) & 255) as i32;
    let bb = (a & 255) as i32 - (b & 255) as i32;
    r * r + gg * gg + bb * bb
}

/// A stable hash of a world position (integers) to [0, 1).
pub fn hash(x: f64, y: f64, salt: f64) -> f64 {
    let mut h = i32_of(x * 374_761_393.0 + y * 668_265_263.0 + salt * 2_147_483_647.0);
    h = imul(h ^ ((h as u32) >> 13) as i32, 1_274_126_177);
    h ^= ((h as u32) >> 16) as i32;
    f64::from(h as u32) / 4_294_967_296.0
}

/// A stable hash of one integer to [0, 1).
pub fn hash1(n: f64) -> f64 {
    let mut h = imul(i32_of(n) ^ 0x9e37_79b9_u32 as i32, 0x85eb_ca6b_u32 as i32);
    h ^= ((h as u32) >> 13) as i32;
    h = imul(h, 0xc2b2_ae35_u32 as i32);
    h ^= ((h as u32) >> 16) as i32;
    f64::from(h as u32) / 4_294_967_296.0
}

/// A stable hash of (x, y, salt) to [0, 1), murmur-finalized (the ski run's).
pub fn hash_murmur(x: f64, y: f64, s: f64) -> f64 {
    let mut h = imul(i32_of(x), 0x27d4_eb2d)
        ^ imul(i32_of(y).wrapping_add(0x3c6e_f372), 0x1656_67b1)
        ^ imul(i32_of(s), 0x9e37_79b9_u32 as i32);
    h = imul(h ^ ((h as u32) >> 15) as i32, 0x85eb_ca6b_u32 as i32);
    h = imul(h ^ ((h as u32) >> 13) as i32, 0xc2b2_ae35_u32 as i32);
    h ^= ((h as u32) >> 16) as i32;
    f64::from(h as u32) / 4_294_967_296.0
}

/// Quadrant glyphs by pixel mask: top-left 1, top-right 2, bottom-left 4, bottom-right 8.
pub const QUAD: [u32; 16] = [
    0x20, 0x2598, 0x259d, 0x2580, 0x2596, 0x258c, 0x259e, 0x259b, 0x2597, 0x259a, 0x2590, 0x259c,
    0x2584, 0x2599, 0x259f, 0x2588,
];

/// The lower-block glyph filling `eighths` (0..8) of a cell from the bottom.
pub fn lower_block(eighths: f64) -> u32 {
    if eighths <= 0.0 {
        0x20
    } else {
        0x2580 + eighths.min(8.0) as u32
    }
}

/// Pixels this close (squared RGB distance) to a group's seed blend into it.
pub const NEAR: i32 = 1200;

/// One color for a quadrant group: the seed blended with its near pixels.
fn group_color(q: &[u32; 4], mask: u32, want: u32, seed: u32, near: i32) -> u32 {
    let (mut r, mut gg, mut b, mut n) = (0.0, 0.0, 0.0, 0.0);
    for (p, &v) in q.iter().enumerate() {
        if (mask >> p) & 1 != want || dist(v, seed) > near {
            continue;
        }
        r += f64::from((v >> 16) & 255);
        gg += f64::from((v >> 8) & 255);
        b += f64::from(v & 255);
        n += 1.0;
    }
    if n == 0.0 {
        return seed;
    }
    ((i32_of(r / n + 0.5) << 16) | (i32_of(gg / n + 0.5) << 8) | i32_of(b / n + 0.5)) as u32
}

/// Braille dot bits by (column 0..1, row 0..3) within a cell: `0x2800 | bits` is the glyph.
pub const BRAILLE: [[u32; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// A cell's four pixels fitted to one quadrant glyph: see `fit_quad`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct QuadFit {
    pub mask: u32,
    pub fg: u32,
    pub bg: u32,
    pub spread: i32,
}

/// Fold a cell's four pixels (top-left, top-right, bottom-left, bottom-right)
/// into the two colors that best fit them. `keep` (0..3, or -1) names a pixel
/// whose color must survive.
pub fn fit_quad(q: &[u32; 4], fit: &mut QuadFit, near: i32, keep: i32) {
    let (mut bi, mut bj, mut best) = (0usize, 0usize, 0);
    for i in 0..3 {
        for j in i + 1..4 {
            let d = dist(q[i], q[j]);
            if d > best {
                best = d;
                bi = i;
                bj = j;
            }
        }
    }
    fit.spread = best;
    if keep >= 0 && best > 0 {
        let keep = keep as usize;
        let (mut far, mut far_d) = (0usize, -1);
        for (p, &v) in q.iter().enumerate() {
            let d = dist(v, q[keep]);
            if d > far_d {
                far_d = d;
                far = p;
            }
        }
        if far_d > 0 {
            bi = keep;
            bj = far;
        }
    }
    if best == 0 {
        fit.mask = 0;
        fit.fg = q[0];
        fit.bg = q[0];
        return;
    }
    let (si, sj) = (q[bi], q[bj]);
    let mut mask = 0;
    for (p, &v) in q.iter().enumerate() {
        if dist(v, sj) < dist(v, si) {
            mask |= 1 << p;
        }
    }
    fit.mask = mask;
    fit.fg = group_color(q, mask, 1, sj, near);
    fit.bg = group_color(q, mask, 0, si, near);
}
