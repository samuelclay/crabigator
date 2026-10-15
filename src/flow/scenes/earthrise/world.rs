//! The original lunar height field and lit, rotating globe. Float32 storage
//! deliberately follows the upstream typed arrays.
use super::*;
use std::sync::OnceLock;
const EYE: f64 = 37.;
const F: f64 = 112.;
const RM: f64 = 1500.;
const ZMAX: f64 = 520.;
const ER: f64 = 22.;
const TW: usize = 192;
const TH: usize = 96;
fn norm(v: [f64; 3]) -> [f64; 3] {
    let m = v.iter().fold(0.0_f64, |a, b| a.max(b.abs()));
    let (mut sum, mut compensation) = (0., 0.);
    for x in v {
        let n = x.abs() / m;
        let s = n * n - compensation;
        let p = sum + s;
        compensation = (p - sum) - s;
        sum = p;
    }
    let l = sum.sqrt() * m;
    v.map(|c| c / l)
}
fn sun() -> [f64; 3] {
    norm([0.94, 0.14, -0.3])
}
fn light() -> [f64; 3] {
    norm([0.82, 0.16, 0.75])
}
fn hypot3(x: f64, y: f64, z: f64) -> f64 {
    let max = x.abs().max(y.abs()).max(z.abs());
    if max == 0. {
        return 0.;
    }
    let (mut sum, mut c) = (0., 0.);
    for v in [x, y, z] {
        let n = v / max;
        let s = n * n - c;
        let p = sum + s;
        c = (p - sum) - s;
        sum = p;
    }
    sum.sqrt() * max
}
pub(super) fn sun_screen() -> (f64, f64) {
    let s = sun();
    let l = hypot(s[0], s[1]);
    (s[0] / l, -s[1] / l)
}
fn h(x: f64, y: f64) -> f64 {
    hash(x, y, 0.)
}
pub(super) fn noise(x: f64, y: f64, period: f64) -> f64 {
    let xi = x.floor();
    let yi = y.floor();
    let fx = x - xi;
    let fy = y - yi;
    let u = fx * fx * (3. - 2. * fx);
    let v = fy * fy * (3. - 2. * fy);
    let mut x0 = xi;
    let mut x1 = xi + 1.;
    if period != 0. {
        x0 = ((xi % period) + period) % period;
        x1 = (x0 + 1.) % period;
    }
    let a = h(x0, yi);
    let b = h(x1, yi);
    let c = h(x0, yi + 1.);
    let d = h(x1, yi + 1.);
    a + (b - a) * u + (c - a) * v + (a - b - c + d) * u * v
}
pub(super) fn fbm(x: f64, y: f64, octaves: usize, period: f64) -> f64 {
    let (mut s, mut n, mut amp, mut f) = (0., 0., 0.5, 1.);
    for _ in 0..octaves {
        s += amp * noise(x * f, y * f, period * f);
        n += amp;
        amp *= 0.5;
        f *= 2.;
    }
    s / n
}
fn bowl(q: f64, r: f64) -> f64 {
    r * ((if q < 1. { -0.36 * (1. - q * q) } else { 0. })
        + 0.14 * (-((q - 1.) / 0.25).powi(2)).exp())
}
fn craters(x: f64, z: f64, cell: f64, salt: f64, r0: f64, r1: f64, p: f64) -> f64 {
    let ci = (x / cell).floor() as i32;
    let cj = (z / cell).floor() as i32;
    let mut out = 0.;
    for j in cj - 1..=cj + 1 {
        let j = j as f64;
        for i in ci - 1..=ci + 1 {
            let i = i as f64;
            if h(i + salt, j - salt) > p {
                continue;
            }
            let cx = (i + h(i, j + salt * 3.)) * cell;
            let cz = (j + h(i + salt * 5., j)) * cell;
            let e = h(j + salt, i - salt * 7.);
            let r = cell * (r0 + (r1 - r0) * e * e);
            let dx = x - cx;
            let dz = z - cz;
            let d2 = dx * dx + dz * dz;
            if d2 > 4. * r * r {
                continue;
            }
            out += bowl(d2.sqrt() / r, r) * 0.95;
        }
    }
    out
}
fn height(x: f64, z: f64) -> f64 {
    let mut out =
        6. * fbm(x * 0.006 + 50., z * 0.006 + 50., 3, 0.) + 0.6 * fbm(x * 0.04, z * 0.04, 2, 0.);
    if z > 150. {
        let lift = smooth(150., 300., z);
        let m = 1. - (2. * fbm(x * 0.006 + 7., z * 0.006, 4, 0.) - 1.).abs();
        out += lift * (28. * (-((x + 260.) / 230.).powi(2)).exp() + 45. * (m * m - 0.35));
        let ridge = (-((z - 205.) / 20.).powi(2)).exp() * (-((x + 140.) / 130.).powi(2)).exp();
        out += ridge * 22. * (0.35 + fbm(x * 0.018 + 3., z * 0.01, 3, 0.));
    }
    if z > 90. {
        out += craters(x, z, 110., 11., 0.14, 0.34, 0.55) * smooth(90., 150., z);
    }
    if z < 300. {
        out += craters(x, z, 34., 23., 0.12, 0.36, 0.85);
    }
    if z < 170. {
        out += craters(x, z, 10., 37., 0.12, 0.34, 0.85) * smooth(170., 110., z);
    }
    let dx = x + 24.;
    let dz = z - 58.;
    let q = (dx * dx + dz * dz).sqrt() / 16.;
    if q < 2. {
        out += bowl(q, 16.);
    }
    if z < 90. {
        let c = 5.;
        let ci = (x / c).floor();
        let cj = (z / c).floor();
        if h(ci + 91., cj) < 0.14 {
            let bx = (ci + 0.2 + 0.6 * h(ci, cj + 92.)) * c;
            let bz = (cj + 0.2 + 0.6 * h(ci + 93., cj)) * c;
            let br = 0.3 + 0.5 * h(ci + 94., cj + 95.).powi(2);
            let d2 = ((x - bx).powi(2) + (z - bz).powi(2)) / (br * br);
            if d2 < 4. {
                out += br * 0.9 * (-d2 * 1.4).exp();
            }
        }
        for [bx, bz, br] in [
            [30., 50., 2.4],
            [62., 63., 1.8],
            [12., 70., 1.3],
            [-4., 45., 1.2],
            [90., 55., 1.6],
            [46., 90., 1.4],
        ] {
            let d2 = ((x - bx).powi(2) + (z - bz).powi(2)) / (br * br);
            if d2 < 1. {
                out += br * (0.85 + 0.3 * noise(x * 1.3, z * 1.3, 0.)) * (1. - d2).sqrt();
            }
        }
    }
    out
}
fn cam_y() -> f64 {
    static Y: OnceLock<f64> = OnceLock::new();
    *Y.get_or_init(|| height(0., 7.) + 20.)
}
fn skyline(ang: f64) -> f64 {
    let sx = ang.sin();
    let cz = ang.cos();
    let mut top = f64::INFINITY;
    let mut d = 20.;
    while d < ZMAX {
        let y = height(sx * d, cz * d) - (d * d) / (2. * RM);
        top = top.min(EYE - (F * (y - cam_y())) / d);
        d += 0.03 + d * 0.012;
    }
    top
}
fn ground_light(x: f64, z: f64, y: f64, cam: f64, crest: f64) -> f64 {
    let e = 0.1 + z * 0.004;
    let hx = (height(x + e, z) - height(x - e, z)) / (2. * e);
    let hz = (height(x, z + e) - height(x, z - e)) / (2. * e);
    let nl = hypot3(hx, 1., hz);
    let sun = sun();
    let lam = (-hx * sun[0] + sun[1] - hz * sun[2]) / nl;
    let vx = -x;
    let vy = cam - y;
    let vz = -z;
    let vl = hypot3(vx, vy, vz);
    let mu = 0.02_f64.max((-hx * vx + vy - hz * vz) / (nl * vl));
    let mut lit: f64 = 0.;
    if lam > 0. {
        lit = 1.;
        let mut s =
            (0.1 + z * 0.003) * (0.6 + 0.8 * hash((x * 37.).floor(), (z * 37.).floor(), 41.));
        while s < 120. {
            let d = y + sun[1] * s - height(x + sun[0] * s, z + sun[2] * s);
            if d < 0. {
                lit = 0.;
                break;
            }
            lit = lit.min((d * 30.) / s);
            s += 0.05 + s * 0.2;
        }
        lit = smooth(0., 1., lit);
    }
    let albedo = 0.5
        + 0.9 * fbm(x * 0.025 + 3., z * 0.025, 3, 0.)
        + 0.3 * (fbm(x * 0.35, z * 0.35, 2, 0.) - 0.5)
        - 0.22 * smooth(0.46, 0.64, fbm(x * 0.0035, z * 0.0035 + 20., 3, 0.));
    let ls = if lam > 0. {
        ((0.2 * lam) / (lam + mu) + 2.6 * lam) * lit
    } else {
        0.
    };
    let mut b = (1. - (-ls * 1.5).exp()) * albedo;
    if crest < 2. && lit > 0.2 {
        b = b.max((if crest < 1. { 0.85 } else { 0.55 }) * albedo);
    }
    b
}
struct Maps {
    r: Vec<f32>,
    g: Vec<f32>,
    b: Vec<f32>,
    sea: Vec<f32>,
    cloud: Vec<f32>,
}
fn maps() -> &'static Maps {
    static MAPS: OnceLock<Maps> = OnceLock::new();
    MAPS.get_or_init(|| {
        let n = TW * TH;
        let (mut tr, mut tg, mut tb, mut sea, mut cloud, mut elev) = (
            vec![0f32; n],
            vec![0f32; n],
            vec![0f32; n],
            vec![0f32; n],
            vec![0f32; n],
            vec![0f32; n],
        );
        let storms = [
            [0.9, 0.8, 1.],
            [3.1, -0.85, -1.],
            [4.6, 0.62, 1.],
            [1.9, 0.25, 1.],
            [5.6, -0.55, -1.],
        ];
        for j in 0..TH {
            let v = (j as f64 + 0.5) / TH as f64;
            let lat = (0.5 - v) * PI;
            let al = lat.abs();
            for i in 0..TW {
                let u = i as f64 / TW as f64;
                let lon = u * PI * 2.;
                let t = j * TW + i;
                let wx = fbm(u * 6., v * 3. + 9., 3, 6.);
                elev[t] =
                    (fbm(u * 8. + 1.6 * wx, v * 4., 5, 8.) - 0.04 * smooth(1.2, 1.5, al)) as f32;
                let mut cx = u * 14.;
                let mut cy = v * 7.;
                for [slon, slat, spin] in storms {
                    let mut dl = lon - slon;
                    dl -= round(dl / (PI * 2.)) * PI * 2.;
                    let lx = dl * lat.cos();
                    let ly = lat - slat;
                    let a = spin * 5. * (-hypot(lx, ly) / 0.2).exp();
                    if a * spin > 0.02 {
                        let ca = a.cos();
                        let sa = a.sin();
                        cx += ((lx * ca - ly * sa - lx) / (PI * 2.)) * 14.;
                        cy -= ((lx * sa + ly * ca - ly) / PI) * 7.;
                    }
                }
                let q = fbm(cx * 0.5 + 3., cy * 1.2, 3, 7.);
                let n0 = fbm(cx + 1.6 * q, cy * 1.3 + 0.5 * q, 5, 14.);
                let nn = 0.4 * n0
                    + 0.6 * (1. - (2. * fbm(cx * 1.5 + 2.2 * q, cy * 1.6 + 9., 4, 21.) - 1.).abs());
                let belt = 0.05 * (-(lat / 0.12).powi(2)).exp()
                    - 0.07 * (-((al - 0.42) / 0.16).powi(2)).exp()
                    + 0.05 * (-((al - 0.95) / 0.25).powi(2)).exp();
                cloud[t] = (nn + belt) as f32;
            }
        }
        let quantile = |a: &[f32], p: f64| {
            let mut a = a.to_vec();
            a.sort_by(f32::total_cmp);
            a[(p * (a.len() - 1) as f64).floor() as usize] as f64
        };
        let shore = quantile(&elev, 0.7);
        let c0 = quantile(&cloud, 0.6);
        let c1 = quantile(&cloud, 0.86);
        for j in 0..TH {
            let v = (j as f64 + 0.5) / TH as f64;
            let al = ((0.5 - v) * PI).abs();
            for i in 0..TW {
                let u = i as f64 / TW as f64;
                let t = j * TW + i;
                let e = elev[t] as f64;
                let land = smooth(shore - 0.004, shore + 0.006, e);
                let ice = smooth(1.22, 1.32, al + 0.12 * fbm(u * 12., v * 6., 2, 12.));
                let grain = fbm(u * 36. + 2., v * 18., 3, 36.) - 0.5;
                let arid = clamp01(
                    (-((al - 0.4) / 0.22).powi(2)).exp()
                        * (0.1 + 1.2 * fbm(u * 10. + 4., v * 5., 3, 10.))
                        + (e - shore - 0.04) * 4.
                        + 0.8 * grain,
                );
                let relief = 1. + 1.2 * grain - 2.5 * 0.0_f64.max(e - shore - 0.08);
                let mut r = lerp(0.13, 0.4, arid) * relief;
                let mut g = lerp(0.25, 0.34, arid) * relief;
                let mut b = lerp(0.08, 0.19, arid) * relief;
                let shelf = smooth(shore - 0.06, shore, e);
                r = lerp(lerp(0.025, 0.07, shelf), r, land);
                g = lerp(lerp(0.11, 0.3, shelf), g, land);
                b = lerp(lerp(0.38, 0.62, shelf), b, land);
                tr[t] = lerp(r, 0.92, ice) as f32;
                tg[t] = lerp(g, 0.95, ice) as f32;
                tb[t] = lerp(b, 0.99, ice) as f32;
                sea[t] = ((1. - land) * (1. - ice)) as f32;
                cloud[t] = smooth(c0, c1, cloud[t] as f64) as f32;
            }
        }
        Maps {
            r: tr,
            g: tg,
            b: tb,
            sea,
            cloud,
        }
    })
}
fn sample(a: &[f32], lon: f64, vy: f64) -> f64 {
    let fx = ((((lon / (PI * 2.)) % 1.) + 1.) % 1.) * TW as f64;
    let x0 = fx.floor() as usize % TW;
    let ax = fx - fx.floor();
    let x1 = (x0 + 1) % TW;
    let y0 = clamp(vy.floor(), 0., (TH - 2) as f64) as usize;
    let ay = clamp01(vy - y0 as f64);
    let aa = a[y0 * TW + x0] as f64;
    let b = a[y0 * TW + x1] as f64;
    let c = a[y0 * TW + TW + x0] as f64;
    let d = a[y0 * TW + TW + x1] as f64;
    aa + (b - aa) * ax + (c - aa) * ay + (aa - b - c + d) * ax * ay
}
pub(super) fn earth_at(mut dx: f64, mut dy: f64, er: f64, spin: f64, drift: f64) -> [f32; 4] {
    let m = maps();
    dx *= ER / er;
    dy *= ER / er;
    let (mut cr, mut cg, mut cb, mut a) = (0., 0., 0., 0.0_f64);
    let d = hypot(dx, dy);
    let l = light();
    let hv = norm([l[0], l[1], l[2] + 1.]);
    if d >= ER - 1. && d < ER + 12. {
        let side = smooth(-0.3, 0.75, (dx * l[0] - dy * l[1]) / d);
        let mut g = (-0.0_f64.max(d - ER) / 1.2).exp() * 0.55 * side;
        if g < 0.05 {
            g = 0.;
        }
        cr += 0.25 * g;
        cg += 0.52 * g;
        cb += 1. * g;
        a = a.max(clamp01(g * 3.));
    }
    if d < ER {
        let nx = dx / ER;
        let ny = -dy / ER;
        let q2 = nx * nx + ny * ny;
        let nz = (1. - q2).sqrt();
        let (ct, st, cn, sn) = (0.4_f64.cos(), 0.4_f64.sin(), 0.22_f64.cos(), 0.22_f64.sin());
        let ax = nx * ct + ny * st;
        let ay0 = -nx * st + ny * ct;
        let ay = ay0 * cn - nz * sn;
        let az = ay0 * sn + nz * cn;
        let lat = clamp(ay, -1., 1.).asin();
        let lon = ax.atan2(az) + 3.5 + spin;
        let vy = (0.5 - lat / PI) * TH as f64 - 0.5;
        let ndl = nx * l[0] + ny * l[1] + nz * l[2];
        let day = smooth(-0.005, 0.06, ndl) * (0.45 + 0.8 * clamp01(ndl).sqrt());
        let dusk = (-((ndl - 0.02) / 0.035).powi(2)).exp();
        let cl = sample(&m.cloud, lon + drift, vy);
        let sh = sample(&m.cloud, lon + drift - 0.03, vy + 0.4);
        let sw = sample(&m.sea, lon, vy);
        let shade = 1. - 0.45 * sh * (1. - cl);
        let mut er = lerp(sample(&m.r, lon, vy) * shade, 0.95, cl);
        let mut eg = lerp(sample(&m.g, lon, vy) * shade, 0.97, cl);
        let mut eb = lerp(sample(&m.b, lon, vy) * shade, 1., cl);
        er *= day * (1. + 0.06 * dusk);
        eg *= day * (1. - 0.03 * dusk);
        eb *= day * (1. - 0.1 * dusk);
        let glint =
            0.0_f64.max(nx * hv[0] + ny * hv[1] + nz * hv[2]).powi(70) * 0.8 * sw * (1. - cl);
        let rim = (1. - nz).powf(2.2) * smooth(0., 0.3, ndl);
        er += glint * 0.95 + rim * 0.2;
        eg += glint * 0.92 + rim * 0.42;
        eb += glint * 0.85 + rim * 0.85;
        let edge = smooth(1., 0.95, q2.sqrt());
        cr = lerp(cr, er, edge);
        cg = lerp(cg, eg, edge);
        cb = lerp(cb, eb, edge);
        a = lerp(a, clamp01(er.max(eg).max(eb) * 12.), edge);
    }
    [cr as f32, cg as f32, cb as f32, a as f32]
}
#[derive(Default)]
pub(super) struct Layout {
    pub pw: usize,
    pub ph: usize,
    pub s: f64,
    pub sy: f64,
    pub col0: f64,
    pub row0: f64,
    k: f64,
    pub ex: f64,
    pub ey_low: f64,
    pub ey_high: f64,
    pub er: f64,
    pub ground: Vec<f32>,
    pub near: Vec<u8>,
    sub: usize,
    at: Vec<f32>,
    pub horizon: f64,
}
impl Layout {
    pub fn new(pw: usize, ph: usize, tall: bool) -> Self {
        let (s, col0, row0, ex, er, ey_low, ey_high);
        if pw == 0 || ph == 0 {
            (s, col0, row0, ex, er, ey_low, ey_high) = (1., 0., 0., 141., ER, 32., 32.);
        } else if !tall {
            s = 2.4;
            let sy = 2. * s;
            er = 3.4 * sy;
            col0 = 100. - 0.5 * pw as f64 * s;
            let k = 1.0_f64.min((2.8 * F) / (pw as f64 * s));
            let sky_at = |col: f64| skyline(((col - 100.) / F) * k);
            let mut sky: Vec<f64> = (0..24)
                .map(|i| sky_at(col0 + ((i as f64 + 0.5) / 24.) * pw as f64 * s))
                .collect();
            sky.sort_by(f64::total_cmp);
            row0 = sky[12] - 0.6 * ph as f64 * sy;
            let mut xx = col0 + 0.86 * pw as f64 * s;
            let mut low = f64::NEG_INFINITY;
            for i in 0..=12 {
                let col = col0 + (0.62 + ((0.86 - 0.62) * i as f64) / 12.) * pw as f64 * s;
                let under = sky_at(col - er * 0.6)
                    .min(sky_at(col))
                    .min(sky_at(col + er * 0.6));
                if under > low {
                    low = under;
                    xx = col;
                }
            }
            ex = xx;
            let top = row0 + er + (0.4 * sy).max(0.08 * er);
            ey_low = top.max(low + er * (2. * 0.62 - 1.));
            ey_high = top.max(low + er * (2. * -0.14 - 1.));
        } else {
            s = ((2. * ER) / (0.74 * pw as f64)).max((2. * ER) / (0.4 * 2. * ph as f64));
            col0 = 141. - 0.5 * pw as f64 * s;
            row0 = 32. - 0.42 * ph as f64 * 2. * s;
            ex = 141.;
            er = ER;
            ey_low = 32. + 0.5 * ER;
            ey_high = 32. - 0.35 * ER;
        }
        let sy = 2. * s;
        let sub = clamp(round(sy), 1., 3.) as usize;
        let rows = ph * sub;
        let at = (0..rows)
            .map(|i| (row0 + ((i as f64 + 0.5) / sub as f64) * sy) as f32)
            .collect();
        Self {
            pw,
            ph,
            s,
            sy,
            col0,
            row0,
            k: 1.0_f64.min((2.8 * F) / (pw.max(1) as f64 * s)),
            ex,
            ey_low,
            ey_high,
            er,
            ground: vec![-1.; pw * ph],
            near: vec![0; pw * ph],
            sub,
            at,
            horizon: row0 + ph as f64 * sy * 0.6,
        }
    }
    pub fn build(&mut self) {
        let rows = self.ph * self.sub;
        let (mut gx, mut gz, mut gy) = (vec![0f32; rows], vec![0f32; rows], vec![0f32; rows]);
        let cam = cam_y();
        for x in 0..self.pw {
            let col = self.col0 + (x as f64 + 0.5) * self.s;
            let ang = ((col - 100.) / F) * self.k;
            let sx = ang.sin();
            let cz = ang.cos();
            let mut top = rows;
            let mut d = 3.;
            let (mut prev_y, mut prev_d, mut prev_f) = (0., 0., 1e9);
            let mut skyline = f64::INFINITY;
            while d < ZMAX && top > 0 {
                let x = sx * d;
                let z = cz * d;
                let hy = height(x, z);
                let y = hy - (d * d) / (2. * RM);
                let yf = EYE - (F * (y - cam)) / d;
                while top > 0 && self.at[top - 1] as f64 >= yf {
                    top -= 1;
                    let i = top;
                    let a = if prev_f > yf + 1e-6 {
                        clamp01((prev_f - self.at[i] as f64) / (prev_f - yf))
                    } else {
                        1.
                    };
                    let dd = if prev_d != 0. { lerp(prev_d, d, a) } else { d };
                    gx[i] = (sx * dd) as f32;
                    gz[i] = (cz * dd) as f32;
                    gy[i] = (if prev_d != 0. {
                        lerp(prev_y, hy, a)
                    } else {
                        hy
                    }) as f32;
                }
                skyline = skyline.min(yf);
                prev_f = yf;
                prev_y = hy;
                prev_d = d;
                d += 0.03 + d * 0.012;
            }
            for y in 0..self.ph {
                let (mut sum, mut m) = (0., 0);
                for j in 0..self.sub {
                    let i = y * self.sub + j;
                    if i < top {
                        continue;
                    }
                    sum += ground_light(
                        gx[i] as f64,
                        gz[i] as f64,
                        gy[i] as f64,
                        cam,
                        (self.at[i] as f64 - skyline) / self.sy,
                    );
                    m += 1;
                }
                if m * 2 > self.sub {
                    self.ground[y * self.pw + x] = (sum / m as f64) as f32;
                }
            }
        }
        let mut sum = 0;
        for x in 0..self.pw {
            let mut y = 0;
            while y < self.ph && self.ground[y * self.pw + x] < 0. {
                y += 1;
            }
            sum += y;
        }
        if self.pw > 0 {
            self.horizon = self.row0 + (sum as f64 / self.pw as f64) * self.sy;
        }
        let air = (13. * self.er) / ER;
        for y in 0..self.ph {
            let row = self.row0 + (y as f64 + 0.5) * self.sy;
            if row <= self.ey_high - self.er - air - self.sy || row >= self.ey_low + self.er + air {
                continue;
            }
            for x in 0..self.pw {
                let col = self.col0 + (x as f64 + 0.5) * self.s;
                let i = y * self.pw + x;
                if self.ground[i] < 0. && (col - self.ex).abs() < self.er + air {
                    self.near[i] = 1;
                }
            }
        }
    }
}
