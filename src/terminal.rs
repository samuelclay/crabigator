//! Terminal handling module
//!
//! Groups all terminal-related functionality:
//! - ANSI escape sequences
//! - Input encoding
//! - PTY management

pub mod escape;
pub mod input;
pub mod pty;

pub use input::forward_key_to_pty;
pub use pty::ClaudePty;
