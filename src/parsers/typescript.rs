use regex::Regex;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct TypeScriptParser;

impl DiffParser for TypeScriptParser {
    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".ts")
            || filename.ends_with(".tsx")
            || filename.ends_with(".js")
            || filename.ends_with(".jsx")
    }

    fn parse(&self, diff: &str, _filename: &str) -> Vec<ChangeNode> {
        let mut changes = Vec::new();

        // Regex patterns for TypeScript/JavaScript constructs
        let class_re = Regex::new(r"^\s*(export\s+)?(abstract\s+)?class\s+(\w+)").unwrap();
        let function_re = Regex::new(
            r"^\s*(export\s+)?(async\s+)?function\s+(\w+)|^\s*(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?\(",
        )
        .unwrap();
        let method_re =
            Regex::new(r"^\s*(public|private|protected|static|async|\s)*(\w+)\s*\([^)]*\)\s*[:{]")
                .unwrap();
        let arrow_fn_re =
            Regex::new(r"^\s*(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?(\([^)]*\)|[^=])\s*=>")
                .unwrap();
        let interface_re = Regex::new(r"^\s*(export\s+)?interface\s+(\w+)").unwrap();
        let type_re = Regex::new(r"^\s*(export\s+)?type\s+(\w+)").unwrap();

        let mut current_class: Option<ChangeNode> = None;

        for line in diff.lines() {
            let is_added = line.starts_with('+') && !line.starts_with("+++");
            let is_removed = line.starts_with('-') && !line.starts_with("---");

            if !is_added && !is_removed {
                continue;
            }

            let change_type = if is_added {
                ChangeType::Added
            } else {
                ChangeType::Deleted
            };

            let content = &line[1..];

            // Check for class definitions
            if let Some(caps) = class_re.captures(content) {
                if let Some(class_node) = current_class.take() {
                    if !class_node.children.is_empty() {
                        changes.push(class_node);
                    }
                }

                let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                current_class = Some(ChangeNode {
                    kind: NodeKind::Class,
                    name: name.to_string(),
                    change_type: ChangeType::Modified,
                    children: Vec::new(),
                });
                continue;
            }

            // Check for interface definitions
            if let Some(caps) = interface_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Trait) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Trait, // Using Trait for interfaces
                        name: name.to_string(),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
                continue;
            }

            // Check for type definitions
            if let Some(caps) = type_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Other) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Other,
                        name: format!("type {}", name),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
                continue;
            }

            // Check for function declarations
            if let Some(caps) = function_re.captures(content) {
                let name = caps
                    .get(3)
                    .or_else(|| caps.get(6))
                    .map(|m| m.as_str())
                    .unwrap_or("unknown");

                if current_class.is_none() {
                    if !changes
                        .iter()
                        .any(|c| c.name == name && c.kind == NodeKind::Function)
                    {
                        changes.push(ChangeNode {
                            kind: NodeKind::Function,
                            name: name.to_string(),
                            change_type: change_type.clone(),
                            children: Vec::new(),
                        });
                    }
                }
                continue;
            }

            // Check for arrow functions
            if let Some(caps) = arrow_fn_re.captures(content) {
                let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");

                if current_class.is_none() {
                    if !changes
                        .iter()
                        .any(|c| c.name == name && c.kind == NodeKind::Function)
                    {
                        changes.push(ChangeNode {
                            kind: NodeKind::Function,
                            name: name.to_string(),
                            change_type: change_type.clone(),
                            children: Vec::new(),
                        });
                    }
                }
                continue;
            }

            // Check for methods (inside classes)
            if current_class.is_some() {
                if let Some(caps) = method_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");

                    // Skip constructor and common keywords
                    if name != "constructor"
                        && name != "if"
                        && name != "for"
                        && name != "while"
                        && name != "switch"
                    {
                        if let Some(ref mut class_node) = current_class {
                            if !class_node.children.iter().any(|c| c.name == name) {
                                class_node.children.push(ChangeNode {
                                    kind: NodeKind::Method,
                                    name: name.to_string(),
                                    change_type: change_type.clone(),
                                    children: Vec::new(),
                                });
                            }
                        }
                    }
                }
            }
        }

        // Don't forget the last class
        if let Some(class_node) = current_class {
            if !class_node.children.is_empty() {
                changes.push(class_node);
            }
        }

        changes
    }
}
