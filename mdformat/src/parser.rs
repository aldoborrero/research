use comrak::nodes::AstNode;
use comrak::{parse_document, Arena, Options};

use crate::config::Config;

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
