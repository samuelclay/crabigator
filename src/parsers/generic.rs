use super::{ChangeNode, DiffParser};

pub struct GenericParser;

impl DiffParser for GenericParser {
    fn language(&self) -> &'static str {
        "Other"
    }

    fn supports(&self, _filename: &str) -> bool {
        // Generic parser supports all files as a fallback
        true
    }

    fn extract_function_from_context(&self, _context: &str) -> Option<String> {
        // Generic parser can't extract function context
        None
    }

    fn parse(&self, _diff: &str, _filename: &str) -> Vec<ChangeNode> {
        // Generic parser returns nothing - semantic changes are only
        // meaningful for files with language-specific parsers.
        // Line counts are already shown in the git widget.
        Vec::new()
    }
}
