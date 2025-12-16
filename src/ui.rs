//! UI modules for status bar rendering
//!
//! Each widget is responsible for rendering its own section of the status bar.
//! Widgets use raw ANSI escape sequences for terminal output.

mod changes;
mod git;
mod stats;
mod status_bar;
pub mod utils;

pub use changes::draw_changes_widget;
pub use git::draw_git_widget;
pub use stats::draw_stats_widget;
pub use status_bar::{draw_status_bar, Layout};
