//! Platform abstraction layer
//!
//! Defines a common interface for different AI assistant platforms.
//! Currently supports Claude Code and Codex CLI, with extensibility for future platforms.

pub mod claude_code;
pub mod codex_cli;

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlatformKind {
    #[default]
    Claude,
    Codex,
}

impl PlatformKind {
    pub fn parse(value: &str) -> Option<Self> {
        match value.to_ascii_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Some(Self::Claude),
            "codex" | "codecs" | "openai" => Some(Self::Codex),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    pub fn command(self) -> &'static str {
        self.as_str()
    }

    pub fn display_name(self) -> &'static str {
        match self {
            Self::Claude => "Claude",
            Self::Codex => "Codex",
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
        sorted.sort_by(|a, b| b.1.cmp(&a.1));
        sorted.truncate(n);
        sorted
    }
}

/// Trait for platform-specific implementations
pub trait Platform {
    /// Platform identifier
    #[allow(dead_code)]
    fn kind(&self) -> PlatformKind;

    /// Command to launch the platform CLI
    fn command(&self) -> &'static str;

    /// Ensure hooks are installed and up-to-date
    fn ensure_hooks_installed(&self) -> Result<()>;

    /// Load current stats from the platform's data source
    fn load_stats(&self, cwd: &str) -> Result<PlatformStats>;

    /// Clean up stats file on exit (default: no-op)
    fn cleanup_stats(&self, _cwd: &str) {}
}

pub fn platform_for(kind: PlatformKind) -> Box<dyn Platform> {
    match kind {
        PlatformKind::Claude => Box::new(claude_code::ClaudeCodePlatform::new()),
        PlatformKind::Codex => Box::new(codex_cli::CodexPlatform::new()),
    }
}
