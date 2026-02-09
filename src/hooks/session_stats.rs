use std::time::{Instant, SystemTime, UNIX_EPOCH};

use crate::platforms::{Platform, PlatformStats, SessionState};
use crate::ui::sparkline::bin_timestamps;

#[derive(Clone, Debug)]
pub struct SessionStats {
    pub work_seconds: u64,
    /// Base accumulated thinking time (before current thinking session)
    thinking_base: u64,
    /// Stats from the platform's hook system
    pub platform_stats: PlatformStats,
    /// Timestamp of last platform stats check
    last_stats_check: f64,
    session_start: Instant,
    /// Unix timestamp when session started (for sparkline binning)
    session_start_unix: f64,
    /// Instant when thinking started (None when not thinking)
    thinking_since: Option<Instant>,
    /// Previous prompts count (for change detection)
    last_prompts: u32,
    /// Unix timestamp when prompts last changed
    pub prompts_changed_at: Option<f64>,
    /// Previous completions count (for change detection)
    last_completions: u32,
    /// Unix timestamp when completions last changed
    pub completions_changed_at: Option<f64>,
    /// Previous compressions count (for change detection)
    last_compressions: u32,
    /// Unix timestamp when compressions last changed
    pub compressions_changed_at: Option<f64>,
    /// Whether the user interrupted during thinking (ESC/Ctrl+C)
    /// Cleared when platform reports a new state
    interrupted: bool,
    /// Terminal title currently has Braille spinner prefix (real-time from PTY)
    title_has_spinner: bool,
    /// Screen content shows "Esc to cancel" after last ❯ prompt
    screen_shows_input_wait: bool,
}

impl SessionStats {
    pub fn new() -> Self {
        let now_unix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        Self {
            work_seconds: 0,
            thinking_base: 0,
            platform_stats: PlatformStats::default(),
            last_stats_check: 0.0,
            session_start: Instant::now(),
            session_start_unix: now_unix,
            thinking_since: None,
            last_prompts: 0,
            prompts_changed_at: None,
            last_completions: 0,
            completions_changed_at: None,
            last_compressions: 0,
            compressions_changed_at: None,
            interrupted: false,
            title_has_spinner: false,
            screen_shows_input_wait: false,
        }
    }

    /// Mark as interrupted (called when ESC/Ctrl+C during thinking)
    pub fn set_interrupted(&mut self) {
        self.interrupted = true;
    }

    /// Update title spinner state (called from write_pty_output on every title change)
    pub fn set_title_spinner(&mut self, has_spinner: bool) {
        self.title_has_spinner = has_spinner;
    }

    /// Update screen input-wait state (called from handle_pty_output_capture)
    pub fn set_screen_input_wait(&mut self, waiting: bool) {
        self.screen_shows_input_wait = waiting;
    }

    /// Get the effective session state (considering secondary signal overrides)
    pub fn effective_state(&self) -> SessionState {
        if self.interrupted {
            return SessionState::Interrupted;
        }

        let hook_state = self.platform_stats.state;

        // Spinner active but hooks don't say Thinking → override to Thinking
        if self.title_has_spinner && hook_state != SessionState::Thinking {
            return SessionState::Thinking;
        }

        // Screen shows "Esc to cancel" but hooks say Thinking → override to Permission
        if self.screen_shows_input_wait && hook_state == SessionState::Thinking {
            return SessionState::Permission;
        }

        hook_state
    }

    /// Called each tick to update session time and thinking time
    pub fn tick(&mut self) {
        self.work_seconds = self.session_start.elapsed().as_secs();

        // Track thinking time only when actively thinking (not permission/question/interrupted/etc)
        let is_thinking = self.effective_state() == SessionState::Thinking;
        if is_thinking {
            if self.thinking_since.is_none() {
                // Just started thinking - start the timer
                self.thinking_since = Some(Instant::now());
            }
        } else if let Some(since) = self.thinking_since.take() {
            // Stopped thinking - add elapsed to base
            self.thinking_base += since.elapsed().as_secs();
        }
    }

    /// Get total thinking time (base + current session if thinking)
    pub fn thinking_seconds(&self) -> u64 {
        self.thinking_base
            + self
                .thinking_since
                .map(|s| s.elapsed().as_secs())
                .unwrap_or(0)
    }

    /// Refresh platform stats from the platform's data source
    pub fn refresh_platform_stats(&mut self, platform: &dyn Platform, cwd: &str) {
        if let Ok(stats) = platform.load_stats(cwd) {
            // Only update if stats have changed
            let last_updated = stats.last_updated.unwrap_or(0.0);
            if last_updated > self.last_stats_check {
                self.last_stats_check = last_updated;

                // Clear interrupted flag when any hook event occurs
                // (e.g., user submitted new prompt, Stop event, tool use, etc.)
                // This handles the case where state stays "thinking" after interrupt + new prompt
                self.interrupted = false;

                // When hooks report a state change, clear screen-based signal overrides.
                // Hooks are the authoritative source — they fire synchronously with Claude
                // Code's state machine. Screen content (title spinner, "Esc to cancel")
                // lags behind by 0.5–2s, and the effective_state() overrides can mask real
                // transitions, causing:
                //   - New permission prompts to be invisible on the dashboard (eff stays
                //     "permission" from the screen override through the real permission)
                //   - Stale permission state after answering (screen still has "Esc to cancel")
                //   - Delayed question detection (spinner override keeps eff as "thinking")
                if stats.state != self.platform_stats.state {
                    self.screen_shows_input_wait = false;
                    self.title_has_spinner = false;
                }

                // Track when prompts/completions change
                let now = SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs_f64();

                if stats.prompts != self.last_prompts {
                    self.last_prompts = stats.prompts;
                    self.prompts_changed_at = Some(now);
                }

                if stats.completions != self.last_completions {
                    self.last_completions = stats.completions;
                    self.completions_changed_at = Some(now);
                }

                if stats.compressions != self.last_compressions {
                    self.last_compressions = stats.compressions;
                    self.compressions_changed_at = Some(now);
                }

                self.platform_stats = stats;
            }
        }
    }

    /// Format a duration in seconds as compact string: "Xm", "Xh Ym", "Xd Yh Zm"
    fn format_duration(seconds: u64) -> String {
        let days = seconds / 86400;
        let hours = (seconds % 86400) / 3600;
        let mins = (seconds % 3600) / 60;

        if days > 0 {
            if hours > 0 && mins > 0 {
                format!("{}d {}h {}m", days, hours, mins)
            } else if hours > 0 {
                format!("{}d {}h", days, hours)
            } else if mins > 0 {
                format!("{}d {}m", days, mins)
            } else {
                format!("{}d", days)
            }
        } else if hours > 0 {
            if mins > 0 {
                format!("{}h {}m", hours, mins)
            } else {
                format!("{}h", hours)
            }
        } else {
            format!("{}m", mins)
        }
    }

    /// Format work/session time as compact string: "just now", "Xm", "Xh Ym", "Xd Yh Zm"
    pub fn format_work(&self) -> String {
        if self.work_seconds < 60 {
            "just now".to_string()
        } else {
            Self::format_duration(self.work_seconds)
        }
    }

    /// Format thinking time as compact string, or None if no thinking has occurred
    pub fn format_thinking(&self) -> Option<String> {
        let secs = self.thinking_seconds();
        if secs == 0 {
            None
        } else if secs < 60 {
            Some(format!("{}s", secs))
        } else {
            Some(Self::format_duration(secs))
        }
    }

    /// Get session start time as Unix timestamp
    pub fn session_start_unix(&self) -> f64 {
        self.session_start_unix
    }

    /// Get binned tool usage for sparkline rendering
    pub fn tool_usage_bins(&self, num_bins: usize) -> Vec<u32> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs_f64();
        bin_timestamps(
            &self.platform_stats.tool_timestamps,
            self.session_start_unix,
            now,
            num_bins,
        )
    }
}

impl Default for SessionStats {
    fn default() -> Self {
        Self::new()
    }
}
