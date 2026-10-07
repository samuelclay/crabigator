//! One night for every scene that has one (flow's `hooks/night.ts`).

/// The night sky just above the horizon: near-black with a breath of blue.
pub const NIGHT_HORIZON: u32 = 0x070b16;
/// The night sky overhead, and by the edge of space.
pub const NIGHT_ZENITH: u32 = 0x020308;
/// A star, and a fainter (or twinkling) one.
pub const STAR: u32 = 0xeef0f4;
pub const STAR_DIM: u32 = 0x7d8696;
/// The moon.
pub const MOON: u32 = 0xf4f0da;
/// The moon hangs this far across the grid, centered on this row.
pub const MOON_ACROSS: f64 = 0.82;
pub const MOON_ROW: f64 = 1.0;

/// The moon's center on a quadrant pixel layer for a grid `columns` × `rows`.
pub fn moon_pixel(columns: f64, rows: f64) -> (f64, f64) {
    (
        columns * 2.0 * MOON_ACROSS,
        2.0 * MOON_ROW.min((rows - 3.0).max(0.0)) + 1.0,
    )
}

/// The moon's radius on a quadrant pixel layer, in pixel widths.
pub fn moon_radius(tall: bool) -> f64 {
    if tall {
        2.6
    } else {
        1.6
    }
}

/// How much of a moon of radius r centered at (mx, my) covers the pixel at (x, y), 0..1.
pub fn moon_cover(x: f64, y: f64, mx: f64, my: f64, r: f64) -> f64 {
    let d = super::js::hypot(x - mx, (y - my) * 2.0);
    if d >= r + 0.5 {
        0.0
    } else if d <= r - 0.5 {
        1.0
    } else {
        r + 0.5 - d
    }
}
