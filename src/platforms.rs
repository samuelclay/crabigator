//! Platform abstraction layer
//!
//! Defines a common interface for different AI assistant platforms.
//! Currently supports Claude Code, with extensibility for future platforms.

pub mod claude_code;

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

/// Get the current platform implementation
/// Currently only supports Claude Code
pub fn current_platform() -> impl Platform {
    claude_code::ClaudeCodePlatform::new()
}
