//! How the session's work moves the scene (flow's `hooks/activity.ts`, less
//! what crabigator can't hear: streamed text, effort, failed commands):
//! events add heat that cools every frame, and with fade (always on in
//! crabigator) each change of state relights the scene to full and lets it
//! ease back down over two minutes. A bright scene has just changed; embers
//! have sat a while.

use super::scene::Tint;

/// Per-frame cooling: at 70 ms frames, a burst halves in ~2.4 s.
const DECAY: f64 = 0.98;
const MAX_HEAT: f64 = 5.0;
/// Activity from a subagent's loop counts at this weight.
const SUBAGENT_WEIGHT: f64 = 0.5;
/// The frame the activity model counts in, ms.
pub const FRAME_MS: f64 = 70.0;
/// Frames of gray tips after a compaction.
const COMPACT_SMOKE: f64 = 40.0;
/// Fade: how long a change of state takes to ease back to the base.
pub const FADE_MS: f64 = 120_000.0;
/// The effort floor when the effort isn't known.
const DEFAULT_FLOOR: f64 = 3.0;

#[derive(Clone, Debug)]
pub struct Activity {
    pub heat: f64,
    pub is_turn_active: bool,
    pub running_agents: u32,
    pub tools_in_flight: u32,
    pub floor: f64,
    pub smoke_frames: f64,
    pub context_percent: f64,
    /// Time since the state last changed, for fade.
    pub since_change_ms: f64,
}

impl Default for Activity {
    fn default() -> Self {
        Self {
            heat: 0.0,
            is_turn_active: false,
            running_agents: 0,
            tools_in_flight: 0,
            floor: DEFAULT_FLOOR,
            smoke_frames: 0.0,
            context_percent: 0.0,
            since_change_ms: FADE_MS,
        }
    }
}

impl Activity {
    fn add(&mut self, n: f64) {
        self.heat = (self.heat + n).min(MAX_HEAT);
    }

    pub fn turn_started(&mut self) {
        self.is_turn_active = true;
    }

    pub fn turn_ended(&mut self) {
        self.is_turn_active = false;
    }

    /// The state changed: fade relights from full.
    pub fn changed(&mut self) {
        self.since_change_ms = 0.0;
    }

    /// Code written: a flare scaled by the lines changed.
    pub fn edited(&mut self, lines: f64, is_subagent: bool) {
        self.add((1.0 + lines / 15.0).min(4.0) * if is_subagent { SUBAGENT_WEIGHT } else { 1.0 });
    }

    pub fn ran_command(&mut self, is_subagent: bool) {
        self.add(if is_subagent { SUBAGENT_WEIGHT } else { 1.0 });
    }

    pub fn read(&mut self, is_subagent: bool) {
        self.add(if is_subagent { 0.25 } else { 0.5 });
    }

    /// The Agent tool: a small spark; the running count does the rest.
    pub fn spawned_agent(&mut self) {
        self.add(0.5);
    }

    /// A compaction: the scene drops to nothing, shows smoke, then picks up again.
    pub fn compacted(&mut self) {
        self.heat = 0.0;
        self.smoke_frames = COMPACT_SMOKE;
    }

    pub fn is_working(&self) -> bool {
        self.is_turn_active || self.running_agents > 0
    }

    /// Fade's light, from 1 at a change of state to 0 two minutes on (a cosine ease).
    pub fn fade(&self) -> f64 {
        if self.since_change_ms >= FADE_MS {
            0.0
        } else {
            0.5 * (1.0 + (std::f64::consts::PI * self.since_change_ms / FADE_MS).cos())
        }
    }

    /// Levels from running subagents: diminishing, so a swarm doesn't pin 10.
    pub fn agent_boost(&self) -> f64 {
        if self.running_agents > 0 {
            1.2 * f64::from(1 + self.running_agents).log2()
        } else {
            0.0
        }
    }

    /// Advance `frames` frames (a slow tick covers several) of cooling.
    pub fn tick(&mut self, frames: f64) {
        self.since_change_ms = (self.since_change_ms + frames * FRAME_MS).min(FADE_MS);
        self.heat *= DECAY.powf(frames);
        if self.heat < 0.01 {
            self.heat = 0.0;
        }
        if self.is_working() && self.tools_in_flight > 0 && self.heat < 1.0 {
            self.heat = 1.0;
        }
        self.smoke_frames = (self.smoke_frames - frames).max(0.0);
    }

    /// Fade's dial: the base (the effort floor while a turn runs, the idle
    /// floor otherwise), lifted toward 10 by how recently the state changed,
    /// plus the work's own flares and the subagents.
    pub fn strength_faded(&self, idle_floor: f64) -> f64 {
        let base = if self.is_turn_active {
            self.floor
        } else if self.is_working() {
            1.0
        } else {
            idle_floor
        };
        let lit = base + self.fade() * (10.0 - base) + self.heat + self.agent_boost();
        let lowest = if self.is_working() { 1.0 } else { 0.0 };
        super::js::round(lit).clamp(lowest, 10.0)
    }

    /// How much company subagents add: 15 per running subagent.
    pub fn coverage_boost(&self) -> f64 {
        (f64::from(self.running_agents) * 15.0).min(60.0)
    }

    pub fn tint(&self) -> Tint {
        if self.smoke_frames > 0.0 {
            Tint::Smoke
        } else if self.context_percent >= 85.0 {
            Tint::Blue
        } else {
            Tint::Normal
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(a: &mut Activity, ms: f64) {
        let mut t = 0.0;
        while t < ms {
            a.tick(1.0);
            t += FRAME_MS;
        }
    }

    #[test]
    fn a_change_relights_to_full_and_fades_back_over_two_minutes() {
        let mut a = Activity::default();
        assert_eq!(a.strength_faded(1.0), 1.0);
        a.changed();
        assert_eq!(a.strength_faded(1.0), 10.0);
        run(&mut a, 20_000.0);
        assert!(a.strength_faded(1.0) >= 8.0);
        run(&mut a, 40_000.0);
        let mid = a.strength_faded(1.0);
        assert!((3.0..8.0).contains(&mid), "{mid}");
        run(&mut a, FADE_MS - 60_000.0);
        assert_eq!(a.strength_faded(1.0), 1.0);
    }

    #[test]
    fn a_turn_sits_at_its_floor_once_faded_and_work_flares_it() {
        let mut a = Activity::default();
        a.turn_started();
        a.changed();
        run(&mut a, FADE_MS + 1000.0);
        assert_eq!(a.strength_faded(1.0), DEFAULT_FLOOR);
        a.edited(12.0, false);
        assert!(a.strength_faded(1.0) > DEFAULT_FLOOR);
        a.compacted();
        assert_eq!(a.tint(), Tint::Smoke);
    }
}
