//! Claude Code's agent panel: the list under its prompt footer of the main
//! loop and each running subagent.
//!
//! ```text
//!   ⏺ main
//!   ◯ general-purpose  Sleep 45 then reply done
//!   ◯ general-purpose  Sleep 45 then reply done
//!   ↓ 1 more
//! ```
//!
//! When the panel grows, Ghostty can be left with stale rows until the window
//! is resized, so crabigator watches its height (`AgentPanelWatch`) and
//! repaints when it grows.

use std::time::{Duration, Instant};

/// How long a height must hold before it counts: Claude redraws the panel
/// now and then, and a screen read mid-redraw can miss it for a moment.
const SETTLE: Duration = Duration::from_millis(500);

/// The agent panel's height over time: says when it has grown and held.
#[derive(Debug)]
pub struct AgentPanelWatch {
    /// The height that last held for `SETTLE`.
    settled: usize,
    /// The height last read, and since when.
    seen: usize,
    seen_since: Instant,
}

impl Default for AgentPanelWatch {
    fn default() -> Self {
        Self {
            settled: 0,
            seen: 0,
            seen_since: Instant::now(),
        }
    }
}

impl AgentPanelWatch {
    /// Note the panel's height as the screen shows it now.
    pub fn see(&mut self, rows: usize, now: Instant) {
        if rows != self.seen {
            self.seen = rows;
            self.seen_since = now;
        }
    }

    /// Whether the panel has settled taller than it last held: time to
    /// repaint. A blip (a height that doesn't hold) never counts.
    pub fn grew(&mut self, now: Instant) -> bool {
        if self.seen == self.settled || now.duration_since(self.seen_since) < SETTLE {
            return false;
        }
        let grew = self.seen > self.settled;
        self.settled = self.seen;
        grew
    }
}

/// How many rows the agent panel takes on the screen (escape codes stripped),
/// counting its "main" heading; 0 when there is none.
pub fn agent_panel_rows(screen: &str) -> usize {
    let lines: Vec<&str> = screen.lines().collect();
    // The panel sits below the prompt box: look only past its last `❯` line.
    let prompt = lines
        .iter()
        .rposition(|line| line.trim_start().starts_with('❯'))
        .map_or(0, |at| at + 1);
    let Some(heading) = lines[prompt..].iter().position(|line| is_heading(line)) else {
        return 0;
    };
    let heading = prompt + heading;
    1 + lines[heading + 1..]
        .iter()
        .take_while(|line| !line.trim().is_empty())
        .count()
}

/// The panel's heading: one marker glyph, then "main".
fn is_heading(line: &str) -> bool {
    let mut words = line.split_whitespace();
    let marker = words.next();
    marker.is_some_and(|m| m.chars().count() == 1 && !m.is_ascii())
        && words.next() == Some("main")
        && words.next().is_none()
}

#[cfg(test)]
mod tests {
    use super::*;

    const FOOTER: &str =
        "─────────\n❯ \n─────────\n  user@host:~/project ‹main*›\n  ⏵⏵ auto mode on · ← 1 agent\n";

    #[test]
    fn no_panel_is_zero_rows() {
        assert_eq!(agent_panel_rows(FOOTER), 0);
        assert_eq!(agent_panel_rows(""), 0);
    }

    #[test]
    fn counts_the_heading_the_agents_and_the_more_line() {
        let screen = format!(
            "{FOOTER}\n  ⏺ main\n  ◯ general-purpose  Sleep 45 then reply done\n  ◯ Explore  Listing files\n  ↓ 1 more\n\n\n"
        );
        assert_eq!(agent_panel_rows(&screen), 4);
        // Other platforms draw the glyphs differently.
        let screen = format!("{FOOTER}\n  ● main\n  ○ general-purpose  Reading a file\n");
        assert_eq!(agent_panel_rows(&screen), 2);
    }

    #[test]
    fn the_watch_says_grew_once_a_taller_panel_holds() {
        let start = Instant::now();
        let at = |ms: u64| start + Duration::from_millis(ms);
        let mut watch = AgentPanelWatch::default();
        watch.see(0, start);
        watch.see(4, at(0));
        assert!(!watch.grew(at(400)), "not held long enough yet");
        assert!(watch.grew(at(500)));
        assert!(!watch.grew(at(600)), "once per growth");
        // A blip (gone for 120 ms, then back) never settles: no repaint.
        watch.see(0, at(6000));
        watch.see(4, at(6120));
        assert!(!watch.grew(at(7000)));
        // Shrinking settles quietly; growing back is growth again.
        watch.see(3, at(8000));
        assert!(!watch.grew(at(8600)));
        watch.see(4, at(9000));
        assert!(watch.grew(at(9500)));
    }

    #[test]
    fn a_transcript_line_about_main_is_not_the_panel() {
        let screen = format!("⏺ Merged into main\n  main\n⏺ main\n{FOOTER}");
        assert_eq!(agent_panel_rows(&screen), 0);
    }
}
