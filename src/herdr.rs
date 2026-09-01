//! Report session lifecycle to Herdr when Crabigator runs inside a Herdr pane.
//!
//! Herdr injects `HERDR_ENV=1`, `HERDR_PANE_ID`, and `HERDR_BIN_PATH`. Outside
//! Herdr this module is a no-op.

use std::env;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::hooks::SessionStats;
use crate::platforms::{ActivePrompt, PlatformKind, SessionState};

const SOURCE: &str = "herdr:crabigator";
const AGENT: &str = "crabigator";

static SEQ: AtomicU64 = AtomicU64::new(0);

fn next_seq() -> u64 {
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0);
    let prev = SEQ.load(Ordering::Relaxed);
    let next = now.max(prev.saturating_add(1));
    SEQ.store(next, Ordering::Relaxed);
    next
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HerdrState {
    Idle,
    Working,
    Blocked,
}

impl HerdrState {
    fn as_str(self) -> &'static str {
        match self {
            Self::Idle => "idle",
            Self::Working => "working",
            Self::Blocked => "blocked",
        }
    }
}

pub fn map_session_state(state: SessionState) -> HerdrState {
    match state {
        SessionState::Thinking => HerdrState::Working,
        SessionState::Permission | SessionState::Question => HerdrState::Blocked,
        SessionState::Ready | SessionState::Complete | SessionState::Interrupted => HerdrState::Idle,
    }
}

pub fn display_agent(platform: PlatformKind) -> String {
    format!("crabigator · {}", platform.as_str())
}

pub fn session_ref(platform: PlatformKind, native_id: Option<&str>) -> Option<String> {
    let id = native_id.map(str::trim).filter(|id| !id.is_empty())?;
    Some(format!("{}:{id}", platform.as_str()))
}

fn blocked_message(stats: &SessionStats) -> Option<String> {
    match stats.effective_state() {
        SessionState::Permission => stats
            .platform_stats
            .permission
            .as_ref()
            .map(|p| p.tool.clone())
            .or_else(|| Some("permission".into())),
        SessionState::Question => match stats.active_prompt() {
            Some(ActivePrompt::Question { questions }) => questions
                .first()
                .map(|q| q.question.clone())
                .or_else(|| Some("question".into())),
            Some(ActivePrompt::Permission { tool_name, .. }) => Some(tool_name.clone()),
            Some(ActivePrompt::ExitPlan) => Some("plan".into()),
            None => Some("question".into()),
        },
        _ => None,
    }
}

pub struct HerdrReporter {
    enabled: bool,
    bin: PathBuf,
    pane_id: String,
    last_state: Option<HerdrState>,
    last_message: Option<String>,
    last_display: Option<String>,
    last_session: Option<String>,
    released: bool,
}

impl HerdrReporter {
    pub fn from_env() -> Self {
        let enabled = env::var("HERDR_ENV").ok().as_deref() == Some("1");
        let pane_id = env::var("HERDR_PANE_ID").unwrap_or_default();
        let bin = env::var("HERDR_BIN_PATH")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("herdr"));
        let enabled = enabled && !pane_id.is_empty() && !bin.as_os_str().is_empty();
        Self {
            enabled,
            bin,
            pane_id,
            last_state: None,
            last_message: None,
            last_display: None,
            last_session: None,
            released: false,
        }
    }

    pub fn sync(&mut self, stats: &SessionStats, platform: PlatformKind) {
        if !self.enabled || self.released {
            return;
        }

        let state = map_session_state(stats.effective_state());
        let message = blocked_message(stats);
        let display = display_agent(platform);
        let session = session_ref(
            platform,
            stats.platform_stats.native_session_id.as_deref(),
        );

        let state_changed = self.last_state != Some(state) || self.last_message != message;
        let display_changed = self.last_display.as_deref() != Some(display.as_str());
        let session_changed = self.last_session != session;

        if state_changed || session_changed {
            self.report_agent(state, message.as_deref(), session.as_deref());
            self.last_state = Some(state);
            self.last_message = message;
            self.last_session = session;
        }

        if display_changed {
            self.report_display(&display, platform);
            self.last_display = Some(display);
        }
    }

    pub fn release(&mut self) {
        if !self.enabled || self.released {
            return;
        }
        self.released = true;
        let mut cmd = Command::new(&self.bin);
        cmd.arg("pane")
            .arg("release-agent")
            .arg(&self.pane_id)
            .arg("--source")
            .arg(SOURCE)
            .arg("--agent")
            .arg(AGENT)
            .arg("--seq")
            .arg(next_seq().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = cmd.spawn();
    }

    fn report_agent(&self, state: HerdrState, message: Option<&str>, session: Option<&str>) {
        let mut cmd = Command::new(&self.bin);
        cmd.arg("pane")
            .arg("report-agent")
            .arg(&self.pane_id)
            .arg("--source")
            .arg(SOURCE)
            .arg("--agent")
            .arg(AGENT)
            .arg("--state")
            .arg(state.as_str())
            .arg("--seq")
            .arg(next_seq().to_string());
        if let Some(message) = message.filter(|m| !m.is_empty()) {
            cmd.arg("--message").arg(message);
        }
        if let Some(session) = session.filter(|s| !s.is_empty()) {
            cmd.arg("--agent-session-id").arg(session);
        }
        cmd.stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = cmd.spawn();
    }

    fn report_display(&self, display: &str, platform: PlatformKind) {
        let mut cmd = Command::new(&self.bin);
        cmd.arg("pane")
            .arg("report-metadata")
            .arg(&self.pane_id)
            .arg("--source")
            .arg(SOURCE)
            .arg("--agent")
            .arg(AGENT)
            .arg("--applies-to-source")
            .arg(SOURCE)
            .arg("--display-agent")
            .arg(display)
            .arg("--token")
            .arg(format!("platform={}", platform.as_str()))
            .arg("--seq")
            .arg(next_seq().to_string())
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null());
        let _ = cmd.spawn();
    }
}

impl Drop for HerdrReporter {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn maps_session_states() {
        assert_eq!(
            map_session_state(SessionState::Thinking),
            HerdrState::Working
        );
        assert_eq!(
            map_session_state(SessionState::Permission),
            HerdrState::Blocked
        );
        assert_eq!(
            map_session_state(SessionState::Question),
            HerdrState::Blocked
        );
        assert_eq!(map_session_state(SessionState::Ready), HerdrState::Idle);
        assert_eq!(map_session_state(SessionState::Complete), HerdrState::Idle);
        assert_eq!(
            map_session_state(SessionState::Interrupted),
            HerdrState::Idle
        );
    }

    #[test]
    fn labels_inner_platform() {
        assert_eq!(display_agent(PlatformKind::Claude), "crabigator · claude");
        assert_eq!(display_agent(PlatformKind::Codex), "crabigator · codex");
        assert_eq!(
            display_agent(PlatformKind::Opencode),
            "crabigator · opencode"
        );
        assert_eq!(display_agent(PlatformKind::Grok), "crabigator · grok");
    }

    #[test]
    fn encodes_resume_session_refs() {
        assert_eq!(
            session_ref(PlatformKind::Claude, Some("abc-123")).as_deref(),
            Some("claude:abc-123")
        );
        assert_eq!(
            session_ref(PlatformKind::Codex, Some("thread")).as_deref(),
            Some("codex:thread")
        );
        assert_eq!(
            session_ref(PlatformKind::Opencode, Some("ses_1")).as_deref(),
            Some("opencode:ses_1")
        );
        assert_eq!(
            session_ref(PlatformKind::Grok, Some("01abc")).as_deref(),
            Some("grok:01abc")
        );
        assert_eq!(session_ref(PlatformKind::Claude, Some("")), None);
        assert_eq!(session_ref(PlatformKind::Claude, None), None);
    }
}
