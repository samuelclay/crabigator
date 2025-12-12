use super::{ChangeNode, ChangeType, DiffParser, NodeKind};

pub struct GenericParser;

impl DiffParser for GenericParser {
    fn supports(&self, _filename: &str) -> bool {
        // Generic parser supports all files as a fallback
        true
    }

    fn parse(&self, diff: &str, _filename: &str) -> Vec<ChangeNode> {
        let mut changes = Vec::new();

        let mut added_lines = 0u32;
        let mut removed_lines = 0u32;

        for line in diff.lines() {
            if line.starts_with('+') && !line.starts_with("+++") {
                added_lines += 1;
            } else if line.starts_with('-') && !line.starts_with("---") {
                removed_lines += 1;
            }
        }

        // Create a summary node showing line changes
        if added_lines > 0 || removed_lines > 0 {
            let change_type = if added_lines > 0 && removed_lines > 0 {
                ChangeType::Modified
            } else if added_lines > 0 {
                ChangeType::Added
            } else {
                ChangeType::Deleted
            };

            changes.push(ChangeNode {
                kind: NodeKind::Other,
                name: format!("+{} -{}", added_lines, removed_lines),
                change_type,
                children: Vec::new(),
            });
        }

        changes
    }
}
