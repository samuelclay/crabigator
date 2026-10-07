//! JavaScript's number rules, where flow's TypeScript relies on them, so the
//! port draws the same frames from the same seed.

/// `x | 0`: truncate toward zero, wrapping modulo 2^32 (NaN and ±∞ give 0).
#[inline]
pub fn i32_of(x: f64) -> i32 {
    if !x.is_finite() {
        return 0;
    }
    let t = x.trunc();
    if t.abs() < 2_147_483_648.0 {
        t as i32
    } else {
        (t.rem_euclid(4_294_967_296.0) as u64) as u32 as i32
    }
}

/// `x >>> 0`: like `i32_of`, read as unsigned.
#[inline]
pub fn u32_of(x: f64) -> u32 {
    i32_of(x) as u32
}

/// `Math.imul(a, b)`.
#[inline]
pub fn imul(a: i32, b: i32) -> i32 {
    a.wrapping_mul(b)
}

/// `Math.round(x)`: halves go up (toward +∞), unlike Rust's `round`.
#[inline]
pub fn round(x: f64) -> f64 {
    let f = x.floor();
    if x - f >= 0.5 {
        f + 1.0
    } else {
        f
    }
}

/// `Math.fround(x)`: what a `Float32Array` stores.
#[inline]
pub fn fround(x: f64) -> f64 {
    x as f32 as f64
}

/// `Math.min(a, b)` for numbers that are never NaN.
#[inline]
pub fn min(a: f64, b: f64) -> f64 {
    if a < b {
        a
    } else {
        b
    }
}

/// `Math.max(a, b)` for numbers that are never NaN.
#[inline]
pub fn max(a: f64, b: f64) -> f64 {
    if a > b {
        a
    } else {
        b
    }
}

/// `Math.hypot(x, y)` as V8 computes it (scaled by the larger, a
/// compensated sum of squares, then the root), which can round differently
/// from libm's `hypot`.
pub fn hypot(x: f64, y: f64) -> f64 {
    if x.is_infinite() || y.is_infinite() {
        return f64::INFINITY;
    }
    if x.is_nan() || y.is_nan() {
        return f64::NAN;
    }
    let (ax, ay) = (x.abs(), y.abs());
    let max = if ay > ax { ay } else { ax };
    if max == 0.0 {
        return 0.0;
    }
    let (mut sum, mut compensation) = (0.0_f64, 0.0_f64);
    for v in [ax, ay] {
        let n = v / max;
        let summand = n * n - compensation;
        let preliminary = sum + summand;
        compensation = (preliminary - sum) - summand;
        sum = preliminary;
    }
    sum.sqrt() * max
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounding_and_wrapping_follow_javascript() {
        assert_eq!(round(2.5), 3.0);
        assert_eq!(round(-2.5), -2.0);
        assert_eq!(round(-0.4), 0.0);
        assert_eq!(round(1.49), 1.0);
        assert_eq!(round(0.499_999_999_999_999_94), 0.0);
        assert_eq!(i32_of(3.9), 3);
        assert_eq!(i32_of(-3.9), -3);
        assert_eq!(i32_of(4_294_967_296.0 + 5.0), 5);
        assert_eq!(i32_of(2_147_483_648.0), -2_147_483_648);
        assert_eq!(u32_of(-1.0), 4_294_967_295);
        assert_eq!(imul(0x7fff_ffff, 2), -2);
    }
}
