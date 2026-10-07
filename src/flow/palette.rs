//! A colour for each session: every colour of a finished frame turned around
//! the hue wheel in OKLCH, lightness and chroma kept, so the scene keeps its
//! shape and contrast (flow's `hooks/palette.ts`). The terminal's own colour
//! and greys stay as they are. A scene says which hue it mostly is and how
//! far it may turn; a sky turns only a little, and only near its own hue.

use std::collections::HashMap;

use super::cells::{Cells, DEFAULT_COLOR};

/// Colours closer to grey than this (OKLCH chroma) keep their colour.
const GREY: f64 = 0.02;
/// Memoized colours a palette keeps before starting afresh.
const MEMO: usize = 4096;
/// Past `spread`, a colour's turn fades out over this many degrees.
const SPREAD_FADE: f64 = 30.0;

/// How a scene takes a palette: its main hue (OKLCH degrees), how far around
/// the wheel it may turn, and how near its main hue a colour must be to turn
/// at all (`None`: every colour turns).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SceneHue {
    pub key: f64,
    pub range: f64,
    pub spread: Option<f64>,
}

/// An angle folded into -180..180.
pub fn wrap(deg: f64) -> f64 {
    (deg + 180.0).rem_euclid(360.0) - 180.0
}

fn to_linear(c: u32) -> f64 {
    let v = f64::from(c) / 255.0;
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}

fn to_byte(v: f64) -> u32 {
    let s = if v <= 0.003_130_8 {
        12.92 * v
    } else {
        1.055 * v.powf(1.0 / 2.4) - 0.055
    };
    super::js::round(s.clamp(0.0, 1.0) * 255.0) as u32
}

/// 0xRRGGBB to OKLab.
pub fn oklab(color: u32) -> [f64; 3] {
    let r = to_linear((color >> 16) & 255);
    let g = to_linear((color >> 8) & 255);
    let b = to_linear(color & 255);
    let l = (0.412_221_470_8 * r + 0.536_332_536_3 * g + 0.051_445_992_9 * b).cbrt();
    let m = (0.211_903_498_2 * r + 0.680_699_545_1 * g + 0.107_396_956_6 * b).cbrt();
    let s = (0.088_302_461_9 * r + 0.281_718_837_6 * g + 0.629_978_700_5 * b).cbrt();
    [
        0.210_454_255_3 * l + 0.793_617_785 * m - 0.004_072_046_8 * s,
        1.977_998_495_1 * l - 2.428_592_205 * m + 0.450_593_709_9 * s,
        0.025_904_037_1 * l + 0.782_771_766_2 * m - 0.808_675_766 * s,
    ]
}

fn linear_of(lightness: f64, a: f64, b: f64) -> [f64; 3] {
    let l = (lightness + 0.396_337_777_4 * a + 0.215_803_757_3 * b).powi(3);
    let m = (lightness - 0.105_561_345_8 * a - 0.063_854_172_8 * b).powi(3);
    let s = (lightness - 0.089_484_177_5 * a - 1.291_485_548 * b).powi(3);
    [
        4.076_741_662_1 * l - 3.307_711_591_3 * m + 0.230_969_929_2 * s,
        -1.268_438_004_6 * l + 2.609_757_401_1 * m - 0.341_319_396_5 * s,
        -0.004_196_086_3 * l - 0.703_418_614_7 * m + 1.707_614_701 * s,
    ]
}

fn in_gamut(rgb: [f64; 3]) -> bool {
    rgb.iter().all(|&c| (-1e-4..=1.0001).contains(&c))
}

/// OKLCH (hue in degrees) to 0xRRGGBB, chroma pulled in until the colour exists in sRGB.
pub fn from_oklch(lightness: f64, chroma: f64, hue: f64) -> u32 {
    let rad = hue.to_radians();
    let (sa, ca) = rad.sin_cos();
    let mut rgb = linear_of(lightness, chroma * ca, chroma * sa);
    if !in_gamut(rgb) {
        let (mut lo, mut hi) = (0.0, chroma);
        for _ in 0..12 {
            let mid = (lo + hi) / 2.0;
            if in_gamut(linear_of(lightness, mid * ca, mid * sa)) {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        rgb = linear_of(lightness, lo * ca, lo * sa);
    }
    (to_byte(rgb[0]) << 16) | (to_byte(rgb[1]) << 8) | to_byte(rgb[2])
}

/// A colour's OKLCH chroma and hue (degrees).
pub fn chroma_hue(color: u32) -> (f64, f64) {
    let [_, a, b] = oklab(color);
    (a.hypot(b), b.atan2(a).to_degrees().rem_euclid(360.0))
}

/// The hue of the most colourful of these, or `None` when they're all near grey.
pub fn hue_of(colors: &[u32]) -> Option<f64> {
    colors
        .iter()
        .map(|&c| chroma_hue(c))
        .fold(None, |best: Option<(f64, f64)>, ch| match best {
            Some(b) if b.0 >= ch.0 => Some(b),
            _ => Some(ch),
        })
        .filter(|(chroma, _)| *chroma >= GREY * 2.0)
        .map(|(_, hue)| hue)
}

/// A turn of the hue wheel for every colour of a frame.
#[derive(Clone, Debug)]
pub struct Palette {
    /// Degrees; 0 is the scene's own colours.
    pub shift: f64,
    /// Only colours near this hue (key, spread) turn.
    near: Option<(f64, f64)>,
    memo: HashMap<u32, u32>,
}

impl Palette {
    pub fn natural() -> Self {
        Self {
            shift: 0.0,
            near: None,
            memo: HashMap::new(),
        }
    }

    /// The palette that turns a scene toward a hue: a scene that turns any way
    /// lands on it; one that turns less goes the same way in proportion.
    pub fn toward(scene: SceneHue, target: Option<f64>) -> Self {
        let Some(target) = target else {
            return Self::natural();
        };
        let shift = wrap(target - scene.key) * scene.range / 180.0;
        if shift.abs() < 1.0 {
            return Self::natural();
        }
        Self {
            shift,
            near: scene.spread.map(|spread| (scene.key, spread)),
            memo: HashMap::new(),
        }
    }

    pub fn is_natural(&self) -> bool {
        self.shift == 0.0
    }

    /// One colour turned (the terminal's own colour and greys as they are).
    pub fn color(&mut self, c: u32) -> u32 {
        if c == DEFAULT_COLOR || self.shift == 0.0 {
            return c;
        }
        if let Some(&hit) = self.memo.get(&c) {
            return hit;
        }
        let [lightness, a, b] = oklab(c);
        let chroma = a.hypot(b);
        let mut out = c;
        if chroma >= GREY {
            let hue = b.atan2(a).to_degrees();
            let mut turn = self.shift;
            if let Some((key, spread)) = self.near {
                let off = wrap(hue - key).abs() - spread;
                turn *= (1.0 - off / SPREAD_FADE).clamp(0.0, 1.0);
            }
            if turn.abs() >= 0.5 {
                out = from_oklch(lightness, chroma, hue + turn);
            }
        }
        if self.memo.len() >= MEMO {
            self.memo.clear();
        }
        self.memo.insert(c, out);
        out
    }

    /// `src` with every colour turned, written into `out` (the same size).
    pub fn apply(&mut self, src: &Cells, out: &mut Cells) {
        if out.columns != src.columns || out.rows != src.rows {
            *out = Cells::new(src.columns, src.rows);
        }
        for (from, to) in src.words.chunks_exact(3).zip(out.words.chunks_exact_mut(3)) {
            to[0] = from[0];
            to[1] = self.color(from[1]);
            to[2] = self.color(from[2]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn oklab_round_trips_and_a_turn_lands_on_the_hue() {
        for c in [0xff6a00, 0x3355ff, 0x22aa55, 0x9b30ff, 0xd04020] {
            let [l, a, b] = oklab(c);
            let back = from_oklch(l, a.hypot(b), b.atan2(a).to_degrees());
            for shift in [0, 8, 16] {
                let (x, y) = ((back >> shift) & 255, (c >> shift) & 255);
                assert!(x.abs_diff(y) <= 1, "{c:06x} → {back:06x}");
            }
        }
        let mut p = Palette {
            shift: 120.0,
            near: None,
            memo: HashMap::new(),
        };
        let turned = p.color(0xd04020);
        assert!((oklab(turned)[0] - oklab(0xd04020)[0]).abs() < 0.02);
        let dh = wrap(chroma_hue(turned).1 - chroma_hue(0xd04020).1);
        assert!((dh - 120.0).abs() < 8.0, "{dh}");
    }

    #[test]
    fn greys_and_the_terminal_colour_stay() {
        let mut p = Palette::toward(
            SceneHue {
                key: 43.0,
                range: 180.0,
                spread: None,
            },
            Some(305.0),
        );
        assert!(!p.is_natural());
        for c in [DEFAULT_COLOR, 0x000000, 0x808080, 0xffffff] {
            assert_eq!(p.color(c), c);
        }
    }

    #[test]
    fn a_sky_turns_its_own_hue_and_leaves_the_balloon() {
        let sky = SceneHue {
            key: 251.0,
            range: 45.0,
            spread: Some(60.0),
        };
        let mut p = Palette::toward(sky, Some(25.0));
        assert!(p.shift.abs() <= 45.0);
        assert_ne!(p.color(0x6db8ec), 0x6db8ec); // day blue turns
        assert_eq!(p.color(0xe8402a), 0xe8402a); // a red stripe doesn't
    }

    #[test]
    fn the_hue_of_a_mark_colour() {
        let violet = hue_of(&[0x3b1266]).unwrap();
        assert!((wrap(violet - 305.0)).abs() < 20.0, "{violet}");
        assert_eq!(hue_of(&[0x1a1a1a, 0x202020]), None);
        assert_eq!(hue_of(&[]), None);
    }
}
