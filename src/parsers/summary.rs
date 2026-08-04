//! Diff summary and parser trait

use anyhow::Result;
use regex::Regex;
use std::path::Path;
use tokio::process::Command;

use super::types::{ChangeNode, ChangeType, FileChanges, LanguageChanges, NodeKind};
use super::{GenericParser, ObjCParser, PythonParser, RustParser, SwiftParser, TypeScriptParser};

/// Trait for language-specific diff parsers
pub trait DiffParser: Send + Sync {
    /// Language name for display (e.g., "Python", "JavaScript", "Rust")
    fn language(&self) -> &'static str;
    /// Check if this parser supports the given filename
    fn supports(&self, filename: &str) -> bool;
    /// Parse diff content and return semantic changes
    fn parse(&self, diff: &str, filename: &str) -> Vec<ChangeNode>;
    /// Extract function name from a hunk context line (language-specific)
    fn extract_function_from_context(&self, context: &str) -> Option<String>;
    /// Extract (function name, enclosing scope labels) from a hunk context
    /// line. Default: name only, no scope information.
    fn extract_scoped_context(&self, context: &str) -> Option<(String, Vec<String>)> {
        self.extract_function_from_context(context)
            .map(|name| (name, Vec::new()))
    }
}

#[derive(Clone, Debug, Default)]
pub struct DiffSummary {
    pub files: Vec<FileChanges>,
    pub loading: bool,
}

impl DiffSummary {
    /// Get changes grouped by language for display
    pub fn by_language(&self) -> Vec<LanguageChanges> {
        use std::collections::HashMap;

        // Merge changes by (language, kind, name, scope, file_path) to combine
        // stats. Scope and file_path prevent merging same-named symbols from
        // different scopes/files.
        type ChangeKey = (NodeKind, String, Vec<String>, Option<String>);
        type LangChanges = HashMap<String, HashMap<ChangeKey, ChangeNode>>;
        let mut by_lang: LangChanges = HashMap::new();

        for file in &self.files {
            let lang_entry = by_lang.entry(file.language.clone()).or_default();
            for change in &file.changes {
                // Skip empty context-only entries (no changed lines attributed)
                if change.additions == 0 && change.deletions == 0 {
                    continue;
                }
                let key = (
                    change.kind.clone(),
                    change.name.clone(),
                    change.scope.clone(),
                    change.file_path.clone(),
                );
                lang_entry
                    .entry(key)
                    .and_modify(|existing| {
                        existing.additions += change.additions;
                        existing.deletions += change.deletions;
                    })
                    .or_insert_with(|| change.clone());
            }
        }

        let mut result: Vec<_> = by_lang
            .into_iter()
            .map(|(language, changes_map)| {
                let mut changes: Vec<ChangeNode> = changes_map.into_values().collect();
                bake_scoped_names(&mut changes);
                // Sort changes by name, then file_path for consistent ordering
                changes.sort_by(|a, b| {
                    a.name
                        .cmp(&b.name)
                        .then_with(|| a.file_path.cmp(&b.file_path))
                });
                LanguageChanges { language, changes }
            })
            .collect();

        // Sort by language name for consistent display
        result.sort_by(|a, b| a.language.cmp(&b.language));
        result
    }

    /// Total number of changes across all languages
    pub fn total_changes(&self) -> usize {
        self.files.iter().map(|f| f.changes.len()).sum()
    }
}

/// Bake `parent › name` display names into each change. The immediate parent
/// scope is always shown when one exists; further ancestors — ending with the
/// file stem as a last resort — are prepended recursively only while the
/// display name remains ambiguous within the language group.
fn bake_scoped_names(changes: &mut [ChangeNode]) {
    use std::collections::HashMap;

    fn display_name(change: &ChangeNode, depth: usize, with_file: bool) -> String {
        let mut parts: Vec<String> = Vec::new();
        if with_file {
            if let Some(stem) = change
                .file_path
                .as_deref()
                .and_then(|p| Path::new(p).file_stem())
            {
                parts.push(stem.to_string_lossy().into_owned());
            }
        }
        let start = change.scope.len().saturating_sub(depth);
        parts.extend(change.scope[start..].iter().cloned());
        parts.push(change.name.clone());
        parts.join(" › ")
    }

    // depth = trailing scope entries shown; start with the immediate parent.
    let mut depths: Vec<usize> = changes.iter().map(|c| c.scope.len().min(1)).collect();
    let mut use_file: Vec<bool> = vec![false; changes.len()];

    loop {
        let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
        for (i, change) in changes.iter().enumerate() {
            groups
                .entry(display_name(change, depths[i], use_file[i]))
                .or_default()
                .push(i);
        }

        let mut expanded = false;
        for indices in groups.values().filter(|v| v.len() > 1) {
            for &i in indices {
                if depths[i] < changes[i].scope.len() {
                    depths[i] += 1;
                    expanded = true;
                } else if !use_file[i] && changes[i].file_path.is_some() {
                    use_file[i] = true;
                    expanded = true;
                }
            }
        }
        if !expanded {
            break;
        }
    }

    for (i, change) in changes.iter_mut().enumerate() {
        change.name = display_name(change, depths[i], use_file[i]);
    }
}

impl DiffSummary {
    pub fn new() -> Self {
        Self {
            loading: true,
            ..Self::default()
        }
    }

    #[allow(dead_code)]
    pub async fn refresh(&self) -> Result<Self> {
        let cwd = std::env::current_dir()?;
        self.refresh_in_dir(&cwd).await
    }

    pub async fn refresh_in_dir(&self, dir: &Path) -> Result<Self> {
        let profile = std::env::var("CRABIGATOR_PROFILE").is_ok();
        let start = std::time::Instant::now();
        let mut summary = DiffSummary::default();

        // Get the diff output
        let output = Command::new("git")
            .args(["diff", "--no-color"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(dir)
            .output()
            .await?;

        if !output.status.success() {
            return Ok(summary);
        }

        let diff_output = String::from_utf8_lossy(&output.stdout);

        // Also get staged changes
        let staged_output = Command::new("git")
            .args(["diff", "--cached", "--no-color"])
            .env("GIT_OPTIONAL_LOCKS", "0")
            .current_dir(dir)
            .output()
            .await?;

        let staged_diff = String::from_utf8_lossy(&staged_output.stdout);
        let combined_diff = format!("{}\n{}", diff_output, staged_diff);

        // Parse the diff into file chunks
        let file_diffs = parse_diff_into_files(&combined_diff);

        // Create parsers
        let parsers: Vec<Box<dyn DiffParser>> = vec![
            Box::new(RustParser),
            Box::new(TypeScriptParser),
            Box::new(PythonParser),
            Box::new(SwiftParser),
            Box::new(ObjCParser),
            Box::new(GenericParser),
        ];

        for (filename, file_diff) in file_diffs {
            // Find the appropriate parser
            let parser = parsers
                .iter()
                .find(|p| p.supports(&filename))
                .unwrap_or(&parsers[parsers.len() - 1]); // fallback to generic

            let language = parser.language().to_string();

            // Parse for new definitions
            let mut changes = parser.parse(&file_diff, &filename);

            // Also parse hunk headers for modifications to existing functions
            let modified = parse_hunk_modifications(&file_diff, parser.as_ref(), &filename);

            // Add modified functions that aren't already in changes. Compare
            // by name only: the same symbol may carry scope when found in the
            // diff body but not when named in a hunk header, and keeping both
            // would double-count its lines.
            for mod_change in modified {
                if !changes.iter().any(|c| c.name == mod_change.name) {
                    changes.push(mod_change);
                }
            }

            if !changes.is_empty() {
                summary.files.push(FileChanges {
                    path: filename,
                    language,
                    changes,
                });
            }
        }

        if profile && start.elapsed().as_millis() > 100 {
            if let Ok(mut f) = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open("/tmp/crabigator-profile.log")
            {
                use std::io::Write;
                let _ = writeln!(
                    f,
                    "[profile] DiffSummary::refresh took {:?}",
                    start.elapsed()
                );
            }
        }

        Ok(summary)
    }
}

/// Parse hunk headers and context lines to detect modifications inside existing functions
fn parse_hunk_modifications(
    diff: &str,
    parser: &dyn DiffParser,
    filename: &str,
) -> Vec<ChangeNode> {
    use std::collections::HashMap;
    use std::sync::LazyLock;

    let file_path = Some(filename.to_string());

    // Track changes with their line counts and line number,
    // keyed by (name, scope): (additions, deletions, line_number)
    type HunkKey = (String, Vec<String>);
    let mut change_map: HashMap<HunkKey, (usize, usize, Option<usize>)> = HashMap::new();
    // Pattern captures: 1=new_line_start, 2=context (compiled once)
    static HUNK_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@\s*(.*)$").unwrap());
    let hunk_re = &*HUNK_RE;

    let mut in_hunk = false;
    let mut current_hunk_func: Option<HunkKey> = None;
    let mut current_hunk_line: Option<usize> = None;

    for line in diff.lines() {
        // Check for hunk header
        if let Some(caps) = hunk_re.captures(line) {
            in_hunk = true;
            current_hunk_func = None;

            // Extract line number from capture group 1
            current_hunk_line = caps.get(1).and_then(|m| m.as_str().parse().ok());

            // Try to extract function from hunk header context (if present)
            if let Some(context) = caps.get(2) {
                let context_str = context.as_str().trim();
                if !context_str.is_empty() {
                    current_hunk_func = parser.extract_scoped_context(context_str);
                }
            }
            continue;
        }

        // If we're in a hunk and haven't found a function yet, check context lines
        if in_hunk && current_hunk_func.is_none() {
            // Context lines start with space (unchanged lines around the change)
            if let Some(context_str) = line.strip_prefix(' ') {
                if let Some(scoped) = parser.extract_scoped_context(context_str) {
                    current_hunk_func = Some(scoped);
                }
            }
        }

        // When we hit an added/removed line, record the function if found
        if in_hunk && !line.starts_with("+++") && !line.starts_with("---") {
            let is_added = line.starts_with('+');
            let is_removed = line.starts_with('-');

            if is_added || is_removed {
                if let Some(ref func_name) = current_hunk_func {
                    let entry =
                        change_map
                            .entry(func_name.clone())
                            .or_insert((0, 0, current_hunk_line));
                    if is_added {
                        entry.0 += 1;
                    } else {
                        entry.1 += 1;
                    }
                }
            }
        }

        // Reset on new file
        if line.starts_with("diff --git") {
            in_hunk = false;
            current_hunk_func = None;
            current_hunk_line = None;
        }
    }

    change_map
        .into_iter()
        .map(
            |((name, scope), (additions, deletions, line_number))| ChangeNode {
                kind: NodeKind::Function,
                name,
                scope,
                change_type: ChangeType::Modified,
                additions,
                deletions,
                file_path: file_path.clone(),
                line_number,
                children: Vec::new(),
            },
        )
        .collect()
}

fn parse_diff_into_files(diff: &str) -> Vec<(String, String)> {
    let mut files = Vec::new();
    let mut current_file = String::new();
    let mut current_diff = String::new();

    for line in diff.lines() {
        if line.starts_with("diff --git") {
            // Save previous file if any
            if !current_file.is_empty() {
                files.push((current_file.clone(), current_diff.clone()));
            }

            // Extract filename from "diff --git a/path b/path"
            if let Some(b_path) = line.split(" b/").nth(1) {
                current_file = b_path.to_string();
            } else {
                current_file = String::new();
            }
            current_diff = String::new();
        } else {
            current_diff.push_str(line);
            current_diff.push('\n');
        }
    }

    // Don't forget the last file
    if !current_file.is_empty() {
        files.push((current_file, current_diff));
    }

    files
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(name: &str, scope: &[&str], file: &str) -> ChangeNode {
        ChangeNode {
            kind: NodeKind::Function,
            name: name.to_string(),
            scope: scope.iter().map(|s| s.to_string()).collect(),
            change_type: ChangeType::Modified,
            additions: 1,
            deletions: 1,
            file_path: Some(file.to_string()),
            line_number: None,
            children: Vec::new(),
        }
    }

    #[test]
    fn scoped_names_show_immediate_parent() {
        let mut changes = vec![
            node("describe", &["thread mention DM routing"], "a.test.ts"),
            node("describe", &["final delivery"], "b.test.ts"),
            node("deliver", &[], "deliver.ts"),
        ];
        bake_scoped_names(&mut changes);
        let names: Vec<_> = changes.iter().map(|c| c.name.as_str()).collect();
        assert!(names.contains(&"thread mention DM routing › describe"));
        assert!(names.contains(&"final delivery › describe"));
        // Unambiguous top-level symbols stay short
        assert!(names.contains(&"deliver"));
    }

    #[test]
    fn ambiguous_names_extend_ancestors_recursively() {
        let mut changes = vec![
            node("describe", &["retries"], "final-delivery.test.ts"),
            node("describe", &["retries"], "thread-routing.test.ts"),
        ];
        bake_scoped_names(&mut changes);
        let mut names: Vec<_> = changes.iter().map(|c| c.name.as_str()).collect();
        names.sort();
        // Scope chain exhausted while still ambiguous: file stem prepended
        assert_eq!(
            names,
            vec![
                "final-delivery.test › retries › describe",
                "thread-routing.test › retries › describe",
            ]
        );
    }

    #[test]
    fn nested_scope_extends_only_until_unique() {
        let mut changes = vec![
            node("describe", &["outer A", "shared"], "a.test.ts"),
            node("describe", &["outer B", "shared"], "a.test.ts"),
        ];
        bake_scoped_names(&mut changes);
        let mut names: Vec<_> = changes.iter().map(|c| c.name.as_str()).collect();
        names.sort();
        assert_eq!(
            names,
            vec!["outer A › shared › describe", "outer B › shared › describe"]
        );
    }
}
