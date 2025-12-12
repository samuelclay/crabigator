use std::time::Instant;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub struct ClaudeStats {
    pub idle_seconds: u64,
    pub work_seconds: u64,
    pub tokens_used: u64,
    pub messages_count: u32,
    last_activity: Instant,
}

#[allow(dead_code)]
impl ClaudeStats {
    pub fn new() -> Self {
        Self {
            idle_seconds: 0,
            work_seconds: 0,
            tokens_used: 0,
            messages_count: 0,
            last_activity: Instant::now(),
        }
    }

    pub fn update_idle(&mut self) {
        self.idle_seconds = self.last_activity.elapsed().as_secs();
    }

    pub fn record_activity(&mut self) {
        self.last_activity = Instant::now();
        self.idle_seconds = 0;
    }

    pub fn add_tokens(&mut self, count: u64) {
        self.tokens_used += count;
    }

    pub fn increment_messages(&mut self) {
        self.messages_count += 1;
    }
}

impl Default for ClaudeStats {
    fn default() -> Self {
        Self::new()
    }
}
