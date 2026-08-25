//! UI modules for status bar rendering
//!
//! Each widget is responsible for rendering its own section of the status bar.
//! Widgets use raw ANSI escape sequences for terminal output.

mod changes;
pub(crate) mod cooldown;
mod git;
pub(crate) mod handoff;
mod pairing;
pub(crate) mod pr_cells;
pub mod sparkline;
mod stats;
mod status_bar;
mod time;
pub mod utils;

pub use changes::{changes_natural_rows, draw_changes_widget};
pub use git::{draw_git_widget, git_natural_rows};
pub use handoff::{
    draw_pr_handoff, draw_pr_separator, draw_recap_handoff, pr_handoff_rows, pr_separator_rows,
    recap_handoff_rows, total_handoff_rows, MAX_RECAP_ROWS,
};
pub use pairing::{draw_pairing_banner, draw_update_banner, PairingState};
pub use stats::{
    draw_stats_widget, stats_render_rows, stats_use_compact_layout, throbber_frame_index,
};
pub(crate) use stats::{session_state_badge, session_state_icon, STATE_BADGE_WIDTH};
pub use status_bar::{
    compute_dynamic_status_rows, draw_status_bar, handoff_rows, split_terminal_rows, Layout,
};

pub(crate) const PROMPT_ICON: &str = "⟩";
pub(crate) const COMPLETION_ICON: &str = "⋖";

/// Common layout parameters for widget rendering
#[derive(Clone, Copy)]
pub struct WidgetArea {
    pub pty_rows: u16,
    pub col: u16,
    pub row: u16,
    pub width: u16,
    pub height: u16,
}
