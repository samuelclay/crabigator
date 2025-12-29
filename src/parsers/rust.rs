use regex::Regex;
use std::collections::HashMap;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct RustParser;

impl DiffParser for RustParser {
    fn language(&self) -> &'static str {
        "Rust"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".rs")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        // Rust hunk context patterns:
        // "fn name(" or "pub fn name(" or "async fn name("
        // "impl Type" or "impl Trait for Type"
        let fn_re = Regex::new(r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap();
        let impl_re = Regex::new(r"impl(?:<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap();

        if let Some(caps) = fn_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        if let Some(caps) = impl_re.captures(context) {
            let type_name = caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
            let trait_name = caps.get(1).map(|m| m.as_str());
            return Some(if let Some(trait_n) = trait_name {
                format!("{} for {}", trait_n, type_name)
            } else {
                type_name.to_string()
            });
        }
        None
    }

    fn parse(&self, diff: &str, filename: &str) -> Vec<ChangeNode> {
        // Track changes with their line counts
        // Key: (kind, name), Value: (change_type, additions, deletions, line_number)
        type ChangeMap = HashMap<(NodeKind, String), (ChangeType, usize, usize, Option<usize>)>;
        let mut change_map: ChangeMap = HashMap::new();

        // Regex patterns for Rust constructs
        let fn_re = Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)").unwrap();
        let impl_re = Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap();
        let struct_re = Regex::new(r"^\s*(pub\s+)?struct\s+(\w+)").unwrap();
        let enum_re = Regex::new(r"^\s*(pub\s+)?enum\s+(\w+)").unwrap();
        let trait_re = Regex::new(r"^\s*(pub\s+)?trait\s+(\w+)").unwrap();
        let mod_re = Regex::new(r"^\s*(pub\s+)?mod\s+(\w+)").unwrap();
        let const_re = Regex::new(r"^\s*(pub\s+)?const\s+(\w+)").unwrap();
        // Pattern for hunk headers: @@ -old,count +new,count @@ context
        // Captures: 1=new_line_start, 2=context
        let hunk_re = Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@\s*(.*)$").unwrap();

        // Current context: which function/impl we're inside
        let mut current_context: Option<(NodeKind, String)> = None;
        // Track current line number in the new file
        let mut current_line: usize = 0;
        let file_path = Some(filename.to_string());

        for line in diff.lines() {
            // Check for hunk headers with function context
            if let Some(caps) = hunk_re.captures(line) {
                // Extract new file line number from hunk header
                if let Some(line_num) = caps.get(1) {
                    current_line = line_num.as_str().parse().unwrap_or(1);
                }
                if let Some(context) = caps.get(2) {
                    let context_str = context.as_str();
                    // Try to extract function name from context
                    if let Some(fn_caps) = fn_re.captures(context_str) {
                        let fn_name = fn_caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                        current_context = Some((NodeKind::Function, fn_name.to_string()));
                        // Pre-register as modified (will be updated with line counts)
                        let key = (NodeKind::Function, fn_name.to_string());
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0, Some(current_line)));
                    }
                    // Check for impl block in context
                    else if let Some(impl_caps) = impl_re.captures(context_str) {
                        let type_name = impl_caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
                        let trait_name = impl_caps.get(1).map(|m| m.as_str());
                        let name = if let Some(trait_n) = trait_name {
                            format!("{} for {}", trait_n, type_name)
                        } else {
                            type_name.to_string()
                        };
                        current_context = Some((NodeKind::Impl, name.clone()));
                        let key = (NodeKind::Impl, name);
                        change_map.entry(key).or_insert((ChangeType::Modified, 0, 0, Some(current_line)));
                    } else {
                        // No function context in hunk header
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

            // Check context lines for function/struct/impl definitions to track current scope
            if is_context {
                current_line += 1; // Context lines appear in new file
                let content = &line[1..];
                // Check for impl blocks in context
                if let Some(caps) = impl_re.captures(content) {
                    let type_name = caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
                    let trait_name = caps.get(1).map(|m| m.as_str());
                    let name = if let Some(trait_n) = trait_name {
                        format!("{} for {}", trait_n, type_name)
                    } else {
                        type_name.to_string()
                    };
                    current_context = Some((NodeKind::Impl, name));
                }
                // Check for functions in context
                else if let Some(caps) = fn_re.captures(content) {
                    let fn_name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Function, fn_name.to_string()));
                }
                // Check for structs in context
                else if let Some(caps) = struct_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Struct, name.to_string()));
                }
                // Check for enums in context
                else if let Some(caps) = enum_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Enum, name.to_string()));
                }
                // Check for traits in context
                else if let Some(caps) = trait_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Trait, name.to_string()));
                }
                continue;
            }

            if !is_added && !is_removed {
                continue;
            }

            // Increment line number for added lines (they appear in new file)
            if is_added {
                current_line += 1;
            }

            let content = &line[1..]; // Strip the +/- prefix

            // Check if this line defines a new construct
            let mut found_definition = false;

            // Check for impl blocks
            if let Some(caps) = impl_re.captures(content) {
                let type_name = caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
                let trait_name = caps.get(1).map(|m| m.as_str());
                let name = if let Some(trait_n) = trait_name {
                    format!("{} for {}", trait_n, type_name)
                } else {
                    type_name.to_string()
                };
                let key = (NodeKind::Impl, name);
                let entry = change_map.entry(key.clone()).or_insert((
                    if is_added { ChangeType::Added } else { ChangeType::Deleted },
                    0,
                    0,
                    if is_added { Some(current_line) } else { None },
                ));
                if is_added { entry.1 += 1; } else { entry.2 += 1; }
                current_context = Some(key);
                found_definition = true;
            }

            // Check for functions
            if !found_definition {
                if let Some(caps) = fn_re.captures(content) {
                    let fn_name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Function, fn_name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                        if is_added { Some(current_line) } else { None },
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for structs
            if !found_definition {
                if let Some(caps) = struct_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Struct, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                        if is_added { Some(current_line) } else { None },
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for enums
            if !found_definition {
                if let Some(caps) = enum_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Enum, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                        if is_added { Some(current_line) } else { None },
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for traits
            if !found_definition {
                if let Some(caps) = trait_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Trait, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                        if is_added { Some(current_line) } else { None },
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for modules
            if !found_definition {
                if let Some(caps) = mod_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Module, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                        if is_added { Some(current_line) } else { None },
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    current_context = Some(key);
                    found_definition = true;
                }
            }

            // Check for consts
            if !found_definition {
                if let Some(caps) = const_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Const, name.to_string());
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                        if is_added { Some(current_line) } else { None },
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    found_definition = true;
                }
            }

            // If not a definition line, add to current context
            if !found_definition {
                if let Some(ref key) = current_context {
                    let entry = change_map
                        .entry(key.clone())
                        .or_insert((ChangeType::Modified, 0, 0, None));
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
            .map(|((kind, name), (change_type, additions, deletions, line_number))| ChangeNode {
                kind,
                name,
                change_type,
                additions,
                deletions,
                file_path: file_path.clone(),
                line_number,
                children: Vec::new(),
            })
            .collect()
    }
}
