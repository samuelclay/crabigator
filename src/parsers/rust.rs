use regex::Regex;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct RustParser;

impl DiffParser for RustParser {
    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".rs")
    }

    fn parse(&self, diff: &str, _filename: &str) -> Vec<ChangeNode> {
        let mut changes = Vec::new();

        // Regex patterns for Rust constructs
        let fn_re = Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)").unwrap();
        let impl_re = Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap();
        let struct_re = Regex::new(r"^\s*(pub\s+)?struct\s+(\w+)").unwrap();
        let enum_re = Regex::new(r"^\s*(pub\s+)?enum\s+(\w+)").unwrap();
        let trait_re = Regex::new(r"^\s*(pub\s+)?trait\s+(\w+)").unwrap();
        let mod_re = Regex::new(r"^\s*(pub\s+)?mod\s+(\w+)").unwrap();
        let const_re = Regex::new(r"^\s*(pub\s+)?const\s+(\w+)").unwrap();
        // Pattern for hunk headers with function context: @@ -line,count +line,count @@ context
        let hunk_re = Regex::new(r"^@@[^@]+@@\s*(.*)$").unwrap();

        let mut current_impl: Option<ChangeNode> = None;

        for line in diff.lines() {
            // Check for hunk headers with function context
            if let Some(caps) = hunk_re.captures(line) {
                if let Some(context) = caps.get(1) {
                    let context_str = context.as_str();
                    // Try to extract function name from context
                    if let Some(fn_caps) = fn_re.captures(context_str) {
                        let fn_name = fn_caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                        // Add function if not already present
                        if !changes.iter().any(|c: &ChangeNode| c.name == fn_name && c.kind == NodeKind::Function) {
                            changes.push(ChangeNode {
                                kind: NodeKind::Function,
                                name: fn_name.to_string(),
                                change_type: ChangeType::Modified,
                                children: Vec::new(),
                            });
                        }
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
                        if !changes.iter().any(|c: &ChangeNode| c.name == name && c.kind == NodeKind::Impl) {
                            changes.push(ChangeNode {
                                kind: NodeKind::Impl,
                                name,
                                change_type: ChangeType::Modified,
                                children: Vec::new(),
                            });
                        }
                    }
                }
                continue;
            }

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

            let content = &line[1..]; // Strip the +/- prefix

            // Check for impl blocks
            if let Some(caps) = impl_re.captures(content) {
                // Save previous impl if any
                if let Some(impl_node) = current_impl.take() {
                    if !impl_node.children.is_empty() {
                        changes.push(impl_node);
                    }
                }

                let type_name = caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
                let trait_name = caps.get(1).map(|m| m.as_str());

                let name = if let Some(trait_n) = trait_name {
                    format!("{} for {}", trait_n, type_name)
                } else {
                    type_name.to_string()
                };

                current_impl = Some(ChangeNode {
                    kind: NodeKind::Impl,
                    name,
                    change_type: ChangeType::Modified,
                    children: Vec::new(),
                });
                continue;
            }

            // Check for functions (methods if inside impl)
            if let Some(caps) = fn_re.captures(content) {
                let fn_name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");

                let node = ChangeNode {
                    kind: if current_impl.is_some() {
                        NodeKind::Method
                    } else {
                        NodeKind::Function
                    },
                    name: fn_name.to_string(),
                    change_type: change_type.clone(),
                    children: Vec::new(),
                };

                if let Some(ref mut impl_node) = current_impl {
                    // Check if method already exists in children
                    if !impl_node.children.iter().any(|c| c.name == fn_name) {
                        impl_node.children.push(node);
                    }
                } else {
                    // Check if function already exists in changes
                    if !changes.iter().any(|c| c.name == fn_name && c.kind == NodeKind::Function) {
                        changes.push(node);
                    }
                }
                continue;
            }

            // Check for structs
            if let Some(caps) = struct_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Struct) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Struct,
                        name: name.to_string(),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
                continue;
            }

            // Check for enums
            if let Some(caps) = enum_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Enum) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Enum,
                        name: name.to_string(),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
                continue;
            }

            // Check for traits
            if let Some(caps) = trait_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Trait) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Trait,
                        name: name.to_string(),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
                continue;
            }

            // Check for modules
            if let Some(caps) = mod_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Module) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Module,
                        name: name.to_string(),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
                continue;
            }

            // Check for consts
            if let Some(caps) = const_re.captures(content) {
                let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                if !changes.iter().any(|c| c.name == name && c.kind == NodeKind::Const) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Const,
                        name: name.to_string(),
                        change_type: change_type.clone(),
                        children: Vec::new(),
                    });
                }
            }
        }

        // Don't forget the last impl block
        if let Some(impl_node) = current_impl {
            if !impl_node.children.is_empty() {
                changes.push(impl_node);
            }
        }

        changes
    }
}
