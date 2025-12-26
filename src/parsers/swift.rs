use regex::Regex;
use std::collections::HashMap;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct SwiftParser;

impl DiffParser for SwiftParser {
    fn language(&self) -> &'static str {
        "Swift"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".swift")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        // Swift hunk context patterns:
        // "func name(" or "private func name(" or "public func name("
        // "class Name" or "struct Name" or "enum Name"
        let fn_re = Regex::new(r"(?:(?:public|private|internal|fileprivate|open)\s+)?(?:static\s+)?func\s+(\w+)").unwrap();
        let class_re = Regex::new(r"(?:(?:public|private|internal|fileprivate|open)\s+)?(?:final\s+)?class\s+(\w+)").unwrap();
        let struct_re = Regex::new(r"(?:(?:public|private|internal|fileprivate|open)\s+)?struct\s+(\w+)").unwrap();
        let enum_re = Regex::new(r"(?:(?:public|private|internal|fileprivate|open)\s+)?enum\s+(\w+)").unwrap();
        let extension_re = Regex::new(r"extension\s+(\w+)").unwrap();

        if let Some(caps) = fn_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = class_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = struct_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = enum_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = extension_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        None
    }

    fn parse(&self, diff: &str, _filename: &str) -> Vec<ChangeNode> {
        let mut change_map: HashMap<(NodeKind, String), (ChangeType, usize, usize)> = HashMap::new();

        // Regex patterns for Swift constructs
        let fn_re = Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?(?:static\s+)?(?:override\s+)?func\s+(\w+)").unwrap();
        let class_re = Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?(?:final\s+)?class\s+(\w+)").unwrap();
        let struct_re = Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?struct\s+(\w+)").unwrap();
        let enum_re = Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?enum\s+(\w+)").unwrap();
        let protocol_re = Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?protocol\s+(\w+)").unwrap();
        let extension_re = Regex::new(r"^\s*extension\s+(\w+)").unwrap();
        let hunk_re = Regex::new(r"^@@[^@]+@@\s*(.*)$").unwrap();

        let mut current_context: Option<(NodeKind, String)> = None;

        for line in diff.lines() {
            // Check for hunk headers with function context
            if let Some(caps) = hunk_re.captures(line) {
                if let Some(context) = caps.get(1) {
                    let context_str = context.as_str();
                    if let Some(fn_caps) = fn_re.captures(context_str) {
                        let fn_name = fn_caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                        current_context = Some((NodeKind::Function, fn_name.to_string()));
                        let key = (NodeKind::Function, fn_name.to_string());
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0));
                    } else if let Some(class_caps) = class_re.captures(context_str) {
                        let name = class_caps.get(1).map(|m| m.as_str()).unwrap_or("Unknown");
                        current_context = Some((NodeKind::Class, name.to_string()));
                        let key = (NodeKind::Class, name.to_string());
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0));
                    } else if let Some(ext_caps) = extension_re.captures(context_str) {
                        let name = ext_caps.get(1).map(|m| m.as_str()).unwrap_or("Unknown");
                        current_context = Some((NodeKind::Impl, name.to_string()));
                        let key = (NodeKind::Impl, name.to_string());
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0));
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

            if !is_added && !is_removed {
                continue;
            }

            let content = &line[1..];
            let mut found_definition = false;

            // Check for classes
            if let Some(caps) = class_re.captures(content) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                let key = (NodeKind::Class, name.to_string());
                let entry = change_map.entry(key.clone()).or_insert((
                    if is_added { ChangeType::Added } else { ChangeType::Deleted },
                    0, 0,
                ));
                if is_added { entry.1 += 1; } else { entry.2 += 1; }
                current_context = Some(key);
                found_definition = true;
            }

            // Check for structs
            if !found_definition {
                if let Some(caps) = struct_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Struct, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0, 0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for enums
            if !found_definition {
                if let Some(caps) = enum_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Enum, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0, 0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for protocols
            if !found_definition {
                if let Some(caps) = protocol_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Trait, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0, 0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for extensions
            if !found_definition {
                if let Some(caps) = extension_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Impl, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0, 0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for functions
            if !found_definition {
                if let Some(caps) = fn_re.captures(content) {
                    let fn_name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Function, fn_name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0, 0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // If not a definition line, add to current context
            if !found_definition {
                if let Some(ref key) = current_context {
                    if let Some(entry) = change_map.get_mut(key) {
                        if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    }
                }
            }
        }

        change_map
            .into_iter()
            .map(|((kind, name), (change_type, additions, deletions))| ChangeNode {
                kind,
                name,
                change_type,
                additions,
                deletions,
                children: Vec::new(),
            })
            .collect()
    }
}
