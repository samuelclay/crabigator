//! The flow column: an ambient scene at the right of the status widgets that
//! moves with the session's work.
//!
//! The scenes are a Rust port of flow (github.com/robdmac/flow), a Claude
//! Code plugin by Rob Macrae; `src/flow/FORK.md` records what was ported
//! from which upstream commit, so upstream changes can be carried over.
//!
//! Each session claims its own scene (`claim`), so sessions side by side
//! differ, and the scene takes the colour of the session's mark. The scene
//! fades: every change of state relights it, and it eases back down over two
//! minutes, so a bright scene has just changed and embers have sat a while.
//! Work (tool calls, compactions) flares it on top.

mod activity;
mod ansi;
mod cells;
mod claim;
mod clouds;
mod feed;
mod js;
mod night;
mod palette;
mod pixels;
#[cfg(test)]
mod reference;
mod scene;
mod scenes;
mod sky;

use std::time::{Duration, Instant};

use chrono::Timelike;

use crate::platforms::{PlatformStats, SessionState};
use crate::session_mark::SessionMark;
use crate::ui::FlowRect;

use activity::{Activity, FRAME_MS};
use cells::Cells;
use feed::{ActivityFeed, Heard};
use palette::Palette;
use scene::{Scene, SceneDef, Tint};

/// Frame pace while busy (~14 fps), and when calm (8 fps).
const BUSY: Duration = Duration::from_millis(70);
const CALM: Duration = Duration::from_millis(125);
/// The idle floor: a low glow, never dark (the column is always there).
const IDLE_GLOW: f64 = 1.0;
/// By the clock, night runs from this hour until the next.
const NIGHT_FROM: u32 = 19;
const DAY_FROM: u32 = 7;
/// crabigator says a tool finished, not how much it wrote: a typical edit's worth.
const EDIT_LINES: f64 = 12.0;

/// What a tick changed, for the event loop.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FlowChange {
    /// A new frame to paint.
    pub new_frame: bool,
    /// The column appeared (its first frame): the layout changes.
    pub column_toggled: bool,
}

/// The session's scene, stepped in place, and where it is drawn.
pub struct FlowColumn {
    def: &'static SceneDef,
    scene: Box<dyn Scene>,
    palette: Palette,
    turned: Cells,
    activity: Activity,
    feed: ActivityFeed,
    /// Where the column would draw for the current layout (its size).
    geometry: Option<FlowRect>,
    /// Where the screen shows the column now.
    drawn: Option<FlowRect>,
    frame: Option<Vec<String>>,
    last_tick: Instant,
    /// The session's state as last seen (`None` before the first).
    state: Option<SessionState>,
}

impl FlowColumn {
    /// Claim this session's scene (published at once with its mark), seeded
    /// by the session, in the colour of its mark.
    pub fn new(session_id: &str, mark: SessionMark) -> Self {
        let name = claim::assign_scene(session_id, mark);
        let def = scene::scene_def(&name).unwrap_or(&scene::scenes()[0]);
        let target = claim::mark_accent(mark).and_then(|[r, g, b]| {
            palette::hue_of(&[(u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)])
        });
        Self {
            def,
            scene: (def.make)(f64::from(claim::scene_seed(session_id))),
            palette: Palette::toward(def.hue, target),
            turned: Cells::new(0, 0),
            activity: Activity::default(),
            feed: ActivityFeed::default(),
            geometry: None,
            drawn: None,
            frame: None,
            last_tick: Instant::now(),
            state: None,
        }
    }

    /// The scene this session shows.
    pub fn scene(&self) -> &'static str {
        self.def.name
    }

    /// How long until the next frame: calm scenes step slower.
    pub fn pace(&mut self) -> Duration {
        let dials = *self.scene.dials();
        if dials.strength <= 1.0 && dials.tint == Tint::Normal {
            CALM
        } else {
            BUSY
        }
    }

    /// Advance the activity by the time since the last tick, and step and
    /// draw the scene when the column has room.
    pub fn tick(&mut self) -> FlowChange {
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_tick).as_secs_f64() * 1000.0;
        self.last_tick = now;
        self.activity.tick(elapsed / FRAME_MS);
        let Some(rect) = self.geometry else {
            return FlowChange::default();
        };
        let night = is_night(chrono::Local::now().hour());
        let dials = self.scene.dials();
        dials.strength = self.activity.strength_faded(IDLE_GLOW);
        dials.coverage_boost = self.activity.coverage_boost();
        dials.tint = self.activity.tint();
        dials.night = night;
        let tint = dials.tint;
        self.scene
            .ensure(usize::from(rect.width), usize::from(rect.rows));
        self.scene.step();
        let grid = self.scene.grid();
        // The context-full tint must read as blue: no palette then.
        let rows = if self.palette.is_natural() || tint == Tint::Blue {
            ansi::grid_to_ansi(grid)
        } else {
            self.palette.apply(grid, &mut self.turned);
            ansi::grid_to_ansi(&self.turned)
        };
        if self.frame.as_ref() == Some(&rows) {
            return FlowChange::default();
        }
        let column_toggled = self.frame.is_none();
        self.frame = Some(rows);
        FlowChange {
            new_frame: true,
            column_toggled,
        }
    }

    /// Whether the column has a frame to show (so it takes its room).
    pub fn has_frame(&self) -> bool {
        self.frame.is_some()
    }

    /// The latest frame's rows.
    pub fn frame_rows(&self) -> Option<&[String]> {
        self.frame.as_deref()
    }

    /// Where the column would draw for the current layout (`None`: no room).
    pub fn set_geometry(&mut self, rect: Option<FlowRect>) {
        self.geometry = rect;
    }

    /// Where the last full draw put the column on screen (`None`: not shown).
    pub fn set_drawn(&mut self, rect: Option<FlowRect>) {
        self.drawn = rect;
    }

    pub fn drawn_rect(&self) -> Option<FlowRect> {
        self.drawn
    }

    /// What changed in a stats refresh: tool calls finished, compactions.
    pub fn observe_stats(&mut self, stats: &PlatformStats) {
        for heard in self.feed.observe(stats) {
            match heard {
                Heard::Tool(name) => hear_tool(&mut self.activity, &name),
                Heard::Compact => {
                    self.activity.compacted();
                    self.activity.changed();
                }
            }
        }
    }

    /// The session's state: each change relights the scene. The first state
    /// seen is where the session starts, not a change: a session that just
    /// opened sits at its base like any other idle one.
    pub fn sync_state(&mut self, state: SessionState) {
        let previous = self.state.replace(state);
        if previous == Some(state) {
            return;
        }
        if state == SessionState::Thinking {
            self.activity.turn_started();
        } else {
            self.activity.turn_ended();
        }
        if previous.is_some() {
            self.activity.changed();
        }
    }
}

/// A finished tool call, by what it does, as each agent names it.
fn hear_tool(activity: &mut Activity, name: &str) {
    match name.to_ascii_lowercase().as_str() {
        "bash" | "exec_command" | "shell" | "local_shell" | "run_terminal_cmd" | "powershell" => {
            activity.ran_command(false)
        }
        "edit" | "write" | "multiedit" | "apply_patch" | "notebookedit" | "patch"
        | "str_replace_editor" | "create" => activity.edited(EDIT_LINES, false),
        "task" | "agent" => activity.spawned_agent(),
        _ => activity.read(false),
    }
}

/// Night by the local clock: 19:00 to 7:00.
fn is_night(hour: u32) -> bool {
    !(DAY_FROM..NIGHT_FROM).contains(&hour)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn column(scene: &str) -> FlowColumn {
        let def = scene::scene_def(scene).unwrap();
        FlowColumn {
            def,
            scene: (def.make)(7.0),
            palette: Palette::natural(),
            turned: Cells::new(0, 0),
            activity: Activity::default(),
            feed: ActivityFeed::default(),
            geometry: None,
            drawn: None,
            frame: None,
            last_tick: Instant::now(),
            state: None,
        }
    }

    fn rect(width: u16, rows: u16) -> FlowRect {
        FlowRect {
            widget_pty_rows: 30,
            col: 100,
            width,
            rows,
        }
    }

    #[test]
    fn frames_come_once_there_is_room_and_rows_fit_the_column() {
        let mut flow = column("fire");
        assert_eq!(flow.tick(), FlowChange::default());
        flow.set_geometry(Some(rect(18, 6)));
        flow.sync_state(SessionState::Thinking);
        let first = flow.tick();
        assert!(first.new_frame && first.column_toggled);
        let rows = flow.frame_rows().unwrap();
        assert_eq!(rows.len(), 6);
        for row in rows {
            assert_eq!(crate::ui::utils::strip_ansi_len(row), 18);
        }
    }

    #[test]
    fn a_session_opens_at_its_base_and_each_change_relights_it() {
        let mut flow = column("fire");
        // Opening (the first state seen) isn't a change: embers.
        flow.sync_state(SessionState::Ready);
        assert_eq!(flow.activity.strength_faded(IDLE_GLOW), IDLE_GLOW);
        flow.sync_state(SessionState::Thinking);
        assert_eq!(flow.activity.strength_faded(IDLE_GLOW), 10.0);
        flow.activity.since_change_ms = activity::FADE_MS;
        flow.sync_state(SessionState::Thinking); // no change
        assert!(flow.activity.strength_faded(IDLE_GLOW) < 10.0);
        flow.sync_state(SessionState::Complete);
        assert_eq!(flow.activity.strength_faded(IDLE_GLOW), 10.0);
    }

    #[test]
    fn tools_flare_the_scene_by_what_they_do() {
        let mut activity = Activity::default();
        hear_tool(&mut activity, "apply_patch");
        let edit = activity.heat;
        assert!(edit > 1.0);
        let mut activity = Activity::default();
        hear_tool(&mut activity, "Bash");
        assert_eq!(activity.heat, 1.0);
        let mut activity = Activity::default();
        hear_tool(&mut activity, "Grep");
        assert_eq!(activity.heat, 0.5);
    }

    #[test]
    fn night_follows_the_clock() {
        assert!(is_night(23) && is_night(3) && is_night(19));
        assert!(!is_night(7) && !is_night(12) && !is_night(18));
    }
}
