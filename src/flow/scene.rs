//! What every scene is driven by, whatever it draws (flow's `Scene`
//! interface in `hooks/styles.ts`), and the list of scenes.

use super::agents::AgentDial;
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
#[derive(Clone, Debug, PartialEq)]
pub struct Dials {
    /// 0 draws nothing; 1 is calm idle; 10 is the busiest.
    pub strength: f64,
    pub waiting: bool,
    pub agents: Vec<AgentDial>,
    /// Above 0 while subagents run: more company in the scene.
    pub coverage_boost: f64,
    pub tint: Tint,
    /// Night, for the scenes that have one.
    pub night: bool,
    /// The session's hue (OKLCH degrees) for a scene's figure to wear (its
    /// balloon, skier, surfer, rocket): crabigator's, not flow's. `None`: the
    /// figure's own colours, as flow draws them.
    pub accent: Option<f64>,
}

impl Default for Dials {
    fn default() -> Self {
        Self {
            strength: 8.0,
            waiting: false,
            agents: Vec::new(),
            coverage_boost: 0.0,
            tint: Tint::Normal,
            night: false,
            accent: None,
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
    /// How the whole frame turns toward the session's hue (palettes).
    pub hue: SceneHue,
    /// The scene has a figure that wears the session's hue instead (`accent`):
    /// its frame keeps its own colours, a sky its blue.
    pub figure: bool,
    pub make: fn(f64) -> Box<dyn Scene>,
}

/// Every scene, in flow's `/flow next` order.
pub fn scenes() -> &'static [SceneDef] {
    &SCENES
}

static SCENES: [SceneDef; 15] = [
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
    super::scenes::train::DEF,
    super::scenes::earthrise::DEF,
    super::scenes::doom::DEF,
    super::scenes::doom::DOOM3D,
    super::scenes::roadster::DEF,
];

/// A scene by name.
pub fn scene_def(name: &str) -> Option<&'static SceneDef> {
    SCENES.iter().find(|def| def.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drawing_does_not_advance_scenes_or_change_later_frames() {
        for def in scenes() {
            for (columns, rows) in [(24, 8), (120, 5), (22, 50)] {
                let mut viewed = (def.make)(7.0);
                let mut skipped = (def.make)(7.0);
                for scene in [&mut viewed, &mut skipped] {
                    scene.ensure(columns, rows);
                    scene.dials().strength = 10.0;
                }
                for frame in 0..80 {
                    viewed.step();
                    skipped.step();
                    let first = viewed.grid().words.clone();
                    assert_eq!(
                        viewed.grid().words,
                        first,
                        "{} {columns}x{rows}, frame {frame}: repeated draw",
                        def.name
                    );
                    if frame % 3 != 0 {
                        assert_eq!(
                            skipped.grid().words,
                            first,
                            "{} {columns}x{rows}, frame {frame}: skipped draw",
                            def.name
                        );
                    }
                }
            }
        }
    }
}
