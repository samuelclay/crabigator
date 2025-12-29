use regex::Regex;
use std::collections::HashMap;

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

    fn parse(&self, diff: &str, filename: &str) -> Vec<ChangeNode> {
        let file_path = Some(filename.to_string());
        // Track changes with their line counts
        // Key: (kind, name), Value: (change_type, additions, deletions)
        let mut change_map: HashMap<(NodeKind, String), (ChangeType, usize, usize)> = HashMap::new();

        // Regex patterns for Python constructs
        let class_re = Regex::new(r"^class\s+(\w+)").unwrap();
        let def_re = Regex::new(r"^(\s*)(?:async\s+)?def\s+(\w+)").unwrap();
        let hunk_re = Regex::new(r"^@@[^@]+@@\s*(.*)$").unwrap();

        // Current context: which function/class we're inside
        let mut current_context: Option<(NodeKind, String)> = None;

        for line in diff.lines() {
            // Check for hunk headers with function context
            if let Some(caps) = hunk_re.captures(line) {
                if let Some(context) = caps.get(1) {
                    let context_str = context.as_str();
                    if let Some(fn_name) = self.extract_function_from_context(context_str) {
                        let key = (NodeKind::Function, fn_name.clone());
                        change_map.entry(key.clone()).or_insert((ChangeType::Modified, 0, 0));
                        current_context = Some(key);
                    } else {
                        current_context = None;
                    }
                } else {
                    current_context = None;
                }
                continue;
            }

            let is_added = line.starts_with('+') && !line.starts_with("+++");
            let is_removed = line.starts_with('-') && !line.starts_with("---");
            let is_context = line.starts_with(' ');

            // Check context lines for function/class definitions to track current scope
            if is_context {
                let content = &line[1..];
                // Check for class definitions in context
                if let Some(caps) = class_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Class, name.to_string()));
                }
                // Check for function/method definitions in context
                else if let Some(caps) = def_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    // Skip dunder methods except __init__
                    if !(name.starts_with("__") && name.ends_with("__") && name != "__init__") {
                        current_context = Some((NodeKind::Function, name.to_string()));
                    }
                }
                continue;
            }

            if !is_added && !is_removed {
                continue;
            }

            let content = &line[1..];
            let mut found_definition = false;

            // Check for class definitions
            if let Some(caps) = class_re.captures(content) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                let key = (NodeKind::Class, name.to_string());
                let entry = change_map.entry(key.clone()).or_insert((
                    if is_added { ChangeType::Added } else { ChangeType::Deleted },
                    0,
                    0,
                ));
                if is_added { entry.1 += 1; } else { entry.2 += 1; }
                current_context = Some(key);
                found_definition = true;
            }

            // Check for function/method definitions
            if !found_definition {
                if let Some(caps) = def_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");

                    // Skip dunder methods except __init__
                    if name.starts_with("__") && name.ends_with("__") && name != "__init__" {
                        continue;
                    }

                    let key = (NodeKind::Function, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // If not a definition line, add to current context
            if !found_definition {
                if let Some(ref key) = current_context {
                    let entry = change_map
                        .entry(key.clone())
                        .or_insert((ChangeType::Modified, 0, 0));
                    if is_added {
                        entry.1 += 1;
                    } else {
                        entry.2 += 1;
                    }
                }
            }
        }

        // Convert map to vec of ChangeNodes
        change_map
            .into_iter()
            .map(|((kind, name), (change_type, additions, deletions))| ChangeNode {
                kind,
                name,
                change_type,
                additions,
                deletions,
                file_path: file_path.clone(),
                line_number: None, // TODO: extract from hunk headers
                children: Vec::new(),
            })
            .collect()
    }
}
