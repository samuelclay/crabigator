//! Diff summary and parser trait

use anyhow::Result;
use regex::Regex;
use tokio::process::Command;

use super::types::{ChangeNode, ChangeType, FileChanges, LanguageChanges, NodeKind};
use super::{GenericParser, PythonParser, RustParser, TypeScriptParser};

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

        let mut by_lang: HashMap<String, Vec<ChangeNode>> = HashMap::new();

        for file in &self.files {
            let entry = by_lang.entry(file.language.clone()).or_default();
            entry.extend(file.changes.iter().cloned());
        }

        let mut result: Vec<_> = by_lang
            .into_iter()
            .map(|(language, changes)| LanguageChanges { language, changes })
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
        let mut summary = DiffSummary::default();

        // Get the diff output
        let output = Command::new("git")
            .args(["diff", "--no-color"])
            .output()
            .await?;

        if !output.status.success() {
            return Ok(summary);
        }

        let diff_output = String::from_utf8_lossy(&output.stdout);

        // Also get staged changes
        let staged_output = Command::new("git")
            .args(["diff", "--cached", "--no-color"])
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
            let modified = parse_hunk_modifications(&file_diff, parser.as_ref());

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

        Ok(summary)
    }
}

/// Parse hunk headers and context lines to detect modifications inside existing functions
fn parse_hunk_modifications(diff: &str, parser: &dyn DiffParser) -> Vec<ChangeNode> {
    let mut changes = Vec::new();
    let hunk_re = Regex::new(r"^@@[^@]+@@\s*(.*)$").unwrap();

    let mut in_hunk = false;
    let mut current_hunk_func: Option<String> = None;

    for line in diff.lines() {
        // Check for hunk header
        if let Some(caps) = hunk_re.captures(line) {
            in_hunk = true;
            current_hunk_func = None;

            // Try to extract function from hunk header context (if present)
            if let Some(context) = caps.get(1) {
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
            if line.starts_with(' ') {
                let context_str = &line[1..];
                if let Some(func_name) = parser.extract_function_from_context(context_str) {
                    current_hunk_func = Some(func_name);
                }
            }
        }

        // When we hit an added/removed line, record the function if found
        if in_hunk && (line.starts_with('+') || line.starts_with('-'))
            && !line.starts_with("+++") && !line.starts_with("---")
        {
            if let Some(ref func_name) = current_hunk_func {
                if !changes.iter().any(|c: &ChangeNode| c.name == *func_name) {
                    changes.push(ChangeNode {
                        kind: NodeKind::Function,
                        name: func_name.clone(),
                        change_type: ChangeType::Modified,
                        children: Vec::new(),
                    });
                }
            }
        }

        // Reset on new file
        if line.starts_with("diff --git") {
            in_hunk = false;
            current_hunk_func = None;
        }
    }

    changes
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
