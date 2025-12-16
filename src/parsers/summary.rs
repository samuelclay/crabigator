//! Diff summary and parser trait

use anyhow::Result;
use tokio::process::Command;

use super::types::{ChangeNode, FileChanges};
use super::{GenericParser, PythonParser, RustParser, TypeScriptParser};

/// Trait for language-specific diff parsers
pub trait DiffParser: Send + Sync {
    fn supports(&self, filename: &str) -> bool;
    fn parse(&self, diff: &str, filename: &str) -> Vec<ChangeNode>;
}

#[derive(Clone, Debug, Default)]
pub struct DiffSummary {
    pub files: Vec<FileChanges>,
    pub loading: bool,
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

            let changes = parser.parse(&file_diff, &filename);

            if !changes.is_empty() {
                summary.files.push(FileChanges {
                    path: filename,
                    changes,
                });
            }
        }

        Ok(summary)
    }
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
