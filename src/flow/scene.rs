//! What every scene is driven by, whatever it draws (flow's `Scene`
//! interface in `hooks/styles.ts`), and the list of scenes.

use super::cells::Cells;
use super::palette::SceneHue;

/// What shows over a scene: smoke after a failure or a compaction, blue
/// when the context is nearly full.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Tint {
    #[default]
    Normal,
    Smoke,
    Blue,
}

/// The dials a scene reads each frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Dials {
    /// 0 draws nothing; 1 is calm idle; 10 is the busiest.
    pub strength: f64,
    /// Above 0 while subagents run: more company in the scene.
    pub coverage_boost: f64,
    pub tint: Tint,
    /// Night, for the scenes that have one.
    pub night: bool,
}

impl Default for Dials {
    fn default() -> Self {
        Self {
            strength: 8.0,
            coverage_boost: 0.0,
            tint: Tint::Normal,
            night: false,
        }
    }
}

/// A scene: stepped a frame at a time, drawn into its own cell grid.
pub trait Scene {
    fn dials(&mut self) -> &mut Dials;
    fn ensure(&mut self, columns: usize, rows: usize);
    fn step(&mut self);
    /// The current frame's cells.
    fn grid(&mut self) -> &Cells;
}

/// A scene's entry in the list: its name, its main hue (for palettes), and
/// how to build one from a seed. (Every scene is told whether it's night; the
/// ones without a night ignore it.)
pub struct SceneDef {
    pub name: &'static str,
    pub hue: SceneHue,
    pub make: fn(f64) -> Box<dyn Scene>,
}

/// Every scene, in flow's `/flow next` order.
pub fn scenes() -> &'static [SceneDef] {
    &SCENES
}

static SCENES: [SceneDef; 10] = [
    super::scenes::fire::DEF,
    super::scenes::warp::DEF,
    super::scenes::avalon::DEF,
    super::scenes::balloon::DEF,
    super::scenes::engine::DEF,
    super::scenes::rocket::FALCON,
    super::scenes::rocket::STARSHIP,
    super::scenes::surf::DEF,
    super::scenes::ski::DEF,
    super::scenes::bubbles::DEF,
];

/// A scene by name.
pub fn scene_def(name: &str) -> Option<&'static SceneDef> {
    SCENES.iter().find(|def| def.name == name)
}
