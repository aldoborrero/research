use comrak::nodes::AstNode;
use comrak::{parse_document, Arena, Options};

use crate::config::Config;

/// Parse a Markdown string into a comrak AST with default options.
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

/// Parse a Markdown string with explicit comrak options.
///
/// Used by the plugin system where extensions configure their own options.
pub fn parse_with_options<'a>(
    input: &str,
    options: &Options,
    arena: &'a Arena<AstNode<'a>>,
) -> &'a AstNode<'a> {
    parse_document(arena, input, options)
}
