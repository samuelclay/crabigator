use regex::Regex;
use std::sync::LazyLock;

use super::scope_walker::{walk_diff, Definition, ScopeRules};
use super::{ChangeNode, DiffParser, NodeKind};

pub struct PythonParser;

/// Dunder methods other than __init__ aren't worth a row of their own.
fn is_skipped_dunder(name: &str) -> bool {
    name.starts_with("__") && name.ends_with("__") && name != "__init__"
}

impl ScopeRules for PythonParser {
    fn match_definition(&self, content: &str) -> Option<Definition> {
        static CLASS_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*class\s+(\w+)").unwrap());
        static DEF_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(?:async\s+)?def\s+(\w+)").unwrap());

        if let Some(caps) = CLASS_RE.captures(content) {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Class, name));
        }
        if let Some(caps) = DEF_RE.captures(content) {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            if !is_skipped_dunder(name) {
                return Some(Definition::scoped(NodeKind::Function, name));
            }
        }
        None
    }

    fn match_hunk_context(&self, context: &str) -> Option<Definition> {
        // Hunk contexts historically resolve to a plain function node.
        self.extract_function_from_context(context)
            .map(|name| Definition::scoped(NodeKind::Function, name))
    }
}

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
        walk_diff(self, diff, filename)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parsers::ChangeType;

    #[test]
    fn methods_carry_class_scope_and_line_numbers() {
        let diff = "\
@@ -10,5 +10,6 @@ class Publisher:
     def publish(self, event):
+        event = normalize(event)
         return self.send(event)
";
        let nodes = PythonParser.parse(diff, "publisher.py");
        let method = nodes
            .iter()
            .find(|n| n.name == "publish")
            .expect("method tracked");
        assert_eq!(method.scope, vec!["Publisher".to_string()]);
        assert_eq!(method.additions, 1);
        assert_eq!(method.line_number, Some(11));
    }

    #[test]
    fn added_top_level_function() {
        let diff = "\
@@ -1,2 +1,5 @@
 import os
+
+def plan():
+    return \"planning\"
";
        let nodes = PythonParser.parse(diff, "main.py");
        let f = nodes.iter().find(|n| n.name == "plan").expect("fn tracked");
        assert_eq!(f.change_type, ChangeType::Added);
        assert!(f.scope.is_empty());
        assert_eq!(f.additions, 2);
        assert_eq!(f.line_number, Some(3));
    }
}
