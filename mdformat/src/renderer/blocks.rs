//! Block-level renderers: headings, paragraphs, code blocks, lists,
//! block quotes, thematic breaks, and HTML blocks.

use comrak::nodes::AstNode;

use super::RenderContext;

/// ATX heading opening: emit `# ` prefix.
pub fn heading_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit ATX heading prefix")
}

/// ATX heading closing: emit newline.
pub fn heading_leave(_ctx: &mut RenderContext<'_>) {
    todo!("emit heading newline")
}

/// Paragraph opening.
pub fn paragraph_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("handle paragraph open (blank line separation, tight list check)")
}

/// Paragraph closing.
pub fn paragraph_leave(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("handle paragraph close")
}

/// Fenced code block: emit opening fence, content, and closing fence.
pub fn code_block_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit fenced code block with backticks")
}

/// List opening: push tight/loose state.
pub fn list_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("push list state (tight/loose, ordered/unordered)")
}

/// List closing: pop tight/loose state.
pub fn list_leave(_ctx: &mut RenderContext<'_>) {
    todo!("pop list state")
}

/// List item opening: emit bullet or number marker.
pub fn list_item_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit list item marker (- or N.)")
}

/// List item closing.
pub fn list_item_leave(_ctx: &mut RenderContext<'_>) {
    todo!("handle list item close")
}

/// Block quote opening: push `> ` prefix onto indent stack.
pub fn block_quote_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("push blockquote indent prefix")
}

/// Block quote closing: pop `> ` prefix from indent stack.
pub fn block_quote_leave(_ctx: &mut RenderContext<'_>) {
    todo!("pop blockquote indent prefix")
}

/// Thematic break: emit `___`.
pub fn thematic_break(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit thematic break as ___")
}

/// HTML block: pass through unchanged.
pub fn html_block_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("pass through HTML block content")
}
