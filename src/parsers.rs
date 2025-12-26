//! Language-specific diff parsers
//!
//! Parses git diffs to extract semantic information about code changes.

mod generic;
mod objc;
mod python;
mod rust;
mod summary;
mod swift;
mod types;
mod typescript;

pub use generic::GenericParser;
pub use objc::ObjCParser;
pub use python::PythonParser;
pub use rust::RustParser;
pub use summary::{DiffParser, DiffSummary};
pub use swift::SwiftParser;
pub use types::{ChangeNode, ChangeType, LanguageChanges, NodeKind};
pub use typescript::TypeScriptParser;
