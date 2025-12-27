//! Python hook script for Claude Code stats tracking
//!
//! The hook script handles Claude Code events and writes session stats
//! to a JSON file that crabigator reads for its stats widget.

/// Current hook version - should match Cargo.toml version
pub const HOOK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Python hook script content (loaded from stats_hook.py at compile time)
///
/// Handles these Claude Code events:
/// - PermissionRequest: Permission dialog shown
/// - PostToolUse: Tool execution completed
/// - Stop: Claude finished responding
/// - SubagentStop: Subagent task completed
/// - PreCompact: Context compression triggered
/// - UserPromptSubmit: User submitted input
pub const HOOK_SCRIPT: &str = include_str!("stats_hook.py");

/// Get the hook script content with version embedded
pub fn script_with_version() -> String {
    HOOK_SCRIPT.replace("{VERSION}", HOOK_VERSION)
}
