//! Block-level renderers: headings, paragraphs, code blocks, lists,
//! block quotes, thematic breaks, and HTML blocks.

use comrak::nodes::{
    AstNode, ListType, NodeCodeBlock, NodeHeading, NodeHtmlBlock, NodeList, NodeValue,
};

use super::{ListState, RenderContext};

/// ATX heading opening: emit `# ` prefix.
pub fn heading_enter(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let data = node.data.borrow();
    if let NodeValue::Heading(NodeHeading { level, .. }) = data.value {
        if ctx.needs_blank_line {
            ctx.ensure_blank_line();
        }
        let hashes: String = std::iter::repeat('#').take(level as usize).collect();
        ctx.write(&hashes);
        ctx.write(" ");
    }
}

/// ATX heading closing: emit newline.
pub fn heading_leave(ctx: &mut RenderContext<'_>) {
    ctx.write("\n");
    ctx.needs_blank_line = true;
}

/// Paragraph opening.
pub fn paragraph_enter(_node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    if ctx.needs_blank_line && !ctx.is_tight() {
        ctx.ensure_blank_line();
    }
}

/// Paragraph closing.
pub fn paragraph_leave(_node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    ctx.write("\n");
    ctx.needs_blank_line = true;
}

/// Fenced code block: emit opening fence, content, and closing fence.
///
/// Always uses backtick fences. The fence length is the minimum (3) unless
/// the content contains a run of backticks that would conflict.
pub fn code_block_enter(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let data = node.data.borrow();
    if let NodeValue::CodeBlock(ref cb) = data.value {
        render_code_block(ctx, &cb.info, &cb.literal);
    }
}

/// Render a code block with explicit info/literal (used by plugin code formatters).
pub fn code_block_enter_with(
    _node: &AstNode<'_>,
    ctx: &mut RenderContext<'_>,
    cb: &NodeCodeBlock,
) {
    render_code_block(ctx, &cb.info, &cb.literal);
}

fn render_code_block(ctx: &mut RenderContext<'_>, info: &str, literal: &str) {
    if ctx.needs_blank_line {
        ctx.ensure_blank_line();
    }

    let fence_len = code_fence_length(literal);
    let fence: String = std::iter::repeat('`').take(fence_len).collect();

    ctx.write(&fence);
    if !info.is_empty() {
        ctx.write(info);
    }
    ctx.write("\n");

    // Write content — write() handles prefix insertion at line starts
    if !literal.is_empty() {
        ctx.write(literal);
        if !literal.ends_with('\n') {
            ctx.write("\n");
        }
    }

    ctx.write(&fence);
    ctx.write("\n");
    ctx.needs_blank_line = true;
}

/// Determine the minimum fence length (>= 3) for a code block whose
/// content is `literal`. The fence must be longer than any consecutive
/// run of backticks in the content.
fn code_fence_length(literal: &str) -> usize {
    let mut max_run = 0;
    let mut current_run = 0;
    for ch in literal.chars() {
        if ch == '`' {
            current_run += 1;
            if current_run > max_run {
                max_run = current_run;
            }
        } else {
            current_run = 0;
        }
    }
    std::cmp::max(3, max_run + 1)
}

/// List opening: push tight/loose state.
pub fn list_enter(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let data = node.data.borrow();
    if let NodeValue::List(NodeList {
        list_type,
        tight,
        start,
        ..
    }) = data.value
    {
        if ctx.needs_blank_line {
            ctx.ensure_blank_line();
        }

        ctx.list_stack.push(ListState {
            tight,
            ordered: list_type == ListType::Ordered,
            next_number: start as u32,
        });
    }
}

/// List closing: pop list state.
pub fn list_leave(ctx: &mut RenderContext<'_>) {
    ctx.list_stack.pop();
    ctx.needs_blank_line = true;
}

/// List item opening: emit bullet or number marker.
pub fn list_item_enter(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let is_first_item = node.previous_sibling().is_none();

    let (ordered, tight) = ctx
        .list_stack
        .last()
        .map(|s| (s.ordered, s.tight))
        .unwrap_or((false, true));

    if !is_first_item {
        if !tight {
            ctx.ensure_blank_line();
        }
    }

    let marker = if ordered {
        let num = ctx.list_stack.last().map(|s| s.next_number).unwrap_or(1);
        format!("{}. ", num)
    } else {
        "- ".to_string()
    };

    let indent_width = marker.len();
    let indent: String = std::iter::repeat(' ').take(indent_width).collect();

    // Write the marker
    ctx.write(&marker);

    // Push indent for continuation lines of this item
    ctx.prefix_stack.push(indent);

    // Advance the counter
    if let Some(state) = ctx.list_stack.last_mut() {
        if state.ordered {
            state.next_number += 1;
        }
    }

    // Reset blank line flag so paragraph_enter inside this item
    // doesn't insert a blank line right after the marker.
    ctx.needs_blank_line = false;
}

/// List item closing.
pub fn list_item_leave(ctx: &mut RenderContext<'_>) {
    ctx.prefix_stack.pop();
    ctx.needs_blank_line = false;
}

/// Block quote opening: push `> ` prefix.
pub fn block_quote_enter(_node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    if ctx.needs_blank_line {
        ctx.ensure_blank_line();
    }
    ctx.prefix_stack.push("> ".to_string());
}

/// Block quote closing: pop `> ` prefix.
pub fn block_quote_leave(ctx: &mut RenderContext<'_>) {
    ctx.prefix_stack.pop();
    ctx.needs_blank_line = true;
}

/// Thematic break: emit `___`.
pub fn thematic_break(_node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    if ctx.needs_blank_line {
        ctx.ensure_blank_line();
    }
    ctx.write("___\n");
    ctx.needs_blank_line = true;
}

/// HTML block: pass through unchanged.
pub fn html_block_enter(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let data = node.data.borrow();
    if let NodeValue::HtmlBlock(NodeHtmlBlock { ref literal, .. }) = data.value {
        if ctx.needs_blank_line {
            ctx.ensure_blank_line();
        }
        ctx.write(literal.trim_end());
        ctx.write("\n");
        ctx.needs_blank_line = true;
    }
}
