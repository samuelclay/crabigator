use regex::Regex;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct PythonParser;

impl DiffParser for PythonParser {
    fn language(&self) -> &'static str {
        "Python"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".py")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        // Python hunk context: "def function_name(" or "async def function_name(" or "class ClassName"
        let def_re = Regex::new(r"(?:async\s+)?def\s+(\w+)").unwrap();
        let class_re = Regex::new(r"class\s+(\w+)").unwrap();

        if let Some(caps) = def_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = class_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        None
    }

    fn parse(&self, diff: &str, _filename: &str) -> Vec<ChangeNode> {
        let mut changes = Vec::new();

        // Regex patterns for Python constructs
        let class_re = Regex::new(r"^class\s+(\w+)").unwrap();
        let def_re = Regex::new(r"^(\s*)def\s+(\w+)").unwrap();
        let async_def_re = Regex::new(r"^(\s*)async\s+def\s+(\w+)").unwrap();

        let mut current_class: Option<ChangeNode> = None;
        let mut class_indent: usize = 0;

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
                // Save previous class if any
                if let Some(class_node) = current_class.take() {
                    if !class_node.children.is_empty() {
                        changes.push(class_node);
                    }
                }

                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                class_indent = content.len() - content.trim_start().len();

                current_class = Some(ChangeNode {
                    kind: NodeKind::Class,
                    name: name.to_string(),
                    change_type: ChangeType::Modified,
                    children: Vec::new(),
                });
                continue;
            }

            // Check for function/method definitions
            let def_match = def_re.captures(content).or_else(|| async_def_re.captures(content));

            if let Some(caps) = def_match {
                let indent_str = caps.get(1).map(|m| m.as_str()).unwrap_or("");
                let indent = indent_str.len();
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");

                // Skip dunder methods except __init__
                if name.starts_with("__") && name.ends_with("__") && name != "__init__" {
                    continue;
                }

                // If we're inside a class and this is indented, it's a method
                if let Some(ref mut class_node) = current_class {
                    if indent > class_indent {
                        if !class_node.children.iter().any(|c| c.name == name) {
                            class_node.children.push(ChangeNode {
                                kind: NodeKind::Method,
                                name: name.to_string(),
                                change_type: change_type.clone(),
                                children: Vec::new(),
                            });
                        }
                        continue;
                    } else {
                        // We've exited the class
                        let class_node = current_class.take().unwrap();
                        if !class_node.children.is_empty() {
                            changes.push(class_node);
                        }
                    }
                }

                // Top-level function
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
