use std::time::Instant;

#[derive(Clone, Debug)]
pub struct ClaudeStats {
    pub idle_seconds: u64,
    pub work_seconds: u64,
    pub tokens_used: u64,
    pub messages_count: u32,
    last_activity: Instant,
    session_start: Instant,
}

impl ClaudeStats {
    pub fn new() -> Self {
        let now = Instant::now();
        Self {
            idle_seconds: 0,
            work_seconds: 0,
            tokens_used: 0,
            messages_count: 0,
            last_activity: now,
            session_start: now,
        }
    }

    /// Called each tick to update idle time based on last activity
    pub fn tick(&mut self) {
        self.idle_seconds = self.last_activity.elapsed().as_secs();
        self.work_seconds = self.session_start.elapsed().as_secs();
    }

    #[allow(dead_code)]
    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
        self.idle_seconds = 0;
    }

    #[allow(dead_code)]
    pub fn add_tokens(&mut self, count: u64) {
        self.tokens_used += count;
    }

    #[allow(dead_code)]
    pub fn increment_messages(&mut self) {
        self.messages_count += 1;
    }

    /// Format idle time as compact string: "just now" or "Xm", "Xh Ym", "Xd Yh Zm"
    /// Returns "just now" for < 60 seconds (caller displays as "Active")
    /// Returns time format for >= 60 seconds (caller displays as "Idle")
    pub fn format_idle(&self) -> String {
        if self.idle_seconds < 60 {
            "just now".to_string()
        } else {
            Self::format_duration(self.idle_seconds)
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

impl Default for ClaudeStats {
    fn default() -> Self {
        Self::new()
    }
}
