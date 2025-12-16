//! Shared types for diff parsing

#[derive(Clone, Debug, PartialEq)]
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
    Added,
    Modified,
    Deleted,
}

#[derive(Clone, Debug)]
pub struct ChangeNode {
    pub kind: NodeKind,
    pub name: String,
    #[allow(dead_code)]
    pub change_type: ChangeType,
    pub children: Vec<ChangeNode>,
}

#[derive(Clone, Debug)]
pub struct FileChanges {
    #[allow(dead_code)]
    pub path: String,
    pub changes: Vec<ChangeNode>,
}
