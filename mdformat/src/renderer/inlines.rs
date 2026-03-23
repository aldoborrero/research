//! Inline renderers: text, emphasis, strong, code spans, links,
//! images, breaks, and inline HTML.

use comrak::nodes::AstNode;

use super::RenderContext;

/// Render a text node, escaping Markdown-significant characters.
pub fn text(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit escaped text content")
}

/// Render an inline code span.
pub fn code(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit code span with normalized backticks")
}

/// Emphasis opening: emit `*`.
pub fn emph_enter(_ctx: &mut RenderContext<'_>) {
    todo!("emit * for emphasis open")
}

/// Emphasis closing: emit `*`.
pub fn emph_leave(_ctx: &mut RenderContext<'_>) {
    todo!("emit * for emphasis close")
}

/// Strong emphasis opening: emit `**`.
pub fn strong_enter(_ctx: &mut RenderContext<'_>) {
    todo!("emit ** for strong open")
}

/// Strong emphasis closing: emit `**`.
pub fn strong_leave(_ctx: &mut RenderContext<'_>) {
    todo!("emit ** for strong close")
}

/// Link opening: emit `[`.
pub fn link_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit link open bracket")
}

/// Link closing: emit `](url "title")`.
pub fn link_leave(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit link URL and title")
}

/// Image opening: emit `![`.
pub fn image_enter(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit image open")
}

/// Image closing: emit `](src "title")`.
pub fn image_leave(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("emit image src and title")
}

/// Soft break: emit newline.
pub fn soft_break(_ctx: &mut RenderContext<'_>) {
    todo!("emit soft break as newline")
}

/// Hard break: emit backslash + newline.
pub fn hard_break(_ctx: &mut RenderContext<'_>) {
    todo!("emit hard break as backslash-newline")
}

/// Inline HTML: pass through unchanged.
pub fn html_inline(_node: &AstNode<'_>, _ctx: &mut RenderContext<'_>) {
    todo!("pass through inline HTML")
}
