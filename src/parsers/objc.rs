use regex::Regex;
use std::sync::LazyLock;

use super::scope_walker::{walk_diff, Definition, ScopeRules};
use super::{ChangeNode, DiffParser, NodeKind};

pub struct ObjCParser;

impl ScopeRules for ObjCParser {
    fn match_definition(&self, content: &str) -> Option<Definition> {
        static METHOD_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*[-+]\s*\([^)]+\)\s*(\w+)").unwrap());
        static INTERFACE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*@interface\s+(\w+)").unwrap());
        static IMPL_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*@implementation\s+(\w+)").unwrap());
        static PROTOCOL_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*@protocol\s+(\w+)").unwrap());

        let kinds: [(&LazyLock<Regex>, NodeKind); 4] = [
            (&METHOD_RE, NodeKind::Method),
            (&IMPL_RE, NodeKind::Impl),
            (&INTERFACE_RE, NodeKind::Class),
            (&PROTOCOL_RE, NodeKind::Trait),
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

impl DiffParser for ObjCParser {
    fn language(&self) -> &'static str {
        "Objective-C"
    }

    fn supports(&self, filename: &str) -> bool {
        filename.ends_with(".m") || filename.ends_with(".mm") || filename.ends_with(".h")
    }

    fn extract_function_from_context(&self, context: &str) -> Option<String> {
        // Objective-C hunk context patterns:
        // "- (void)methodName" or "+ (id)classMethod:"
        // "@interface ClassName" or "@implementation ClassName"
        let method_re = Regex::new(r"^[-+]\s*\([^)]+\)\s*(\w+)").unwrap();
        let interface_re = Regex::new(r"@interface\s+(\w+)").unwrap();
        let impl_re = Regex::new(r"@implementation\s+(\w+)").unwrap();
        let protocol_re = Regex::new(r"@protocol\s+(\w+)").unwrap();

        for re in [&method_re, &interface_re, &impl_re, &protocol_re] {
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
    fn methods_get_line_numbers() {
        let diff = "\
@@ -12,5 +12,6 @@ @implementation Publisher
 - (void)publish {
+    [self normalize];
     [self send];
 }
";
        let nodes = ObjCParser.parse(diff, "Publisher.m");
        let m = nodes
            .iter()
            .find(|n| n.name == "publish")
            .expect("method tracked");
        assert_eq!(m.additions, 1);
        assert_eq!(m.line_number, Some(13));
    }
}
