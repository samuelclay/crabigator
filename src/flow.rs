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
/// The assistant's output bytes per streamed character (escape codes and
/// repaints are most of the bytes), while a turn runs.
const OUTPUT_PER_CHAR: f64 = 4.0;

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
    /// The session's seed, for a new scene's randomness.
    seed: f64,
    /// The hue the session's mark asks for (`None`: natural colours).
    target: Option<f64>,
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
        let seed = f64::from(claim::scene_seed(session_id));
        Self {
            def,
            scene: (def.make)(seed),
            palette: palette_for(def, target),
            turned: Cells::new(0, 0),
            activity: Activity::default(),
            feed: ActivityFeed::default(),
            geometry: None,
            drawn: None,
            frame: None,
            last_tick: Instant::now(),
            state: None,
            seed,
            target,
        }
    }

    /// The scene this session shows.
    pub fn scene(&self) -> &'static str {
        self.def.name
    }

    /// Rotate to the next scene (ctrl+]), in the session's colour, and step
    /// it once so it shows at once. Answers its name.
    pub fn next_scene(&mut self) -> &'static str {
        let all = scene::scenes();
        let at = all
            .iter()
            .position(|def| def.name == self.def.name)
            .unwrap_or(0);
        let def = &all[(at + 1) % all.len()];
        self.def = def;
        self.scene = (def.make)(self.seed);
        self.palette = palette_for(def, self.target);
        self.tick();
        def.name
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
        dials.accent = if self.def.figure { self.target } else { None };
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

    /// The assistant wrote `bytes` to the terminal: while a turn runs, that is
    /// the closest crabigator comes to hearing it stream.
    pub fn hear_output(&mut self, bytes: usize) {
        if self.activity.is_turn_active {
            self.activity.streamed(bytes as f64 / OUTPUT_PER_CHAR);
        }
    }

    /// What the assistant's screen says (escape codes stripped): Claude Code's
    /// effort in its banner ("with xhigh effort"), which sets a turn's floor.
    /// (Not its footer's "← 1 agent": that counts agents to switch to, idle
    /// ones too, and would keep the scene from ever dying down.)
    pub fn read_screen(&mut self, text: &str) {
        if let Some(effort) = effort_on_screen(text) {
            self.activity.floor = effort_floor(effort);
        }
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

/// How a scene takes the session's hue: a figure scene's figure wears it (the
/// frame keeps its own colours); any other scene's whole frame turns toward it.
fn palette_for(def: &SceneDef, target: Option<f64>) -> Palette {
    if def.figure {
        Palette::natural()
    } else {
        Palette::toward(def.hue, target)
    }
}

/// The effort Claude Code's banner names ("… with xhigh effort"), if it shows.
fn effort_on_screen(text: &str) -> Option<&str> {
    let at = text.rfind(" effort")?;
    let before = text[..at].trim_end();
    let word = before.rsplit(' ').next()?;
    let with = before[..before.len() - word.len()].trim_end();
    with.ends_with("with")
        .then_some(word)
        .filter(|w| matches!(*w, "low" | "medium" | "high" | "xhigh" | "max"))
}

/// The effort floor for an effort level (flow's `effortFloor`).
fn effort_floor(effort: &str) -> f64 {
    match effort {
        "low" => 2.0,
        "high" => 4.0,
        "xhigh" => 5.0,
        "max" => 6.0,
        _ => 3.0,
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
            seed: 7.0,
            target: None,
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
    fn the_next_scene_comes_round_to_the_first_again() {
        let mut flow = column("fire");
        flow.set_geometry(Some(rect(18, 6)));
        let mut seen = vec![flow.scene()];
        for _ in 1..scene::scenes().len() {
            seen.push(flow.next_scene());
            assert!(flow.has_frame());
        }
        let names: Vec<&str> = scene::scenes().iter().map(|def| def.name).collect();
        assert_eq!(seen, names);
        assert_eq!(flow.next_scene(), "fire");
    }

    #[test]
    fn a_figure_scene_wears_the_session_hue_and_keeps_its_sky() {
        let mut flow = column("balloon");
        flow.target = Some(305.0);
        flow.palette = palette_for(flow.def, flow.target);
        flow.set_geometry(Some(rect(24, 8)));
        flow.tick();
        assert!(flow.palette.is_natural());
        assert_eq!(flow.scene.dials().accent, Some(305.0));
        let mut fire = column("fire");
        fire.target = Some(305.0);
        assert!(!palette_for(fire.def, fire.target).is_natural());
        fire.set_geometry(Some(rect(24, 8)));
        fire.tick();
        assert_eq!(fire.scene.dials().accent, None);
    }

    /// Every scene in a hue, printed here to look at:
    /// `FLOW_HUE=305 cargo test flow::tests::preview -- --ignored --nocapture`
    /// (FLOW_LEVEL, FLOW_SIZE=30x9 too).
    #[test]
    #[ignore]
    fn preview() {
        let env = |k: &str| std::env::var(k).ok();
        let hue = env("FLOW_HUE")
            .and_then(|h| h.parse().ok())
            .unwrap_or(305.0);
        let level: f64 = env("FLOW_LEVEL")
            .and_then(|l| l.parse().ok())
            .unwrap_or(4.0);
        let (w, h) = env("FLOW_SIZE")
            .and_then(|s| {
                let (w, h) = s.split_once('x')?;
                Some((w.parse().ok()?, h.parse().ok()?))
            })
            .unwrap_or((30u16, 9u16));
        for def in scene::scenes() {
            let mut flow = column(def.name);
            flow.target = Some(hue);
            flow.palette = palette_for(def, flow.target);
            flow.activity.floor = level;
            flow.activity.turn_started();
            flow.set_geometry(Some(rect(w, h)));
            for _ in 0..60 {
                flow.tick();
            }
            println!("{} (hue {hue})", def.name);
            for row in flow.frame_rows().unwrap() {
                println!("{row}");
            }
        }
    }

    #[test]
    fn the_screen_tells_the_effort_but_its_agents_dont_keep_the_scene_up() {
        let banner =
            "Claude Code v2.1.293\nOpus 5.5 (1M context) with xhigh effort · Claude Team\n";
        assert_eq!(effort_on_screen(banner), Some("xhigh"));
        assert_eq!(effort_on_screen("no effort here"), None);
        assert_eq!(effort_on_screen("with great effort"), None);
        let footer = "transcript…\n\n❯ \n  ▸▸ auto mode on (shift+tab to cycle) · ← 1 agent\n";
        let mut flow = column("fire");
        flow.read_screen(&format!("{banner}{footer}"));
        assert_eq!(flow.activity.floor, 5.0);
        // A turn that ended two minutes ago, with an agent in the footer: embers.
        flow.sync_state(SessionState::Thinking);
        flow.sync_state(SessionState::Complete);
        flow.activity.tick(activity::FADE_MS / FRAME_MS + 1.0);
        assert_eq!(flow.activity.strength_faded(IDLE_GLOW), IDLE_GLOW);
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
