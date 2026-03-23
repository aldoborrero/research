/// Configuration for the MkDocs plugin.
#[derive(Debug, Clone)]
pub struct MkDocsConfig {
    /// Number of spaces for list indentation (default: 4 for MkDocs).
    pub indent_count: usize,
    /// Align continuation lines in numbered lists to 3 spaces for semantic breaks.
    pub align_semantic_breaks_in_lists: bool,
    /// Disable math/LaTeX (Arithmatex) handling.
    pub no_math: bool,
    /// Don't escape undefined link references.
    pub ignore_missing_references: bool,
}

impl Default for MkDocsConfig {
    fn default() -> Self {
        Self {
            indent_count: 4,
            align_semantic_breaks_in_lists: false,
            no_math: false,
            ignore_missing_references: false,
        }
    }
}
