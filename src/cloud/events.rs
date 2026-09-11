//! Cloud event types for streaming to the server
//!
//! These types mirror the TypeScript SessionEvent types on the server.

use serde::{Deserialize, Serialize};

use crate::git::GitState;
use crate::parsers::{ChangeType, DiffSummary, NodeKind};

/// Session state matching the server's SessionState type
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum CloudSessionState {
    Ready,
    Thinking,
    Permission,
    Question,
    Complete,
    Interrupted,
}

impl From<crate::platforms::SessionState> for CloudSessionState {
    fn from(state: crate::platforms::SessionState) -> Self {
        match state {
            crate::platforms::SessionState::Ready => CloudSessionState::Ready,
            crate::platforms::SessionState::Thinking => CloudSessionState::Thinking,
            crate::platforms::SessionState::Permission => CloudSessionState::Permission,
            crate::platforms::SessionState::Question => CloudSessionState::Question,
            crate::platforms::SessionState::Complete => CloudSessionState::Complete,
            crate::platforms::SessionState::Interrupted => CloudSessionState::Interrupted,
        }
    }
}

/// Scrollback event - append-only diff of newly added lines
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollbackEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// Newly appended lines since last update
    pub diff: String,
    /// Total line count (for verification)
    pub total_lines: usize,
}

impl ScrollbackEvent {
    pub fn new(diff: String, total_lines: usize) -> Self {
        Self {
            event_type: "scrollback".to_string(),
            diff,
            total_lines,
        }
    }
}

/// Scrollback history event - full accumulated scrollback for initial sync
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScrollbackHistoryEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// Full accumulated scrollback content
    pub content: String,
}

impl ScrollbackHistoryEvent {
    pub fn new(content: String) -> Self {
        Self {
            event_type: "scrollback_history".to_string(),
            content,
        }
    }
}

/// State change event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StateEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub state: CloudSessionState,
    /// Unix timestamp (ms)
    pub timestamp: u64,
}

impl StateEvent {
    pub fn new(state: CloudSessionState) -> Self {
        Self {
            event_type: "state".to_string(),
            state,
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

/// Liveness-only event.
///
/// Heartbeats keep the cloud session active when the wrapper is still running
/// but the underlying assistant has not produced output for a long time.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HeartbeatEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// Unix timestamp (ms)
    pub timestamp: u64,
}

impl HeartbeatEvent {
    pub fn new() -> Self {
        Self {
            event_type: "heartbeat".to_string(),
            timestamp: chrono::Utc::now().timestamp_millis() as u64,
        }
    }
}

/// Git file status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFile {
    pub path: String,
    /// Git porcelain format: "M ", "??", "A ", etc.
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Recent git commit metadata.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitCommit {
    pub hash: String,
    pub short_hash: String,
    /// Unix timestamp (seconds)
    pub timestamp: u64,
    pub subject: String,
}

/// Git status event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repo_owner: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub repo_name: String,
    pub branch: String,
    pub files: Vec<GitFile>,
    /// Bounded recent commit log, newest first.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_commits: Vec<GitCommit>,
}

impl GitEvent {
    pub fn new(
        repo_owner: String,
        repo_name: String,
        branch: String,
        files: Vec<GitFile>,
        recent_commits: Vec<GitCommit>,
    ) -> Self {
        Self {
            event_type: "git".to_string(),
            repo_owner,
            repo_name,
            branch,
            files,
            recent_commits,
        }
    }
}

/// Code change (function, method, class modification)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CodeChange {
    /// "Function", "Method", "Class", etc.
    pub kind: String,
    /// Symbol name
    pub name: String,
    /// "added", "modified", "deleted"
    pub change_type: String,
    pub additions: usize,
    pub deletions: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_number: Option<usize>,
}

/// Changes grouped by language
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LanguageChanges {
    pub language: String,
    pub changes: Vec<CodeChange>,
}

/// Code changes event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangesEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub by_language: Vec<LanguageChanges>,
}

impl ChangesEvent {
    pub fn new(by_language: Vec<LanguageChanges>) -> Self {
        Self {
            event_type: "changes".to_string(),
            by_language,
        }
    }
}

/// Permission suggestion for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    #[serde(rename = "type")]
    pub suggestion_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub behavior: Option<String>,
}

/// Permission option extracted from screen content
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionOptionCloud {
    /// Option number (1, 2, 3, etc.)
    pub number: u32,
    /// Full text of the option
    pub text: String,
    /// Whether this option is currently selected
    pub selected: bool,
}

/// Permission details for dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PermissionInfo {
    pub tool: String,
    pub suggestions: Vec<PermissionSuggestion>,
    /// Options extracted from screen content (the actual menu items shown to user)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Vec<PermissionOptionCloud>>,
    /// The question being asked (e.g., "Do you want to create test-file.txt?")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub question: Option<String>,
}

/// Session statistics event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub prompts: u32,
    pub completions: u32,
    pub tools: u32,
    pub compressions: u32,
    pub thinking_seconds: u64,
    pub work_seconds: u64,
    /// Current Claude Code mode (normal, auto_accept, plan)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mode: Option<String>,
    /// Permission details when in permission state
    #[serde(skip_serializing_if = "Option::is_none")]
    pub permission: Option<PermissionInfo>,
    /// Model name (e.g., "claude-opus-4-5-20251101")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// Unix timestamp when prompts count last changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prompts_changed_at: Option<f64>,
    /// Unix timestamp when completions count last changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub completions_changed_at: Option<f64>,
    /// Unix timestamp when compressions count last changed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compressions_changed_at: Option<f64>,
    /// Unix timestamps of tool invocations for sparkline
    pub tool_timestamps: Vec<f64>,
    /// Unix timestamp when session started
    pub session_start: f64,
    /// Unix timestamp when session became idle (for idle time display)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub idle_since: Option<f64>,
    /// Autocomplete suggestion text from Claude Code input line
    #[serde(skip_serializing_if = "Option::is_none")]
    pub suggestion: Option<String>,
}

impl StatsEvent {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        prompts: u32,
        completions: u32,
        tools: u32,
        compressions: u32,
        thinking_seconds: u64,
        work_seconds: u64,
        mode: Option<String>,
        permission: Option<PermissionInfo>,
        model: Option<String>,
        prompts_changed_at: Option<f64>,
        completions_changed_at: Option<f64>,
        compressions_changed_at: Option<f64>,
        tool_timestamps: Vec<f64>,
        session_start: f64,
        idle_since: Option<f64>,
        suggestion: Option<String>,
    ) -> Self {
        Self {
            event_type: "stats".to_string(),
            prompts,
            completions,
            tools,
            compressions,
            thinking_seconds,
            work_seconds,
            mode,
            permission,
            model,
            prompts_changed_at,
            completions_changed_at,
            compressions_changed_at,
            tool_timestamps,
            session_start,
            idle_since,
            suggestion,
        }
    }
}

/// ANSI screen snapshot (for late joiners)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreenEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// ANSI-escaped screen content
    pub content: String,
}

impl ScreenEvent {
    pub fn new(content: String) -> Self {
        Self {
            event_type: "screen".to_string(),
            content,
        }
    }
}

/// Terminal title event (from OSC sequences)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// Terminal title extracted from OSC sequences
    pub title: String,
}

impl TitleEvent {
    pub fn new(title: String) -> Self {
        Self {
            event_type: "title".to_string(),
            title,
        }
    }
}

/// Title history event - all titles from the session
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TitleHistoryEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// All terminal titles from this session
    pub history: Vec<String>,
}

/// Slack permalinks pasted during this session.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SlackThreadsEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub threads: Vec<crate::slack::SlackThread>,
}

impl SlackThreadsEvent {
    pub fn new(threads: Vec<crate::slack::SlackThread>) -> Self {
        Self {
            event_type: "slack_threads".to_string(),
            threads,
        }
    }
}

/// Enriched metadata for Slack permalinks attached to tracked PRs (origin and
/// GitHub comment links), so the web PR board can label them like the desktop.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrSlackThreadsEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub threads: Vec<crate::slack::SlackThread>,
}

impl PrSlackThreadsEvent {
    pub fn new(threads: Vec<crate::slack::SlackThread>) -> Self {
        Self {
            event_type: "pr_slack_threads".to_string(),
            threads,
        }
    }
}

impl TitleHistoryEvent {
    pub fn new(history: Vec<String>) -> Self {
        Self {
            event_type: "title_history".to_string(),
            history,
        }
    }
}

/// A selectable option in a prompt
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptOption {
    /// Display label for the option
    pub label: String,
    /// Value to send back when selected (e.g., "1", "y", "n")
    pub value: String,
    /// Optional description
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// A question with options for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CloudQuestion {
    /// The question text
    pub question: String,
    /// Short header/label
    #[serde(skip_serializing_if = "Option::is_none")]
    pub header: Option<String>,
    /// Available options
    pub options: Vec<PromptOption>,
    /// Whether multiple selections are allowed
    #[serde(default)]
    pub multi_select: bool,
    /// Whether free-text "Other" input is allowed
    #[serde(default)]
    pub allows_other: bool,
}

/// One answered question on the "Review your answers" page
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct QuestionReviewAnswer {
    pub question: String,
    pub answer: String,
}

/// Prompt data for the dashboard
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "prompt_type", rename_all = "snake_case")]
pub enum CloudPromptData {
    /// AskUserQuestion prompt. The page fields mirror the terminal dialog
    /// (Claude's pages, Grok's question card) so the dashboard can follow
    /// checkbox toggles and page changes that send no hook event. They are
    /// absent when the screen could not be read.
    Question {
        questions: Vec<CloudQuestion>,
        /// Index into `questions` of the page on screen
        #[serde(default, skip_serializing_if = "Option::is_none")]
        current_question: Option<usize>,
        /// Ticked rows on a multi-select page (1-indexed; the "Type
        /// something" row is `options.len() + 1`)
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        checked: Vec<u32>,
        /// Text typed into the "Type something" row
        #[serde(default, skip_serializing_if = "Option::is_none")]
        custom_text: Option<String>,
        /// Row the terminal cursor is on: the options, then "Type
        /// something", then Submit on multi-select pages
        #[serde(default, skip_serializing_if = "Option::is_none")]
        cursor_row: Option<u32>,
        /// The review page: every question with the answer it will send
        #[serde(default, skip_serializing_if = "Option::is_none")]
        review: Option<Vec<QuestionReviewAnswer>>,
        /// `"grok_card"` when Grok's native question card is on screen.
        /// The dashboard uses Grok keys (digits, Space, ←/→) instead of
        /// Claude's numbered rows.
        #[serde(default, skip_serializing_if = "Option::is_none")]
        ui: Option<String>,
    },
    /// Permission request for a tool
    Permission {
        tool_name: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        tool_input: Option<serde_json::Value>,
        options: Vec<PromptOption>,
        /// Whether "Tab to add additional instructions" is available
        #[serde(skip_serializing_if = "Option::is_none")]
        allows_tab_instructions: Option<bool>,
        /// Currently selected option number (1-indexed, for navigation)
        #[serde(skip_serializing_if = "Option::is_none")]
        selected_option: Option<u32>,
    },
    /// ExitPlanMode (plan approval)
    ExitPlan { options: Vec<PromptOption> },
}

/// Prompt event - sent when entering/leaving interactive states
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PromptEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// The active prompt, or None to clear
    pub prompt: Option<CloudPromptData>,
}

impl PromptEvent {
    pub fn new(prompt: Option<CloudPromptData>) -> Self {
        Self {
            event_type: "prompt".to_string(),
            prompt,
        }
    }
}

/// Union of all cloud event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CloudEvent {
    Scrollback(ScrollbackEvent),
    ScrollbackHistory(ScrollbackHistoryEvent),
    State(StateEvent),
    Heartbeat(HeartbeatEvent),
    Git(GitEvent),
    Changes(ChangesEvent),
    Stats(StatsEvent),
    Screen(ScreenEvent),
    Title(TitleEvent),
    TitleHistory(TitleHistoryEvent),
    SlackThreads(SlackThreadsEvent),
    PrSlackThreads(PrSlackThreadsEvent),
    Prompt(PromptEvent),
    Recap(RecapEvent),
    RecapHistory(RecapHistoryEvent),
    Prs(PrsEvent),
}

/// Pull requests created or updated during this session (for the recap panel).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PrsEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub prs: Vec<crate::pr::SessionPr>,
}

impl PrsEvent {
    pub fn new(prs: Vec<crate::pr::SessionPr>) -> Self {
        Self {
            event_type: "prs".to_string(),
            prs,
        }
    }
}

/// Latest recap state for the dashboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    /// One of: "ready", "updating", "failed", "missing_key", "waiting", "disabled".
    pub status: String,
    /// Failure message when status is "failed".
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Latest finished recap. None while updating, missing key, or failed.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub latest: Option<crate::recap::TurnRecap>,
    /// Lines added/deleted across the latest turn.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub line_delta: Option<crate::recap::TurnLineDelta>,
}

impl RecapEvent {
    pub fn from_state(state: &crate::recap::RecapState) -> Self {
        use crate::recap::RecapStatus;
        let (status, error) = match &state.status {
            RecapStatus::Disabled => ("disabled", None),
            RecapStatus::MissingKey => ("missing_key", None),
            RecapStatus::Waiting => ("waiting", None),
            RecapStatus::Updating => ("updating", None),
            RecapStatus::Ready => ("ready", None),
            RecapStatus::Failed(error) => ("failed", Some(error.clone())),
        };
        Self {
            event_type: "recap".to_string(),
            status: status.to_string(),
            error,
            latest: state.latest.clone(),
            line_delta: state.line_delta,
        }
    }
}

/// Full recap history (all completed recaps in this session, oldest first).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RecapHistoryEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub history: Vec<crate::recap::TurnRecap>,
}

impl RecapHistoryEvent {
    pub fn new(history: Vec<crate::recap::TurnRecap>) -> Self {
        Self {
            event_type: "recap_history".to_string(),
            history,
        }
    }
}

/// A single step in a key sequence
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum KeyStep {
    /// Send a named key: "up", "down", "tab", "enter"
    Key { key: String },
    /// Type raw text
    Text { text: String },
    /// Wait for a delay in milliseconds
    Delay { ms: u32 },
}

/// Message from cloud to desktop (via WebSocket)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CloudToDesktopMessage {
    #[serde(rename = "answer")]
    Answer { text: String },
    #[serde(rename = "ping")]
    Ping,
    /// Send a key sequence (e.g., Shift+Tab for mode switching)
    #[serde(rename = "key")]
    Key { key: String },
    /// Send a multi-step key sequence (for Tab instructions)
    #[serde(rename = "key_sequence")]
    KeySequence { steps: Vec<KeyStep> },
    /// Notify desktop that viewer activity status changed
    /// Desktop can use this to adjust streaming frequency
    #[serde(rename = "viewer_status")]
    ViewerStatus { active: bool },
    /// The group's PR dispositions changed (a ★/☆, ↑/↓, or ✕ click on the
    /// dashboard or an action link); the desktop should refetch them now
    /// instead of waiting for its next poll.
    #[serde(rename = "pr_overrides_changed")]
    PrOverridesChanged,
    /// Request to spawn a new crabigator instance in a directory
    #[serde(rename = "spawn")]
    Spawn {
        cwd: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        platform: Option<String>,
    },
}

/// The page fields of a question prompt, read from the terminal screen.
#[derive(Debug, Default, PartialEq, Eq)]
struct QuestionPageState {
    current_question: Option<usize>,
    checked: Vec<u32>,
    custom_text: Option<String>,
    cursor_row: Option<u32>,
    review: Option<Vec<QuestionReviewAnswer>>,
}

impl QuestionPageState {
    /// Match the dialog on screen against the questions the hook reported.
    fn from_screen(
        questions: &[crate::platforms::Question],
        screen: Option<&crate::parsers::QuestionScreen>,
    ) -> Self {
        use crate::parsers::QuestionScreen;

        match screen {
            None => Self::default(),
            Some(QuestionScreen::Review { answers }) => Self {
                review: Some(
                    answers
                        .iter()
                        .map(|a| QuestionReviewAnswer {
                            question: a.question.clone(),
                            answer: a.answer.clone(),
                        })
                        .collect(),
                ),
                ..Self::default()
            },
            Some(QuestionScreen::Page {
                question,
                rows,
                submit_row,
                page_index,
            }) => {
                let current_question = page_index
                    .filter(|&i| i < questions.len())
                    .or_else(|| find_question(questions, question));
                let option_count = current_question
                    .map(|i| questions[i].options.len() as u32)
                    .unwrap_or_else(|| rows.len().saturating_sub(1) as u32);
                let custom_row = option_count + 1;
                let submit_row_number = option_count + 2;

                let checked = rows
                    .iter()
                    .filter(|r| r.checked == Some(true))
                    .map(|r| r.number)
                    .collect();
                let custom_text = rows
                    .iter()
                    .find(|r| r.number == custom_row && !r.is_placeholder())
                    .map(|r| r.label.clone())
                    .filter(|label| !label.is_empty());
                let cursor_row = rows
                    .iter()
                    .find(|r| r.cursor)
                    .map(|r| r.number)
                    .or_else(|| (*submit_row == Some(true)).then_some(submit_row_number));

                Self {
                    current_question,
                    checked,
                    custom_text,
                    cursor_row,
                    review: None,
                }
            }
        }
    }
}

/// Find which question the screen shows. The screen wraps long questions
/// and may cut them off, so accept a prefix match either way.
fn find_question(questions: &[crate::platforms::Question], shown: &str) -> Option<usize> {
    fn normalize(text: &str) -> String {
        text.split_whitespace().collect::<Vec<_>>().join(" ")
    }
    let shown = normalize(shown);
    if shown.is_empty() {
        return None;
    }
    let normalized: Vec<String> = questions.iter().map(|q| normalize(&q.question)).collect();
    normalized.iter().position(|q| *q == shown).or_else(|| {
        normalized
            .iter()
            .position(|q| !q.is_empty() && (q.starts_with(&shown) || shown.starts_with(q)))
    })
}

/// Helper for building events from crabigator's internal state
pub struct SessionEventBuilder;

impl SessionEventBuilder {
    /// Build a scrollback event from new lines
    pub fn scrollback(diff: String, total_lines: usize) -> CloudEvent {
        CloudEvent::Scrollback(ScrollbackEvent::new(diff, total_lines))
    }

    /// Build a scrollback history event for initial sync
    pub fn scrollback_history(content: String) -> CloudEvent {
        CloudEvent::ScrollbackHistory(ScrollbackHistoryEvent::new(content))
    }

    /// Build a state event
    pub fn state(state: crate::platforms::SessionState) -> CloudEvent {
        CloudEvent::State(StateEvent::new(state.into()))
    }

    /// Build a liveness heartbeat event
    pub fn heartbeat() -> CloudEvent {
        CloudEvent::Heartbeat(HeartbeatEvent::new())
    }

    /// Build a screen event
    pub fn screen(content: String) -> CloudEvent {
        CloudEvent::Screen(ScreenEvent::new(content))
    }

    /// Build a title event
    pub fn title(title: String) -> CloudEvent {
        CloudEvent::Title(TitleEvent::new(title))
    }

    /// Build a title history event
    pub fn title_history(history: Vec<String>) -> CloudEvent {
        CloudEvent::TitleHistory(TitleHistoryEvent::new(history))
    }

    pub fn slack_threads(threads: Vec<crate::slack::SlackThread>) -> CloudEvent {
        CloudEvent::SlackThreads(SlackThreadsEvent::new(threads))
    }

    pub fn pr_slack_threads(threads: Vec<crate::slack::SlackThread>) -> CloudEvent {
        CloudEvent::PrSlackThreads(PrSlackThreadsEvent::new(threads))
    }

    pub fn recap(state: &crate::recap::RecapState) -> CloudEvent {
        CloudEvent::Recap(RecapEvent::from_state(state))
    }

    pub fn recap_history(history: Vec<crate::recap::TurnRecap>) -> CloudEvent {
        CloudEvent::RecapHistory(RecapHistoryEvent::new(history))
    }

    pub fn prs(prs: Vec<crate::pr::SessionPr>) -> CloudEvent {
        CloudEvent::Prs(PrsEvent::new(prs))
    }

    /// Build a git status event
    pub fn git(git_state: &GitState) -> CloudEvent {
        let files = git_state
            .files
            .iter()
            .map(|file| GitFile {
                path: file.path.clone(),
                status: file.status.clone(),
                additions: file.additions,
                deletions: file.deletions,
            })
            .collect();

        let recent_commits = git_state
            .recent_commits
            .iter()
            .map(|commit| GitCommit {
                hash: commit.hash.clone(),
                short_hash: commit.short_hash.clone(),
                timestamp: commit.timestamp,
                subject: commit.subject.clone(),
            })
            .collect();

        CloudEvent::Git(GitEvent::new(
            git_state.repo_owner.clone(),
            git_state.repo_name.clone(),
            git_state.branch.clone(),
            files,
            recent_commits,
        ))
    }

    /// Build a changes event from diff summary
    pub fn changes(diff_summary: &DiffSummary) -> CloudEvent {
        let by_language = diff_summary
            .by_language()
            .into_iter()
            .map(|lang| LanguageChanges {
                language: lang.language,
                changes: lang
                    .changes
                    .into_iter()
                    .map(|change| CodeChange {
                        kind: node_kind_label(&change.kind).to_string(),
                        name: change.name,
                        change_type: change_type_label(&change.change_type).to_string(),
                        additions: change.additions,
                        deletions: change.deletions,
                        file_path: change.file_path,
                        line_number: change.line_number,
                    })
                    .collect(),
            })
            .collect();

        CloudEvent::Changes(ChangesEvent::new(by_language))
    }

    /// Build a stats event from session stats
    ///
    /// `permission_prompt` is parsed from the screen content and provides
    /// the actual menu options and question text.
    pub fn stats(
        session_stats: &crate::hooks::SessionStats,
        permission_prompt: Option<&crate::parsers::PermissionPrompt>,
        suggestion: Option<String>,
    ) -> CloudEvent {
        let stats = &session_stats.platform_stats;
        let total_tools: u32 = stats.tools.values().sum();

        // Convert permission details - use screen-parsed options even if hook data is missing
        let permission = if stats.permission.is_some() || permission_prompt.is_some() {
            // Convert screen-parsed options to cloud format
            let options = permission_prompt.as_ref().map(|p| {
                p.options
                    .iter()
                    .map(|o| PermissionOptionCloud {
                        number: o.number,
                        text: o.text.clone(),
                        selected: o.selected,
                    })
                    .collect()
            });

            // Get question from parsed prompt
            let question = permission_prompt.and_then(|p| p.question.clone());

            // Use hook data if available, otherwise create minimal permission info
            let (tool, suggestions) = if let Some(p) = &stats.permission {
                (
                    p.tool.clone(),
                    p.suggestions
                        .iter()
                        .map(|s| PermissionSuggestion {
                            suggestion_type: s.suggestion_type.clone(),
                            mode: s.mode.clone(),
                            behavior: s.behavior.clone(),
                        })
                        .collect(),
                )
            } else {
                ("unknown".to_string(), vec![])
            };

            Some(PermissionInfo {
                tool,
                suggestions,
                options,
                question,
            })
        } else {
            None
        };

        CloudEvent::Stats(StatsEvent::new(
            stats.prompts,
            stats.completions,
            total_tools,
            stats.compressions,
            session_stats.thinking_seconds(),
            session_stats.work_seconds,
            Some(stats.mode.as_str().to_string()),
            permission,
            stats.model.clone(),
            session_stats.prompts_changed_at,
            session_stats.completions_changed_at,
            session_stats.compressions_changed_at,
            stats.tool_timestamps.clone(),
            session_stats.session_start_unix(),
            stats.idle_since,
            suggestion,
        ))
    }

    /// Build a prompt event from active prompt
    /// `permission_prompt` should be parsed from the screen when prompt is Permission
    pub fn prompt(
        active_prompt: Option<&crate::platforms::ActivePrompt>,
        permission_prompt: Option<&crate::parsers::PermissionPrompt>,
        question_screen: Option<&crate::parsers::QuestionScreen>,
        question_ui: Option<&str>,
    ) -> CloudEvent {
        use crate::platforms::ActivePrompt;

        let prompt_data = active_prompt.map(|ap| match ap {
            ActivePrompt::Question { questions } => {
                let cloud_questions = questions
                    .iter()
                    .map(|q| CloudQuestion {
                        question: q.question.clone(),
                        header: q.header.clone(),
                        options: q
                            .options
                            .iter()
                            .enumerate()
                            .map(|(i, opt)| PromptOption {
                                label: opt.label.clone(),
                                value: (i + 1).to_string(), // 1-indexed
                                description: opt.description.clone(),
                            })
                            .collect(),
                        multi_select: q.multi_select,
                        allows_other: true, // AskUserQuestion always allows "Other"
                    })
                    .collect();
                let page = QuestionPageState::from_screen(questions, question_screen);
                CloudPromptData::Question {
                    questions: cloud_questions,
                    current_question: page.current_question,
                    checked: page.checked,
                    custom_text: page.custom_text,
                    cursor_row: page.cursor_row,
                    review: page.review,
                    ui: question_ui.map(str::to_string),
                }
            }
            ActivePrompt::Permission {
                tool_name,
                tool_input,
            } => {
                // Convert parsed prompt options to PromptOption format
                let options = permission_prompt
                    .map(|p| {
                        p.options
                            .iter()
                            .map(|o| PromptOption {
                                label: o.text.clone(),
                                value: o.number.to_string(),
                                description: None,
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        vec![
                            PromptOption {
                                label: "Yes".to_string(),
                                value: "y".to_string(),
                                description: Some("Allow once".to_string()),
                            },
                            PromptOption {
                                label: "No".to_string(),
                                value: "n".to_string(),
                                description: Some("Deny".to_string()),
                            },
                        ]
                    });

                // Extract allows_tab_instructions and selected_option from parsed prompt
                let allows_tab_instructions = permission_prompt.map(|p| p.allows_tab_instructions);
                let selected_option = permission_prompt
                    .and_then(|p| p.options.iter().find(|o| o.selected).map(|o| o.number));

                CloudPromptData::Permission {
                    tool_name: tool_name.clone(),
                    tool_input: tool_input.clone(),
                    options,
                    allows_tab_instructions,
                    selected_option,
                }
            }
            ActivePrompt::ExitPlan => {
                // Convert parsed prompt options to PromptOption format
                let options = permission_prompt
                    .map(|p| {
                        p.options
                            .iter()
                            .map(|o| PromptOption {
                                label: o.text.clone(),
                                value: o.number.to_string(),
                                description: None,
                            })
                            .collect()
                    })
                    .unwrap_or_else(|| {
                        vec![
                            PromptOption {
                                label: "Yes".to_string(),
                                value: "1".to_string(),
                                description: None,
                            },
                            PromptOption {
                                label: "No".to_string(),
                                value: "2".to_string(),
                                description: None,
                            },
                        ]
                    });
                CloudPromptData::ExitPlan { options }
            }
        });

        CloudEvent::Prompt(PromptEvent::new(prompt_data))
    }
}

fn node_kind_label(kind: &NodeKind) -> &'static str {
    match kind {
        NodeKind::Function => "Function",
        NodeKind::Method => "Method",
        NodeKind::Class => "Class",
        NodeKind::Struct => "Struct",
        NodeKind::Enum => "Enum",
        NodeKind::Trait => "Trait",
        NodeKind::Impl => "Impl",
        NodeKind::Module => "Module",
        NodeKind::Const => "Const",
        NodeKind::Other => "Other",
    }
}

fn change_type_label(change_type: &ChangeType) -> &'static str {
    match change_type {
        ChangeType::Added => "added",
        ChangeType::Modified => "modified",
        ChangeType::Deleted => "deleted",
    }
}

#[cfg(test)]
mod question_page_tests {
    use super::*;
    use crate::parsers::QuestionScreen;
    use crate::platforms::{Question, QuestionOption};

    fn questions() -> Vec<Question> {
        let option = |label: &str| QuestionOption {
            label: label.to_string(),
            description: None,
        };
        vec![
            Question {
                question: "Which crust?".to_string(),
                header: Some("Crust".to_string()),
                options: vec![option("Thin"), option("Thick"), option("Stuffed")],
                multi_select: false,
            },
            Question {
                question: "Which toppings do you want on the pizza?".to_string(),
                header: Some("Toppings".to_string()),
                options: vec![
                    option("Cheese"),
                    option("Pepperoni"),
                    option("Mushrooms"),
                    option("Olives"),
                ],
                multi_select: true,
            },
        ]
    }

    const MULTI_PAGE: &str = "\
←  ☒ Crust  ☐ Toppings  ✔ Submit  →
Which toppings do you want on the
pizza?
  1. [✔] Cheese
  2. [ ] Pepperoni
  3. [✔] Mushrooms
  4. [ ] Olives
  5. [✔] extra garlic
❯    Submit
────────────
  6. Chat about this
";

    #[test]
    fn mirrors_a_multi_select_page() {
        let screen = QuestionScreen::parse(MULTI_PAGE).unwrap();
        let page = QuestionPageState::from_screen(&questions(), Some(&screen));
        assert_eq!(page.current_question, Some(1));
        assert_eq!(page.checked, vec![1, 3, 5]);
        assert_eq!(page.custom_text.as_deref(), Some("extra garlic"));
        assert_eq!(page.cursor_row, Some(6));
        assert!(page.review.is_none());
    }

    #[test]
    fn mirrors_a_single_select_page() {
        let screen = QuestionScreen::parse(
            "←  ☐ Crust  ☐ Toppings  ✔ Submit  →\nWhich crust?\n  1. Thin\n  2. Thick\n  3. Stuffed\n❯ 4. Type something.\n",
        )
        .unwrap();
        let page = QuestionPageState::from_screen(&questions(), Some(&screen));
        assert_eq!(page.current_question, Some(0));
        assert!(page.checked.is_empty());
        assert_eq!(page.custom_text, None);
        assert_eq!(page.cursor_row, Some(4));
    }

    #[test]
    fn mirrors_the_review_page() {
        let screen = QuestionScreen::parse(
            "←  ☒ Crust  ☒ Toppings  ✔ Submit  →\nReview your answers\n ● Which crust?\n   → Thick\nReady to submit your answers?\n❯ 1. Submit answers\n  2. Cancel\n",
        )
        .unwrap();
        let page = QuestionPageState::from_screen(&questions(), Some(&screen));
        assert_eq!(page.current_question, None);
        let review = page.review.expect("review answers");
        assert_eq!(review.len(), 1);
        assert_eq!(review[0].answer, "Thick");
    }

    #[test]
    fn leaves_page_fields_empty_without_a_screen() {
        assert_eq!(
            QuestionPageState::from_screen(&questions(), None),
            QuestionPageState::default()
        );
    }

    #[test]
    fn serializes_page_fields_only_when_present() {
        let event = SessionEventBuilder::prompt(
            Some(&crate::platforms::ActivePrompt::Question {
                questions: questions(),
            }),
            None,
            None,
            None,
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"multi_select\":true"));
        assert!(!json.contains("current_question"));
        assert!(!json.contains("\"checked\""));
        assert!(!json.contains("\"review\""));
        assert!(!json.contains("\"ui\""));
    }

    #[test]
    fn grok_custom_text_row_is_the_cursor_while_typing() {
        let screen = QuestionScreen::parse(
            "┃\n┃  What is your favorite color?\n┃\n┃  1 (○) Red\n┃  2 (○) Blue\n┃  3 (○) Green\n┃  z (●) ❯ periwinkle\n┃\n┃  ↑/↓ navigate · y copy\n",
        )
        .unwrap();
        let option = |label: &str| QuestionOption {
            label: label.to_string(),
            description: None,
        };
        let questions = vec![Question {
            question: "What is your favorite color?".to_string(),
            header: None,
            options: vec![option("Red"), option("Blue"), option("Green")],
            multi_select: false,
        }];
        let page = QuestionPageState::from_screen(&questions, Some(&screen));
        assert_eq!(page.current_question, Some(0));
        assert_eq!(page.custom_text.as_deref(), Some("periwinkle"));
        assert_eq!(page.cursor_row, Some(4));
    }

    #[test]
    fn mirrors_a_grok_checkbox_page() {
        let screen = QuestionScreen::parse(
            "┃\n┃  Which toppings do you want on the pizza?\n┃\n┃  1 [x] Cheese\n┃  2 [ ] Pepperoni\n┃  3 [x] Mushrooms\n┃  4 [ ] Olives\n┃  z [x] extra garlic\n┃\n┃  [2/4] ↑/↓ navigate · ←/→ question · y copy\n",
        )
        .unwrap();
        let page = QuestionPageState::from_screen(&questions(), Some(&screen));
        assert_eq!(page.current_question, Some(1));
        assert_eq!(page.checked, vec![1, 3, 5]);
        assert_eq!(page.custom_text.as_deref(), Some("extra garlic"));
    }

    #[test]
    fn serializes_grok_card_ui() {
        let event = SessionEventBuilder::prompt(
            Some(&crate::platforms::ActivePrompt::Question {
                questions: questions(),
            }),
            None,
            None,
            Some("grok_card"),
        );
        let json = serde_json::to_string(&event).unwrap();
        assert!(json.contains("\"ui\":\"grok_card\""));
    }
}
