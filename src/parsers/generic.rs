use super::{ChangeNode, DiffParser};

pub struct GenericParser;

impl DiffParser for GenericParser {
    fn supports(&self, _filename: &str) -> bool {
        // Generic parser supports all files as a fallback
        true
    }

    fn parse(&self, _diff: &str, _filename: &str) -> Vec<ChangeNode> {
        // Generic parser returns nothing - semantic changes are only
        // meaningful for files with language-specific parsers.
        // Line counts are already shown in the git widget.
        Vec::new()
    }
}
