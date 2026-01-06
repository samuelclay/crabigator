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
}

impl From<crate::platforms::SessionState> for CloudSessionState {
    fn from(state: crate::platforms::SessionState) -> Self {
        match state {
            crate::platforms::SessionState::Ready => CloudSessionState::Ready,
            crate::platforms::SessionState::Thinking => CloudSessionState::Thinking,
            crate::platforms::SessionState::Permission => CloudSessionState::Permission,
            crate::platforms::SessionState::Question => CloudSessionState::Question,
            crate::platforms::SessionState::Complete => CloudSessionState::Complete,
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

/// Git file status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitFile {
    pub path: String,
    /// Git porcelain format: "M ", "??", "A ", etc.
    pub status: String,
    pub additions: usize,
    pub deletions: usize,
}

/// Git status event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GitEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub branch: String,
    pub files: Vec<GitFile>,
}

impl GitEvent {
    pub fn new(branch: String, files: Vec<GitFile>) -> Self {
        Self {
            event_type: "git".to_string(),
            branch,
            files,
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

/// Session statistics event
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StatsEvent {
    #[serde(rename = "type")]
    pub event_type: String,
    pub prompts: u32,
    pub completions: u32,
    pub tools: u32,
    pub thinking_seconds: u64,
    pub work_seconds: u64,
}

impl StatsEvent {
    pub fn new(
        prompts: u32,
        completions: u32,
        tools: u32,
        thinking_seconds: u64,
        work_seconds: u64,
    ) -> Self {
        Self {
            event_type: "stats".to_string(),
            prompts,
            completions,
            tools,
            thinking_seconds,
            work_seconds,
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

/// Union of all cloud event types
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum CloudEvent {
    Scrollback(ScrollbackEvent),
    State(StateEvent),
    Git(GitEvent),
    Changes(ChangesEvent),
    Stats(StatsEvent),
    Screen(ScreenEvent),
    Title(TitleEvent),
}

/// Message from cloud to desktop (via WebSocket)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum CloudToDesktopMessage {
    #[serde(rename = "answer")]
    Answer { text: String },
    #[serde(rename = "ping")]
    Ping,
}

/// Helper for building events from crabigator's internal state
pub struct SessionEventBuilder;

impl SessionEventBuilder {
    /// Build a scrollback event from new lines
    pub fn scrollback(diff: String, total_lines: usize) -> CloudEvent {
        CloudEvent::Scrollback(ScrollbackEvent::new(diff, total_lines))
    }

    /// Build a state event
    pub fn state(state: crate::platforms::SessionState) -> CloudEvent {
        CloudEvent::State(StateEvent::new(state.into()))
    }

    /// Build a screen event
    pub fn screen(content: String) -> CloudEvent {
        CloudEvent::Screen(ScreenEvent::new(content))
    }

    /// Build a title event
    pub fn title(title: String) -> CloudEvent {
        CloudEvent::Title(TitleEvent::new(title))
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

        CloudEvent::Git(GitEvent::new(git_state.branch.clone(), files))
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

    /// Build a stats event from platform stats
    pub fn stats(
        stats: &crate::platforms::PlatformStats,
        work_seconds: u64,
        thinking_seconds: u64,
    ) -> CloudEvent {
        let total_tools: u32 = stats.tools.values().sum();
        CloudEvent::Stats(StatsEvent::new(
            stats.prompts,
            stats.completions,
            total_tools,
            thinking_seconds,
            work_seconds,
        ))
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
