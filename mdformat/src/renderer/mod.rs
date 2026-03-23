pub mod blocks;
pub mod escape;
pub mod inlines;

use comrak::arena_tree::NodeEdge;
use comrak::nodes::{AstNode, NodeValue};

use crate::config::{Config, LineEnding};
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
    prefix_stack: Vec<String>,
    /// Cached concatenation of `prefix_stack`. Invalidated on push/pop.
    cached_prefix: String,
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
            cached_prefix: String::new(),
            needs_blank_line: false,
            list_stack: Vec::new(),
            at_line_start: true,
            config,
        }
    }

    /// Push a prefix onto the stack (e.g., `"> "` or `"  "`).
    pub fn push_prefix(&mut self, prefix: String) {
        self.cached_prefix.push_str(&prefix);
        self.prefix_stack.push(prefix);
    }

    /// Pop the last prefix from the stack.
    pub fn pop_prefix(&mut self) {
        if let Some(removed) = self.prefix_stack.pop() {
            self.cached_prefix.truncate(self.cached_prefix.len() - removed.len());
        }
    }

    /// Get the current combined prefix.
    pub fn current_prefix(&self) -> &str {
        &self.cached_prefix
    }

    /// Write string to output, inserting the current prefix after each newline.
    ///
    /// Processes chunks between newlines rather than individual characters.
    pub fn write(&mut self, s: &str) {
        let mut remaining = s;
        while !remaining.is_empty() {
            if self.at_line_start {
                if remaining.starts_with('\n') {
                    // Empty line — don't emit prefix
                    self.output.push('\n');
                    remaining = &remaining[1..];
                    continue;
                }
                self.output.push_str(&self.cached_prefix);
                self.at_line_start = false;
            }

            match remaining.find('\n') {
                Some(pos) => {
                    // Write up to and including the newline
                    self.output.push_str(&remaining[..=pos]);
                    remaining = &remaining[pos + 1..];
                    self.at_line_start = true;
                }
                None => {
                    // No more newlines — write the rest
                    self.output.push_str(remaining);
                    break;
                }
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
        let trimmed = self.cached_prefix.trim_end();
        if !trimmed.is_empty() {
            // Check if the output already ends with a prefixed blank line
            let check = format!("\n{}\n", trimmed);
            if !self.output.ends_with(&check) {
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

    // Apply line ending normalization
    normalize_line_endings(ctx.output, config)
}

/// Dispatch to the appropriate renderer on node entry.
fn render_node_enter(
    node: &AstNode<'_>,
    ctx: &mut RenderContext<'_>,
    code_formatters: &[Box<dyn CodeFormatter>],
) {
    // Extract data under a short-lived borrow, then call handlers without
    // holding the RefCell borrow — avoids cloning entire NodeValues.
    enum Action {
        None,
        HeadingEnter(u8),
        ParagraphEnter,
        CodeBlock { info: String, literal: String },
        ListEnter(comrak::nodes::NodeList),
        ListItemEnter,
        BlockQuoteEnter,
        ThematicBreak,
        HtmlBlockEnter(String),
        Text(String),
        Code(String),
        EmphEnter,
        StrongEnter,
        LinkEnter,
        ImageEnter,
        SoftBreak,
        LineBreak,
        HtmlInline(String),
    }

    let action = {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Heading(h) => Action::HeadingEnter(h.level),
            NodeValue::Paragraph => Action::ParagraphEnter,
            NodeValue::CodeBlock(cb) => {
                let formatted = find_code_formatter(code_formatters, &cb.info)
                    .and_then(|f| f.format_code(&cb.literal, &cb.info));
                Action::CodeBlock {
                    info: cb.info.clone(),
                    literal: formatted.unwrap_or_else(|| cb.literal.clone()),
                }
            }
            NodeValue::List(l) => Action::ListEnter(l.clone()),
            NodeValue::Item(_) => Action::ListItemEnter,
            NodeValue::BlockQuote => Action::BlockQuoteEnter,
            NodeValue::ThematicBreak => Action::ThematicBreak,
            NodeValue::HtmlBlock(hb) => Action::HtmlBlockEnter(hb.literal.clone()),
            NodeValue::Text(s) => Action::Text(s.clone()),
            NodeValue::Code(c) => Action::Code(c.literal.clone()),
            NodeValue::Emph => Action::EmphEnter,
            NodeValue::Strong => Action::StrongEnter,
            NodeValue::Link(_) => Action::LinkEnter,
            NodeValue::Image(_) => Action::ImageEnter,
            NodeValue::SoftBreak => Action::SoftBreak,
            NodeValue::LineBreak => Action::LineBreak,
            NodeValue::HtmlInline(s) => Action::HtmlInline(s.clone()),
            _ => Action::None,
        }
    };
    // Borrow is dropped — safe to call handlers that may access node.data

    match action {
        Action::None => {}
        Action::HeadingEnter(level) => blocks::heading_enter(level, ctx),
        Action::ParagraphEnter => blocks::paragraph_enter(ctx),
        Action::CodeBlock { info, literal } => blocks::code_block_enter(&info, &literal, ctx),
        Action::ListEnter(list) => blocks::list_enter(&list, ctx),
        Action::ListItemEnter => blocks::list_item_enter(node, ctx),
        Action::BlockQuoteEnter => blocks::block_quote_enter(ctx),
        Action::ThematicBreak => blocks::thematic_break(ctx),
        Action::HtmlBlockEnter(literal) => blocks::html_block_enter(&literal, ctx),
        Action::Text(s) => inlines::text(&s, ctx),
        Action::Code(literal) => inlines::code(&literal, ctx),
        Action::EmphEnter => inlines::emph_enter(ctx),
        Action::StrongEnter => inlines::strong_enter(ctx),
        Action::LinkEnter => inlines::link_enter(ctx),
        Action::ImageEnter => inlines::image_enter(ctx),
        Action::SoftBreak => inlines::soft_break(ctx),
        Action::LineBreak => inlines::hard_break(ctx),
        Action::HtmlInline(s) => inlines::html_inline(&s, ctx),
    }
}

/// Dispatch to the appropriate renderer on node exit.
fn render_node_leave(node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
    enum Action {
        None,
        HeadingLeave,
        ParagraphLeave,
        ListLeave,
        ListItemLeave,
        BlockQuoteLeave,
        EmphLeave,
        StrongLeave,
        LinkLeave(comrak::nodes::NodeLink),
        ImageLeave(comrak::nodes::NodeLink),
    }

    let action = {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Heading(_) => Action::HeadingLeave,
            NodeValue::Paragraph => Action::ParagraphLeave,
            NodeValue::List(_) => Action::ListLeave,
            NodeValue::Item(_) => Action::ListItemLeave,
            NodeValue::BlockQuote => Action::BlockQuoteLeave,
            NodeValue::Emph => Action::EmphLeave,
            NodeValue::Strong => Action::StrongLeave,
            NodeValue::Link(link) => Action::LinkLeave(link.clone()),
            NodeValue::Image(link) => Action::ImageLeave(link.clone()),
            _ => Action::None,
        }
    };

    match action {
        Action::None => {}
        Action::HeadingLeave => blocks::heading_leave(ctx),
        Action::ParagraphLeave => blocks::paragraph_leave(ctx),
        Action::ListLeave => blocks::list_leave(ctx),
        Action::ListItemLeave => blocks::list_item_leave(ctx),
        Action::BlockQuoteLeave => blocks::block_quote_leave(ctx),
        Action::EmphLeave => inlines::emph_leave(ctx),
        Action::StrongLeave => inlines::strong_leave(ctx),
        Action::LinkLeave(link) => inlines::link_leave(&link, ctx),
        Action::ImageLeave(link) => inlines::image_leave(&link, ctx),
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

/// Normalize line endings in the output based on config.
fn normalize_line_endings(output: String, config: &Config) -> String {
    match config.line_ending {
        LineEnding::Lf => output,
        LineEnding::CrLf => output.replace('\n', "\r\n"),
    }
}
