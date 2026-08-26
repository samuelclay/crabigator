//! Platform abstraction layer
//!
//! Defines a common interface for different AI assistant platforms.
//! Currently supports Claude Code and Codex CLI, with extensibility for future platforms.

pub mod claude_code;
pub mod codex_cli;
pub mod opencode;

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformKind {
    #[default]
    Claude,
    Codex,
    Opencode,
}

impl PlatformKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Some(Self::Claude),
            "codex" | "codecs" | "openai" => Some(Self::Codex),
            "opencode" | "open-code" | "open_code" => Some(Self::Opencode),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
            Self::Opencode => "opencode",
        }
    }

    pub fn command(self) -> &'static str {
        self.as_str()
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
            Self::Opencode => "opencode",
        }
    }
}

/// Session state - common states across supported assistants
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Initial state - nothing has happened yet (default on startup)
    #[default]
    Ready,
    /// The assistant is actively processing/generating
    Thinking,
    /// The assistant is waiting for permission approval
    Permission,
    /// The assistant asked a question and is waiting for response
    Question,
    /// The assistant finished responding
    Complete,
    /// The user interrupted the assistant (ESC/Ctrl+C during thinking)
    Interrupted,
}

/// Claude Code operating mode (cycles via Shift+Tab)
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ClaudeMode {
    /// Normal mode - no special indicators
    #[default]
    Normal,
    /// Auto-accept mode - shows "accept edits" indicator
    AutoAccept,
    /// Plan mode - shows "plan mode" indicator
    Plan,
}

impl ClaudeMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Normal => "normal",
            Self::AutoAccept => "auto_accept",
            Self::Plan => "plan",
        }
    }
}

/// An option in an AskUserQuestion prompt
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct QuestionOption {
    /// Display label for the option
    pub label: String,
    /// Optional description of what this option does
    #[serde(default)]
    pub description: Option<String>,
}

/// A single question in an AskUserQuestion prompt
#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct Question {
    /// The question text
    pub question: String,
    /// Short header/label for the question
    #[serde(default)]
    pub header: Option<String>,
    /// Available options to choose from
    #[serde(default)]
    pub options: Vec<QuestionOption>,
    /// Whether multiple options can be selected
    #[serde(default, rename = "multiSelect")]
    pub multi_select: bool,
}

/// Active prompt awaiting user response
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum ActivePrompt {
    /// AskUserQuestion prompt with structured options
    Question { questions: Vec<Question> },
    /// Permission request for a tool
    Permission {
        tool_name: String,
        #[serde(default)]
        tool_input: Option<serde_json::Value>,
    },
    /// ExitPlanMode prompt (plan approval)
    ExitPlan,
}

/// A single hook event entry for debugging
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct HookEvent {
    /// Unix timestamp when event occurred
    pub ts: f64,
    /// Event name (e.g., "PermissionRequest", "PostToolUse")
    pub event: String,
    /// State before the event was processed
    #[serde(default)]
    pub state_before: String,
    /// Additional event-specific details
    #[serde(default)]
    pub details: Option<HashMap<String, serde_json::Value>>,
}

/// Permission suggestion from Claude Code
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PermissionSuggestion {
    #[serde(rename = "type")]
    pub suggestion_type: String,
    #[serde(default)]
    pub mode: Option<String>,
    #[serde(default)]
    pub rules: Option<Vec<serde_json::Value>>,
    #[serde(default)]
    pub behavior: Option<String>,
    #[serde(default)]
    pub destination: Option<String>,
}

/// Permission request details
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PermissionDetails {
    pub tool: String,
    #[serde(default)]
    pub input: serde_json::Value,
    #[serde(default)]
    pub suggestions: Vec<PermissionSuggestion>,
}

/// Statistics collected from a platform's hook system
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlatformStats {
    /// Number of user prompts submitted
    #[serde(default)]
    pub prompts: u32,
    /// Number of assistant responses completed
    #[serde(alias = "messages")]
    pub completions: u32,
    /// Number of subagent task completions
    pub subagent_messages: u32,
    /// Number of context compressions
    pub compressions: u32,
    /// Tool usage counts by tool name
    #[serde(default)]
    pub tools: HashMap<String, u32>,
    /// Unix timestamps of tool calls (for sparkline visualization)
    #[serde(default)]
    pub tool_timestamps: Vec<f64>,
    /// Current session state
    #[serde(default)]
    pub state: SessionState,
    /// Unix timestamp when idle state began (complete/question)
    #[serde(default)]
    pub idle_since: Option<f64>,
    /// Unix timestamp of last update
    pub last_updated: Option<f64>,
    /// Rolling log of hook events for debugging
    #[serde(default)]
    pub event_history: Vec<HookEvent>,
    /// Current Claude Code operating mode (detected from screen)
    #[serde(default)]
    pub mode: ClaudeMode,
    /// Current permission request details (when state is Permission)
    #[serde(default)]
    pub permission: Option<PermissionDetails>,
    /// Model name (e.g., "claude-opus-4-5-20251101")
    #[serde(default)]
    pub model: Option<String>,
    /// Currently active prompt awaiting user response
    #[serde(default)]
    pub active_prompt: Option<ActivePrompt>,
    /// Path to Claude Code transcript JSONL file
    #[serde(default)]
    pub transcript_path: Option<String>,
}

impl PlatformStats {
    /// Get total number of tool calls across all tools
    pub fn total_tool_calls(&self) -> u32 {
        self.tools.values().sum()
    }

    /// Get top N tools by usage count
    #[allow(dead_code)]
    pub fn top_tools(&self, n: usize) -> Vec<(&str, u32)> {
        let mut sorted: Vec<_> = self.tools.iter().map(|(k, v)| (k.as_str(), *v)).collect();
        sorted.sort_by_key(|item| std::cmp::Reverse(item.1));
        sorted.truncate(n);
        sorted
    }
}

/// Trait for platform-specific implementations
pub trait Platform {
    /// Platform identifier
    fn kind(&self) -> PlatformKind;

    /// Command to launch the platform CLI
    fn command(&self) -> &'static str;

    /// Adjust the pass-through CLI arguments for this platform's flag
    /// dialect and prepend any arguments the platform needs to be
    /// observable (default: unchanged).
    fn spawn_args(&self, user_args: Vec<String>) -> Vec<String> {
        user_args
    }

    /// Whether the platform CLI is a full-screen TUI that renders on the
    /// alternate screen. Crabigator strips the buffer switch so the CLI
    /// paints inside the scroll region on the primary buffer (its final
    /// frame survives in scrollback), and answers the CLI's terminal
    /// capability queries, which would otherwise go unanswered and block
    /// its first paint.
    fn uses_alt_screen(&self) -> bool {
        false
    }

    /// Ensure hooks are installed and up-to-date
    fn ensure_hooks_installed(&self) -> Result<()>;

    /// Record that the user sent input to this pane (a keystroke, a paste,
    /// or a remote command injected into the PTY). Platforms that follow the
    /// pane's conversation across in-pane resumes use this to tell a real
    /// resume from a concurrent same-cwd session in another pane: a resume
    /// always starts with input, an idle pane receives none. Default: ignored.
    fn note_user_input(&self) {}

    /// Load current stats from the platform's data source
    fn load_stats(&self, cwd: &str) -> Result<PlatformStats>;

    /// Clean up stats file on exit (default: no-op)
    fn cleanup_stats(&self, _cwd: &str) {}
}

pub fn platform_for(kind: PlatformKind) -> Box<dyn Platform> {
    match kind {
        PlatformKind::Claude => Box::new(claude_code::ClaudeCodePlatform::new()),
        PlatformKind::Codex => Box::new(codex_cli::CodexPlatform::new()),
        PlatformKind::Opencode => Box::new(opencode::OpencodePlatform::new()),
    }
}
