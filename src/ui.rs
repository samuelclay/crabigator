//! UI modules for status bar rendering
//!
//! Each widget is responsible for rendering its own section of the status bar.
//! Widgets use raw ANSI escape sequences for terminal output.

mod changes;
pub(crate) mod cooldown;
mod flow;
mod git;
pub(crate) mod handoff;
mod pairing;
pub(crate) mod pr_cells;
pub mod sparkline;
mod stats;
mod status_bar;
mod time;
pub mod utils;

pub use changes::changes_natural_rows;
pub(crate) use changes::render_changes_rows;
pub use flow::{FlowRect, FlowView};
pub use git::git_natural_rows;
pub(crate) use git::render_git_rows;
pub use handoff::{
    draw_pr_handoff, draw_pr_separator, draw_recap_handoff, pr_handoff_rows, pr_separator_rows,
    recap_handoff_rows, total_handoff_rows, MAX_RECAP_ROWS,
};
pub use pairing::{draw_pairing_banner, draw_update_banner, PairingState};
pub use stats::{stats_render_rows, stats_use_compact_layout, throbber_frame_index};
pub(crate) use stats::{
    render_stats_rows, session_state_badge, session_state_icon, STATE_BADGE_WIDTH,
};
pub use status_bar::{
    compute_dynamic_status_rows, draw_status_bar, flow_column_rect, handoff_rows,
    split_terminal_rows, Layout, StatusBarRenderer,
};

pub(crate) const PROMPT_ICON: &str = "⟩";
pub(crate) const COMPLETION_ICON: &str = "⋖";
