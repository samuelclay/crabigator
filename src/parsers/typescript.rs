use regex::Regex;
use std::sync::LazyLock;

use super::scope_walker::{walk_diff, Definition, ScopeRules};
use super::{ChangeNode, DiffParser, NodeKind};

pub struct TypeScriptParser;

/// Extract (callee, title) from a test-block call like `describe('title', ...)`.
fn test_block(line: &str) -> Option<(String, String)> {
    static TEST_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(r#"^\s*(?:\w+\.)?(describe|context|suite)(?:\.\w+)*\s*\(\s*['"`]([^'"`]+)"#)
            .unwrap()
    });
    let caps = TEST_BLOCK_RE.captures(line)?;
    Some((caps[1].to_string(), caps[2].to_string()))
}

impl ScopeRules for TypeScriptParser {
    fn match_definition(&self, content: &str) -> Option<Definition> {
        static CLASS_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(export\s+)?(abstract\s+)?class\s+(\w+)").unwrap());
        static FUNCTION_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^\s*(export\s+)?(async\s+)?function\s+(\w+)|^\s*(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?\(",
            )
            .unwrap()
        });
        static METHOD_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(r"^\s*(public|private|protected|static|async|\s)*(\w+)\s*\([^)]*\)\s*[:{]")
                .unwrap()
        });
        static ARROW_FN_RE: LazyLock<Regex> = LazyLock::new(|| {
            Regex::new(
                r"^\s*(export\s+)?(const|let|var)\s+(\w+)\s*=\s*(async\s+)?(\([^)]*\)|[^=])\s*=>",
            )
            .unwrap()
        });
        static INTERFACE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(export\s+)?interface\s+(\w+)").unwrap());
        static TYPE_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^\s*(export\s+)?type\s+(\w+)").unwrap());

        // Test blocks are special: keyed by callee (describe/context/suite)
        // but labelled and scoped by their title so same-named blocks split.
        if let Some((callee, title)) = test_block(content) {
            return Some(Definition::titled_block(callee, title));
        }
        if let Some(caps) = CLASS_RE.captures(content) {
            let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Class, name));
        }
        if let Some(caps) = INTERFACE_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Trait, name));
        }
        if let Some(caps) = TYPE_RE.captures(content) {
            // Type definitions have no body scope to track.
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::leaf(NodeKind::Other, format!("type {}", name)));
        }
        if let Some(caps) = FUNCTION_RE.captures(content) {
            let name = caps
                .get(3)
                .or_else(|| caps.get(6))
                .map(|m| m.as_str())
                .unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Function, name));
        }
        if let Some(caps) = ARROW_FN_RE.captures(content) {
            let name = caps.get(3).map(|m| m.as_str()).unwrap_or("unknown");
            return Some(Definition::scoped(NodeKind::Function, name));
        }
        if let Some(caps) = METHOD_RE.captures(content) {
            let name = caps.get(2).map(|m| m.as_str()).unwrap_or("unknown");
            // Skip constructor and common keywords.
            if !matches!(name, "constructor" | "if" | "for" | "while" | "switch") {
                return Some(Definition::scoped(NodeKind::Method, name));
            }
        }
        None
    }

    fn match_hunk_context(&self, context: &str) -> Option<Definition> {
        // Test blocks carry their title as a scope so same-named blocks
        // (describe/context/suite) can be told apart.
        if let Some((callee, title)) = test_block(context) {
            return Some(Definition::titled_block(callee, title));
        }
        // Hunk contexts are free-form: fall back to the looser name-only
        // extraction (bind/prototype/bare-call patterns).
        self.extract_function_from_context(context)
            .map(|name| Definition::scoped(NodeKind::Function, name))
    }
}

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

    fn extract_scoped_context(&self, context: &str) -> Option<(String, Vec<String>)> {
        self.match_hunk_context(context).map(|definition| {
            let mut scope = Vec::new();
            if definition.self_scoped {
                scope.extend(definition.label);
            }
            (definition.name, scope)
        })
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
            if ![
                "if", "for", "while", "switch", "catch", "return", "var", "let", "const",
            ]
            .contains(&name)
                && name.len() > 2
            {
                return Some(name.to_string());
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
    use crate::parsers::ChangeType;

    fn find<'a>(nodes: &'a [ChangeNode], name: &str, scope: &[&str]) -> Option<&'a ChangeNode> {
        nodes
            .iter()
            .find(|n| n.name == name && n.scope == scope.to_vec())
    }

    #[test]
    fn describe_blocks_are_keyed_by_title() {
        let diff = "\
@@ -38,10 +39,11 @@ describe('thread mention DM routing', () => {
   const before = 1;
+  const after = 2;
@@ -266,6 +301,30 @@ describe('thread mention DM routing', () => {
 });
+describe('stripPostbackPromptQuestion', () => {
+  it('removes a trailing postback question', () => {
+    expect(strip(text)).toBe(clean);
+  });
+});
";
        let nodes = TypeScriptParser.parse(diff, "thread-routing.test.ts");

        let routing = find(&nodes, "describe", &["thread mention DM routing"])
            .expect("outer describe tracked by title");
        assert_eq!(routing.additions, 1);
        assert_eq!(routing.line_number, Some(39));

        let strip = find(&nodes, "describe", &["stripPostbackPromptQuestion"])
            .expect("added describe tracked by its own title");
        assert_eq!(strip.change_type, ChangeType::Added);
        // it() blocks roll up into the enclosing describe: 5 added lines total
        assert_eq!(strip.additions, 5);
        // The `});` context line is 301, so the added describe starts at 302
        assert_eq!(strip.line_number, Some(302));
    }

    #[test]
    fn nested_describe_carries_outer_title_in_scope() {
        let diff = "\
@@ -10,3 +10,6 @@ describe('outer suite', () => {
   describe('inner suite', () => {
+    expect(1).toBe(1);
   });
";
        let nodes = TypeScriptParser.parse(diff, "a.test.ts");
        let inner = find(&nodes, "describe", &["outer suite", "inner suite"])
            .expect("inner describe nests under outer title");
        assert_eq!(inner.additions, 1);
    }

    #[test]
    fn class_methods_carry_class_scope() {
        let diff = "\
@@ -5,4 +5,5 @@ class Delivery {
   chunkMetadata(input: string): Meta {
+    const extra = parse(input);
     return meta;
   }
";
        let nodes = TypeScriptParser.parse(diff, "delivery.ts");
        let method =
            find(&nodes, "chunkMetadata", &["Delivery"]).expect("method scoped under its class");
        assert_eq!(method.kind, NodeKind::Method);
        assert_eq!(method.additions, 1);
        assert_eq!(method.line_number, Some(6));
    }

    #[test]
    fn top_level_functions_have_empty_scope() {
        let diff = "\
@@ -1,3 +1,4 @@
+export const deliver = async (payload) => {
+  return send(payload);
+};
";
        let nodes = TypeScriptParser.parse(diff, "deliver.ts");
        let f = find(&nodes, "deliver", &[]).expect("top-level arrow fn");
        assert_eq!(f.change_type, ChangeType::Added);
        assert_eq!(f.line_number, Some(1));
    }
}
