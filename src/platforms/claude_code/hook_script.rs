//! Python hook script for Claude Code stats tracking
//!
//! The hook script handles Claude Code events and writes session stats
//! to a JSON file that crabigator reads for its stats widget.

/// Current hook version - should match Cargo.toml version
pub const HOOK_VERSION: &str = env!("CARGO_PKG_VERSION");

/// Python hook script content (loaded from stats_hook.py at compile time)
///
/// Registered for every hook event in `HOOK_EVENTS` (claude_code.rs). Each
/// event appends a line to the session's `activity.jsonl` for the flow
/// column; the events that move the session's state or counters also update
/// the stats file.
pub const HOOK_SCRIPT: &str = include_str!("stats_hook.py");

/// Get the hook script content with version embedded
pub fn script_with_version() -> String {
    HOOK_SCRIPT.replace("{VERSION}", HOOK_VERSION)
}
