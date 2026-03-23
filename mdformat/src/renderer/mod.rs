pub mod blocks;
pub mod escape;
pub mod inlines;

use comrak::arena_tree::NodeEdge;
use comrak::nodes::{AstNode, NodeValue};

use crate::config::Config;
use crate::plugin::{CodeFormatter, NodeHandled, ParserExtension};

/// State for one level of list nesting.
#[derive(Debug, Clone)]
pub struct ListState {
    /// Whether this list is tight (no blank lines between items).
    pub tight: bool,
    /// Whether this is an ordered list.
    pub ordered: bool,
    /// Current item number for ordered lists.
    pub next_number: u32,
}

/// Render context passed through the rendering pipeline.
pub struct RenderContext<'a> {
    /// The output buffer.
    pub output: String,
    /// Stack of indent prefixes for nested structures (block quotes, list items).
    /// Joined together they form the current line prefix.
    pub prefix_stack: Vec<String>,
    /// Whether a blank line is needed before the next block.
    pub needs_blank_line: bool,
    /// Stack tracking list state at each nesting level.
    pub list_stack: Vec<ListState>,
    /// Whether the cursor is at the start of a line (prefix not yet emitted).
    pub at_line_start: bool,
    /// Formatter configuration.
    pub config: &'a Config,
}

impl<'a> RenderContext<'a> {
    pub fn new(config: &'a Config) -> Self {
        Self {
            output: String::new(),
            prefix_stack: Vec::new(),
            needs_blank_line: false,
            list_stack: Vec::new(),
            at_line_start: true,
            config,
        }
    }

    /// Get the current combined prefix from the prefix stack.
    pub fn current_prefix(&self) -> String {
        self.prefix_stack.concat()
    }

    /// Write string to output, inserting the current prefix after each newline.
    pub fn write(&mut self, s: &str) {
        for ch in s.chars() {
            if self.at_line_start && ch != '\n' {
                let prefix = self.current_prefix();
                self.output.push_str(&prefix);
                self.at_line_start = false;
            }
            self.output.push(ch);
            if ch == '\n' {
                self.at_line_start = true;
            }
        }
    }

    /// Ensure the output has a blank line at the end.
    /// Emits the current prefix on the blank line (important for block quotes).
    pub fn ensure_blank_line(&mut self) {
        if self.output.is_empty() {
            self.needs_blank_line = false;
            return;
        }
        if !self.output.ends_with('\n') {
            self.output.push('\n');
        }
        // Emit a blank line with the current prefix (trimmed of trailing spaces).
        // For block quotes this produces ">\n" instead of just "\n".
        let prefix = self.current_prefix();
        let trimmed = prefix.trim_end();
        if !trimmed.is_empty() {
            // Check if the output already ends with a prefixed blank line
            let expected = format!("\n{}\n", trimmed);
            if !self.output.ends_with(&expected) {
                self.output.push_str(trimmed);
                self.output.push('\n');
            }
        } else if !self.output.ends_with("\n\n") {
            self.output.push('\n');
        }
        self.at_line_start = true;
        self.needs_blank_line = false;
    }

    /// Whether we are currently inside a tight list.
    pub fn is_tight(&self) -> bool {
        self.list_stack.last().map_or(false, |s| s.tight)
    }
}

/// Render a comrak AST into a normalized Markdown string (no plugins).
pub fn render<'a>(root: &'a AstNode<'a>, config: &Config) -> String {
    render_with_plugins(root, config, &[], &[])
}

/// Render a comrak AST into a normalized Markdown string with plugin support.
pub fn render_with_plugins<'a>(
    root: &'a AstNode<'a>,
    config: &Config,
    extensions: &[Box<dyn ParserExtension>],
    code_formatters: &[Box<dyn CodeFormatter>],
) -> String {
    let mut ctx = RenderContext::new(config);

    for edge in root.traverse() {
        match edge {
            NodeEdge::Start(node) => {
                // Let plugins handle the node first
                let handled = extensions
                    .iter()
                    .any(|ext| ext.render_node_enter(node, &mut ctx) == NodeHandled::Handled);
                if !handled {
                    render_node_enter(node, &mut ctx, code_formatters);
                }
            }
            NodeEdge::End(node) => {
                let handled = extensions
                    .iter()
                    .any(|ext| ext.render_node_leave(node, &mut ctx) == NodeHandled::Handled);
                if !handled {
                    render_node_leave(node, &mut ctx);
                }
            }
        }
    }

    // Ensure trailing newline
    if !ctx.output.is_empty() && !ctx.output.ends_with('\n') {
        ctx.output.push('\n');
    }

    ctx.output
}

/// Dispatch to the appropriate renderer on node entry.
fn render_node_enter(
    node: &AstNode<'_>,
    ctx: &mut RenderContext<'_>,
    code_formatters: &[Box<dyn CodeFormatter>],
) {
    let nv = node.data.borrow().value.clone();
    match &nv {
        NodeValue::Document => {}
        NodeValue::Heading(_) => blocks::heading_enter(node, ctx),
        NodeValue::Paragraph => blocks::paragraph_enter(node, ctx),
        NodeValue::CodeBlock(cb) => {
            // Apply code formatters if any match the language
            let formatted = find_code_formatter(code_formatters, &cb.info)
                .and_then(|f| f.format_code(&cb.literal, &cb.info));
            if let Some(code) = formatted {
                let mut patched = cb.clone();
                patched.literal = code;
                blocks::code_block_enter_with(node, ctx, &patched);
            } else {
                blocks::code_block_enter(node, ctx);
            }
        }
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
        _ => {}
    }
}

/// Dispatch to the appropriate renderer on node exit.
fn render_node_leave(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    let nv = node.data.borrow().value.clone();
    match &nv {
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

/// Find a code formatter that matches the given info string.
fn find_code_formatter<'a>(
    formatters: &'a [Box<dyn CodeFormatter>],
    info: &str,
) -> Option<&'a dyn CodeFormatter> {
    // The info string may contain more than just the language (e.g., "rust,linenos").
    // Match against the first word.
    let lang = info.split_whitespace().next().unwrap_or("");
    formatters
        .iter()
        .find(|f| f.lang().eq_ignore_ascii_case(lang))
        .map(|f| f.as_ref())
}
