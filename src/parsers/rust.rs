use regex::Regex;
use std::sync::LazyLock;

use super::scope_walker::{walk_diff, Definition, ScopeRules};
use super::{ChangeNode, DiffParser, NodeKind};

pub struct RustParser;

/// Format an impl block name: "Type" or "Trait for Type".
fn impl_name(caps: &regex::Captures) -> String {
    let type_name = caps.get(2).map(|m| m.as_str()).unwrap_or("Unknown");
    match caps.get(1) {
        Some(trait_name) => format!("{} for {}", trait_name.as_str(), type_name),
        None => type_name.to_string(),
    }
}

impl ScopeRules for RustParser {
    fn match_definition(&self, content: &str) -> Option<Definition> {
        static IMPL_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*impl(?:<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap()
        });
        static FN_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?(async\s+)?fn\s+(\w+)").unwrap());
        static STRUCT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?struct\s+(\w+)").unwrap());
        static ENUM_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?enum\s+(\w+)").unwrap());
        static TRAIT_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?trait\s+(\w+)").unwrap());
        static MOD_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?mod\s+(\w+)").unwrap());
        static CONST_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(pub\s+)?const\s+(\w+)").unwrap());

        if let Some(caps) = IMPL_RE.captures(content) {
            return Some(Definition::scoped(NodeKind::Impl, impl_name(&caps)));
        }
        if let Some(caps) = FN_RE.captures(content) {
            let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Function, name));
        }
        if let Some(caps) = STRUCT_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Struct, name));
        }
        if let Some(caps) = ENUM_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Enum, name));
        }
        if let Some(caps) = TRAIT_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Trait, name));
        }
        if let Some(caps) = MOD_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Module, name));
        }
        if let Some(caps) = CONST_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::leaf(NodeKind::Const, name));
        }
        None
    }

    fn match_hunk_context(&self, context: &str) -> Option<Definition> {
        static FN_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"(?:pub\s+)?(?:async\s+)?fn\s+(\w+)").unwrap());
        static IMPL_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"impl(?:<[^>]*>)?\s+(?:(\w+)\s+for\s+)?(\w+)").unwrap());

        if let Some(caps) = FN_RE.captures(context) {
            let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Function, name));
        }
        if let Some(caps) = IMPL_RE.captures(context) {
            return Some(Definition::scoped(NodeKind::Impl, impl_name(&caps)));
        }
        None
    }
}

impl DiffParser for RustParser {
    fn language(&self) -> &'static str {
        "Rust"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".rs")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        self.match_hunk_context(context)
            .map(|definition| definition.name)
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
    fn fns_carry_impl_scope_and_line_numbers() {
        let diff = "\
@@ -20,5 +20,6 @@ impl Publisher {
     pub fn publish(&self) -> Result<()> {
+        self.normalize();
         self.send()
     }
";
        let nodes = RustParser.parse(diff, "src/publisher.rs");
        let f = nodes
            .iter()
            .find(|n| n.name == "publish")
            .expect("fn tracked");
        assert_eq!(f.scope, vec!["Publisher".to_string()]);
        assert_eq!(f.additions, 1);
        assert_eq!(f.line_number, Some(21));
    }

    #[test]
    fn added_top_level_fn() {
        let diff = "\
@@ -1,2 +1,6 @@
 use anyhow::Result;
+
+pub fn plan() -> &'static str {
+    \"planning\"
+}
";
        let nodes = RustParser.parse(diff, "src/lib.rs");
        let f = nodes.iter().find(|n| n.name == "plan").expect("fn tracked");
        assert_eq!(f.change_type, ChangeType::Added);
        assert!(f.scope.is_empty());
        assert_eq!(f.line_number, Some(3));
    }
}
