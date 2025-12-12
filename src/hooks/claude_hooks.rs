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

    /// Format idle time as compact string: "just now", "1m", "2m", etc.
    pub fn format_idle(&self) -> String {
        if self.idle_seconds < 5 {
            "just now".to_string()
        } else if self.idle_seconds < 60 {
            format!("{}s", self.idle_seconds)
        } else if self.idle_seconds < 3600 {
            format!("{}m", self.idle_seconds / 60)
        } else {
            format!("{}h", self.idle_seconds / 3600)
        }
    }

    /// Format work/session time as compact string
    pub fn format_work(&self) -> String {
        if self.work_seconds < 60 {
            format!("{}s", self.work_seconds)
        } else if self.work_seconds < 3600 {
            let mins = self.work_seconds / 60;
            let secs = self.work_seconds % 60;
            if secs == 0 {
                format!("{}m", mins)
            } else {
                format!("{}m{}s", mins, secs)
            }
        } else {
            let hours = self.work_seconds / 3600;
            let mins = (self.work_seconds % 3600) / 60;
            if mins == 0 {
                format!("{}h", hours)
            } else {
                format!("{}h{}m", hours, mins)
            }
        }
    }
}

impl Default for ClaudeStats {
    fn default() -> Self {
        Self::new()
    }
}
