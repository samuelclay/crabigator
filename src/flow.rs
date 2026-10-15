//! The flow column: an ambient scene at the right of the status widgets that
//! moves with the session's work.
//!
//! The scenes are a Rust port of flow (github.com/robdmac/flow), a Claude
//! Code plugin by Rob Macrae; `src/flow/FORK.md` records what was ported
//! from which upstream commit, so upstream changes can be carried over.
//!
//! Random claims a scene per session (`claim`), so sessions side by side
//! differ. A named choice is saved for future sessions. The scene takes
//! the colour of the session's mark. The scene
//! follows the work as flow's plugin does: work (tool calls, compactions)
//! flares it, a turn climbs with its effort and length, and subagents add to
//! it. With `[flow] fade = true`, every change of state also relights it, and
//! it eases back down over two minutes, so a bright scene has just changed
//! and embers have sat a while. Claude Code's hooks tell
//! the work as it happens (`hooks`); other assistants' stats tell it a
//! refresh later (`feed`).
//!
//! ctrl+] cycles enabled scenes, Settings and Off. The choice is
//! saved in `[flow]`, so new sessions start the same way; the
//! separator keeps a label naming the key that turns it back on.

mod activity;
mod agents;
mod ansi;
mod cells;
mod claim;
mod clouds;
mod crew;
mod feed;
mod hooks;
mod js;
mod night;
mod palette;
mod picker;
mod pixel_scene;
mod pixels;
#[cfg(test)]
mod preview;
#[cfg(test)]
mod reference;
mod scene;
mod scenes;
mod sky;
mod waiting;

use std::path::PathBuf;
use std::time::{Duration, Instant};

use chrono::Timelike;

use crate::config::FlowPreferences;
use crate::platforms::{PlatformStats, SessionState};
use crate::session_mark::SessionMark;
use crate::ui::{FlowRect, FlowView};

use activity::{Activity, FRAME_MS};
use cells::Cells;
use feed::{ActivityFeed, Heard};
use hooks::HookFeed;
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
/// While Claude Code's hooks report streamed text this recently, its output
/// bytes aren't counted again (they stand in for thinking, which no hook shows).
const STREAM_HEARD: Duration = Duration::from_millis(1500);

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
    /// The scene this session uses when Random is selected.
    random_scene: &'static SceneDef,
    session_id: Option<String>,
    random: bool,
    /// Turned off in the picker or config: only the shortcut label remains.
    off: bool,
    scene: Box<dyn Scene>,
    palette: Palette,
    turned: Cells,
    activity: Activity,
    /// Each change of state relights the scene (`[flow] fade`).
    preferences: FlowPreferences,
    picker: Option<picker::Picker>,
    feed: ActivityFeed,
    /// Claude Code's activity log (`None`: other assistants).
    hooks: Option<HookFeed>,
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
    /// by the session, in the colour of its mark. `hook_log` is the activity
    /// log Claude Code's hooks write, for a session that has one.
    pub fn new(
        session_id: &str,
        mark: SessionMark,
        hook_log: Option<PathBuf>,
        preferences: &FlowPreferences,
    ) -> Self {
        let name = claim::assign_scene(session_id, mark, &preferences.disabled_scenes);
        let random_scene = name
            .as_deref()
            .and_then(scene::scene_def)
            .unwrap_or(&scene::scenes()[0]);
        let chosen = scene::scene_def(&preferences.scene).filter(|def| {
            !preferences
                .disabled_scenes
                .iter()
                .any(|name| name == def.name)
        });
        let def = chosen.unwrap_or(random_scene);
        if chosen.is_some() {
            crate::session_mark::publish_stub(session_id, mark, Some(def.name));
        }
        let target = claim::mark_accent(mark).and_then(|[r, g, b]| {
            palette::hue_of(&[(u32::from(r) << 16) | (u32::from(g) << 8) | u32::from(b)])
        });
        let seed = f64::from(claim::scene_seed(session_id));
        let mut column = Self {
            def,
            random_scene,
            session_id: Some(session_id.to_string()),
            random: chosen.is_none(),
            off: !preferences.enabled || name.is_none(),
            scene: (def.make)(seed),
            palette: palette_for(def, target),
            turned: Cells::new(0, 0),
            activity: Activity::default(),
            preferences: preferences.clone(),
            picker: None,
            feed: ActivityFeed::default(),
            hooks: hook_log.map(HookFeed::open),
            geometry: None,
            drawn: None,
            frame: None,
            last_tick: Instant::now(),
            state: None,
            seed,
            target,
        };
        column.palette = palette_for(def, column.color_target());
        column
    }

    /// The event loop still ticks while editing an off column.
    pub fn should_tick(&self) -> bool {
        !self.off || self.picker.is_some()
    }

    pub fn picker_open(&self) -> bool {
        self.picker.is_some()
    }

    pub fn open_picker(&mut self) {
        self.picker = Some(picker::Picker::new(self.preferences()));
        self.tick();
    }

    /// Next shortcut choice: Random, enabled scenes, Settings, Off.
    /// Settings is transient; scene/off changes are applied only after saving.
    pub fn cycle_preferences(&mut self) -> Option<FlowPreferences> {
        let mut prefs = self.preferences();
        let allowed: Vec<_> = scene::scenes()
            .iter()
            .filter(|def| !prefs.disabled_scenes.iter().any(|name| name == def.name))
            .collect();
        if self.picker_open() {
            prefs.enabled = false;
        } else if allowed.is_empty() {
            self.open_picker();
            return None;
        } else if self.off {
            prefs.enabled = true;
            prefs.scene = "random".into();
        } else if self.random {
            prefs.scene = allowed[0].name.into();
        } else if let Some(next) = allowed
            .iter()
            .position(|def| def.name == self.def.name)
            .and_then(|at| allowed.get(at + 1))
        {
            prefs.scene = next.name.into();
        } else {
            self.open_picker();
            return None;
        }
        Some(prefs)
    }

    pub fn picker_key(&mut self, key: crossterm::event::KeyEvent) -> Option<FlowPreferences> {
        let action = self.picker.as_mut()?.key(key);
        match action {
            picker::Action::Save(prefs) => return Some(prefs),
            picker::Action::Cancel => self.close_picker(),
            picker::Action::Edit => {
                self.tick();
            }
        }
        None
    }

    pub fn picker_save_failed(&mut self) {
        if let Some(picker) = &mut self.picker {
            picker.error = true;
        }
        self.tick();
    }

    pub fn close_picker(&mut self) {
        self.picker = None;
        self.frame = None;
        if self.off {
            self.drawn = None;
        } else {
            self.tick();
        }
    }

    /// Canonical settings are persisted and applied together.
    pub fn normalize_preferences(mut prefs: FlowPreferences) -> FlowPreferences {
        prefs.level = prefs.level.map(|n| n.min(10));
        let allowed = |def: &SceneDef| !prefs.disabled_scenes.iter().any(|name| name == def.name);
        if !scene::scenes().iter().any(allowed) {
            prefs.enabled = false;
        }
        if scene::scene_def(&prefs.scene).is_some_and(|def| !allowed(def)) {
            prefs.scene = "random".into();
        }
        prefs
    }

    pub fn apply_preferences(&mut self, prefs: FlowPreferences) {
        let prefs = Self::normalize_preferences(prefs);
        let allowed = |def: &SceneDef| !prefs.disabled_scenes.iter().any(|name| name == def.name);
        if !allowed(self.random_scene) {
            let name = match &self.session_id {
                Some(id) => claim::reclaim_scene(id, &prefs.disabled_scenes),
                None => claim::claim_scene(&self.seed.to_string(), &[], &prefs.disabled_scenes),
            };
            if let Some(def) = name.as_deref().and_then(scene::scene_def) {
                self.random_scene = def;
            }
        }

        if self.off && prefs.enabled {
            if let Some(hooks) = &mut self.hooks {
                hooks.skip_ahead();
            }
        }
        self.off = !prefs.enabled;
        self.random = scene::scene_def(&prefs.scene).is_none();
        let def = scene::scene_def(&prefs.scene).unwrap_or(self.random_scene);
        self.preferences = prefs;
        self.picker = None;
        self.frame = None;
        self.palette = palette_for(def, self.color_target());
        if self.def.name != def.name {
            self.show(def);
        } else {
            self.tick();
        }
        if self.off {
            self.drawn = None;
        }
    }

    pub fn preferences(&self) -> FlowPreferences {
        FlowPreferences {
            scene: self.selection().into(),
            enabled: !self.off,
            ..self.preferences.clone()
        }
    }

    fn color_target(&self) -> Option<f64> {
        match self.preferences.color.as_str() {
            "natural" => None,
            "session" => self.target,
            name => picker::COLORS
                .iter()
                .find(|(n, _)| *n == name)
                .map_or(self.target, |(_, hue)| *hue),
        }
    }

    /// The scene this session shows (`None`: off).
    pub fn scene(&self) -> Option<&'static str> {
        self.is_on().then_some(self.def.name)
    }

    /// Whether the column is on: only then is it stepped.
    pub fn is_on(&self) -> bool {
        !self.off
    }

    /// The saved choice and displayed label, distinct from Random's actual scene.
    pub fn selection(&self) -> &'static str {
        if self.random {
            "random"
        } else {
            self.def.name
        }
    }

    /// Show `def` in the session's colour, stepped once so it shows at once.
    fn show(&mut self, def: &'static SceneDef) {
        self.def = def;
        self.scene = (def.make)(self.seed);
        self.palette = palette_for(def, self.color_target());
        self.tick();
    }

    /// How long until the next frame: calm scenes step slower.
    pub fn pace(&mut self) -> Duration {
        let dials = self.scene.dials();
        if dials.strength <= 1.0 && dials.tint == Tint::Normal {
            CALM
        } else {
            BUSY
        }
    }

    /// Hear what the hooks said since the last tick, advance the activity by
    /// the time since then, and step and draw the scene when the column has room.
    pub fn tick(&mut self) -> FlowChange {
        let now = Instant::now();
        if let Some(hooks) = self.hooks.as_mut() {
            hooks.poll(&mut self.activity, now);
        }
        let elapsed = now.duration_since(self.last_tick).as_secs_f64() * 1000.0;
        self.last_tick = now;
        self.activity.tick(elapsed / FRAME_MS);
        let Some(rect) = self.geometry.filter(|_| !self.off || self.picker.is_some()) else {
            return FlowChange::default();
        };
        if let Some(picker) = &self.picker {
            let rows = picker.rows(usize::from(rect.width), usize::from(rect.rows));
            let changed = self.frame.as_ref() != Some(&rows);
            let column_toggled = self.frame.is_none();
            self.frame = Some(rows);
            return FlowChange {
                new_frame: changed,
                column_toggled,
            };
        }
        let night = self
            .preferences
            .night
            .unwrap_or_else(|| is_night(chrono::Local::now().hour()));
        let level = self.level();
        let waiting = self.preferences.level.is_none() && self.activity.waiting();
        let target = self.color_target();
        let dials = self.scene.dials();
        dials.strength = level;
        dials.waiting = waiting;
        dials.agents.clone_from(&self.activity.agents);
        dials.coverage_boost = self.activity.coverage_boost();
        dials.tint = if self.preferences.level.is_none() {
            self.activity.tint()
        } else {
            Tint::Normal
        };
        dials.night = night;
        dials.accent = if self.def.figure { target } else { None };
        let tint = dials.tint;
        self.scene
            .ensure(usize::from(rect.width), usize::from(rect.rows));
        self.scene.step();
        let grid = self.scene.grid();
        // The context-full tint must read as blue: no palette then.
        let rows = if self.palette.is_natural() || tint == Tint::Blue || waiting {
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

    /// The scene's level: what the work calls for (flow's `strength`), or
    /// with fade, also lit by how recently the state changed.
    fn level(&self) -> f64 {
        if let Some(level) = self.preferences.level {
            return f64::from(level.min(10));
        }
        if self.preferences.fade {
            self.activity.strength_faded(IDLE_GLOW)
        } else {
            self.activity.strength(IDLE_GLOW)
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

    /// What the status bar shows: the frame and its scene; off, the label
    /// that tells how to turn it on, where the column would have room.
    pub fn view(&self) -> Option<FlowView<'_>> {
        if self.picker.is_some() {
            return Some(FlowView::Scene {
                rows: self.frame.as_deref()?,
                scene: "settings",
                random: false,
            });
        }
        if self.off {
            return self.geometry.map(|_| FlowView::Off);
        }
        Some(FlowView::Scene {
            rows: self.frame.as_deref()?,
            scene: self.def.name,
            random: self.random,
        })
    }

    /// Whether ctrl+] is the column's: while the column or its off label shows.
    pub fn takes_key(&self) -> bool {
        if self.off {
            self.geometry.is_some()
        } else {
            self.drawn.is_some()
        }
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

    /// The assistant wrote `bytes` to the terminal: while a turn runs, that
    /// stands in for its stream, unless the hooks just told the real text.
    pub fn hear_output(&mut self, bytes: usize) {
        let text_heard = self
            .hooks
            .as_ref()
            .and_then(|hooks| hooks.streamed_at)
            .is_some_and(|at| at.elapsed() < STREAM_HEARD);
        if self.activity.is_turn_active && !text_heard {
            self.activity
                .streamed(bytes as f64 / OUTPUT_PER_CHAR, false);
        }
    }

    /// What the assistant's screen says (escape codes stripped): Claude Code's
    /// effort in its banner ("with xhigh effort"), which sets a turn's floor
    /// until the hooks tell it (the banner never changes; `/effort` does).
    /// (Not its footer's "← 1 agent": that counts agents to switch to, idle
    /// ones too, and would keep the scene from ever dying down.)
    pub fn read_screen(&mut self, text: &str) {
        if self.hooks.as_ref().is_some_and(|hooks| hooks.knows_effort) {
            return;
        }
        if let Some(effort) = effort_on_screen(text) {
            self.activity.floor = effort_floor(effort);
        }
    }

    /// What changed in a stats refresh: tool calls finished, compactions.
    /// Once Claude Code's hooks speak, they tell these sooner, and the stats
    /// go unheard.
    pub fn observe_stats(&mut self, stats: &PlatformStats) {
        if self.hooks.as_ref().is_some_and(|hooks| hooks.live) {
            return;
        }
        use crate::platforms::PlatformAgentState;
        self.activity.agents = stats
            .flow_agents
            .iter()
            .map(|agent| agents::AgentDial {
                id: agent.id.clone(),
                state: match agent.state {
                    PlatformAgentState::Working => agents::AgentState::Working,
                    PlatformAgentState::Idle => agents::AgentState::Idle,
                    PlatformAgentState::Waiting => agents::AgentState::Waiting,
                    PlatformAgentState::Done => agents::AgentState::Done,
                },
                ok: agent.ok,
                task: agent.task.clone(),
                kind: agent.kind.clone(),
                ms: agent.ms,
            })
            .collect();
        self.activity.running_agents = stats
            .flow_agents
            .iter()
            .filter(|a| a.state != PlatformAgentState::Done)
            .count() as u32;
        let waiting = stats
            .flow_agents
            .iter()
            .any(|a| a.state == PlatformAgentState::Waiting);
        if self.activity.agent_waiting != waiting {
            self.activity.changed();
        }
        self.activity.agent_waiting = waiting;
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
        self.activity.screen_waiting =
            matches!(state, SessionState::Permission | SessionState::Question);
        if state == SessionState::Thinking {
            self.activity.turn_started();
        } else if !self.activity.screen_waiting {
            self.activity.turn_ended();
            if let Some(hooks) = self.hooks.as_mut() {
                hooks.turn_ended();
            }
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
            random_scene: def,
            session_id: None,
            random: false,
            off: false,
            scene: (def.make)(7.0),
            palette: Palette::natural(),
            turned: Cells::new(0, 0),
            activity: Activity::default(),
            preferences: FlowPreferences {
                fade: true,
                ..FlowPreferences::default()
            },
            picker: None,
            feed: ActivityFeed::default(),
            hooks: None,
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
    fn a_wait_holds_the_turn_clock_and_manual_settings_override_activity() {
        let mut flow = column("fire");
        flow.set_geometry(Some(rect(24, 8)));
        flow.sync_state(SessionState::Thinking);
        flow.activity.tick(400.0);
        let before = flow.activity.turn_frames;
        flow.sync_state(SessionState::Permission);
        flow.activity.tick(800.0);
        assert_eq!(flow.activity.turn_frames, before);
        assert!(flow.level() <= 2.0);
        flow.sync_state(SessionState::Thinking);
        assert_eq!(flow.activity.turn_frames, before);
        assert!(flow.level() > 2.0);
        flow.sync_state(SessionState::Question);
        flow.apply_preferences(FlowPreferences {
            scene: "fire".into(),
            level: Some(7),
            night: Some(true),
            color: "violet".into(),
            ..FlowPreferences::default()
        });
        assert_eq!(flow.level(), 7.0);
        let dials = flow.scene.dials();
        assert!(dials.night && !dials.waiting);
        assert_eq!(dials.tint, Tint::Normal);
        assert_eq!(flow.color_target(), Some(305.0));
    }

    #[test]
    fn opening_an_off_picker_is_visible_and_cancel_restores_off() {
        let mut flow = column("fire");
        flow.off = true;
        flow.set_geometry(Some(rect(15, 4)));
        flow.open_picker();
        assert!(flow.should_tick() && flow.has_frame() && flow.picker_open());
        assert!(matches!(
            flow.view(),
            Some(FlowView::Scene {
                scene: "settings",
                ..
            })
        ));
        flow.close_picker();
        assert!(!flow.should_tick() && !flow.has_frame());
        assert!(matches!(flow.view(), Some(FlowView::Off)));
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
    fn codex_startup_spinner_does_not_relight_fade_but_a_real_turn_does() {
        use crate::hooks::SessionStats;
        use crate::platforms::PlatformKind;
        let mut flow = column("doom3d");
        let mut stats = SessionStats::new();
        flow.sync_state(stats.effective_state());
        for spinning in [true, true, false, true, false] {
            stats.set_title_spinner(spinning, PlatformKind::Codex);
            assert_eq!(stats.effective_state(), SessionState::Ready);
            flow.sync_state(stats.effective_state());
            flow.hear_output(1000); // startup repainting is not streamed work
            flow.activity.tick(10.0);
            assert_eq!(flow.level(), IDLE_GLOW);
            assert!(!flow.activity.is_turn_active);
        }
        // task_started arrives in the rollout even without another title update.
        stats.platform_stats.state = SessionState::Thinking;
        flow.sync_state(stats.effective_state());
        assert_eq!(flow.level(), 10.0);
        assert!(flow.activity.is_turn_active);
        stats.platform_stats.state = SessionState::Complete;
        flow.sync_state(stats.effective_state());
        assert_eq!(flow.level(), 10.0, "real completion still relights fade");
        assert!(!flow.activity.is_turn_active);
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
    fn without_fade_the_scene_follows_the_work_as_flow_does() {
        let mut flow = column("fire");
        flow.preferences.fade = false;
        flow.sync_state(SessionState::Ready);
        flow.sync_state(SessionState::Thinking);
        // A turn starting sits at its effort floor, not relit to 10.
        assert_eq!(flow.level(), 3.0);
        flow.sync_state(SessionState::Complete);
        assert_eq!(flow.level(), IDLE_GLOW);
        // After the turn, a background subagent running a command: the plain
        // level, as flow's plugin shows it beside the column.
        flow.activity.running_agents = 1;
        flow.activity.tools_in_flight = 1;
        flow.activity.tick(1.0);
        assert_eq!(flow.level(), 3.0);
        flow.preferences.fade = true;
        assert_eq!(flow.level(), 10.0);
    }

    fn advance(flow: &mut FlowColumn) {
        if let Some(prefs) = flow.cycle_preferences() {
            flow.apply_preferences(prefs);
        }
    }

    #[test]
    fn shortcut_visits_all_scenes_settings_off_then_random() {
        let mut flow = column("doom3d");
        flow.set_geometry(Some(rect(15, 4)));
        flow.apply_preferences(FlowPreferences::default());
        for def in scene::scenes() {
            advance(&mut flow);
            assert_eq!(flow.selection(), def.name);
            assert!(!flow.picker_open());
        }
        advance(&mut flow);
        assert!(flow.picker_open());
        advance(&mut flow);
        assert!(!flow.picker_open() && !flow.is_on());
        advance(&mut flow);
        assert_eq!(flow.selection(), "random");
        assert_eq!(flow.scene(), Some("doom3d"));
    }

    #[test]
    fn saved_exclusions_match_the_effective_scene_and_off_state() {
        let prefs = FlowColumn::normalize_preferences(FlowPreferences {
            scene: "doom3d".into(),
            disabled_scenes: vec!["doom3d".into()],
            ..FlowPreferences::default()
        });
        let saved = toml::to_string(&prefs).unwrap();
        let mut restored: FlowPreferences = toml::from_str(&saved).unwrap();
        assert_eq!(restored.scene, "random");
        restored.disabled_scenes = claim::scene_names().to_vec();
        let prefs = FlowColumn::normalize_preferences(restored);
        let restored: FlowPreferences = toml::from_str(&toml::to_string(&prefs).unwrap()).unwrap();
        assert!(!restored.enabled);
    }

    #[test]
    fn exclusions_replace_disabled_choices_and_skip_them_in_the_cycle() {
        let mut flow = column("doom3d");
        flow.set_geometry(Some(rect(15, 4)));
        let disabled_scenes = scene::scenes()
            .iter()
            .filter(|def| def.name != "train")
            .map(|def| def.name.to_string())
            .collect();
        flow.apply_preferences(FlowPreferences {
            scene: "doom3d".into(),
            disabled_scenes,
            ..FlowPreferences::default()
        });
        assert_eq!(flow.selection(), "random");
        assert_eq!(flow.scene(), Some("train"));
        advance(&mut flow);
        assert_eq!(flow.selection(), "train");
        advance(&mut flow);
        assert!(flow.picker_open());
        advance(&mut flow);
        advance(&mut flow);
        assert_eq!(flow.scene(), Some("train"));
        assert_eq!(flow.selection(), "random");
        let mut prefs = flow.preferences();
        prefs.disabled_scenes.push("train".into());
        flow.apply_preferences(prefs);
        assert!(!flow.is_on());
        advance(&mut flow);
        assert!(flow.picker_open(), "all unchecked still reaches settings");
        advance(&mut flow);
        assert!(!flow.is_on());
        let mut prefs = flow.preferences();
        prefs.disabled_scenes.retain(|name| name != "earthrise");
        prefs.enabled = true;
        flow.apply_preferences(prefs);
        assert_eq!(flow.scene(), Some("earthrise"));
    }

    #[test]
    fn settings_select_every_scene_random_and_off() {
        let mut flow = column("avalon");
        flow.set_geometry(Some(rect(18, 6)));
        for def in scene::scenes() {
            flow.apply_preferences(FlowPreferences {
                scene: def.name.into(),
                ..FlowPreferences::default()
            });
            assert_eq!(flow.scene(), Some(def.name));
            assert_eq!(flow.selection(), def.name);
            assert!(flow.has_frame());
        }
        flow.apply_preferences(FlowPreferences {
            enabled: false,
            ..FlowPreferences::default()
        });
        assert!(!flow.has_frame() && flow.drawn_rect().is_none());
        assert!(matches!(flow.view(), Some(FlowView::Off)));
        assert!(flow.takes_key());
        flow.apply_preferences(FlowPreferences::default());
        assert_eq!(flow.scene(), Some("avalon"));
        assert_eq!(flow.selection(), "random");
    }

    #[test]
    fn a_saved_scene_starts_new_sessions_on_that_scene() {
        let mut flow = column("avalon");
        flow.random = true;
        flow.apply_preferences(FlowPreferences {
            scene: "fire".into(),
            ..FlowPreferences::default()
        });
        let preferences = FlowPreferences {
            scene: flow.selection().to_string(),
            ..FlowPreferences::default()
        };
        let saved = toml::to_string(&preferences).unwrap();
        let preferences: FlowPreferences = toml::from_str(&saved).unwrap();
        let dir = tempfile::Builder::new()
            .prefix("crabigator-flow-choice-")
            .tempdir_in("/tmp")
            .unwrap();
        let name = dir.path().file_name().unwrap().to_str().unwrap();
        let session_id = name.strip_prefix("crabigator-").unwrap();
        let mark = SessionMark::from_seed(session_id);
        let started = FlowColumn::new(session_id, mark, None, &preferences);
        assert_eq!(started.scene(), Some("fire"));
        assert_eq!(started.selection(), "fire");
        let mirror: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(dir.path().join("inspect.json")).unwrap(),
        )
        .unwrap();
        assert_eq!(mirror["flow_scene"], "fire");
    }

    #[test]
    fn off_shows_its_label_only_where_the_column_would_have_room() {
        let mut flow = column("fire");
        flow.off = true;
        assert!(flow.view().is_none() && !flow.takes_key());
        flow.set_geometry(Some(rect(18, 6)));
        assert!(matches!(flow.view(), Some(FlowView::Off)));
        assert!(flow.takes_key());
        assert_eq!(flow.scene(), None);
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

    #[test]
    fn new_scene_session_and_explicit_colors_change_frames_and_natural_is_exact() {
        for name in ["train", "earthrise", "doom", "doom3d"] {
            for (width, rows) in [(15, 4), (24, 8), (40, 12)] {
                let render = |color: &str, target: f64| {
                    let mut flow = column(name);
                    flow.target = Some(target);
                    flow.apply_preferences(FlowPreferences {
                        scene: name.into(),
                        color: color.into(),
                        level: Some(6),
                        night: Some(false),
                        fade: false,
                        ..FlowPreferences::default()
                    });
                    flow.set_geometry(Some(rect(width, rows)));
                    for _ in 0..60 {
                        flow.tick();
                    }
                    let raw = flow.scene.grid().clone();
                    (flow.frame.unwrap(), raw)
                };
                let (natural, raw) = render("natural", 305.0);
                let (session, session_raw) = render("session", 305.0);
                let (explicit, explicit_raw) = render("violet", 70.0);
                assert_eq!(natural, ansi::grid_to_ansi(&raw), "natural {name}");
                assert_eq!(raw.words, session_raw.words, "simulation {name}");
                assert_eq!(raw.words, explicit_raw.words, "simulation {name}");
                assert_eq!(session, explicit, "explicit overrides session {name}");
                assert_ne!(natural, session, "visible recoloring {name} {width}x{rows}");
            }
        }
    }

    #[test]
    fn new_scene_palettes_preserve_waiting_frames() {
        for name in ["train", "earthrise", "doom", "doom3d"] {
            let mut flow = column(name);
            flow.target = Some(305.0);
            flow.apply_preferences(FlowPreferences {
                scene: name.into(),
                color: "violet".into(),
                night: Some(false),
                fade: false,
                ..FlowPreferences::default()
            });
            flow.activity.screen_waiting = true;
            flow.set_geometry(Some(rect(24, 8)));
            for _ in 0..90 {
                flow.tick();
            }
            assert!(!flow.palette.is_natural(), "{name} has a selected palette");
            assert!(flow.scene.dials().waiting);
            assert_eq!(
                flow.frame.as_ref().unwrap(),
                &ansi::grid_to_ansi(flow.scene.grid()),
                "{name} keeps the scene's breathing sepia and signal lamps"
            );
        }
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
