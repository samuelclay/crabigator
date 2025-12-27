use regex::Regex;
use std::collections::HashMap;

use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct TypeScriptParser;

impl DiffParser for TypeScriptParser {
    fn language(&self) -> &'static str {
        "JavaScript"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".ts")
            || filename.ends_with(".tsx")
            || filename.ends_with(".js")
            || filename.ends_with(".jsx")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        // JS/TS hunk context patterns (in priority order):
        // 1. Named function: "function name(" or "async function name("
        // 2. Class: "class Name"
        // 3. Object method: "name: function(" or "name(" at start of line
        // 4. Method binding: ".bind('name'," - extract event name
        // 5. Arrow function assigned: "const name = (" or "const name = async ("
        // 6. Prototype method: "Foo.prototype.name = function"
        let function_re = Regex::new(r"(?:async\s+)?function\s+(\w+)").unwrap();
        let class_re = Regex::new(r"class\s+(\w+)").unwrap();
        let object_method_re = Regex::new(r"^\s*(\w+)\s*:\s*(?:async\s+)?function").unwrap();
        let method_call_re = Regex::new(r"^\s*(\w+)\s*\(").unwrap();
        let bind_re = Regex::new(r#"\.bind\s*\(\s*['"](\w+)['"]"#).unwrap();
        let arrow_fn_re = Regex::new(r"(?:const|let|var)\s+(\w+)\s*=\s*(?:async\s+)?\(").unwrap();
        let prototype_re = Regex::new(r"(\w+)\.prototype\.(\w+)\s*=").unwrap();

        // Named function - highest priority
        if let Some(caps) = function_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        // Class definition
        if let Some(caps) = class_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        // Object method: "name: function"
        if let Some(caps) = object_method_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        // Prototype method: "Foo.prototype.bar = function"
        if let Some(caps) = prototype_re.captures(context) {
            let method = caps.get(2).map(|m| m.as_str())?;
            return Some(method.to_string());
        }
        // Event binding: ".bind('eventName',"
        if let Some(caps) = bind_re.captures(context) {
            let event = caps.get(1).map(|m| m.as_str())?;
            return Some(format!("on:{}", event));
        }
        // Arrow function: "const name = ("
        if let Some(caps) = arrow_fn_re.captures(context) {
            return caps.get(1).map(|m| m.as_str().to_string());
        }
        // Method call at line start: "name("
        if let Some(caps) = method_call_re.captures(context) {
            let name = caps.get(1).map(|m| m.as_str())?;
            // Skip common keywords and short names that are likely variables
            if !["if", "for", "while", "switch", "catch", "return", "var", "let", "const"].contains(&name)
                && name.len() > 2 {
                return Some(name.to_string());
            }
        }
        None
    }

    fn parse(&self, diff: &str, _filename: &str) -> Vec<ChangeNode> {
        // Track changes with their line counts
        // Key: (kind, name), Value: (change_type, additions, deletions)
        let mut change_map: HashMap<(NodeKind, String), (ChangeType, usize, usize)> = HashMap::new();

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
                    let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Class, name.to_string()));
                }
                // Check for function definitions in context
                else if let Some(caps) = function_re.captures(content) {
                    let name = caps
                        .get(3)
                        .or_else(|| caps.get(6))
                        .map(|m| m.as_str())
                        .unwrap_or("unknown");
                    current_context = Some((NodeKind::Function, name.to_string()));
                }
                // Check for arrow functions in context
                else if let Some(caps) = arrow_fn_re.captures(content) {
                    let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
                    current_context = Some((NodeKind::Function, name.to_string()));
                }
                // Check for methods in context
                else if let Some(caps) = method_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    if name != "constructor"
                        && name != "if"
                        && name != "for"
                        && name != "while"
                        && name != "switch"
                    {
                        current_context = Some((NodeKind::Method, name.to_string()));
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
                let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
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

            // Check for interface definitions
            if !found_definition {
                if let Some(caps) = interface_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Trait, name.to_string());
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

            // Check for type definitions
            if !found_definition {
                if let Some(caps) = type_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    let key = (NodeKind::Other, format!("type {}", name));
                    let entry = change_map.entry(key.clone()).or_insert((
                        if is_added { ChangeType::Added } else { ChangeType::Deleted },
                        0,
                        0,
                    ));
                    if is_added { entry.1 += 1; } else { entry.2 += 1; }
                    found_definition = true;
                }
            }

            // Check for function declarations
            if !found_definition {
                if let Some(caps) = function_re.captures(content) {
                    let name = caps
                        .get(3)
                        .or_else(|| caps.get(6))
                        .map(|m| m.as_str())
                        .unwrap_or("unknown");
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

            // Check for arrow functions
            if !found_definition {
                if let Some(caps) = arrow_fn_re.captures(content) {
                    let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
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

            // Check for methods (inside classes)
            if !found_definition {
                if let Some(caps) = method_re.captures(content) {
                    let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
                    // Skip constructor and common keywords
                    if name != "constructor"
                        && name != "if"
                        && name != "for"
                        && name != "while"
                        && name != "switch"
                    {
                        let key = (NodeKind::Method, name.to_string());
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
                children: Vec::new(),
            })
            .collect()
    }
}
