pub mod blocks;
pub mod escape;
pub mod inlines;

use comrak::arena_tree::NodeEdge;
use comrak::nodes::{AstNode, NodeValue};

use crate::config::Config;

/// Render context passed through the rendering pipeline.
pub struct RenderContext<'a> {
    /// The output buffer.
    pub output: String,
    /// Current indentation prefix (for nested structures).
    pub indent: String,
    /// Whether a blank line is needed before the next block.
    pub needs_blank_line: bool,
    /// Stack tracking tight/loose list state.
    pub tight_stack: Vec<bool>,
    /// Formatter configuration.
    pub config: &'a Config,
}

impl<'a> RenderContext<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            output: String::new(),
            indent: String::new(),
            needs_blank_line: false,
            tight_stack: Vec::new(),
            config,
        }
    }

    /// Ensure the output ends with a blank line separator between blocks.
    pub fn ensure_blank_line(&mut self) {
        if self.needs_blank_line && !self.output.is_empty() {
            if !self.output.ends_with("\n\n") {
                if !self.output.ends_with('\n') {
                    self.output.push('\n');
                }
                self.output.push('\n');
            }
        }
        self.needs_blank_line = false;
    }

    /// Whether we are currently inside a tight list.
    pub fn is_tight(&self) -> bool {
        self.tight_stack.last().copied().unwrap_or(false)
    }
}

/// Render a comrak AST into a normalized Markdown string.
pub fn render<'a>(root: &'a AstNode<'a>, config: &Config) -> String {
    let mut ctx = RenderContext::new(config);

    for edge in root.traverse() {
        match edge {
            NodeEdge::Start(node) => render_node_enter(node, &mut ctx),
            NodeEdge::End(node) => render_node_leave(node, &mut ctx),
        }
    }

    // Ensure trailing newline
    if !ctx.output.is_empty() && !ctx.output.ends_with('\n') {
        ctx.output.push('\n');
    }

    ctx.output
}

/// Dispatch to the appropriate renderer on node entry.
fn render_node_enter(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Document => {}
        NodeValue::Heading(_) => blocks::heading_enter(node, ctx),
        NodeValue::Paragraph => blocks::paragraph_enter(node, ctx),
        NodeValue::CodeBlock(_) => blocks::code_block_enter(node, ctx),
        NodeValue::List(_) => blocks::list_enter(node, ctx),
        NodeValue::Item(_) => blocks::list_item_enter(node, ctx),
        NodeValue::BlockQuote => blocks::block_quote_enter(node, ctx),
        NodeValue::ThematicBreak => blocks::thematic_break(node, ctx),
        NodeValue::HtmlBlock(_) => blocks::html_block_enter(node, ctx),
        NodeValue::Text(_) => inlines::text(node, ctx),
        NodeValue::Code(_) => inlines::code(node, ctx),
        NodeValue::Emph => inlines::emph_enter(ctx),
        NodeValue::Strong => inlines::strong_enter(ctx),
        NodeValue::Link(_) => inlines::link_enter(node, ctx),
        NodeValue::Image(_) => inlines::image_enter(node, ctx),
        NodeValue::SoftBreak => inlines::soft_break(ctx),
        NodeValue::LineBreak => inlines::hard_break(ctx),
        NodeValue::HtmlInline(_) => inlines::html_inline(node, ctx),
        _ => {} // Other node types handled as needed
    }
}

/// Dispatch to the appropriate renderer on node exit.
fn render_node_leave(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let data = node.data.borrow();
    match &data.value {
        NodeValue::Document => {}
        NodeValue::Heading(_) => blocks::heading_leave(ctx),
        NodeValue::Paragraph => blocks::paragraph_leave(node, ctx),
        NodeValue::CodeBlock(_) => {}
        NodeValue::List(_) => blocks::list_leave(ctx),
        NodeValue::Item(_) => blocks::list_item_leave(ctx),
        NodeValue::BlockQuote => blocks::block_quote_leave(ctx),
        NodeValue::Emph => inlines::emph_leave(ctx),
        NodeValue::Strong => inlines::strong_leave(ctx),
        NodeValue::Link(_) => inlines::link_leave(node, ctx),
        NodeValue::Image(_) => inlines::image_leave(node, ctx),
        _ => {}
    }
}
