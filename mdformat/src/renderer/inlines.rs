//! Inline renderers: text, emphasis, strong, code spans, links,
//! images, breaks, and inline HTML.

use comrak::nodes::NodeLink;

use super::escape::escape_markdown;
use super::RenderContext;

/// Render a text node, escaping Markdown-significant characters.
pub fn text(literal: &str, ctx: &mut RenderContext<'_>) {
    ctx.write(&escape_markdown(literal));
}

/// Render an inline code span.
///
/// Chooses the minimum number of backticks that avoids collision with
/// backtick runs inside the content. Adds a space pad when the content
/// starts or ends with a backtick.
pub fn code(literal: &str, ctx: &mut RenderContext<'_>) {
    let ticks = choose_backtick_count(literal);
    let fence: String = std::iter::repeat('`').take(ticks).collect();

    // Pad with space if content starts or ends with backtick, or is empty
    let needs_space = literal.starts_with('`') || literal.ends_with('`') || literal.is_empty();

    ctx.write(&fence);
    if needs_space {
        ctx.write(" ");
    }
    ctx.write(literal);
    if needs_space {
        ctx.write(" ");
    }
    ctx.write(&fence);
}

/// Find the minimum number of backticks (>= 1) that doesn't appear as
/// a consecutive run within `content`.
fn choose_backtick_count(content: &str) -> usize {
    let mut max_run = 0;
    let mut current_run = 0;
    for ch in content.chars() {
        if ch == '`' {
            current_run += 1;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 0;
        }
    }
    if max_run == 0 {
        1
    } else {
        max_run + 1
    }
}

/// Emphasis opening: emit `*`.
pub fn emph_enter(ctx: &mut RenderContext<'_>) {
    ctx.write("*");
}

/// Emphasis closing: emit `*`.
pub fn emph_leave(ctx: &mut RenderContext<'_>) {
    ctx.write("*");
}

/// Strong emphasis opening: emit `**`.
pub fn strong_enter(ctx: &mut RenderContext<'_>) {
    ctx.write("**");
}

/// Strong emphasis closing: emit `**`.
pub fn strong_leave(ctx: &mut RenderContext<'_>) {
    ctx.write("**");
}

/// Link opening: emit `[`.
pub fn link_enter(ctx: &mut RenderContext<'_>) {
    ctx.write("[");
}

/// Link closing: emit `](url)` or `](url "title")`.
pub fn link_leave(link: &NodeLink, ctx: &mut RenderContext<'_>) {
    ctx.write("](");
    ctx.write(&link.url);
    if !link.title.is_empty() {
        ctx.write(" \"");
        ctx.write(&link.title);
        ctx.write("\"");
    }
    ctx.write(")");
}

/// Image opening: emit `![`.
pub fn image_enter(ctx: &mut RenderContext<'_>) {
    ctx.write("![");
}

/// Image closing: emit `](src)` or `](src "title")`.
pub fn image_leave(link: &NodeLink, ctx: &mut RenderContext<'_>) {
    ctx.write("](");
    ctx.write(&link.url);
    if !link.title.is_empty() {
        ctx.write(" \"");
        ctx.write(&link.title);
        ctx.write("\"");
    }
    ctx.write(")");
}

/// Soft break: emit newline (preserves wrapping).
pub fn soft_break(ctx: &mut RenderContext<'_>) {
    ctx.write("\n");
}

/// Hard break: emit backslash + newline.
pub fn hard_break(ctx: &mut RenderContext<'_>) {
    ctx.write("\\\n");
}

/// Inline HTML: pass through unchanged.
pub fn html_inline(literal: &str, ctx: &mut RenderContext<'_>) {
    ctx.write(literal);
}
