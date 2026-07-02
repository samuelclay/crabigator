//! Background generation of short OSC 8 terminal titles.
//!
//! Claude Code publishes its own OSC 8 terminal title every turn, but Codex
//! never does. When no native title has arrived for a while, we synthesize a
//! 3-6 word title from the latest turn transcript using the same Anthropic key
//! the recap feature uses — refreshed every few turns and cached in between.
//!
//! Generated titles are prefixed with [`GENERATED_TITLE_MARKER`] so it's
//! visually obvious they were authored by Crabigator rather than the agent.

use std::path::PathBuf;
use std::sync::mpsc;

use anyhow::Result;
use serde_json::{json, Value};

use crate::config::Config;
use crate::platforms::{PlatformKind, PlatformStats, SessionState};
use crate::recap::{
    call_anthropic, collect_latest_turn_text, read_anthropic_api_key_env, DEFAULT_RECAP_MODEL,
};

/// Marker prefixed to titles we generated, so they're distinguishable from a
/// native title published by Claude or Codex.
pub const GENERATED_TITLE_MARKER: &str = "⟁ ";

/// Regenerate our own title at most once every this many completed turns.
const REGEN_EVERY_TURNS: u32 = 5;
/// A native title is considered stale once this many turns have completed
/// without one arriving — at which point we take over and generate one.
const NATIVE_STALE_AFTER_TURNS: u32 = 2;
/// Word budget for a generated title.
const MAX_TITLE_WORDS: usize = 6;

#[derive(Clone)]
struct TitleJob {
    platform: PlatformKind,
    transcript_path: Option<PathBuf>,
    model: String,
    api_key: String,
}

/// Owns turn detection and background title generation for sessions whose agent
/// doesn't publish its own title (or has stopped publishing one).
pub struct TitleManager {
    api_key: Option<String>,
    model: String,
    initialized: bool,
    last_completion_count: u32,
    /// Completed turns observed this session.
    completed_turns: u32,
    /// `completed_turns` value when generation was last kicked off.
    turns_at_last_generation: Option<u32>,
    /// `completed_turns` value when a native title last arrived.
    last_native_title_turn: Option<u32>,
    pending: Option<mpsc::Receiver<std::result::Result<String, String>>>,
    generated: Option<String>,
}

impl TitleManager {
    pub fn load() -> Self {
        let config = Config::load().unwrap_or_default();
        let api_key = Config::read_recap_api_key()
            .ok()
            .flatten()
            .or_else(read_anthropic_api_key_env);
        let model = config
            .recap_model
            .unwrap_or_else(|| DEFAULT_RECAP_MODEL.to_string());

        Self {
            api_key,
            model,
            initialized: false,
            last_completion_count: 0,
            completed_turns: 0,
            turns_at_last_generation: None,
            last_native_title_turn: None,
            pending: None,
            generated: None,
        }
    }

    /// Latest generated title (without the marker), if any.
    pub fn generated_title(&self) -> Option<&str> {
        self.generated.as_deref()
    }

    /// Whether the agent's native title is recent enough that we should defer to
    /// it rather than generating our own.
    pub fn native_is_fresh(&self) -> bool {
        match self.last_native_title_turn {
            Some(turn) => self.completed_turns.saturating_sub(turn) < NATIVE_STALE_AFTER_TURNS,
            None => false,
        }
    }

    /// Record that the agent just published its own (non-default) title.
    pub fn note_native_title(&mut self) {
        self.last_native_title_turn = Some(self.completed_turns);
    }

    /// Fold in the latest platform stats: detect a completed turn, advance the
    /// turn counter, and kick off generation when due. Returns true when the
    /// generated title changed (i.e. the display should refresh).
    pub fn update(
        &mut self,
        platform: PlatformKind,
        stats: &PlatformStats,
        effective_state: SessionState,
    ) -> bool {
        let mut changed = self.poll();

        if !self.initialized {
            self.initialized = true;
            self.last_completion_count = stats.completions;
            return changed;
        }

        let completed_turn = stats.completions > self.last_completion_count
            && effective_state == SessionState::Complete;
        if completed_turn {
            self.last_completion_count = stats.completions;
            self.completed_turns = self.completed_turns.saturating_add(1);
            changed |= self.maybe_generate(platform, stats);
        }

        changed
    }

    /// Kick off a background generation if we have a key, nothing is in flight,
    /// the native title is stale, and the regeneration cadence allows it.
    fn maybe_generate(&mut self, platform: PlatformKind, stats: &PlatformStats) -> bool {
        let Some(api_key) = self.api_key.clone() else {
            return false;
        };
        if self.pending.is_some() || self.native_is_fresh() {
            return false;
        }

        let due = match self.turns_at_last_generation {
            None => true,
            Some(turn) => self.completed_turns.saturating_sub(turn) >= REGEN_EVERY_TURNS,
        };
        if !due {
            return false;
        }
        self.turns_at_last_generation = Some(self.completed_turns);

        let job = TitleJob {
            platform,
            transcript_path: stats.transcript_path.as_ref().map(PathBuf::from),
            model: self.model.clone(),
            api_key,
        };

        let (tx, rx) = mpsc::channel();
        std::thread::spawn(move || {
            let _ = tx.send(generate_title(job).map_err(|e| e.to_string()));
        });
        self.pending = Some(rx);
        // Nothing visible changed yet — the result arrives on a later poll().
        false
    }

    /// Check the background worker for a finished title.
    fn poll(&mut self) -> bool {
        let Some(rx) = &self.pending else {
            return false;
        };
        match rx.try_recv() {
            Ok(Ok(title)) if !title.is_empty() => {
                self.generated = Some(title);
                self.pending = None;
                true
            }
            // Empty result or generation error: keep whatever title we had and
            // try again next cadence window. Title generation is best-effort.
            Ok(_) => {
                self.pending = None;
                false
            }
            Err(mpsc::TryRecvError::Empty) => false,
            Err(mpsc::TryRecvError::Disconnected) => {
                self.pending = None;
                false
            }
        }
    }
}

fn generate_title(job: TitleJob) -> Result<String> {
    let transcript = collect_latest_turn_text(job.platform, job.transcript_path.as_deref())?;
    let has_prompt = transcript
        .user_prompt
        .as_deref()
        .map(|p| !p.trim().is_empty())
        .unwrap_or(false);
    if transcript.activity.trim().is_empty() && !has_prompt {
        anyhow::bail!("no transcript activity to title");
    }

    let request = build_title_request(&job.model, &transcript);
    let text = call_anthropic(&job.api_key, request)?;
    Ok(clean_title(&text))
}

fn build_title_request(model: &str, transcript: &crate::recap::TurnTranscript) -> Value {
    let user_prompt = transcript
        .user_prompt
        .as_deref()
        .unwrap_or("(last user prompt unavailable)");

    let prompt = format!(
        "Write a short terminal window title for what this coding session is working on.\n\
         Last user prompt, for context:\n{user_prompt}\n\n\
         Recent agent activity:\n{}\n\n\
         Rules:\n\
         - 3 to 6 words. Title Case.\n\
         - Name the task or goal, not the tool (no \"Claude\"/\"Codex\"/\"Crabigator\").\n\
         - No surrounding quotes, no leading symbol, no trailing punctuation.\n\
         - Output ONLY the title text on a single line, nothing else.",
        transcript.activity
    );

    json!({
        "model": model,
        "max_tokens": 32,
        "temperature": 0.3,
        "messages": [
            {"role": "user", "content": prompt}
        ]
    })
}

/// Normalize the model's output into a clean, bounded title. Skips blank or
/// decoration-only lines (e.g. a ```` ``` ```` code fence) so a fenced reply
/// still yields the title on the line within.
fn clean_title(text: &str) -> String {
    for raw in text.lines() {
        let line = raw
            .trim()
            .trim_matches(|c: char| c == '"' || c == '\'' || c == '`')
            .trim_end_matches(['.', '!', ',', ':', ';'])
            .trim();
        if line.is_empty() {
            continue;
        }
        return line
            .split_whitespace()
            .take(MAX_TITLE_WORDS)
            .collect::<Vec<_>>()
            .join(" ");
    }
    String::new()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn manager() -> TitleManager {
        TitleManager {
            api_key: Some("test".to_string()),
            model: DEFAULT_RECAP_MODEL.to_string(),
            initialized: true,
            last_completion_count: 0,
            completed_turns: 0,
            turns_at_last_generation: None,
            last_native_title_turn: None,
            pending: None,
            generated: None,
        }
    }

    #[test]
    fn clean_title_bounds_words_and_strips_decoration() {
        assert_eq!(
            clean_title("\"Refactor The Title Rendering Path Now Please\"."),
            "Refactor The Title Rendering Path Now"
        );
        assert_eq!(
            clean_title("```\nFix Branch Truncation\n```"),
            "Fix Branch Truncation"
        );
        assert_eq!(
            clean_title("Add Dashboard Auth Flow"),
            "Add Dashboard Auth Flow"
        );
    }

    #[test]
    fn native_freshness_decays_with_turns() {
        let mut m = manager();
        assert!(!m.native_is_fresh()); // never seen one

        m.note_native_title(); // arrives at turn 0
        assert!(m.native_is_fresh());

        m.completed_turns = 1;
        assert!(m.native_is_fresh()); // within the stale window

        m.completed_turns = NATIVE_STALE_AFTER_TURNS;
        assert!(!m.native_is_fresh()); // aged out
    }

    #[test]
    fn generation_is_gated_by_native_freshness_and_cadence() {
        let stats = PlatformStats {
            transcript_path: Some("/tmp/does-not-exist.jsonl".to_string()),
            ..PlatformStats::default()
        };

        // Fresh native title (Claude case): never generate.
        let mut m = manager();
        m.note_native_title();
        m.completed_turns = 1;
        assert!(!m.maybe_generate(PlatformKind::Claude, &stats));
        assert!(m.pending.is_none());

        // No native title (Codex case): generate on the first completed turn.
        let mut m = manager();
        m.completed_turns = 1;
        m.maybe_generate(PlatformKind::Codex, &stats);
        assert!(m.pending.is_some());
        assert_eq!(m.turns_at_last_generation, Some(1));

        // ...and not again until the cadence window elapses.
        m.pending = None;
        m.completed_turns = 1 + REGEN_EVERY_TURNS - 1;
        assert!(!m.maybe_generate(PlatformKind::Codex, &stats));
        assert!(m.pending.is_none());

        m.completed_turns = 1 + REGEN_EVERY_TURNS;
        m.maybe_generate(PlatformKind::Codex, &stats);
        assert!(m.pending.is_some());
    }
}
