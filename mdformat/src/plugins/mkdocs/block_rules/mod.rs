pub mod admonition;
pub mod abbreviation;
pub mod caption;
pub mod content_tabs;
pub mod definition_list;
pub mod mkdocstrings;
pub mod snippet;

/// Result of a block rule matching at a given line.
#[derive(Debug, Clone)]
pub struct BlockMatch {
    /// Number of input lines consumed by this match.
    pub line_count: usize,
    /// The raw content of the matched block (preserved verbatim).
    pub raw_content: String,
}

/// A rule that identifies and matches MkDocs-specific block syntax.
///
/// Each implementation recognizes one type of MkDocs block construct
/// (e.g., admonitions, content tabs) and reports how many lines it consumes.
pub trait BlockRule: Send + Sync {
    /// Name of this rule (e.g., "admonition", "content_tabs").
    fn name(&self) -> &'static str;

    /// Try to match at `lines[index]`. Returns `Some(BlockMatch)` if
    /// this rule matches, consuming `line_count` lines starting from `index`.
    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch>;
}
