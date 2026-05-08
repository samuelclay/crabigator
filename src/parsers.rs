//! Language-specific diff parsers
//!
//! Parses git diffs to extract semantic information about code changes.

mod generic;
mod objc;
pub mod permission_prompt;
mod python;
mod rust;
pub mod suggestion;
mod summary;
mod swift;
mod types;
mod typescript;

pub use generic::GenericParser;
pub use objc::ObjCParser;
pub use permission_prompt::{is_interrupted, strip_ansi_for_debug, PermissionPrompt};
pub use python::PythonParser;
pub use rust::RustParser;
pub use suggestion::SuggestionTracker;
pub use summary::{DiffParser, DiffSummary};
pub use swift::SwiftParser;
#[allow(unused_imports)]
pub use types::{ChangeNode, ChangeType, FileChanges, LanguageChanges, NodeKind};
pub use typescript::TypeScriptParser;
