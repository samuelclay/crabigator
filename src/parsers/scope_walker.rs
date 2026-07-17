//! Shared diff-walking engine with scope tracking and line numbers.
//!
//! Walks unified diff lines while maintaining an indentation-based stack of
//! named scopes, so changed lines attribute to the innermost enclosing
//! definition. Each resulting node carries its enclosing-scope chain (for
//! `parent › name` disambiguation) and the new-file line number of the first
//! line seen for it (for IDE hyperlinks).

use std::collections::HashMap;
use std::sync::LazyLock;

use regex::Regex;

use super::types::{ChangeNode, ChangeType, NodeKind};

/// A definition matched on a single line of code.
pub struct Definition {
    pub kind: NodeKind,
    pub name: String,
    /// Label pushed onto the scope chain for children; defaults to `name`.
    pub label: Option<String>,
    /// Whether the label is part of the node's own identity (test blocks
    /// keyed by their title).
    pub self_scoped: bool,
    /// Whether the definition opens a body that nested changes attribute to.
    pub opens_scope: bool,
}

impl Definition {
    /// A definition that opens a body scope (function, class, impl, ...).
    pub fn scoped(kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            label: None,
            self_scoped: false,
            opens_scope: true,
        }
    }

    /// A definition without a tracked body (type alias, const, ...).
    pub fn leaf(kind: NodeKind, name: impl Into<String>) -> Self {
        Self {
            kind,
            name: name.into(),
            label: None,
            self_scoped: false,
            opens_scope: false,
        }
    }

    /// A block keyed by a display title, like `describe('title', ...)` —
    /// the title joins the node's own scope chain so same-named blocks split.
    pub fn titled_block(name: impl Into<String>, title: impl Into<String>) -> Self {
        Self {
            kind: NodeKind::Function,
            name: name.into(),
            label: Some(title.into()),
            self_scoped: true,
            opens_scope: true,
        }
    }
}

/// Language hooks for [`walk_diff`].
pub trait ScopeRules {
    /// Match a definition on a line of code (diff prefix stripped).
    fn match_definition(&self, content: &str) -> Option<Definition>;

    /// Match the enclosing definition named in a hunk header. Hunk contexts
    /// are free-form, so languages may match more loosely than
    /// `match_definition`. Defaults to `match_definition`.
    fn match_hunk_context(&self, context: &str) -> Option<Definition> {
        self.match_definition(context)
    }
}

/// A named scope on the nesting stack while walking diff lines.
struct ScopeEntry {
    /// Indentation (spaces) of the line that opened this scope.
    indent: usize,
    /// Label children prepend to their scope chain.
    label: String,
    /// The node key that changed lines inside this scope attribute to.
    key: ChangeKey,
}

type ChangeKey = (NodeKind, String, Vec<String>);
type ChangeStats = (ChangeType, usize, usize, Option<usize>);

/// Count a changed definition line toward its own node.
fn record_definition(
    change_map: &mut HashMap<ChangeKey, ChangeStats>,
    key: &ChangeKey,
    is_added: bool,
    is_removed: bool,
    line_number: Option<usize>,
) {
    if !is_added && !is_removed {
        return;
    }
    let entry = change_map.entry(key.clone()).or_insert((
        if is_added {
            ChangeType::Added
        } else {
            ChangeType::Deleted
        },
        0,
        0,
        line_number,
    ));
    if is_added {
        entry.1 += 1;
    } else {
        entry.2 += 1;
    }
}

/// Count a changed line toward the enclosing scope's node.
fn bump(
    change_map: &mut HashMap<ChangeKey, ChangeStats>,
    key: &ChangeKey,
    is_added: bool,
    line_number: Option<usize>,
) {
    let entry = change_map
        .entry(key.clone())
        .or_insert((ChangeType::Modified, 0, 0, line_number));
    if is_added {
        entry.1 += 1;
    } else {
        entry.2 += 1;
    }
}

/// Build the node key and scope-stack entry for a matched definition.
fn definition_entry(definition: Definition, enclosing: Vec<String>, indent: usize) -> ScopeEntry {
    let label = definition.label.unwrap_or_else(|| definition.name.clone());
    let mut scope = enclosing;
    if definition.self_scoped {
        scope.push(label.clone());
    }
    ScopeEntry {
        indent,
        label,
        key: (definition.kind, definition.name, scope),
    }
}

/// Walk a single-file diff, attributing changed lines to definitions.
pub fn walk_diff(rules: &dyn ScopeRules, diff: &str, filename: &str) -> Vec<ChangeNode> {
    static HUNK_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@\s*(.*)$").unwrap());

    let file_path = Some(filename.to_string());
    // Key: (kind, name, scope), Value: (change_type, additions, deletions, line)
    let mut change_map: HashMap<ChangeKey, ChangeStats> = HashMap::new();
    // Stack of enclosing named scopes, innermost last
    let mut scope_stack: Vec<ScopeEntry> = Vec::new();
    // Current line number in the new file, advanced by context/added lines
    let mut new_line: usize = 0;

    for line in diff.lines() {
        // Hunk headers reset scope tracking to the header's context
        if let Some(caps) = HUNK_RE.captures(line) {
            scope_stack.clear();
            new_line = caps
                .get(1)
                .and_then(|m| m.as_str().parse().ok())
                .unwrap_or(0);
            if let Some(definition) = caps
                .get(2)
                .map(|c| c.as_str())
                .filter(|c| !c.trim().is_empty())
                .and_then(|c| rules.match_hunk_context(c))
            {
                let entry = definition_entry(definition, Vec::new(), 0);
                change_map.entry(entry.key.clone()).or_insert((
                    ChangeType::Modified,
                    0,
                    0,
                    Some(new_line),
                ));
                scope_stack.push(entry);
            }
            continue;
        }

        let is_added = line.starts_with('+') && !line.starts_with("+++");
        let is_removed = line.starts_with('-') && !line.starts_with("---");
        let is_context = line.starts_with(' ');
        if !is_added && !is_removed && !is_context {
            continue;
        }

        // Removed lines don't exist in the new file, so they don't advance
        let line_number = (new_line > 0).then_some(new_line);
        if is_added || is_context {
            new_line += 1;
        }

        let content = &line[1..];
        let trimmed = content.trim_start();
        if trimmed.is_empty() {
            continue; // blank lines carry no indentation signal
        }
        let indent = content.len() - trimmed.len();

        // Closing braces belong to the scope they terminate: attribute
        // them before popping. Everything else pops outdented scopes first.
        if trimmed.starts_with('}') || trimmed.starts_with(')') {
            if is_added || is_removed {
                if let Some(entry) = scope_stack.last() {
                    bump(&mut change_map, &entry.key, is_added, line_number);
                }
            }
            scope_stack.retain(|s| s.indent < indent);
            continue;
        }
        scope_stack.retain(|s| s.indent < indent);

        if let Some(definition) = rules.match_definition(content) {
            let enclosing: Vec<String> = scope_stack.iter().map(|s| s.label.clone()).collect();
            let opens_scope = definition.opens_scope;
            let entry = definition_entry(definition, enclosing, indent);
            record_definition(
                &mut change_map,
                &entry.key,
                is_added,
                is_removed,
                line_number,
            );
            if opens_scope {
                scope_stack.push(entry);
            }
        } else if is_added || is_removed {
            // Not a definition line: attribute to the innermost scope
            if let Some(entry) = scope_stack.last() {
                bump(&mut change_map, &entry.key, is_added, line_number);
            }
        }
    }

    // Convert map to vec of ChangeNodes
    change_map
        .into_iter()
        .map(
            |((kind, name, scope), (change_type, additions, deletions, line_number))| ChangeNode {
                kind,
                name,
                scope,
                change_type,
                additions,
                deletions,
                file_path: file_path.clone(),
                line_number,
                children: Vec::new(),
            },
        )
        .collect()
}
