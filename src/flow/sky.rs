//! The world the balloon and the rockets fly through (flow's `hooks/sky.ts`):
//! the level is a target altitude, eased toward, and the world scrolls past
//! the vehicle as it climbs. A sky painted per row by that row's altitude,
//! trees and houses on the ground, birds, the layered clouds, then stars and
//! Earth's blue rim. Every scenery cell is a pure function of its world
//! position, so the world is stable as it scrolls. Night falls and lifts
//! over a couple of seconds.
//!
//! flow's `SkyWorld` is a base class; here it is `SkyWorld` (the state) and
//! the `SkyScene` trait, whose default methods are the base class's and
//! which a scene overrides where flow's subclass does.

use super::cells::{Cells, DEFAULT_COLOR};
use super::clouds::{layered, snap};
use super::js::{i32_of, round};
use super::night::{MOON, MOON_ACROSS, MOON_ROW, NIGHT_HORIZON, NIGHT_ZENITH, STAR, STAR_DIM};
use super::pixels::{g, hash, lower_block, mix};
use super::scene::Dials;

/// How much taller the atmosphere is than the cloud painter's own scale.
pub const SCALE: f64 = 2.0;
/// Target altitude (world rows) for each level; 0 is off.
const ALTITUDE: [f64; 11] = [
    0.0, 0.0, 6.0, 14.0, 24.0, 36.0, 50.0, 66.0, 84.0, 104.0, 128.0,
];
pub const CLOUDS_FROM: f64 = 6.0 * SCALE;
pub const CLOUDS_TO: f64 = 44.0 * SCALE;
pub const STARS_FROM: f64 = 40.0 * SCALE;
pub const EARTH_FROM: f64 = 56.0 * SCALE;
/// How fast altitude eases toward its target, per frame.
const CLIMB: f64 = 0.03;
/// Columns per frame the whole cloud band drifts.
pub const CLOUD_DRIFT: f64 = 0.1;

const SKY: [(f64, u32); 7] = [
    (0.0 * SCALE, 0x9fd3f2),
    (12.0 * SCALE, 0x6db8ec),
    (28.0 * SCALE, 0x3f86d4),
    (42.0 * SCALE, 0x24508f),
    (52.0 * SCALE, 0x121f45),
    (57.0 * SCALE, 0x070b1c),
    (61.0 * SCALE, 0x000000),
];
/// Space: black, the darkest thing in the sky.
const SPACE: u32 = 0x000000;
const SOIL: u32 = 0x5a3d24;
const NIGHT_SKY: [(f64, u32); 3] = [
    (0.0, NIGHT_HORIZON),
    (30.0 * SCALE, NIGHT_ZENITH),
    (61.0 * SCALE, 0x000000),
];
const NIGHT_SOIL: u32 = 0x22170e;
/// Moonlight: what clouds and lit things lean toward at night.
const MOONLIGHT: u32 = 0x9fb4d6;

pub mod colors {
    pub const GRASS: u32 = 0x4f9a3d;
    pub const GRASS_NIGHT: u32 = 0x1d3a1a;
    pub const TREE: u32 = 0x2f6b2a;
    pub const TREE_NIGHT: u32 = 0x15291a;
    pub const HOUSE: u32 = 0xc9a46a;
    pub const WINDOW: u32 = 0xffd27a;
    pub const BIRD: u32 = 0x5a5a62;
    pub const EARTH: u32 = 0x4a90d9;
}

fn ramp(stops: &[(f64, u32)], y: f64) -> u32 {
    for i in 1..stops.len() {
        let (y1, c1) = stops[i];
        if y <= y1 {
            let (y0, c0) = stops[i - 1];
            return mix(c0, c1, (y - y0) / (y1 - y0));
        }
    }
    SPACE
}

/// The background for a world row by day.
pub fn sky_color(y: f64) -> u32 {
    if y < 0.0 {
        SOIL
    } else {
        ramp(&SKY, y)
    }
}

/// The background for a world row at night.
fn night_sky_color(y: f64) -> u32 {
    if y < 0.0 {
        NIGHT_SOIL
    } else {
        ramp(&NIGHT_SKY, y)
    }
}

/// A cloud colour by moonlight.
fn moonlit(c: u32, sky: u32) -> u32 {
    mix(mix(c, sky, 0.55), MOONLIGHT, 0.12)
}

/// One cell of scenery: a glyph, its colour, and its background (`None`: the sky's).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SceneryCell {
    pub glyph: u32,
    pub fg: u32,
    pub bg: Option<u32>,
}

/// The sky world's state: what flow's `SkyWorld` holds.
pub struct SkyWorld {
    pub dials: Dials,
    /// How far night has fallen, 0..1, eased toward `night` (-1 before the first step).
    pub k_night: f64,
    pub columns: usize,
    pub rows: usize,
    pub out: Cells,
    /// This frame's background per grid row.
    pub row_bg: Vec<u32>,
    pub alt: f64,
    pub t: f64,
}

impl SkyWorld {
    pub fn new(seed: f64) -> Self {
        Self {
            dials: Dials::default(),
            k_night: -1.0,
            columns: 0,
            rows: 0,
            out: Cells::new(0, 0),
            row_bg: Vec::new(),
            alt: 0.0,
            t: seed % 10_000.0,
        }
    }

    pub fn ensure(&mut self, columns: usize, rows: usize) {
        if columns == self.columns && rows == self.rows {
            return;
        }
        self.columns = columns;
        self.rows = rows;
        self.out = Cells::new(columns, rows);
    }

    /// The altitude this level aims for.
    pub fn target(&self) -> f64 {
        ALTITUDE[self.dials.strength.clamp(0.0, 10.0) as usize]
    }

    /// The first half of a step: the clock, and night falling or lifting.
    pub fn tick(&mut self) {
        self.t += 1.0;
        let k = if self.dials.night { 1.0 } else { 0.0 };
        if self.k_night < 0.0 {
            self.k_night = k;
        }
        self.k_night += (k - self.k_night) * 0.05;
        if (k - self.k_night).abs() < 0.01 {
            self.k_night = k;
        }
    }

    /// How dark it is this frame, 0 day .. 1 night.
    pub fn dark(&self) -> f64 {
        if self.k_night < 0.0 {
            if self.dials.night {
                1.0
            } else {
                0.0
            }
        } else {
            self.k_night
        }
    }

    /// Move the altitude one frame toward its target.
    pub fn advance(&mut self) {
        let target = self.target();
        self.alt += (target - self.alt) * CLIMB;
        if (target - self.alt).abs() < 0.01 {
            self.alt = target;
        }
    }

    /// Wind for single-cell things (birds, stars): higher air moves faster.
    pub fn wind(&self, y: f64) -> f64 {
        if y <= 0.0 {
            0.0
        } else {
            ((self.t * (0.04 + y * 0.002)) % 100_000.0).floor()
        }
    }

    /// The background for a world row, by day, by night, or as night falls.
    pub fn sky_at(&self, y: f64) -> u32 {
        let k = self.dark();
        if k <= 0.0 {
            sky_color(y)
        } else if k >= 1.0 {
            night_sky_color(y)
        } else {
            mix(sky_color(y), night_sky_color(y), k)
        }
    }

    /// A cloud colour as lit this frame.
    pub fn cloud_light(&self, c: u32, sky: u32) -> u32 {
        let k = self.dark();
        if k <= 0.0 {
            return c;
        }
        let m = moonlit(c, sky);
        if k >= 1.0 {
            m
        } else {
            mix(c, m, k)
        }
    }

    /// Trees and houses on the ground row (lit at night): flow's default `groundFeature`.
    pub fn base_ground_feature(&self, x: f64) -> Option<SceneryCell> {
        let h = hash(x, 0.0, 1.0);
        if h < 0.06 {
            return Some(SceneryCell {
                glyph: g('♣'),
                fg: mix(colors::TREE, colors::TREE_NIGHT, self.dark()),
                bg: None,
            });
        }
        if h < 0.08 {
            return Some(SceneryCell {
                glyph: g('⌂'),
                fg: mix(colors::HOUSE, colors::WINDOW, self.dark()),
                bg: None,
            });
        }
        None
    }

    /// The moon, high in the night sky, where the sky is open.
    pub fn draw_moon(&mut self) {
        let k = self.dark();
        if k <= 0.02 || self.rows < 3 {
            return;
        }
        let x = (self.columns as f64 * MOON_ACROSS).floor() as usize;
        let r = MOON_ROW.min(self.rows as f64 - 3.0) as usize;
        let i = r * self.columns + x;
        if x < self.columns && self.out.code_point(i) == 0x20 {
            let bg = self.out.background(i);
            let fg = if k >= 1.0 { MOON } else { mix(bg, MOON, k) };
            self.out.set(i, g('●'), fg, bg);
        }
    }
}

/// flow's `SkyWorld` methods a scene may override; the defaults are flow's.
pub trait SkyScene {
    fn world(&self) -> &SkyWorld;
    fn world_mut(&mut self) -> &mut SkyWorld;
    /// The vehicle's height in rows on this grid.
    fn vehicle_height(&self) -> f64;
    /// Draw the vehicle (and anything around it) over the world, its top at `top`.
    fn draw_vehicle(&mut self, out: &mut Cells, top: f64);

    /// Move the altitude one frame toward its target.
    fn advance(&mut self) {
        self.world_mut().advance();
    }

    /// The row the vehicle's top sits on at rest: one row is left under it for the ground.
    fn rest_top(&self) -> f64 {
        (self.world().rows as f64 - self.vehicle_height() - 1.0).max(0.0)
    }

    /// How far the world has scrolled down: the altitude the on-screen climb can't show.
    fn scroll(&self) -> f64 {
        let rest = self.rest_top();
        let ceiling = rest.min((self.world().rows as f64 * 0.25).floor().max(1.0));
        (round(self.world().alt) - (rest - ceiling)).max(0.0)
    }

    /// The row the vehicle's top is drawn on.
    fn vehicle_top(&self) -> f64 {
        self.rest_top() - (round(self.world().alt) - self.scroll())
    }

    /// What stands on the ground row at world column x.
    fn ground_feature(&self, x: f64) -> Option<SceneryCell> {
        self.world().base_ground_feature(x)
    }

    /// The scenery at world (x, y), or `None` for open sky.
    fn scenery(&self, x: f64, y: f64, sky: u32) -> Option<SceneryCell> {
        base_scenery(self, x, y, sky)
    }

    /// One frame: the clock, night, then the altitude.
    fn sky_step(&mut self) {
        self.world_mut().tick();
        self.advance();
    }

    /// The frame: sky, scenery, Earth's rim, the moon, then the vehicle.
    fn sky_grid(&mut self) -> &Cells {
        let (w, h) = (self.world().columns, self.world().rows);
        if self.world().dials.strength <= 0.0 {
            let out = &mut self.world_mut().out;
            for i in 0..w * h {
                out.blank(i);
            }
            return &self.world().out;
        }
        let scroll = self.scroll();
        let mut out = std::mem::take(&mut self.world_mut().out);
        let mut row_bg = Vec::with_capacity(h);
        for r in 0..h {
            let y = scroll - 1.0 + (h - 1 - r) as f64;
            let bg = self.world().sky_at(y);
            row_bg.push(bg);
            for x in 0..w {
                let i = r * w + x;
                match self.scenery(x as f64, y, bg) {
                    Some(s) => out.set(i, s.glyph, s.fg, s.bg.unwrap_or(bg)),
                    None => out.set(i, 0x20, DEFAULT_COLOR, bg),
                }
            }
        }
        if self.world().alt >= EARTH_FROM {
            let wf = w as f64;
            for x in 0..w {
                let edge = (x as f64 + 0.5 - wf / 2.0) / (wf / 2.0);
                let eighths = (1.0 + 3.0 * (1.0 - edge * edge) + hash(x as f64, 0.0, 74.0)).floor();
                out.set(
                    (h - 1) * w + x,
                    lower_block(eighths.clamp(1.0, 4.0)),
                    colors::EARTH,
                    row_bg[h - 1],
                );
            }
        }
        self.world_mut().row_bg = row_bg;
        self.world_mut().out = out;
        self.world_mut().draw_moon();
        let top = self.vehicle_top();
        let mut out = std::mem::take(&mut self.world_mut().out);
        self.draw_vehicle(&mut out, top);
        self.world_mut().out = out;
        &self.world().out
    }
}

/// flow's default `scenery`: grass, the ground's features, drifting clouds,
/// birds on the wind, and stars high up (or everywhere at night).
pub fn base_scenery<S: SkyScene + ?Sized>(s: &S, x: f64, y: f64, sky: u32) -> Option<SceneryCell> {
    let world = s.world();
    if y == -1.0 {
        return Some(SceneryCell {
            glyph: g('▀'),
            fg: mix(colors::GRASS, colors::GRASS_NIGHT, world.dark()),
            bg: None,
        });
    }
    if y == 0.0 {
        return s.ground_feature(x);
    }
    if y < -1.0 {
        return None;
    }
    let drift = ((world.t * CLOUD_DRIFT) % 100_000.0).floor();
    let cloud = if (CLOUDS_FROM..=CLOUDS_TO).contains(&y) {
        layered((x + drift) / SCALE, y / SCALE, sky).and_then(|cloud| {
            let fg = snap(world.cloud_light(cloud.fg, sky), sky);
            let bg = snap(world.cloud_light(cloud.bg, sky), sky);
            (fg != sky || bg != sky).then_some(SceneryCell {
                glyph: cloud.glyph,
                fg,
                bg: Some(bg),
            })
        })
    } else {
        None
    };
    let x = x + world.wind(y);
    if (2.0..=14.0 * SCALE).contains(&y) && hash(x, y, 2.0) < 0.005 {
        let glyph = if (i32_of(world.t) >> 3) % 2 != 0 {
            'v'
        } else {
            '~'
        };
        return Some(SceneryCell {
            glyph: g(glyph),
            fg: colors::BIRD,
            bg: cloud.map(|c| c.bg.unwrap_or(c.fg)),
        });
    }
    if cloud.is_some() {
        return cloud;
    }
    let dark = world.dark();
    if y >= STARS_FROM || (dark > 0.0 && y >= 2.0) {
        let p = (0.02 * dark).max(((y - STARS_FROM) * (0.004 / SCALE)).min(0.1));
        let h = hash(x, y, 7.0);
        if h < p {
            let twinkle = hash(x, y + f64::from(i32_of(world.t) >> 2), 8.0) < 0.15;
            return Some(SceneryCell {
                glyph: g(if h < p * 0.2 { '*' } else { '·' }),
                fg: if twinkle { STAR_DIM } else { STAR },
                bg: None,
            });
        }
    }
    None
}
