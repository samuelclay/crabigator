//! Platform abstraction layer
//!
//! Defines a common interface for different AI assistant platforms.
//! Supports Claude Code and Codex.

pub mod claude_code;
pub mod codex;

use std::collections::HashMap;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// Session state - the 4 possible states Claude can be in
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SessionState {
    /// Initial state - nothing has happened yet (default on startup)
    #[default]
    Ready,
    /// Claude is actively processing/generating
    Thinking,
    /// Claude asked a question and is waiting for response
    Question,
    /// Claude finished responding
    Complete,
}

/// Statistics collected from a platform's hook system
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlatformStats {
    /// Number of assistant messages/responses
    pub messages: u32,
    /// Number of subagent task completions
    pub subagent_messages: u32,
    /// Number of context compressions
    pub compressions: u32,
    /// Tool usage counts by tool name
    #[serde(default)]
    pub tools: HashMap<String, u32>,
    /// Current session state
    #[serde(default)]
    pub state: SessionState,
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
    /// Ensure hooks are installed and up-to-date
    fn ensure_hooks_installed(&self) -> Result<()>;

    /// Load current stats from the platform's data source
    fn load_stats(&self, cwd: &str) -> Result<PlatformStats>;
}

/// Supported platform types
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PlatformType {
    /// Claude Code (Anthropic)
    ClaudeCode,
    /// Codex (OpenAI)
    Codex,
}

impl PlatformType {
    /// Get the CLI command name for this platform
    pub fn cli_name(&self) -> &'static str {
        match self {
            PlatformType::ClaudeCode => "claude",
            PlatformType::Codex => "codex",
        }
    }

    /// Get display name for UI
    pub fn display_name(&self) -> &'static str {
        match self {
            PlatformType::ClaudeCode => "Claude",
            PlatformType::Codex => "Codex",
        }
    }

    /// Parse platform type from string (case-insensitive)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "claude" | "claude-code" | "claude_code" => Some(PlatformType::ClaudeCode),
            "codex" | "openai" => Some(PlatformType::Codex),
            _ => None,
        }
    }
}

impl Default for PlatformType {
    fn default() -> Self {
        PlatformType::ClaudeCode
    }
}

/// Get a platform implementation by type
pub fn get_platform(platform_type: PlatformType) -> Box<dyn Platform + Send + Sync> {
    match platform_type {
        PlatformType::ClaudeCode => Box::new(claude_code::ClaudeCodePlatform::new()),
        PlatformType::Codex => Box::new(codex::CodexPlatform::new()),
    }
}
