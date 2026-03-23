use comrak::{parse_document, Arena, Options};
use comrak::nodes::AstNode;

use crate::config::Config;

/// Parsed Markdown document, owning the arena and AST.
pub struct ParsedDocument<'a> {
    /// The arena that owns all AST nodes.
    _arena: &'a Arena<AstNode<'a>>,
    /// Root node of the AST.
    pub root: &'a AstNode<'a>,
}

/// Parse a Markdown string into a comrak AST.
///
/// The returned `&AstNode` is the root of the document tree. Callers
/// must provide an `Arena` that outlives the returned reference.
pub fn parse_with_arena<'a>(
    input: &str,
    _config: &Config,
    arena: &'a Arena<AstNode<'a>>,
) -> &'a AstNode<'a> {
    let mut options = Options::default();
    options.parse.smart = false; // Keep punctuation as-is
    parse_document(arena, input, &options)
}

/// Convenience wrapper: parse input into an arena-allocated AST.
///
/// Returns the arena and root node together. This is used by the
/// top-level `format_str` API.
pub fn parse(input: &str, config: &Config) -> (Arena<AstNode<'static>>, *const ()) {
    // NOTE: This is a placeholder signature. The real implementation will
    // use a scoped API where the arena lifetime is tied to the render call.
    // For now, the actual parse+render flow is handled inline in format_str.
    let _ = (input, config);
    todo!("parse() will be replaced by direct arena usage in format_str")
}
