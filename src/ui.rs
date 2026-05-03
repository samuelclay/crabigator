//! UI modules for status bar rendering
//!
//! Each widget is responsible for rendering its own section of the status bar.
//! Widgets use raw ANSI escape sequences for terminal output.

mod changes;
mod git;
mod handoff;
mod pairing;
pub mod sparkline;
mod stats;
mod status_bar;
pub mod utils;

pub use changes::draw_changes_widget;
pub use git::draw_git_widget;
pub use handoff::{draw_recap_handoff, HANDOFF_RESERVED_ROWS};
pub use pairing::{draw_pairing_banner, draw_update_banner, PairingState};
pub use stats::{draw_stats_widget, throbber_frame_index};
pub use status_bar::{draw_status_bar, split_terminal_rows, Layout};

/// Common layout parameters for widget rendering
#[derive(Clone, Copy)]
pub struct WidgetArea {
    pub pty_rows: u16,
    pub col: u16,
    pub row: u16,
    pub width: u16,
    pub height: u16,
}
