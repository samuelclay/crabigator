//! Two launch sites in the sky world (flow's `hooks/rocket.ts`): a Falcon 9
//! and a Starship, each beside a lattice launch tower (Starship's with two
//! catch arms). The level is the rocket's target altitude. At 1 it stands on
//! the pad, fuelled, venting white vapour. When the level rises it ignites,
//! holds a beat while steam billows off the pad, then lifts off: slowly, then
//! faster, the plume (white-hot core, orange, fading red) trailing a contrail
//! that falls away below as the clouds go by, until at 10 it hangs among the
//! stars over Earth's rim. High in the climb (about level 7) the stack
//! separates: the spent booster falls away and the upper stage flies on, on
//! one narrower engine. Only the booster comes home: when the level falls
//! back the upper stage goes on to orbit and the booster comes down, grid
//! fins out, coasting, then lights a landing burn. Once the falling booster
//! has left the frame the screen splits (side by side in the band, top and
//! bottom in a tall pane): the booster's side follows it back down through
//! the sky a layer a second, then slides away; the other stays with the
//! upper stage, whatever the level. Falcon's booster puts its legs out only
//! for its landing burn, on a landing zone out of sight of the pad.
//!
//! Falcon carries Dragon: in orbit it parts from the second stage (which
//! drifts off, to burn up). Brought home, Dragon drops its trunk, comes in
//! glowing, opens two drogues then four striped mains and splashes down at
//! sea; a moment later the camera slides back along the coast to the pad,
//! reset for the next flight. Starship's booster is caught by the tower's
//! arms in mid-air and lowered onto the mount. Brought home, the Ship comes
//! in belly-first, tiles glowing, flips upright for its landing burn and
//! splashes down, is carried in by barge and transporter to beside the
//! waiting booster, and the arms reach over, lift it, swing it across and
//! lower it on. Then the stack goes back to venting. A mission only climbs:
//! a dip in the work holds the level, and only back at 1 does it come home,
//! a level a second, all the way.
//!
//! Everything is drawn into a pixel layer at quadrant resolution (two pixels
//! per cell across, two down) and composited over the sky: each cell keeps
//! the two colours that best fit its four pixels, so the rocket, the tower's
//! lattice and the plume are twice as fine as the grid, and soft things
//! (vapour, steam, the plume's tail) blend into whatever sky is behind them.
//!
//! Dials: running subagents add vapour and more tower lights; a failed
//! command makes the engines sputter a grey, smoky plume (on the pad the
//! vents fume grey and the tower lights burn low; in orbit it coughs smoke
//! back along its track); a nearly-full context burns a blue methane-ish
//! plume, turns the tower's lights blue all the way up, and in orbit fires
//! blue-white thruster puffs off the nose. Either tint keeps a thin burn
//! going in orbit, where the engines would otherwise coast dark.
//!
//! flow's `LaunchSite` subclasses `SkyWorld`; here it is one struct over a
//! `SkyWorld`, told which rocket it is, implementing `SkyScene` for the
//! sky's scenery. flow's sound (`sounds`, `ambience`) is left out.

mod draw;
mod particles;
mod sprites;

use std::f64::consts::PI;

use crate::flow::cells::{Cells, Rng, DEFAULT_COLOR};
use crate::flow::clouds::{layered, snap};
use crate::flow::js::{i32_of, round};
use crate::flow::night::{STAR, STAR_DIM};
use crate::flow::palette::SceneHue;
use crate::flow::pixels::{clamp, g, hash, mix, QuadFit};
use crate::flow::scene::{Dials, Scene, SceneDef, Tint};
use crate::flow::sky::{base_scenery, SceneryCell, SkyScene, SkyWorld};

use particles::Particles;
use sprites::{falcon_specs, starship_specs, Look, Spec, Sprite, FALCON_LOOK, STARSHIP_LOOK};

pub const FALCON: SceneDef = SceneDef {
    name: "falcon",
    hue: SceneHue {
        key: 251.0,
        range: 45.0,
        spread: Some(60.0),
    },
    make: |seed| Box::new(LaunchSite::new(Rocket::Falcon, seed)),
};

pub const STARSHIP: SceneDef = SceneDef {
    name: "starship",
    hue: SceneHue {
        key: 251.0,
        range: 45.0,
        spread: Some(60.0),
    },
    make: |seed| Box::new(LaunchSite::new(Rocket::Starship, seed)),
};

/// `n` rounded up to even.
fn ceil_even(n: f64) -> f64 {
    n + f64::from(i32_of(n) & 1)
}

/// Concrete, with the flame trench under the engines; the sea.
const PAD: SceneryCell = SceneryCell {
    glyph: g('▀'),
    fg: 0x9a9da3,
    bg: Some(0x6b6e74),
};
const TRENCH: SceneryCell = SceneryCell {
    glyph: g('▀'),
    fg: 0x4a4c50,
    bg: Some(0x6b6e74),
};
const SEA_FG: u32 = 0x3a7cc0;
const SEA_BG: u32 = 0x1d4e8e;

/// Frames from ignition to liftoff: the hold-down while the engines spool up.
const IGNITE: f64 = 20.0;
/// Frames for the arms to swing shut (or open).
const CLOSE: f64 = 12.0;
/// Frames (~1 s at 14 fps) the rocket holds each level on its way to a new
/// one, so a jump from 1 to 10 (or back) plays every stage: each cloud
/// layer, the dark sky, insertion, the turn to horizontal, full orbit.
const STAGE_FRAMES: f64 = 14.0;
/// Rows per frame per frame of braking, and the top speeds up and down.
const DEC: f64 = 0.03;
const VMAX: f64 = 2.2;
const VMAX_DOWN: f64 = 1.6;
const ACC_DOWN: f64 = 0.025;

/// What the site is doing.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum State {
    Rest,
    Ignite,
    Fly,
    Catch,
    Lower,
    Release,
    Landed,
    Carry,
    Stack,
    Pan,
}

/// What flies as the rocket: the whole stack, the upper stage after
/// separation, or the booster coming home (and Dragon's pieces).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Part {
    Full,
    Upper,
    Booster,
    Dragon,
    Capsule,
    Stage2,
    Trunk,
}

/// A stage that has parted from the rocket: drawn sliding away along its
/// axis (`d` sprite rows), fading out.
#[derive(Clone, Copy, Debug)]
struct Ghost {
    part: Part,
    d: f64,
    v: f64,
    acc: f64,
    life: f64,
    max: f64,
}

/// The layer (world rows) where the climbing stack separates: about level 7.
const SEP_LAYER: f64 = 80.0;

// Flight. Aloft, the level picks a layer of the atmosphere (world rows, the
// sky's scale: 2 per cloud-painter row) and a climb speed; the rocket never
// stops climbing, and the world above the layer is folded into a stack of
// tiles of that layer, each offset sideways and crossfaded at its seams, so
// the layer's clouds keep streaming past for as long as it stays there.
/// The layer each level flies in: 2 low cumulus ... 9 the edge of space, 10 orbit.
const LAYER: [f64; 11] = [
    0.0, 0.0, 22.0, 32.0, 44.0, 56.0, 70.0, 86.0, 140.0, 140.0, 140.0,
];
/// Climb speed per level, world rows per frame.
const SPEED: [f64; 11] = [0.0, 0.0, 0.3, 0.42, 0.55, 0.7, 0.85, 1.0, 1.0, 1.0, 1.0];
/// A tile of folded sky, its seam crossfade, and the sideways offset between tiles.
const TILE: f64 = 14.0;
const FADE: f64 = 3.0;
const TILE_X: f64 = 397.0;
/// The layered painter's scale, and the world rows its clouds span.
const PAINT: f64 = 2.0;
const CLOUD_LO: f64 = 11.0;
const CLOUD_HI: f64 = 90.0;
/// Orbit, per level from 8: insertion (tilted), nearly horizontal,
/// full-speed horizontal orbit. The tilt is in radians from upright, nose
/// toward travel; the speed is how fast the stars stream.
const TILT_BAND: [f64; 3] = [0.9, 1.27, PI / 2.0];
const TILT_TALL: [f64; 3] = [0.52, 1.27, PI / 2.0];
const ORBIT_SPEED: [f64; 3] = [1.6, 2.6, 3.8];
/// The engines in orbit: a thin burn at insertion, less, then coasting.
const ORBIT_BURN: [f64; 3] = [0.3, 0.2, 0.0];
/// In orbit under a tint the engines never coast dark: at least this much burn shows it.
const TINT_BURN: f64 = 0.6;
/// In a tall pane, the camera keeps this fraction of it above the nose while flying.
const HEADROOM: f64 = 0.28;

const DOT: u32 = g('·');
const STAR_GLYPH: u32 = g('*');
const VERT: u32 = g('│');
const HORIZ: u32 = g('─');
const DIAG: u32 = g('╱');
const TOP_HALF: u32 = g('▀');

/// Which rocket a site launches.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Rocket {
    Falcon,
    Starship,
}

impl Rocket {
    fn specs(self) -> &'static [Spec; 2] {
        match self {
            Rocket::Falcon => falcon_specs(),
            Rocket::Starship => starship_specs(),
        }
    }

    fn look(self) -> &'static Look {
        match self {
            Rocket::Falcon => &FALCON_LOOK,
            Rocket::Starship => &STARSHIP_LOOK,
        }
    }
}

/// What one step of flight goes by, worked out at its start.
struct Flight {
    rest: f64,
    home: bool,
    lv: f64,
    want: f64,
    catches: bool,
    ship_home: bool,
    sea: f64,
    cap_sea: f64,
    goal: f64,
    homeward: bool,
    hurry: f64,
}

/// A launch site: the sky world, the rocket on its pad (or flying, or
/// coming home), and on the way home perhaps a second site on a split screen.
pub struct LaunchSite {
    world: SkyWorld,
    rocket: Rocket,
    /// The level being acted out: it walks toward the asked-for level a stage at a time.
    staged: f64,
    since_stage: f64,
    /// Coming home: the acted-out level walking down to 1.
    descending: bool,
    state: State,
    timer: f64,
    fly_time: f64,
    v: f64,
    /// Engine throttle 0..1, eased.
    thr: f64,
    /// The arms' height (pixels) and how far they've swung shut (0 open .. 1 gripping).
    arm_y: f64,
    reach: f64,
    /// The service arm: 1 across to the rocket, 0 swung away.
    service: f64,
    /// What flies now, and a stage parted from it, still drawn while it falls or flies away.
    part: Part,
    ghost: Option<Ghost>,
    /// How faded in the booster is after the cut to it coming home (0..1).
    fade_in: f64,
    /// The rocket's place along the ground from the launch mount (pixels).
    pos: f64,
    /// The camera's place along the ground (pixels).
    view: f64,
    /// The landing zone, off along the ground from the mount (pixels, left), or 0 for none.
    lz: f64,
    /// Restacking Starship: the Ship's place along the ground, its height
    /// above its stacked place (pixels), and the step.
    ship_x: f64,
    stack_off: f64,
    phase: i32,
    /// How far the arms reach out past the rocket's axis (pixels, left).
    arm_x: f64,
    /// Coming home (and then being stacked again): it can't relaunch until it's back together.
    returning: bool,
    /// The split screen: a second site following the booster home while
    /// this one stays with the upper stage; its width (cells, easing in and
    /// out) and the size it's drawn at.
    twin_site: Option<Box<LaunchSite>>,
    split_w: f64,
    twin_w: f64,
    twin_h: f64,
    /// The twin's frames since its booster came to rest (then the panel slides away).
    twin_done: f64,
    /// Separated: the split screen opens once the falling booster has left the frame.
    twin_due: bool,
    /// This site only brings a booster home (the split screen's side): no Ship to stack after.
    booster_only: bool,
    /// Starship's booster is back on the mount, waiting for the Ship.
    booster_home: bool,
    /// Splash: frames since it came down in the sea (spray).
    splash: f64,
    /// The recovery vessel's place along the ground (pixels); NaN when there's none.
    carrier_x: f64,
    /// Coming home through the atmosphere: frames since it began (0 = not).
    burn: f64,
    /// The Ship coming in belly-first: 0 upright .. 1 flat, eased.
    flop: f64,
    /// Dragon's capsule leaning into its lifting entry (radians), eased.
    lean: f64,
    /// The Ship's end at sea: frames of fireball since it tipped over.
    boom: f64,
    /// Dragon's parachutes: 0 none, 1 drogues, 2 mains; how far open (0..1).
    chute: f64,
    chute_open: f64,
    /// The split screen runs top and bottom in a tall pane.
    split_tall: bool,
    braking: bool,
    rng: Rng,
    /// The layer it's flying in (world rows), eased; -1 until the first frame.
    layer: f64,
    /// Tiles folded away so far: offsets each tile's clouds sideways.
    wraps: f64,
    /// 0 climbing upright .. 1 in orbit: tilted, stars streaking, Earth below.
    orbit: f64,
    /// The star field's offset (columns, rows) and velocity this frame.
    star_x: f64,
    star_y: f64,
    star_vx: f64,
    star_vy: f64,
    earth_x: f64,
    /// The tilt from upright (radians) and the orbital speed, both eased.
    tilt: f64,
    orbit_speed: f64,
    /// Pixel rows the camera keeps above the nose (eased).
    cam_p: f64,
    /// Not yet stepped: the first frame starts in the level's own stage, not on the pad.
    fresh: bool,
    /// Room kept above the nose for Dragon's parachutes, eased in as they open (pixels).
    room: f64,

    // The pixel layer: colour and coverage per pixel, and the cells touched this frame.
    pw: usize,
    ph: usize,
    pc: Vec<u32>,
    pa: Vec<f32>,
    touched: Vec<u8>,
    list: Vec<usize>,
    n_touched: usize,
    /// Pixel row of world half-row 0 this frame: a world half-row y is pixel row base - y.
    base: f64,
    fit: QuadFit,
    /// Vapour, steam, smoke and contrail, in world pixels.
    parts: Particles,

    // The site's layout for this grid.
    geo_key: i64,
    tall: bool,
    ox: f64,
    body_px: f64,
    tx: f64,
    pad_l: f64,
    pad_r: f64,
}

impl LaunchSite {
    fn new(rocket: Rocket, seed: f64) -> Self {
        Self {
            world: SkyWorld::new(seed),
            rocket,
            staged: -1.0,
            since_stage: STAGE_FRAMES,
            descending: false,
            state: State::Rest,
            timer: 0.0,
            fly_time: 0.0,
            v: 0.0,
            thr: 0.0,
            arm_y: -1.0,
            reach: 0.0,
            service: 1.0,
            part: Part::Full,
            ghost: None,
            fade_in: 1.0,
            pos: 0.0,
            view: 0.0,
            lz: 0.0,
            ship_x: 0.0,
            stack_off: 0.0,
            phase: 0,
            arm_x: 0.0,
            returning: false,
            twin_site: None,
            split_w: 0.0,
            twin_w: 0.0,
            twin_h: 0.0,
            twin_done: 0.0,
            twin_due: false,
            booster_only: false,
            booster_home: false,
            splash: 0.0,
            carrier_x: f64::NAN,
            burn: 0.0,
            flop: 0.0,
            lean: 0.0,
            boom: 0.0,
            chute: 0.0,
            chute_open: 0.0,
            split_tall: false,
            braking: false,
            rng: Rng::new(seed * 2_654_435_761.0),
            layer: -1.0,
            wraps: 0.0,
            orbit: 0.0,
            star_x: 0.0,
            star_y: 0.0,
            star_vx: 0.0,
            star_vy: 0.0,
            earth_x: 0.0,
            tilt: 0.0,
            orbit_speed: 0.0,
            cam_p: 0.0,
            fresh: true,
            room: 0.0,
            pw: 0,
            ph: 0,
            pc: Vec::new(),
            pa: Vec::new(),
            touched: Vec::new(),
            list: Vec::new(),
            n_touched: 0,
            base: 0.0,
            fit: QuadFit::default(),
            parts: Particles::new(),
            geo_key: -1,
            tall: false,
            ox: 0.0,
            body_px: 0.0,
            tx: 0.0,
            pad_l: 0.0,
            pad_r: 0.0,
        }
    }

    /// Another site like this one, for the split screen.
    fn twin(&self) -> LaunchSite {
        match self.rocket {
            Rocket::Falcon => LaunchSite::new(Rocket::Falcon, self.world.t),
            Rocket::Starship => LaunchSite::new(Rocket::Starship, self.world.t + 1.0),
        }
    }

    /// The rocket's layout on this grid (set by `geo`).
    fn spec(&self) -> &'static Spec {
        &self.rocket.specs()[usize::from(self.tall)]
    }

    fn look(&self) -> &'static Look {
        self.rocket.look()
    }

    fn columns(&self) -> f64 {
        self.world.columns as f64
    }

    fn rows(&self) -> f64 {
        self.world.rows as f64
    }

    /// Lay the site out on this grid: the rocket, the tower to its right, the pad under both.
    fn geo(&mut self) {
        let key = self.world.columns as i64 * 65_536 + self.world.rows as i64;
        if key == self.geo_key {
            return;
        }
        self.geo_key = key;
        self.tall = self.world.rows >= 16;
        let s = self.spec();
        let ox0 = -s.body_l;
        let tx0 = ceil_even(ox0 + s.fly.wf());
        let pw = self.columns() * 2.0;
        let mut body = if self.tall {
            2.0 * round((pw / 2.0 - (ox0 + tx0 + s.tower_w) / 2.0) / 2.0)
        } else {
            2.0 * (self.columns() * 0.42).floor()
        };
        body = (-ox0 + 2.0).max(body.min(pw - tx0 - s.tower_w - 2.0));
        self.body_px = ceil_even(body);
        self.ox = self.body_px + ox0;
        self.tx = self.body_px + tx0;
        self.pad_l = ((self.ox - 2.0) / 2.0).floor();
        self.pad_r = ((self.tx + s.tower_w + 1.0) / 2.0).floor();
        // Falcon's booster lands on its legs on a landing zone off to the left,
        // and Starship's Ship splashes down in the sea there: far enough that the
        // pad is out of sight.
        self.lz = if self.booster_only && self.look().catches {
            0.0
        } else {
            -2.0 * ((pw - self.ox + 4.0) / 2.0).ceil()
        };
        if self.state == State::Rest {
            self.world.alt = s.mount / 2.0;
        }
    }

    /// The camera's place along the ground in whole cells' worth of pixels,
    /// so the pixels and the cells move together. With the split screen
    /// open, this side's view sits centred in the right-hand part.
    fn view_x(&self) -> f64 {
        2.0 * round((self.view - if self.split_tall { 0.0 } else { self.split_w }) / 2.0)
    }

    /// Where the Ship or Dragon comes down in the sea, along the ground from
    /// the mount (pixels): past Falcon's landing zone.
    fn sea_x(&self) -> f64 {
        if self.look().catches {
            self.lz
        } else {
            2.0 * self.lz
        }
    }

    /// The sea: everything left of this column (cells), halfway out from the
    /// land (the pad, or Falcon's landing zone) to where it comes down.
    fn shore(&self) -> f64 {
        if self.lz == 0.0 || self.booster_only {
            return -1e9;
        }
        let land = if self.look().catches { 0.0 } else { self.lz };
        ((self.ox + (self.sea_x() + land) / 2.0) / 2.0).floor()
    }

    /// The landing zone's first column (cells).
    fn lz_l(&self) -> f64 {
        ((self.ox + self.lz) / 2.0).floor()
    }

    /// The rocket's bottom, in world half-rows.
    fn apx(&self) -> f64 {
        round(self.world.alt * 2.0)
    }

    /// The scroll that would put the nose exactly `cam_p` pixels below the top.
    fn scroll_exact(&self) -> f64 {
        (self.apx() + self.top_row() + 2.0 + self.chute_room() + round(self.cam_p)
            - 2.0 * self.rows())
            / 2.0
    }

    /// The height above the rocket's bottom (pixels) of the top of what's
    /// flying: the camera frames that, not the whole stack.
    fn top_row(&self) -> f64 {
        self.spec().fly.hf() - self.part_rows(self.part).0
    }

    /// Room kept above the nose for Dragon's parachutes (pixels).
    fn chute_room(&self) -> f64 {
        round(self.room)
    }

    /// Falcon's booster on the split screen is off at its landing zone: no launch site to see.
    fn site_hidden(&self) -> bool {
        self.booster_only && !self.look().catches
    }

    /// The sprite rows a part covers: the upper stage above `stage`, the booster from it down.
    fn part_rows(&self, part: Part) -> (f64, f64) {
        let s = self.spec();
        match part {
            Part::Upper => (0.0, s.stage),
            Part::Booster => (s.stage, s.fly.hf()),
            Part::Dragon => (0.0, s.dragon),
            Part::Capsule => (0.0, s.capsule),
            Part::Trunk => (s.capsule, s.dragon),
            Part::Stage2 => (s.dragon, s.stage),
            Part::Full => (0.0, s.fly.hf()),
        }
    }

    fn go(&mut self, state: State) {
        self.state = state;
        self.timer = 0.0;
    }

    /// The booster's look: grid fins out coming home, legs out only for the landing burn and after.
    fn booster_sprite(&self) -> &'static Sprite {
        let s = self.spec();
        if self.braking
            || matches!(
                self.state,
                State::Landed | State::Catch | State::Lower | State::Release
            )
        {
            &s.land
        } else {
            &s.fins
        }
    }

    /// Where the plume leaves what's flying: its bottom (world half-rows), its
    /// centre (pixels), and how wide it burns (the upper stage's one engine is narrower).
    fn nozzle(&self) -> (f64, f64, f64) {
        let s = self.spec();
        let bottom = self.part_rows(self.part).1;
        (
            self.apx()
                + if bottom != 0.0 {
                    s.fly.hf() - bottom
                } else {
                    0.0
                },
            self.body_px + self.pos + s.body_w / 2.0 - 0.5,
            if self.part == Part::Upper { 0.6 } else { 1.0 },
        )
    }

    /// Where the rocket is drawn: its centre (grid pixels), tilt and scale, eased into orbit.
    fn pose(&self, sp: &Sprite, pos_x: f64, turned: bool) -> (f64, f64, f64, f64) {
        let o = self.orbit;
        let cx = self.ox + pos_x + sp.wf() / 2.0;
        let cy = self.base - self.apx() - sp.hf() + 1.0 + sp.hf() / 2.0;
        if o <= 0.0 && turned && (self.flop > 0.0 || self.lean != 0.0) {
            // Belly-first, tiles down, nose toward the sea (or Dragon leaning into
            // its entry): turned about the middle of what's flying.
            let th = -(PI / 2.0) * self.flop + self.lean;
            let (r0, r1) = self.part_rows(self.part);
            let off = sp.hf() / 2.0 - (r0 + r1) / 2.0;
            let sn = th.sin();
            let cs = th.cos();
            return (cx - 2.0 * off * sn, cy - off + off * cs, th, 1.0);
        }
        if o <= 0.0 {
            return (cx, cy, 0.0, 1.0);
        }
        let k = o * o * (3.0 - 2.0 * o);
        let (pw, ph) = (self.pw as f64, self.ph as f64);
        // In orbit it sits mid-grid, tilted toward its travel; in the spine the
        // camera pulls back so the whole tilted stack fits the narrow pane.
        let ocx = pw / 2.0 + if self.tall { 0.0 } else { -2.0 };
        let ocy =
            (ph - if self.split_tall {
                2.0 * self.split_w
            } else {
                0.0
            }) * if self.tall { 0.42 } else { 0.36 };
        // Shrunk just enough for the tilted stack to fit the pane's width (its right-hand part, split).
        let room = pw
            - if self.split_tall {
                0.0
            } else {
                2.0 * self.split_w
            };
        let sc = if self.tall {
            1.0 - k
                * (1.0
                    - 0.85_f64.min((room - 7.0) / (2.0 * sp.hf() * 0.3_f64.max(self.tilt.sin()))))
        } else {
            1.0
        };
        // Centred on what's flying (after separation, the upper stage), not on the whole stack:
        // the stage's middle is `off` sprite rows from the sprite's, along its axis.
        let (r0, r1) = self.part_rows(self.part);
        let off = sp.hf() / 2.0 - (r0 + r1) / 2.0;
        let sn = self.tilt.sin();
        let cs = self.tilt.cos();
        (
            cx + (ocx - 2.0 * off * sn * sc - cx) * k,
            cy + (ocy + off * cs * sc - cy) * k,
            self.tilt,
            sc,
        )
    }

    /// Where what's flying is in this site's own grid: the cell at its middle (column, row).
    fn focus(&mut self) -> (f64, f64) {
        self.geo();
        let s = self.spec();
        let (r0, r1) = self.part_rows(self.part);
        let col = (self.body_px + self.pos + s.body_w / 2.0 - self.view_x()) / 2.0;
        let base = 2.0 * (self.scroll() + self.rows() - 2.0) + 1.0;
        let row = (base - self.apx() - (s.fly.hf() - (r0 + r1) / 2.0)) / 2.0;
        (col, row)
    }

    // ------------------------------------------------------------ flight

    /// One frame of flight: whatever the site is doing, then the arms, the
    /// throttle, orbit, the camera, the folded sky, the stars, a parted
    /// stage and the particles.
    fn advance_flight(&mut self) {
        self.geo();
        let s = self.spec();
        let rest = s.mount / 2.0;
        let catch_alt = (s.arm_catch - s.grip) / 2.0;
        let home = self.world.target() <= 0.0;
        let lv = clamp(round(self.world.dials.strength), 0.0, 10.0);
        let want = LAYER[lv as usize];
        if self.fresh {
            self.fresh = false;
            // Picked while the level is already flying (a switch from the other
            // rocket, a reload): appear mid-stage rather than launching and racing
            // up through every stage to it.
            if !home && lv >= 2.0 {
                self.settle(lv, want, catch_alt);
            }
        }
        if self.layer < 0.0 {
            self.layer = if home { LAYER[2] } else { want };
        }
        // Falcon: only the booster comes home, onto its legs. Starship: the
        // booster into the arms (on the split screen), the Ship into the sea.
        let catches = self.look().catches;
        let ship_home = catches && !self.booster_only;
        // (The Ship's own bottom at sea level: its sprite's frame sits the booster's height below.)
        let sea = -(s.fly.hf() - s.stage) / 2.0;
        // (Dragon's capsule, the same: its bottom at sea level.)
        let cap_sea = -(s.fly.hf() - s.capsule) / 2.0;
        let goal = if ship_home {
            sea
        } else if catches {
            catch_alt
        } else {
            rest
        };
        // Once it's coming home it lands and is stacked again before it can relaunch.
        let homeward = home || self.returning;
        self.step_twin();
        // Asked to launch meanwhile, the ground crew hurry.
        let hurry = if home { 1.0 } else { 3.0 };
        let f = Flight {
            rest,
            home,
            lv,
            want,
            catches,
            ship_home,
            sea,
            cap_sea,
            goal,
            homeward,
            hurry,
        };
        let mut thr_goal = 0.0;
        let mut arm_goal = s.mount + s.grip;
        self.timer += 1.0;
        self.braking = false;
        if self.fade_in < 1.0 {
            self.fade_in = (self.fade_in + 0.06).min(1.0);
        }
        // From the trunk's jettison on (high and falling fast), so nothing moves when they open.
        let room_goal =
            if self.chute != 0.0 || (self.part == Part::Capsule && self.state == State::Fly) {
                if self.tall {
                    14.0
                } else {
                    4.0
                }
            } else {
                0.0
            };
        self.room += clamp(room_goal - self.room, -0.5, 0.5);

        match self.state {
            State::Rest => {
                self.world.alt = rest;
                self.v = 0.0;
                if self.part == Part::Full {
                    self.returning = false;
                }
                // An empty transporter drives back off.
                if self.carrier_x.is_finite() {
                    self.carrier_x += 2.0;
                    if self.ox + self.carrier_x - self.view_x() > self.pw as f64 + 4.0 {
                        self.carrier_x = f64::NAN;
                    }
                }
                self.reach = (self.reach - 1.0 / CLOSE).max(0.0);
                self.service = (self.service + 0.05).min(1.0);
                if !home {
                    self.go(State::Ignite);
                }
            }
            State::Ignite => {
                self.world.alt = rest;
                // On the pad nothing above the tower shows yet: the layer can be set outright.
                self.layer = want;
                self.service = (self.service - 0.1).max(0.0);
                thr_goal = if self.timer < 4.0 {
                    0.15
                } else {
                    (self.timer / IGNITE).min(1.0)
                };
                if home {
                    self.go(State::Rest);
                } else if self.timer >= IGNITE {
                    self.go(State::Fly);
                    self.fly_time = 0.0;
                }
            }
            State::Fly => self.fly(&f, &mut thr_goal, &mut arm_goal),
            State::Catch => {
                self.world.alt = catch_alt;
                arm_goal = s.arm_catch;
                // Hover on the engines until the arms are up, then swing them shut.
                if (self.arm_y - s.arm_catch).abs() > 0.5 {
                    self.timer = 0.0;
                }
                self.reach = (self.timer / CLOSE).min(1.0);
                thr_goal = if self.reach < 1.0 { 0.45 } else { 0.0 };
                if self.timer >= CLOSE + 8.0 {
                    self.go(State::Lower);
                }
            }
            State::Lower => {
                arm_goal = -1.0;
                let span = (catch_alt - rest).max(0.5);
                let frac = (self.world.alt - rest) / span;
                self.world.alt -= 0.012_f64
                    .max(span * (0.004 + 0.022 * (PI * frac.min(1.0)).sin()))
                    * if self.booster_only { 2.0 } else { 1.0 };
                if self.world.alt <= rest {
                    self.world.alt = rest;
                    self.go(State::Release);
                }
            }
            State::Release => {
                arm_goal = -1.0;
                self.reach = (self.reach - 1.0 / CLOSE).max(0.0);
                // The split screen's booster just stays on the mount.
                if self.reach <= 0.0 {
                    self.go(State::Rest);
                }
            }
            State::Landed => self.landed(&f),
            State::Pan => {
                // Sliding back from the capsule bobbing in the sea to the pad.
                self.world.alt = cap_sea;
                let speed = (4.0 * hurry).min(0.6_f64.max(self.view.abs() * 0.05 * hurry));
                self.view += clamp(-self.view, -speed, speed);
                if self.view.abs() < 0.5 {
                    self.reset_to_pad();
                }
            }
            State::Carry => {
                // The Ship carried back to the pad, the camera with it, on a barge then
                // a transporter, to beside the booster waiting on the mount.
                let to = s.body_l - 3.0 - s.fly.wf();
                self.world.alt = sea + 1.0;
                let speed = (2.5 * hurry).min(0.3_f64.max((to - self.pos).abs() * 0.04 * hurry));
                self.pos += clamp(to - self.pos, -speed, speed);
                self.view = self.pos;
                self.carrier_x = self.pos;
                if (to - self.pos).abs() < 0.01 {
                    // The arms take it from here: the booster on the mount is what stands now.
                    self.ship_x = to;
                    self.pos = 0.0;
                    self.world.alt = rest;
                    self.part = Part::Booster;
                    self.stack_off = 2.0 - (self.apx() + (s.fly.hf() - s.stage));
                    self.go(State::Stack);
                    self.phase = 1;
                }
            }
            State::Stack => arm_goal = self.stack(&f),
        }

        // The carriage rides with the rocket while it grips; otherwise it eases to its goal.
        if arm_goal < 0.0 {
            self.arm_y = self.apx() + s.grip;
        } else if self.arm_y < 0.0 {
            self.arm_y = arm_goal;
        } else {
            self.arm_y += clamp(arm_goal - self.arm_y, -0.6, 0.6);
        }

        self.thr += (thr_goal - self.thr) * 0.35;
        if self.thr < 0.02 && thr_goal == 0.0 {
            self.thr = 0.0;
        }

        let flying = self.state == State::Fly;
        // Into orbit from 8, high enough; out of it as soon as it's asked down.
        let orbit_goal = flying && !homeward && lv >= 8.0 && self.layer > LAYER[8] - 30.0;
        self.orbit += clamp(
            if orbit_goal { 1.0 } else { 0.0 } - self.orbit,
            -0.03,
            0.025,
        );
        if self.orbit < 0.001 {
            self.orbit = 0.0;
        }
        // The tilt and speed ease toward the level's: 8 tilted, 9 nearly flat, 10 flat out.
        let oi = clamp(lv - 8.0, 0.0, 2.0) as usize;
        let tilt_goal = if orbit_goal {
            if self.tall {
                TILT_TALL[oi]
            } else {
                TILT_BAND[oi]
            }
        } else {
            0.0
        };
        self.tilt += clamp(tilt_goal - self.tilt, -0.055, 0.025);
        self.orbit_speed +=
            (if orbit_goal { ORBIT_SPEED[oi] } else { 0.0 } - self.orbit_speed) * 0.04;
        // A tall pane keeps the rocket mid-screen in flight (the ground back in view for the catch).
        let visible = self.rows() - if self.split_tall { self.split_w } else { 0.0 };
        // Headroom above the nose, but never so much the rest of it drops out of the bottom.
        let headroom = round(HEADROOM * visible * 2.0)
            .min((2.0 * visible - (s.fly.hf() - self.part_rows(self.part).0) - 6.0).max(0.0));
        let cam_goal = if self.tall && flying && (!homeward || self.world.alt - goal > 25.0) {
            headroom
        } else {
            0.0
        };
        self.cam_p += clamp(cam_goal - self.cam_p, -0.6, 0.6);
        // Fold away a tile once the real world below the layer is off the grid.
        if flying && !homeward {
            let a = self.layer - TILE / 2.0;
            while self.scroll() - 1.0 - a >= 2.0 * TILE + 1.0 {
                self.world.alt -= TILE;
                self.wraps += 1.0;
                self.parts.shift(-2.0 * TILE);
            }
        }
        // The stars stream past: down as it climbs, and down-and-back along its tilt in orbit.
        let tilt = self.tilt;
        let os = self.orbit * self.orbit_speed;
        self.star_vx = tilt.sin() * os;
        self.star_vy = (if flying {
            self.v * (1.0 - self.orbit)
        } else {
            0.0
        }) + tilt.cos() * os * 0.5;
        self.star_x += self.star_vx;
        self.star_y += self.star_vy;
        self.earth_x += self.orbit * self.orbit_speed * 0.45;

        // A parted stage slides away along the axis and fades out.
        if let Some(gh) = self.ghost.as_mut() {
            gh.v += gh.acc;
            gh.d += gh.v;
            gh.life -= 1.0;
            if gh.life <= 0.0 {
                self.ghost = None;
            }
        }

        self.step_particles();
    }

    /// Flying: climbing, coming home, or (the split screen's side) falling back.
    fn fly(&mut self, f: &Flight, thr_goal: &mut f64, arm_goal: &mut f64) {
        let s = self.spec();
        self.fly_time += 1.0;
        // The service arm swings away for flight.
        self.service = (self.service - 0.1).max(0.0);
        if self.booster_only && !f.home {
            // The split screen's booster falls back through the sky, a layer a
            // second (the level walking down): the world unfolds above it as it drops.
            self.v = (self.v - 0.02).max(-1.0);
            self.world.alt += self.v;
            self.layer += clamp(f.want - self.layer, -0.6, 0.6);
            self.unfold();
            // Super Heavy's boostback, seconds after staging, turns it for the
            // tower; Falcon's entry burn, high up, takes the edge off the heating.
            let burning = if f.catches {
                self.fly_time < 136.0
            } else {
                self.staged == 4.0 && self.since_stage < 14.0
            };
            if burning {
                *thr_goal = 0.5;
            }
            // (Back down into the thick air it makes its sonic boom: flow's sound only.)
            return;
        }
        if !f.homeward {
            // Always climbing: a launch's slow start, then the level's speed, and
            // faster still while it's working up into a higher layer.
            let v_goal = SPEED[f.lv as usize] * (1.0 - 0.8 * self.orbit)
                + (f.want - self.layer).max(0.0) * 0.012;
            if self.v < v_goal {
                self.v = v_goal.min(self.v + 0.006 + self.fly_time.min(120.0) * 0.0004);
            } else {
                self.v = v_goal.max(self.v - 0.02);
            }
            self.world.alt += self.v;
            let rate = (0.6 * self.v.max(0.0)).max(0.05);
            self.layer += clamp(f.want - self.layer, -rate, rate);
            *thr_goal = 1.0 - self.orbit * (1.0 - ORBIT_BURN[(f.lv - 8.0).max(0.0) as usize]);
            if self.world.dials.tint != Tint::Normal {
                *thr_goal = thr_goal.max(TINT_BURN);
            }
            // High enough, the stack separates: the spent booster falls away,
            // the engines cut a moment, and the upper stage lights.
            if self.part == Part::Full && self.layer >= SEP_LAYER {
                self.separate();
            }
            // In orbit Dragon parts from the second stage, which drifts away (to burn up later).
            if s.dragon != 0.0 && self.part == Part::Upper && self.orbit > 0.95 {
                self.part = Part::Dragon;
                self.ghost = Some(Ghost {
                    part: Part::Stage2,
                    d: 0.0,
                    v: -0.02,
                    acc: -0.008,
                    life: 70.0,
                    max: 70.0,
                });
            }
            return;
        }
        self.returning = true;
        if f.ship_home && self.part == Part::Upper && self.burn < 100.0 {
            // The Ship comes in: belly-first, tiles down, glowing, falling back
            // down through the sky (it flips upright only for the landing burn,
            // below). The camera follows it out over the sea.
            self.burn += 1.0;
            // About 60 degrees (nose high) through the glowing part; flat for the belly-flop below it.
            let flat = if self.burn < 85.0 { 0.67 } else { 1.0 };
            self.flop += clamp(flat - self.flop, -0.04, 0.04);
            *thr_goal = 0.0;
            self.v = (self.v - 0.03).max(-1.2);
            self.world.alt += self.v;
            let to = if self.burn < 80.0 { LAYER[4] } else { LAYER[2] };
            self.layer += clamp(to - self.layer, -1.5, 1.5);
            self.unfold();
            let step = ((self.lz - self.pos).abs() / 80.0).max(0.4);
            self.pos += clamp(self.lz - self.pos, -step, step);
            self.view = self.pos;
            return;
        }
        // Brought home before it staged: it stages now (the booster on the split screen).
        if self.part == Part::Full && !self.booster_only {
            self.separate();
        }
        if s.dragon != 0.0 && matches!(self.part, Part::Upper | Part::Dragon | Part::Capsule) {
            self.dragon_home(f, thr_goal);
            return;
        }
        // Falcon's booster flies back across to its landing zone on the way down
        // (Starship's Ship out over the sea), the camera with it: it stays in
        // view and the world slides past.
        if self.lz != 0.0 {
            let frames_left = ((self.world.alt - f.goal) / 0.8).max(10.0);
            let step = ((self.lz - self.pos).abs() / frames_left).max(0.4);
            self.pos += clamp(self.lz - self.pos, -step, step);
            self.view = self.pos;
        }
        let d = f.goal - self.world.alt;
        let brake_v = (2.0 * DEC * d.abs()).sqrt();
        // The Ship, still belly-first, flips upright into its landing burn.
        if self.flop > 0.0 && (self.braking || self.v <= -brake_v * 0.9) {
            self.flop = (self.flop - 0.05).max(0.0);
        }
        if d > 0.0 {
            // Below its goal: climb gently back up to it.
            self.v += 0.006 + self.fly_time.min(120.0) * 0.0004;
            self.v = self.v.min(VMAX).min(brake_v);
            *thr_goal = if self.v >= brake_v - 0.01 && d < 4.0 {
                0.65
            } else {
                1.0
            };
        } else if d < 0.0 {
            self.v = (self.v - ACC_DOWN).max(-VMAX_DOWN);
            if self.v <= -brake_v {
                self.v = -brake_v;
                self.braking = true;
            }
            // Coasting down with the engines off, then the landing burn.
            *thr_goal = if self.braking { 0.6 } else { 0.0 };
        } else {
            *thr_goal = 0.6;
        }
        if d.abs() <= self.v.abs() + 0.02 && self.v.abs() < 0.25 {
            self.world.alt = f.goal;
            self.v = 0.0;
            self.pos = self.lz;
            self.view = self.lz;
            self.go(if f.ship_home {
                State::Landed
            } else if f.catches {
                State::Catch
            } else {
                State::Landed
            });
            if f.ship_home {
                self.splash = 1.0;
            }
        } else {
            self.world.alt += self.v;
        }
        if f.catches && !f.ship_home {
            *arm_goal = s.arm_catch;
        }
    }

    /// Dragon comes home: it parts from the second stage (which goes on to
    /// burn up), drops its trunk, comes in heat shield first glowing, opens
    /// its drogues then its four mains, and comes down in the sea. Its next
    /// ride waits at the pad on the tower's arm.
    fn dragon_home(&mut self, f: &Flight, thr_goal: &mut f64) {
        self.burn += 1.0;
        *thr_goal = 0.0;
        if self.part == Part::Upper {
            self.part = Part::Dragon;
            self.ghost = Some(Ghost {
                part: Part::Stage2,
                d: 0.0,
                v: -0.03,
                acc: -0.012,
                life: 60.0,
                max: 60.0,
            });
        }
        if self.part == Part::Dragon && self.burn > 20.0 {
            self.part = Part::Capsule;
            self.ghost = Some(Ghost {
                part: Part::Trunk,
                d: 0.0,
                v: -0.03,
                acc: -0.015,
                life: 40.0,
                max: 40.0,
            });
        }
        // Out over the sea, the camera with it.
        let sea_x = self.sea_x();
        let step = ((sea_x - self.pos).abs() / 60.0).max(0.4);
        self.pos += clamp(sea_x - self.pos, -step, step);
        self.view = self.pos;
        // Its offset centre of mass trims it at an angle, heat shield forward, while it's hot.
        let lean = if self.burn > 20.0 && self.burn < 95.0 {
            -0.3
        } else {
            0.0
        };
        self.lean += clamp(lean - self.lean, -0.02, 0.02);
        if self.burn < 95.0 {
            // Re-entry: falling through the high sky.
            self.v = (self.v - 0.03).max(-1.2);
            self.world.alt += self.v;
            self.layer += clamp(LAYER[5] - self.layer, -0.6, 0.6);
            self.unfold();
            return;
        }
        let want = if self.burn < 115.0 { 1.0 } else { 2.0 };
        if want != self.chute {
            self.chute = want;
            self.chute_open = 0.0;
        }
        self.chute_open = (self.chute_open + 0.06).min(1.0);
        let sink = if self.chute == 1.0 { -0.9 } else { -0.6 };
        self.v += clamp(sink - self.v, -0.05, 0.05);
        self.world.alt += self.v;
        if self.layer > LAYER[2] + 1.0 {
            self.layer += clamp(LAYER[2] - self.layer, -1.2, 1.2);
            self.unfold();
        } else if self.world.alt <= f.cap_sea {
            self.world.alt = f.cap_sea;
            self.v = 0.0;
            self.pos = self.sea_x();
            self.view = self.pos;
            self.splash = 1.0;
            self.go(State::Landed);
        }
    }

    /// Down on its legs at the landing zone (the split screen's Falcon
    /// booster: it stays there), or in the sea.
    fn landed(&mut self, f: &Flight) {
        let s = self.spec();
        let hurry = f.hurry;
        self.v = 0.0;
        if !f.ship_home && self.part != Part::Capsule {
            self.world.alt = f.rest;
            return;
        }
        // In the sea, its parachutes (Dragon's) settling on the water.
        if self.chute_open > 0.0 {
            self.chute_open = (self.chute_open - 0.05).max(0.0);
        } else {
            self.chute = 0.0;
        }
        if self.part == Part::Capsule {
            // Dragon: a moment in the spray, then the camera slides back along
            // the coast to the pad, reset for the next flight.
            self.world.alt = f.cap_sea;
            if self.timer * hurry >= 24.0 {
                self.service = 1.0;
                self.go(State::Pan);
            }
            return;
        }
        // The Ship: it tips over onto the water (and goes up in a fireball, as
        // they do), then a recovery barge comes out from the shore, slides in
        // under it and lifts it, righted, onto its deck.
        // (Lying flat, its middle sits this much lower: half its width, not half its length.)
        let lie = (s.stage / 2.0 - s.body_w / 4.0) / 2.0;
        if self.timer * hurry < 30.0 {
            self.flop = (self.flop + 0.05 * hurry).min(1.0);
            self.world.alt = f.sea - self.flop * lie;
            if self.timer == 16.0 && hurry == 1.0 {
                self.boom = 1.0;
            }
            return;
        }
        if !self.carrier_x.is_finite() {
            self.carrier_x = self.pos + self.pw as f64 / 2.0 + 2.0 * s.fly.wf();
        }
        if self.carrier_x > self.pos {
            self.world.alt = f.sea - self.flop * lie;
            let speed = (2.0 * hurry).min(0.3_f64.max((self.carrier_x - self.pos) * 0.05 * hurry));
            self.carrier_x = self.pos.max(self.carrier_x - speed);
        } else {
            self.flop = (self.flop - 0.04 * hurry).max(0.0);
            self.world.alt = f.sea + (1.0 - self.flop) - self.flop * lie;
            if self.flop <= 0.0 {
                self.go(State::Carry);
            }
        }
    }

    /// Starship: the Ship rolls in on its transporter; the arms reach over,
    /// grip it, lift it, swing it across onto the booster and let go. Gives
    /// the arms' goal.
    fn stack(&mut self, f: &Flight) -> f64 {
        let s = self.spec();
        let hurry = f.hurry;
        self.world.alt = f.rest;
        // The camera eases back from where the transporter stopped.
        self.view += clamp(-self.view, -0.5, 0.5);
        let park = s.body_l - 3.0 - s.fly.wf();
        let grip = round(s.stage * 0.6);
        let bottom = self.apx() + (s.fly.hf() - s.stage);
        let arm_goal = bottom + round(self.stack_off) + grip;
        let there = (self.arm_y - arm_goal).abs() < 1.0;
        // Once the arms have it, the transporter drives back off the way it came... left.
        if self.phase >= 2 && self.carrier_x.is_finite() {
            self.carrier_x -= 1.5 * hurry;
            if self.ox + self.carrier_x + s.fly.wf() < self.view_x() {
                self.carrier_x = f64::NAN;
            }
        }
        match self.phase {
            0 => {
                self.ship_x += (1.5 * hurry).min(park - self.ship_x);
                if self.ship_x >= park {
                    self.phase = 1;
                }
            }
            1 => {
                self.arm_x = self.ship_x;
                if there {
                    self.reach = (self.reach + hurry / CLOSE).min(1.0);
                }
                if self.reach >= 1.0 {
                    self.phase = 2;
                }
            }
            2 => {
                self.stack_off = (self.stack_off + 0.25 * hurry).min(3.0);
                if self.stack_off >= 3.0 && there {
                    self.phase = 3;
                }
            }
            3 => {
                self.ship_x += (0.4 * hurry).min(-self.ship_x);
                self.arm_x = self.ship_x;
                if self.ship_x >= 0.0 {
                    self.phase = 4;
                }
            }
            4 => {
                self.stack_off = (self.stack_off - 0.2 * hurry).max(0.0);
                if self.stack_off <= 0.0 {
                    self.part = Part::Full;
                    self.phase = 5;
                }
            }
            _ => {
                self.reach = (self.reach - hurry / CLOSE).max(0.0);
                if self.reach <= 0.0 {
                    self.arm_x = 0.0;
                    self.returning = false;
                    self.booster_home = false;
                    self.burn = 0.0;
                    self.flop = 0.0;
                    self.go(State::Rest);
                }
            }
        }
        arm_goal
    }

    /// Start already in a level's steady flight: in its layer, at its speed, in orbit from 8.
    fn settle(&mut self, lv: f64, layer: f64, catch_alt: f64) {
        self.state = State::Fly;
        self.fly_time = 120.0;
        self.world.alt = catch_alt + 200.0;
        self.v = SPEED[lv as usize];
        self.thr = 1.0;
        self.layer = layer;
        self.service = 0.0;
        self.reach = 0.0;
        self.part = if layer >= SEP_LAYER {
            Part::Upper
        } else {
            Part::Full
        };
        self.ghost = None;
        self.pos = 0.0;
        self.view = 0.0;
        if lv >= 8.0 {
            let oi = (lv - 8.0).min(2.0) as usize;
            self.orbit = 1.0;
            self.tilt = if self.tall {
                TILT_TALL[oi]
            } else {
                TILT_BAND[oi]
            };
            self.orbit_speed = ORBIT_SPEED[oi];
        }
        if self.tall {
            self.cam_p = round(HEADROOM * self.rows() * 2.0);
        }
    }

    /// The stack separates: the upper stage flies on. Falcon's spent booster
    /// falls away; Starship's turns back for the tower. Once it's fallen out
    /// of frame a split screen follows it down (side by side in the band,
    /// top and bottom in a tall pane), when there's room for one; else it's
    /// simply home in time.
    fn separate(&mut self) {
        self.part = Part::Upper;
        self.thr = 0.0;
        self.ghost = Some(Ghost {
            part: Part::Booster,
            d: 0.0,
            v: -0.05,
            acc: -0.025,
            life: 48.0,
            max: 48.0,
        });
        let room = if self.tall {
            (self.rows() / 2.0).floor() >= 12.0
        } else {
            (self.columns() / 2.0).floor() >= 9.0
        };
        if room {
            self.twin_due = true;
        } else {
            self.booster_home = true;
        }
    }

    /// Dragon's home: everything back on the pad as it was.
    fn reset_to_pad(&mut self) {
        self.part = Part::Full;
        self.pos = 0.0;
        self.view = 0.0;
        self.world.alt = self.spec().mount / 2.0;
        self.v = 0.0;
        self.returning = false;
        self.booster_home = false;
        self.burn = 0.0;
        self.chute = 0.0;
        self.chute_open = 0.0;
        self.room = 0.0;
        self.lean = 0.0;
        self.ghost = None;
        self.service = 1.0;
        self.go(State::Rest);
    }

    /// Falling through the folded sky: unfold a tile whenever the real world below would come into view.
    fn unfold(&mut self) {
        let a = self.layer - TILE / 2.0;
        while self.scroll() - 1.0 - a < TILE + 1.0 {
            self.world.alt += TILE;
            self.parts.shift(2.0 * TILE);
        }
    }

    /// Open the split screen on the booster falling back, high in the sky.
    fn open_twin(&mut self) {
        self.split_tall = self.tall;
        let (columns, rows) = (self.world.columns, self.world.rows);
        let w = if self.split_tall {
            columns
        } else {
            columns / 2
        };
        let h = if self.split_tall { rows / 2 } else { rows };
        let mut t = Box::new(self.twin());
        t.booster_only = true;
        t.world.dials.night = self.world.dials.night;
        t.world.dials.tint = self.world.dials.tint;
        t.world.dials.strength = 1.0;
        Scene::ensure(t.as_mut(), w, h);
        t.geo();
        t.fresh = false;
        t.part = Part::Booster;
        t.returning = true;
        t.state = State::Fly;
        t.fly_time = 120.0;
        // It falls back down through the sky a layer a second, from where it separated.
        t.staged = 7.0;
        t.since_stage = 0.0;
        t.layer = LAYER[7];
        t.world.alt = t.layer + TILE;
        t.v = -0.6;
        t.service = 0.0;
        self.twin_site = Some(t);
        self.twin_w = w as f64;
        self.twin_h = h as f64;
        self.twin_done = 0.0;
    }

    /// Step the split screen's booster, and open or close the panel around it.
    fn step_twin(&mut self) {
        if self.twin_due && self.ghost.is_none() {
            self.twin_due = false;
            self.open_twin();
        }
        let Some(t) = self.twin_site.as_mut() else {
            return;
        };
        t.world.dials.night = self.world.dials.night;
        t.world.dials.tint = self.world.dials.tint;
        t.world.dials.strength = 1.0;
        Scene::step(t.as_mut());
        // Down: on the mount in the arms, or on its legs at the landing zone.
        if t.state == State::Rest || t.state == State::Landed {
            self.twin_done += 1.0;
        }
        // In over a second; once the booster's been down a moment, out again.
        let open = self.twin_done < 20.0;
        let full = if self.split_tall {
            self.twin_h
        } else {
            self.twin_w
        };
        self.split_w += clamp(if open { full } else { 0.0 } - self.split_w, -0.6, 0.6);
        if !open && self.split_w <= 0.0 {
            self.split_w = 0.0;
            self.twin_site = None;
            self.booster_home = true;
        }
    }

    // ------------------------------------------------------------ the sky

    /// The sky world, folded: below the layer it's the real world (the pad, the
    /// low sky); above, a stack of tiles of the layer, each its clouds offset
    /// sideways and crossfaded at the seams, which the climbing rocket streams
    /// through forever. The sky's colour follows the layer, not the climb; the
    /// clouds glide by sub-row steps; stars stream (and streak in orbit).
    fn draw_frame(&mut self) {
        let (w, h) = (self.world.columns, self.world.rows);
        if self.world.dials.strength <= 0.0 {
            for i in 0..w * h {
                self.world.out.blank(i);
            }
            return;
        }
        self.geo();
        let hf = h as f64;
        let scroll = self.scroll();
        let exact =
            ((self.world.alt * 2.0 + self.top_row() + 2.0 + self.chute_room() + round(self.cam_p)
                - 2.0 * hf)
                / 2.0)
                .max(0.0);
        let phi = exact - scroll;
        let layer = self.layer.max(0.0);
        let sky_shift = (self.world.alt - layer).max(0.0);
        let a = layer - TILE / 2.0;
        let o = self.orbit;
        let drift = ((self.world.t * 0.1) % 100_000.0).floor();
        // The stars' glyph, by how fast and which way they stream.
        let svx = self.star_vx.abs();
        let svy = self.star_vy.abs() * 2.0;
        let star_glyph = if svx + svy < 0.7 {
            0
        } else if svx < svy * 0.5 {
            VERT
        } else if svy < svx * 0.4 {
            HORIZ
        } else {
            DIAG
        };
        let sx = self.star_x.floor();
        let sy = self.star_y.floor();
        // The camera's place along the ground, in cells: the ground, clouds and stars slide past with it.
        let pan_c = self.view_x() / 2.0;
        let dark = self.world.dark();
        let mut out = std::mem::take(&mut self.world.out);
        let mut rb = std::mem::take(&mut self.world.row_bg);
        rb.resize(h, 0);
        for (r, row_bg) in rb.iter_mut().enumerate() {
            let rf = r as f64;
            let y = scroll - 1.0 + (hf - 1.0 - rf);
            let yf = y + phi;
            let real = yf - a < TILE;
            let mut sky_y = yf - sky_shift;
            // Below the layer the sky starts at the horizon; above it, the
            // layer's own sky (the ground only where it really shows).
            if sky_y < 0.0 && (y >= 0.0 || (sky_shift > 0.0 && !(real && y < 0.0))) {
                sky_y = 0.0;
            }
            let mut bg = if real && y < 0.0 {
                self.world.sky_at(y)
            } else {
                self.world.sky_at(sky_y)
            };
            if o > 0.0 {
                bg = mix(bg, 0, o);
            }
            *row_bg = bg;
            // By night the stars are out at every height.
            let night_stars = if !(real && y <= 0.0) { 0.5 * dark } else { 0.0 };
            let stars = (o * 1.3)
                .max(((sky_y - 70.0) / 40.0).min(1.0))
                .max(night_stars)
                * 0.04;
            for x in 0..w {
                let xf = x as f64;
                let i = r * w + x;
                // The ground (only ever in the real world below the layer).
                if real && y <= 0.0 {
                    match self.scenery(xf + pan_c, y, bg) {
                        Some(s) => out.set(i, s.glyph, s.fg, s.bg.unwrap_or(bg)),
                        None => out.set(i, 0x20, DEFAULT_COLOR, bg),
                    }
                    continue;
                }
                // Climbing into orbit the clouds fade into the darkening sky with it;
                // then every cloud colour is snapped to a few steps off the sky (see snap).
                if o < 0.98 {
                    if let Some((c_fg, c_bg)) = self.folded_cloud(xf + drift + pan_c, yf, a, bg) {
                        let fg = snap(if o > 0.0 { mix(c_fg, bg, o) } else { c_fg }, bg);
                        let cb = snap(if o > 0.0 { mix(c_bg, bg, o) } else { c_bg }, bg);
                        if fg != bg || cb != bg {
                            out.set(i, TOP_HALF, fg, cb);
                            continue;
                        }
                    }
                }
                if stars > 0.0 {
                    let hs = hash(xf + sx + pan_c, rf - sy, 7.0);
                    if hs < stars {
                        let bright = if hash(xf + sx + pan_c, rf - sy, 8.0) < 0.5 {
                            STAR
                        } else {
                            STAR_DIM
                        };
                        let glyph = if star_glyph != 0 {
                            star_glyph
                        } else if hs < stars * 0.2 {
                            STAR_GLYPH
                        } else {
                            DOT
                        };
                        out.set(i, glyph, bright, bg);
                        continue;
                    }
                }
                out.set(i, 0x20, DEFAULT_COLOR, bg);
            }
        }
        self.world.row_bg = rb;
        self.world.out = out;
        self.world.draw_moon();
        let mut out = std::mem::take(&mut self.world.out);
        self.draw_site(&mut out);
        self.draw_split(&mut out);
        self.world.out = out;
    }

    /// The split screen: the twin's view in the left-hand part sliding in from
    /// the left edge (a tall pane: the bottom part, rising from the bottom
    /// edge), and a divider.
    fn draw_split(&mut self, out: &mut Cells) {
        let n = round(self.split_w);
        if n <= 0.0 {
            return;
        }
        let Some(mut t) = self.twin_site.take() else {
            return;
        };
        Scene::grid(t.as_mut());
        // The part of the twin's view shown is centred on its booster, so it's in
        // sight from the panel's first sliver, the view widening round it.
        let (fc, fr) = t.focus();
        let src = &t.world.out;
        let (w, h) = (self.world.columns, self.world.rows);
        let (wf, hf) = (w as f64, h as f64);
        let rule =
            |out: &mut Cells, i: f64, glyph: u32| out.set(i as usize, glyph, 0x8a9099, 0x14161a);
        if self.split_tall {
            let from = 0.0_f64.max((self.twin_h - n).min(round(fr - n / 2.0)));
            let mut r = 0.0;
            while r < n && r < hf {
                for x in 0..w {
                    let j = ((from + r) * self.twin_w) as usize + x;
                    let i = ((hf - n + r) * wf) as usize + x;
                    out.set(i, src.code_point(j), src.foreground(j), src.background(j));
                }
                r += 1.0;
            }
            if n < hf {
                for x in 0..w {
                    rule(out, (hf - n - 1.0) * wf + x as f64, HORIZ);
                }
            }
        } else {
            let from = 0.0_f64.max((self.twin_w - n).min(round(fc - n / 2.0)));
            for r in 0..h {
                let rf = r as f64;
                let mut x = 0.0;
                while x < n && x < wf {
                    let j = (rf * self.twin_w + from + x) as usize;
                    out.set(
                        (rf * wf + x) as usize,
                        src.code_point(j),
                        src.foreground(j),
                        src.background(j),
                    );
                    x += 1.0;
                }
                if n < wf {
                    rule(out, rf * wf + n, VERT);
                }
            }
        }
        self.twin_site = Some(t);
    }

    /// The clouds at column x, world row yf, folded into the layer's tiles
    /// above anchor a: the top and bottom colours, or `None` for open sky.
    fn folded_cloud(&self, x: f64, yf: f64, a: f64, sky: u32) -> Option<(u32, u32)> {
        let u = yf - a;
        let k = if u < 0.0 { -1.0 } else { (u / TILE).floor() };
        let kk = k.max(0.0);
        let cy = yf - kk * TILE;
        let xs = (kk + self.wraps) * TILE_X;
        let first = self.cloud(x + xs, cy, sky);
        if k < 0.0 {
            return first;
        }
        // Near a seam, crossfade with the neighbouring tile so no edge shows.
        let f = u - k * TILE;
        let (cy2, xs2, wgt) = if k >= 1.0 && f < FADE {
            (cy + TILE, xs - TILE_X, 0.5 + (0.5 * f) / FADE)
        } else if f > TILE - FADE {
            (cy - TILE, xs + TILE_X, 0.5 + (0.5 * (TILE - f)) / FADE)
        } else {
            return first;
        };
        let second = self.cloud(x + xs2, cy2, sky);
        if first.is_none() && second.is_none() {
            return None;
        }
        let (fg1, bg1) = first.unwrap_or((sky, sky));
        let (fg2, bg2) = second.unwrap_or((sky, sky));
        Some((mix(fg2, fg1, wgt), mix(bg2, bg1, wgt)))
    }

    /// A cloud sample from the layered painter: its top and bottom half, lit for the hour.
    fn cloud(&self, x: f64, y: f64, sky: u32) -> Option<(u32, u32)> {
        if !(CLOUD_LO..=CLOUD_HI).contains(&y) {
            return None;
        }
        let c = layered(x / PAINT, y / PAINT, sky)?;
        Some((
            self.world.cloud_light(c.fg, sky),
            self.world.cloud_light(c.bg, sky),
        ))
    }
}

impl SkyScene for LaunchSite {
    fn world(&self) -> &SkyWorld {
        &self.world
    }

    fn world_mut(&mut self) -> &mut SkyWorld {
        &mut self.world
    }

    fn vehicle_height(&self) -> f64 {
        (self.spec().fly.hf() / 2.0).ceil()
    }

    fn draw_vehicle(&mut self, out: &mut Cells, _top: f64) {
        self.draw_site(out);
    }

    fn advance(&mut self) {
        self.advance_flight();
    }

    /// The world scrolls just enough to keep the rocket's nose on the grid.
    fn scroll(&self) -> f64 {
        self.scroll_exact().ceil().max(0.0)
    }

    fn ground_feature(&self, x: f64) -> Option<SceneryCell> {
        // Clear ground around the launch site (and its landing zone): no trees or houses near the pad.
        if x >= self.pad_l.min(self.lz_l()) - 3.0 && x <= self.pad_r + 3.0 {
            return None;
        }
        // Nothing stands on the sea.
        if x < self.shore() {
            return None;
        }
        self.world.base_ground_feature(x)
    }

    fn scenery(&self, x: f64, y: f64, sky: u32) -> Option<SceneryCell> {
        if y == -1.0 {
            let s = self.spec();
            if x >= self.pad_l && x <= self.pad_r && !self.site_hidden() {
                // Concrete, with the flame trench under the engines.
                let mid = f64::from(i32_of(self.body_px + s.body_w / 2.0) >> 1);
                return Some(if x == mid || (s.body_w > 2.0 && x == mid - 1.0) {
                    TRENCH
                } else {
                    PAD
                });
            }
            if x < self.shore() {
                let dark = self.world.dark();
                return Some(SceneryCell {
                    glyph: g('▀'),
                    fg: mix(SEA_FG, 0x10243c, dark),
                    bg: Some(mix(SEA_BG, 0x0a1628, dark)),
                });
            }
            if !self.look().catches
                && self.lz != 0.0
                && x >= self.lz_l()
                && x <= self.lz_l() + (s.land.wf() / 2.0).ceil()
            {
                return Some(PAD);
            }
        }
        base_scenery(self, x, y, sky)
    }
}

impl Scene for LaunchSite {
    fn dials(&mut self) -> &mut Dials {
        &mut self.world.dials
    }

    fn ensure(&mut self, columns: usize, rows: usize) {
        self.world.ensure(columns, rows);
        if self.pw == columns * 2 && self.ph == rows * 2 {
            return;
        }
        // A new size (another layout, a resized pane) mid-split: the panel was
        // laid out for the old one, so it closes; the booster is simply home.
        if self.twin_site.is_some() || self.twin_due {
            self.twin_site = None;
            self.twin_due = false;
            self.split_w = 0.0;
            self.booster_home = true;
        }
        self.pw = columns * 2;
        self.ph = rows * 2;
        self.pc = vec![0; self.pw * self.ph];
        self.pa = vec![0.0; self.pw * self.ph];
        self.touched = vec![0; columns * rows];
        self.list = vec![0; columns * rows];
        self.n_touched = 0;
    }

    /// Step the acted-out level toward the asked one (`strength`), then fly it.
    fn step(&mut self) {
        let asked = clamp(round(self.world.dials.strength), 0.0, 10.0);
        self.since_stage += 1.0;
        if asked == 0.0 || self.staged < 0.0 {
            // Off is instant; a fresh start (a reload, the first frame) resumes as asked.
            self.staged = asked;
            self.descending = false;
        } else if self.since_stage >= STAGE_FRAMES {
            // A mission only climbs: a dip in the work holds it where it is. Asked
            // all the way back to 1 it comes home, a level a second, all the way,
            // whatever's asked meanwhile.
            if self.descending || (asked <= 1.0 && self.staged > 1.0) {
                self.staged -= 1.0;
                self.since_stage = 0.0;
                self.descending = self.staged > 1.0;
            } else if asked > self.staged {
                self.staged += 1.0;
                self.since_stage = 0.0;
            }
        }
        self.world.dials.strength = self.staged;
        self.world.tick();
        self.advance_flight();
    }

    /// Drawn at the acted-out level too, even on a frame drawn without a step.
    fn grid(&mut self) -> &Cells {
        if self.staged >= 0.0 && self.world.dials.strength > 0.0 {
            self.world.dials.strength = self.staged;
        }
        self.draw_frame();
        &self.world.out
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn falcon_frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/falcon.json"), 0.0);
    }

    #[test]
    fn starship_frames_match_flow() {
        crate::flow::reference::check(include_str!("../testdata/starship.json"), 0.0);
    }
}
