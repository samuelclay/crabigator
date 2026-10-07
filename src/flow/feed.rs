//! What the session's stats say it did, as flow hears it: tool calls
//! finished, compactions. Every platform fills the same `PlatformStats`, so
//! this works the same for Claude Code, Codex, opencode and Grok.

use std::collections::HashMap;

use crate::platforms::PlatformStats;

/// The most tool calls passed on from one stats refresh (a burst reads as one flare).
const MAX_TOOLS_PER_REFRESH: usize = 8;

/// Something flow should hear about.
#[derive(Debug, PartialEq, Eq)]
pub enum Heard {
    Tool(String),
    Compact,
}

/// Turns stats refreshes into what changed since the last one.
#[derive(Default)]
pub struct ActivityFeed {
    /// Tool counts at the last refresh; `None` until the first, which only sets them.
    tools: Option<HashMap<String, u32>>,
    compressions: u32,
    last_updated: Option<f64>,
}

impl ActivityFeed {
    /// What happened since the last stats refresh. The first one only notes
    /// where things stand (a resumed session's history isn't news).
    pub fn observe(&mut self, stats: &PlatformStats) -> Vec<Heard> {
        if stats.last_updated == self.last_updated && self.tools.is_some() {
            return Vec::new();
        }
        self.last_updated = stats.last_updated;
        let mut heard = Vec::new();
        if let Some(before) = &self.tools {
            // A count going down means the stats were replaced: start over quietly.
            let replaced = before
                .iter()
                .any(|(name, count)| stats.tools.get(name).copied().unwrap_or(0) < *count);
            if !replaced {
                let mut names: Vec<&String> = stats.tools.keys().collect();
                names.sort();
                for name in names {
                    let now = stats.tools[name];
                    let was = before.get(name).copied().unwrap_or(0);
                    for _ in was..now {
                        if heard.len() < MAX_TOOLS_PER_REFRESH {
                            heard.push(Heard::Tool(name.clone()));
                        }
                    }
                }
                if stats.compressions > self.compressions {
                    heard.push(Heard::Compact);
                }
            }
        }
        self.tools = Some(stats.tools.clone());
        self.compressions = stats.compressions;
        heard
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stats(updated: f64, tools: &[(&str, u32)], compressions: u32) -> PlatformStats {
        PlatformStats {
            last_updated: Some(updated),
            tools: tools
                .iter()
                .map(|(name, count)| (name.to_string(), *count))
                .collect(),
            compressions,
            ..PlatformStats::default()
        }
    }

    #[test]
    fn the_first_refresh_only_sets_the_baseline() {
        let mut feed = ActivityFeed::default();
        assert!(feed.observe(&stats(1.0, &[("Bash", 40)], 2)).is_empty());
    }

    #[test]
    fn new_tool_calls_and_compactions_are_heard_once() {
        let mut feed = ActivityFeed::default();
        feed.observe(&stats(1.0, &[("Bash", 2)], 0));
        let heard = feed.observe(&stats(2.0, &[("Bash", 3), ("Edit", 1)], 1));
        assert_eq!(
            heard,
            vec![
                Heard::Tool("Bash".into()),
                Heard::Tool("Edit".into()),
                Heard::Compact
            ]
        );
        // The same refresh again: nothing new.
        assert!(feed
            .observe(&stats(2.0, &[("Bash", 3), ("Edit", 1)], 1))
            .is_empty());
    }

    #[test]
    fn a_burst_is_capped_and_a_drop_resets_quietly() {
        let mut feed = ActivityFeed::default();
        feed.observe(&stats(1.0, &[("Read", 0)], 0));
        assert_eq!(
            feed.observe(&stats(2.0, &[("Read", 50)], 0)).len(),
            MAX_TOOLS_PER_REFRESH
        );
        assert!(feed.observe(&stats(3.0, &[("Read", 5)], 0)).is_empty());
        assert_eq!(
            feed.observe(&stats(4.0, &[("Read", 6)], 0)),
            vec![Heard::Tool("Read".into())]
        );
    }
}
