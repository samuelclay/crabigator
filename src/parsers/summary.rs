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

        // Merge changes by (language, kind, name, file_path) to combine stats
        // Including file_path prevents merging same-named symbols from different files
        type ChangeKey = (NodeKind, String, Option<String>);
        type LangChanges = HashMap<String, HashMap<ChangeKey, ChangeNode>>;
        let mut by_lang: LangChanges = HashMap::new();

        for file in &self.files {
            let lang_entry = by_lang.entry(file.language.clone()).or_default();
            for change in &file.changes {
                let key = (
                    change.kind.clone(),
                    change.name.clone(),
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
                // Sort changes by name, then file_path for consistent ordering
                changes.sort_by(|a, b| {
                    a.name.cmp(&b.name)
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

impl DiffSummary {
    pub fn new() -> Self {
        Self {
            loading: true,
            ..Self::default()
        }
    }

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

            // Add modified functions that aren't already in changes
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
            if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open("/tmp/crabigator-profile.log") {
                use std::io::Write;
                let _ = writeln!(f, "[profile] DiffSummary::refresh took {:?}", start.elapsed());
            }
        }

        Ok(summary)
    }
}

/// Parse hunk headers and context lines to detect modifications inside existing functions
fn parse_hunk_modifications(diff: &str, parser: &dyn DiffParser, filename: &str) -> Vec<ChangeNode> {
    use std::collections::HashMap;

    let file_path = Some(filename.to_string());

    // Track changes with their line counts and line number: (additions, deletions, line_number)
    let mut change_map: HashMap<String, (usize, usize, Option<usize>)> = HashMap::new();
    // Pattern captures: 1=new_line_start, 2=context
    let hunk_re = Regex::new(r"^@@ -\d+(?:,\d+)? \+(\d+)(?:,\d+)? @@\s*(.*)$").unwrap();

    let mut in_hunk = false;
    let mut current_hunk_func: Option<String> = None;
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
                    current_hunk_func = parser.extract_function_from_context(context_str);
                }
            }
            continue;
        }

        // If we're in a hunk and haven't found a function yet, check context lines
        if in_hunk && current_hunk_func.is_none() {
            // Context lines start with space (unchanged lines around the change)
            if let Some(context_str) = line.strip_prefix(' ') {
                if let Some(func_name) = parser.extract_function_from_context(context_str) {
                    current_hunk_func = Some(func_name);
                }
            }
        }

        // When we hit an added/removed line, record the function if found
        if in_hunk && !line.starts_with("+++") && !line.starts_with("---") {
            let is_added = line.starts_with('+');
            let is_removed = line.starts_with('-');

            if is_added || is_removed {
                if let Some(ref func_name) = current_hunk_func {
                    let entry = change_map.entry(func_name.clone()).or_insert((0, 0, current_hunk_line));
                    if is_added { entry.0 += 1; } else { entry.1 += 1; }
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
        .map(|(name, (additions, deletions, line_number))| ChangeNode {
            kind: NodeKind::Function,
            name,
            change_type: ChangeType::Modified,
            additions,
            deletions,
            file_path: file_path.clone(),
            line_number,
            children: Vec::new(),
        })
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
