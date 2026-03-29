use serde::Deserialize;

/// Configuration for the MkDocs plugin.
///
/// Can be set via `.mdformat.toml` under `[plugin.mkdocs]`:
///
/// ```toml
/// [plugin.mkdocs]
/// align_semantic_breaks_in_lists = true
/// ignore_missing_references = false
/// no_mkdocs_math = false
/// ```
///
/// Or via CLI flags: `--align-semantic-breaks-in-lists`, `--ignore-missing-references`,
/// `--no-mkdocs-math`.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct MkDocsConfig {
    /// Number of spaces for list indentation (default: 4 for MkDocs).
    pub indent_count: usize,
    /// Align continuation lines in numbered lists to 3 spaces for semantic breaks.
    pub align_semantic_breaks_in_lists: bool,
    /// Disable math/LaTeX (Arithmatex) handling.
    pub no_mkdocs_math: bool,
    /// Don't escape undefined link references.
    pub ignore_missing_references: bool,
}

impl Default for MkDocsConfig {
    fn default() -> Self {
        Self {
            indent_count: 4,
            align_semantic_breaks_in_lists: false,
            no_mkdocs_math: false,
            ignore_missing_references: false,
        }
    }
}
