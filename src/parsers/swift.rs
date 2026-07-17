use regex::Regex;
use std::sync::LazyLock;

use super::scope_walker::{walk_diff, Definition, ScopeRules};
use super::{ChangeNode, DiffParser, NodeKind};

pub struct SwiftParser;

impl ScopeRules for SwiftParser {
    fn match_definition(&self, content: &str) -> Option<Definition> {
        static FN_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?(?:static\s+)?(?:override\s+)?func\s+(\w+)").unwrap()
        });
        static CLASS_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?(?:final\s+)?class\s+(\w+)",
            )
            .unwrap()
        });
        static STRUCT_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?struct\s+(\w+)")
                .unwrap()
        });
        static ENUM_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?enum\s+(\w+)")
                .unwrap()
        });
        static PROTOCOL_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(?:(?:public|private|internal|fileprivate|open)\s+)?protocol\s+(\w+)")
                .unwrap()
        });
        static EXTENSION_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*extension\s+(\w+)").unwrap());

        let kinds: [(&LazyLock<Regex>, NodeKind); 6] = [
            (&FN_RE, NodeKind::Function),
            (&CLASS_RE, NodeKind::Class),
            (&STRUCT_RE, NodeKind::Struct),
            (&ENUM_RE, NodeKind::Enum),
            (&PROTOCOL_RE, NodeKind::Trait),
            (&EXTENSION_RE, NodeKind::Impl),
        ];
        for (re, kind) in kinds {
            if let Some(caps) = re.captures(content) {
                let name = caps.get(1).map(|m| m.as_str()).unwrap_or("unknown");
                return Some(Definition::scoped(kind, name));
            }
        }
        None
    }
}

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
        let fn_re = Regex::new(
            r"(?:(?:public|private|internal|fileprivate|open)\s+)?(?:static\s+)?func\s+(\w+)",
        )
        .unwrap();
        let class_re = Regex::new(
            r"(?:(?:public|private|internal|fileprivate|open)\s+)?(?:final\s+)?class\s+(\w+)",
        )
        .unwrap();
        let struct_re =
            Regex::new(r"(?:(?:public|private|internal|fileprivate|open)\s+)?struct\s+(\w+)")
                .unwrap();
        let enum_re =
            Regex::new(r"(?:(?:public|private|internal|fileprivate|open)\s+)?enum\s+(\w+)")
                .unwrap();
        let extension_re = Regex::new(r"extension\s+(\w+)").unwrap();

        for re in [&fn_re, &class_re, &struct_re, &enum_re, &extension_re] {
            if let Some(caps) = re.captures(context) {
                return caps.get(1).map(|m| m.as_str().to_string());
            }
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

    #[test]
    fn funcs_carry_type_scope_and_line_numbers() {
        let diff = "\
@@ -8,5 +8,6 @@ struct Publisher {
     func publish() -> Bool {
+        normalize()
         return send()
     }
";
        let nodes = SwiftParser.parse(diff, "Publisher.swift");
        let f = nodes
            .iter()
            .find(|n| n.name == "publish")
            .expect("func tracked");
        assert_eq!(f.scope, vec!["Publisher".to_string()]);
        assert_eq!(f.line_number, Some(9));
    }
}
