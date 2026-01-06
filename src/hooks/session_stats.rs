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
        }
    }

    /// Called each tick to update session time and thinking time
    pub fn tick(&mut self) {
        self.work_seconds = self.session_start.elapsed().as_secs();

        // Track thinking time only when actively thinking (not permission/question/etc)
        let is_thinking = self.platform_stats.state == SessionState::Thinking;
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
