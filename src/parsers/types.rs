//! Shared types for diff parsing

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum NodeKind {
    Class,
    Function,
    Method,
    Struct,
    Enum,
    Trait,
    Impl,
    Module,
    Const,
    Other,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ChangeType {
    /// A new definition was added
    Added,
    /// Code was modified inside an existing definition
    Modified,
    /// A definition was deleted
    Deleted,
}

#[derive(Clone, Debug)]
pub struct ChangeNode {
    pub kind: NodeKind,
    pub name: String,
    pub change_type: ChangeType,
    pub additions: usize,
    pub deletions: usize,
    /// File path (relative to repo root) for hyperlink generation
    pub file_path: Option<String>,
    /// Line number where the symbol is defined (1-indexed)
    pub line_number: Option<usize>,
    #[allow(dead_code)]
    pub children: Vec<ChangeNode>,
}

#[derive(Clone, Debug)]
pub struct FileChanges {
    #[allow(dead_code)]
    pub path: String,
    pub language: String,
    pub changes: Vec<ChangeNode>,
}

/// Changes grouped by language for display
#[derive(Clone, Debug, Default)]
pub struct LanguageChanges {
    pub language: String,
    pub changes: Vec<ChangeNode>,
}
