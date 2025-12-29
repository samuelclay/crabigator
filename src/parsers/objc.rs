use regex::Regex;
use std::collections::HashMap;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct ObjCParser;

impl DiffParser for ObjCParser {
    fn language(&self) -> &'static str {
        "Objective-C"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".m")
            || filename.ends_with(".mm")
            || filename.ends_with(".h")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        // Objective-C hunk context patterns:
        // "- (void)methodName" or "+ (id)classMethod:"
        // "@interface ClassName" or "@implementation ClassName"
        let method_re = Regex::new(r"^[-+]\s*\([^)]+\)\s*(\w+)").unwrap();
        let interface_re = Regex::new(r"@interface\s+(\w+)").unwrap();
        let impl_re = Regex::new(r"@implementation\s+(\w+)").unwrap();
        let protocol_re = Regex::new(r"@protocol\s+(\w+)").unwrap();

        if let Some(caps) = method_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = interface_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = impl_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = protocol_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        None
    }

    fn parse(&self, diff: &str, filename: &str) -> Vec<ChangeNode> {
        let file_path = Some(filename.to_string());
        let mut change_map: HashMap<(NodeKind, String), (ChangeType, usize, usize)> = HashMap::new();

        // Regex patterns for Objective-C constructs
        // Method: - (returnType)methodName or + (returnType)methodName
        let method_re = Regex::new(r"^\s*[-+]\s*\([^)]+\)\s*(\w+)").unwrap();
        let interface_re = Regex::new(r"^\s*@interface\s+(\w+)").unwrap();
        let impl_re = Regex::new(r"^\s*@implementation\s+(\w+)").unwrap();
        let protocol_re = Regex::new(r"^\s*@protocol\s+(\w+)").unwrap();
        let hunk_re = Regex::new(r"^@@[^@]+@@\s*(.*)$").unwrap();

        let mut current_context: Option<(NodeKind, String)> = None;

        for line in diff.lines() {
            // Check for hunk headers with function context
            if let Some(caps) = hunk_re.captures(line) {
                if let Some(context) = caps.get(1) {
                    let context_str = context.as_str();
                    if let Some(method_caps) = method_re.captures(context_str) {
                        let name = method_caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                        current_context = Some((NodeKind::Method, name.to_string()));
                        let key = (NodeKind::Method, name.to_string());
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0));
                    } else if let Some(impl_caps) = impl_re.captures(context_str) {
                        let name = impl_caps.get(1).map(|m| m.as_str()).unwrap_or("Unknown");
                        current_context = Some((NodeKind::Impl, name.to_string()));
                        let key = (NodeKind::Impl, name.to_string());
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0));
                    } else if let Some(iface_caps) = interface_re.captures(context_str) {
                        let name = iface_caps.get(1).map(|m| m.as_str()).unwrap_or("Unknown");
                        current_context = Some((NodeKind::Class, name.to_string()));
                        let key = (NodeKind::Class, name.to_string());
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
            let is_context = line.starts_with(' ');

            // Check context lines for method/class definitions to track current scope
            if is_context {
                let content = &line[1..];
                if let Some(caps) = method_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Method, name.to_string()));
                } else if let Some(caps) = impl_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Impl, name.to_string()));
                } else if let Some(caps) = interface_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Class, name.to_string()));
                } else if let Some(caps) = protocol_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Trait, name.to_string()));
                }
                continue;
            }

            if !is_added && !is_removed {
                continue;
            }

            let content = &line[1..];
            let mut found_definition = false;

            // Check for @interface
            if let Some(caps) = interface_re.captures(content) {
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

            // Check for @implementation
            if !found_definition {
                if let Some(caps) = impl_re.captures(content) {
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

            // Check for @protocol
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

            // Check for methods
            if !found_definition {
                if let Some(caps) = method_re.captures(content) {
                    let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Method, name.to_string());
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
