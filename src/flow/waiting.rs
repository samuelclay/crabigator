//! The shared breathing, sepia look while the assistant waits for the person.
use super::cells::{Cells, DEFAULT_COLOR};
use super::js::round;
use super::pixels::{clamp, clamp01, luma, mix, ramp_at};
pub const WAIT_EASE: f64 = 0.06;
pub const BREATH_FRAMES: f64 = 56.0;
pub const SEPIA_AMOUNT: f64 = 0.82;
const SEPIA: [u32; 6] = [0, 0x2c1b0c, 0x6a4420, 0xb07a36, 0xe2b46a, 0xfff1d4];
pub fn ease_wait(k: f64, waiting: bool) -> f64 {
    clamp01(k + clamp(if waiting { 1.0 - k } else { -k }, -WAIT_EASE, WAIT_EASE))
}
pub fn breath(t: f64) -> f64 {
    let x = ((t % BREATH_FRAMES) + BREATH_FRAMES) % BREATH_FRAMES / BREATH_FRAMES;
    ((x * 2.0 * std::f64::consts::PI - std::f64::consts::FRAC_PI_2)
        .sin()
        .exp()
        - 1.0 / std::f64::consts::E)
        / (std::f64::consts::E - 1.0 / std::f64::consts::E)
}
pub fn wait_lift(k: f64, t: f64) -> f64 {
    1.0 - 0.11 * clamp01(k) * (1.0 - breath(t))
}
pub fn wait_glow(k: f64, t: f64) -> f64 {
    clamp01(k) * breath(t)
}
pub fn wait_color(c: u32, k: f64, lift: f64, amount: f64, glow: f64) -> u32 {
    if c == DEFAULT_COLOR || k <= 0.0 {
        return c;
    }
    let s = if amount > 0.0 {
        mix(c, ramp_at(&SEPIA, luma(c) / 255.0), k * amount)
    } else {
        c
    };
    let r = round(((s >> 16) & 255) as f64 * lift + 23.0 * glow).min(255.0) as u32;
    let g = round(((s >> 8) & 255) as f64 * lift + 14.0 * glow).min(255.0) as u32;
    let b = round((s & 255) as f64 * lift + 4.0 * glow).min(255.0) as u32;
    (r << 16) | (g << 8) | b
}
pub fn wait_tone(out: &mut Cells, k: f64, t: f64, amount: f64, lamps: Option<&[i32]>) {
    if k <= 0.0 {
        return;
    }
    let lift = wait_lift(k, t);
    let glow = wait_glow(k, t);
    for i in 0..out.len() {
        let fg = out.foreground(i);
        let bg = out.background(i);
        if fg == DEFAULT_COLOR && bg == DEFAULT_COLOR {
            continue;
        }
        let lamp = lamps.and_then(|l| l.get(i)).copied().unwrap_or(-1);
        let color = |c| {
            if c as i32 == lamp {
                wait_color(c, k, lift, 0.0, 0.0)
            } else {
                wait_color(c, k, lift, amount, glow)
            }
        };
        out.set(i, out.code_point(i), color(fg), color(bg));
    }
}
