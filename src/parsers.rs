//! Language-specific diff parsers
//!
//! Parses git diffs to extract semantic information about code changes.

mod generic;
mod python;
mod rust;
mod summary;
mod types;
mod typescript;

pub use generic::GenericParser;
pub use python::PythonParser;
pub use rust::RustParser;
pub use summary::{DiffParser, DiffSummary};
pub use types::{ChangeNode, ChangeType, NodeKind};
pub use typescript::TypeScriptParser;
