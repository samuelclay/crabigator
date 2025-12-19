use std::time::Instant;

use crate::platforms::{Platform, PlatformStats};

#[derive(Clone, Debug)]
pub struct SessionStats {
    pub work_seconds: u64,
    /// Stats from the platform's hook system
    pub platform_stats: PlatformStats,
    /// Timestamp of last platform stats check
    last_stats_check: f64,
    session_start: Instant,
}

impl SessionStats {
    pub fn new() -> Self {
        Self {
            work_seconds: 0,
            platform_stats: PlatformStats::default(),
            last_stats_check: 0.0,
            session_start: Instant::now(),
        }
    }

    /// Called each tick to update session time
    pub fn tick(&mut self) {
        self.work_seconds = self.session_start.elapsed().as_secs();
    }

    /// Refresh platform stats from the platform's data source
    pub fn refresh_platform_stats(&mut self, platform: &dyn Platform, cwd: &str) {
        if let Ok(stats) = platform.load_stats(cwd) {
            // Only update if stats have changed
            let last_updated = stats.last_updated.unwrap_or(0.0);
            if last_updated > self.last_stats_check {
                self.last_stats_check = last_updated;
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
}

impl Default for SessionStats {
    fn default() -> Self {
        Self::new()
    }
}
