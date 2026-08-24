//! Terminal handling module - V2 simplified
//!
//! Groups all terminal-related functionality:
//! - ANSI escape sequences
//! - DSR (Device Status Report) handling
//! - Input encoding
//! - OSC (Operating System Command) scanning
//! - PTY management

pub mod dsr;
pub mod escape;
pub mod ghostty;
pub mod input;
pub mod osc;
pub mod pty;
pub mod queries;

pub mod redraw;

pub use dsr::{DsrChunk, DsrHandler};
pub use input::forward_key_to_pty;
pub use osc::OscScanner;
pub use pty::PlatformPty;
pub use queries::QueryResponder;
pub use redraw::ScrollRegionFilter;
