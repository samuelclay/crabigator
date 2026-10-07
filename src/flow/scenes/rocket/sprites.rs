//! The rockets' pixel sprites, each rocket's layout on the band and in a
//! tall pane, its launch site's colours, and the plume's colour ramps.

use std::sync::OnceLock;

use crate::flow::js::i32_of;
use crate::flow::pixels::mix;

/// A pixel sprite, top row first; -1 lets the sky through.
pub(super) struct Sprite {
    pub w: usize,
    pub h: usize,
    pub c: Vec<i32>,
}

impl Sprite {
    fn new(rows: &[&str], pal: &[(char, u32)]) -> Self {
        let h = rows.len();
        let w = rows[0].chars().count();
        let mut c = vec![-1; w * h];
        for (y, row) in rows.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                if let Some(&(_, p)) = pal.iter().find(|(k, _)| *k == ch) {
                    c[y * w + x] = p as i32;
                }
            }
        }
        Self { w, h, c }
    }

    /// Its width in pixels.
    pub fn wf(&self) -> f64 {
        self.w as f64
    }

    /// Its height in pixels.
    pub fn hf(&self) -> f64 {
        self.h as f64
    }
}

/// The first row holding `ch`: where a sprite's booster starts.
fn first_row(rows: &[&str], ch: char) -> f64 {
    rows.iter()
        .position(|r| r.contains(ch))
        .map_or(-1.0, |i| i as f64)
}

const FALCON_PAL: [(char, u32); 8] = [
    ('W', 0xf4f5f7), // white body, lit side
    ('w', 0xc3c8d0), // its shaded side
    ('K', 0x1d2025), // black interstage, octaweb, stowed legs
    ('G', 0x3a3e46), // grid fins
    ('L', 0x2c2f35), // deployed legs
    ('E', 0x70757d), // Merlin bells
    ('H', 0x2a2c31), // Dragon's heat shield
    ('P', 0x23315a), // the trunk's solar cells
];

const STARSHIP_PAL: [(char, u32); 10] = [
    ('k', 0x3c3f46), // the ship's black heat-shield side
    ('S', 0xd9dde2), // stainless, lit
    ('s', 0xa3a9b1), // stainless, shaded
    ('F', 0x23262b), // flaps
    ('D', 0x3f434a), // hot-staging ring
    ('d', 0x8d939b), // its vents
    ('B', 0xc7ccd2), // Super Heavy, lit
    ('b', 0x989fa8), // Super Heavy, shaded
    ('G', 0x34383f), // grid fins
    ('E', 0x4f535a), // engine bay
];

/// Falcon 9 in the band: one column of body, fins and legs half a column out.
const FALCON_BAND: [&str; 7] = [
    "..Ww..", "..Ww..", "..KK..", ".GWwG.", "..Ww..", "..Ww..", "..KK..",
];
const FALCON_BAND_LAND: [&str; 7] = [
    "..Ww..", "..Ww..", "..KK..", ".GWwG.", "..Ww..", "..Ww..", "L.KK.L",
];

/// Falcon 9 in the spine: Dragon (its nose cap, the capsule, the dark heat
/// shield, the trunk with its solar cells), the second stage, the black
/// interstage, grid fins, the long first stage, stowed legs and the octaweb
/// with its Merlins.
const FALCON_TALL_TOP: [&str; 10] = [
    "....Ww....",
    "...WWWw...",
    "...WWWw...",
    "...HHHH...",
    "...WPPw...",
    "...WWWw...",
    "...WWWw...",
    "...WWWw...",
    "...KKKK...",
    "...KKKK...",
];
const FALCON_TALL_MID: &str = "...WWWw...";
const FALCON_TALL_TAIL: [&str; 6] = [
    "...KWwK...",
    "...KWwK...",
    "...KWwK...",
    "...KWwK...",
    "...KKKK...",
    "....EE....",
];
const FALCON_TALL_LAND_TAIL: [&str; 6] = [
    "..LWWWwL..",
    "..LWWWwL..",
    ".L.WWWw.L.",
    ".L.WWWw.L.",
    "L..KKKK..L",
    "L...EE...L",
];

/// Falcon 9 in the spine, its grid fins as given and its tail as given.
fn falcon_tall(fins: &'static str, tail: &[&'static str]) -> Vec<&'static str> {
    let mut rows = FALCON_TALL_TOP.to_vec();
    rows.push(fins);
    rows.extend(std::iter::repeat_n(FALCON_TALL_MID, 12));
    rows.extend_from_slice(tail);
    rows
}

/// Starship in the band: the ship (black tiles on one side, steel on the
/// other, flaps fore and aft), the dark hot-staging ring, Super Heavy.
const STARSHIP_BAND: [&str; 7] = [
    "..kS..", "FkSSsF", ".kSSs.", "FkSSsF", "GBBBbG", ".BBBb.", ".bbbb.",
];

fn starship_tall() -> Vec<&'static str> {
    let mut rows = vec![
        "....kS....",
        "...kkSS...",
        "..kkkSSs..",
        ".FkkkSSsF.",
        ".FkkkSSsF.",
    ];
    rows.extend(std::iter::repeat_n("..kkkSSs..", 7));
    rows.extend_from_slice(&[
        ".FkkkSSsF.",
        "FFkkkSSsFF",
        "FFkkkSSsFF",
        "..kkkSSs..",
        "..DDDDDD..",
        "..DdDdDd..",
        "GGBBBBBbGG",
    ]);
    rows.extend(std::iter::repeat_n("..BBBBBb..", 17));
    rows.extend_from_slice(&["..bbbbbb..", "..EEEEEE.."]);
    rows
}

/// One rocket in one layout. All in pixels; rows of a sprite count from its top.
pub(super) struct Spec {
    pub fly: Sprite,
    /// The booster coming home: grid fins deployed, legs still stowed.
    pub fins: Sprite,
    /// Its look landing and landed: legs out too.
    pub land: Sprite,
    /// The first sprite row of the booster (the first stage): the rows above are the upper stage.
    pub stage: f64,
    /// Falcon's Dragon at the top of the upper stage: its rows (capsule and
    /// trunk), and the capsule's alone; 0 for none.
    pub dragon: f64,
    pub capsule: f64,
    /// The sprite column the body starts at, and its width.
    pub body_l: f64,
    pub body_w: f64,
    /// Where the arms grip it: pixels above its bottom.
    pub grip: f64,
    /// The launch mount it stands on: its height, so the rocket's resting altitude.
    pub mount: f64,
    pub tower_w: f64,
    pub tower_h: f64,
    /// The arms' height when they catch it.
    pub arm_catch: f64,
    pub arm_thick: f64,
    /// The open arms' stub, foreshortened as they swing out toward the viewer.
    pub arm_stub: f64,
    /// The full plume's length.
    pub plume: f64,
    /// Where vapour vents at rest: sprite column, row, and which way it blows.
    pub vents: &'static [(f64, f64, f64)],
    /// The row a service arm reaches across to at rest (the tall layout only).
    pub service: Option<f64>,
    /// Smoke puff size, as radii at birth and at death.
    pub puff: (f64, f64),
}

/// Falcon 9's layouts: the band's, then the tall pane's.
pub(super) fn falcon_specs() -> &'static [Spec; 2] {
    static SPECS: OnceLock<[Spec; 2]> = OnceLock::new();
    SPECS.get_or_init(|| {
        let tall = falcon_tall("..GWWWwG..", &FALCON_TALL_TAIL);
        [
            Spec {
                fly: Sprite::new(&FALCON_BAND, &FALCON_PAL),
                fins: Sprite::new(&FALCON_BAND, &FALCON_PAL),
                land: Sprite::new(&FALCON_BAND_LAND, &FALCON_PAL),
                stage: first_row(&FALCON_BAND, 'K'),
                dragon: 1.0,
                capsule: 1.0,
                body_l: 2.0,
                body_w: 2.0,
                grip: 1.0,
                mount: 0.0,
                tower_w: 4.0,
                tower_h: 8.0,
                arm_catch: 6.0,
                arm_thick: 1.0,
                arm_stub: 1.0,
                plume: 7.0,
                vents: &[(1.0, 1.0, -1.0), (4.0, 1.0, 1.0), (4.0, 5.0, 1.0)],
                service: None,
                puff: (0.6, 2.2),
            },
            Spec {
                fly: Sprite::new(&tall, &FALCON_PAL),
                fins: Sprite::new(&falcon_tall(".GGWWWwGG.", &FALCON_TALL_TAIL), &FALCON_PAL),
                land: Sprite::new(
                    &falcon_tall(".GGWWWwGG.", &FALCON_TALL_LAND_TAIL),
                    &FALCON_PAL,
                ),
                stage: first_row(&tall, 'K'),
                dragon: first_row(&tall, 'P') + 1.0,
                capsule: first_row(&tall, 'P'),
                body_l: 3.0,
                body_w: 4.0,
                grip: 16.0,
                mount: 0.0,
                tower_w: 4.0,
                tower_h: 38.0,
                arm_catch: 33.0,
                arm_thick: 1.0,
                arm_stub: 2.0,
                plume: 22.0,
                vents: &[
                    (2.0, 6.0, -1.0),
                    (7.0, 5.0, 1.0),
                    (2.0, 25.0, -1.0),
                    (7.0, 26.0, 1.0),
                ],
                service: Some(3.0),
                puff: (1.2, 4.0),
            },
        ]
    })
}

/// Starship's layouts: the band's, then the tall pane's.
pub(super) fn starship_specs() -> &'static [Spec; 2] {
    static SPECS: OnceLock<[Spec; 2]> = OnceLock::new();
    SPECS.get_or_init(|| {
        let tall = starship_tall();
        [
            Spec {
                fly: Sprite::new(&STARSHIP_BAND, &STARSHIP_PAL),
                fins: Sprite::new(&STARSHIP_BAND, &STARSHIP_PAL),
                land: Sprite::new(&STARSHIP_BAND, &STARSHIP_PAL),
                stage: first_row(&STARSHIP_BAND, 'G'),
                dragon: 0.0,
                capsule: 0.0,
                body_l: 1.0,
                body_w: 4.0,
                grip: 1.0,
                mount: 0.0,
                tower_w: 4.0,
                tower_h: 8.0,
                arm_catch: 6.0,
                arm_thick: 1.0,
                arm_stub: 1.0,
                plume: 7.0,
                vents: &[(0.0, 2.0, -1.0), (5.0, 2.0, 1.0), (5.0, 5.0, 1.0)],
                service: None,
                puff: (0.7, 2.4),
            },
            Spec {
                fly: Sprite::new(&tall, &STARSHIP_PAL),
                fins: Sprite::new(&tall, &STARSHIP_PAL),
                land: Sprite::new(&tall, &STARSHIP_PAL),
                stage: first_row(&tall, 'D'),
                dragon: 0.0,
                capsule: 0.0,
                body_l: 2.0,
                body_w: 6.0,
                grip: 18.0,
                mount: 4.0,
                tower_w: 6.0,
                tower_h: 50.0,
                arm_catch: 40.0,
                arm_thick: 2.0,
                arm_stub: 3.0,
                plume: 26.0,
                vents: &[
                    (1.0, 9.0, -1.0),
                    (8.0, 7.0, 1.0),
                    (1.0, 31.0, -1.0),
                    (8.0, 33.0, 1.0),
                ],
                service: Some(8.0),
                puff: (1.4, 4.5),
            },
        ]
    })
}

/// A plume's colours from cold (0) to white-hot (1).
pub(super) type Ramp = [u32; 5];

pub(super) const MERLIN: Ramp = [0x5a1e10, 0xd2461a, 0xff8f1f, 0xffd257, 0xfffbe8];
pub(super) const RAPTOR: Ramp = [0x4a1a2e, 0xd8482a, 0xff9a3a, 0xffdc8f, 0xf2f4ff];
pub(super) const BLUE: Ramp = [0x1a1650, 0x4b3ad6, 0x3f7dff, 0x9cc8ff, 0xf2f8ff];
pub(super) const SMOKE: Ramp = [0x2e2e2e, 0x595959, 0x7e7c78, 0xa89c88, 0xd8c8a8];

/// The ramp's colour at a heat (0..1).
pub(super) fn ramp_color(r: &Ramp, heat: f64) -> u32 {
    let h = if heat <= 0.0 {
        0.0
    } else if heat >= 1.0 {
        4.0
    } else {
        heat * 4.0
    };
    let i = i32_of(h).min(3) as usize;
    mix(r[i], r[i + 1], h - i as f64)
}

/// A rocket's site colours.
pub(super) struct Look {
    pub tower: u32,
    pub arm: u32,
    pub carriage: u32,
    pub ramp: Ramp,
    /// Mechazilla: X-braced, a lightning rod, a carriage the arms ride on.
    pub mechazilla: bool,
    /// The tower's arms catch it coming home; without them it lands on its legs.
    pub catches: bool,
}

pub(super) const FALCON_LOOK: Look = Look {
    tower: 0x5a6069,
    arm: 0x2a2e34,
    carriage: 0x7d848e,
    ramp: MERLIN,
    mechazilla: false,
    catches: false,
};

pub(super) const STARSHIP_LOOK: Look = Look {
    tower: 0x4e545d,
    arm: 0x24272c,
    carriage: 0x8a9099,
    ramp: RAPTOR,
    mechazilla: true,
    catches: true,
};
