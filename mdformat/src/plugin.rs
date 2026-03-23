//! Plugin system for extending mdformat with custom syntax and formatters.
//!
//! mdformat supports two types of plugins, mirroring the Python mdformat design:
//!
//! 1. **Parser extensions** ([`ParserExtension`]) — add support for syntax beyond
//!    CommonMark (e.g., GFM tables, strikethrough, admonitions). These configure
//!    the comrak parser and provide custom renderers for new node types.
//!
//! 2. **Code formatters** ([`CodeFormatter`]) — format code inside fenced code
//!    blocks for specific languages (e.g., format Rust code with rustfmt).
//!
//! # Example: Using plugins
//!
//! ```
//! use mdformat::plugin::FormatterBuilder;
//! use mdformat::plugins::gfm::GfmPlugin;
//!
//! let output = FormatterBuilder::new()
//!     .parser_extension(GfmPlugin::new())
//!     .format_str("| A | B |\n|---|---|\n| 1 | 2 |");
//! ```

use comrak::nodes::AstNode;
use comrak::Options;

use crate::config::Config;
use crate::renderer::RenderContext;

/// Result of a plugin attempting to handle a node.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeHandled {
    /// The plugin rendered this node; skip default rendering.
    Handled,
    /// The plugin did not handle this node; fall through to default.
    Unhandled,
}

/// A parser extension plugin.
///
/// Parser extensions add support for Markdown syntax beyond CommonMark.
/// They configure comrak's parser options and provide custom renderers
/// for the new AST node types that comrak produces.
///
/// # Implementing a plugin
///
/// External crates implement this trait and pass instances to
/// [`FormatterBuilder::parser_extension`].
///
/// ```ignore
/// pub struct MyPlugin;
///
/// impl ParserExtension for MyPlugin {
///     fn name(&self) -> &str { "my-plugin" }
///
///     fn configure_options(&self, options: &mut Options) {
///         // Enable comrak extensions needed by this plugin
///     }
///
///     fn render_node_enter(&self, node: &AstNode<'_>, ctx: &mut RenderContext<'_>) -> NodeHandled {
///         // Handle custom node types on entry
///         NodeHandled::Unhandled
///     }
/// }
/// ```
pub trait ParserExtension {
    /// Unique name for this extension (e.g., `"gfm"`, `"admonitions"`).
    fn name(&self) -> &str;

    /// Configure comrak parser options to enable the syntax this plugin handles.
    fn configure_options(&self, options: &mut Options);

    /// Called when the AST traversal enters a node.
    ///
    /// Return [`NodeHandled::Handled`] to skip default rendering for this node.
    /// The plugin is then responsible for writing all output for this node
    /// (including its children if it traverses them manually).
    fn render_node_enter(
        &self,
        node: &AstNode<'_>,
        ctx: &mut RenderContext<'_>,
    ) -> NodeHandled {
        let _ = (node, ctx);
        NodeHandled::Unhandled
    }

    /// Called when the AST traversal leaves a node.
    ///
    /// Return [`NodeHandled::Handled`] to skip default leave processing.
    fn render_node_leave(
        &self,
        node: &AstNode<'_>,
        ctx: &mut RenderContext<'_>,
    ) -> NodeHandled {
        let _ = (node, ctx);
        NodeHandled::Unhandled
    }

    /// Pre-process the input before parsing.
    ///
    /// Called before the input is passed to comrak. Can be used to
    /// protect custom syntax from being mangled by the parser
    /// (e.g., replacing MkDocs blocks with placeholders).
    fn pre_process(&self, input: &str) -> String {
        input.to_string()
    }

    /// Post-process the final formatted output.
    ///
    /// Called after rendering is complete. Can be used for global
    /// transformations (e.g., normalizing whitespace patterns).
    fn post_process(&self, output: String) -> String {
        output
    }
}

/// A code formatter plugin.
///
/// Code formatters process the content of fenced code blocks for a
/// specific language, similar to how `mdformat-black` formats Python
/// code blocks in the Python mdformat.
///
/// # Example
///
/// ```ignore
/// pub struct RustfmtFormatter;
///
/// impl CodeFormatter for RustfmtFormatter {
///     fn lang(&self) -> &str { "rust" }
///
///     fn format_code(&self, code: &str, info: &str) -> Option<String> {
///         // Run rustfmt on the code and return formatted output
///         Some(formatted)
///     }
/// }
/// ```
pub trait CodeFormatter {
    /// The language identifier this formatter handles (e.g., `"rust"`, `"python"`).
    ///
    /// Matched against the info string of fenced code blocks.
    fn lang(&self) -> &str;

    /// Format a code block's content.
    ///
    /// `code` is the raw code content, `info` is the full info string
    /// (which may contain more than just the language name).
    ///
    /// Return `Some(formatted)` if formatting succeeded, or `None` to
    /// leave the code unchanged.
    fn format_code(&self, code: &str, info: &str) -> Option<String>;
}

/// Builder for configuring a formatter with plugins.
///
/// This is the main entry point for using plugins. Create a builder,
/// register your extensions and formatters, then call `format_str`.
pub struct FormatterBuilder {
    config: Config,
    extensions: Vec<Box<dyn ParserExtension>>,
    code_formatters: Vec<Box<dyn CodeFormatter>>,
}

impl FormatterBuilder {
    /// Create a new formatter builder with default configuration.
    pub fn new() -> Self {
        Self {
            config: Config::default(),
            extensions: Vec::new(),
            code_formatters: Vec::new(),
        }
    }

    /// Set the formatter configuration.
    pub fn config(mut self, config: Config) -> Self {
        self.config = config;
        self
    }

    /// Register a parser extension plugin.
    pub fn parser_extension(mut self, ext: impl ParserExtension + 'static) -> Self {
        self.extensions.push(Box::new(ext));
        self
    }

    /// Register a code formatter plugin.
    pub fn code_formatter(mut self, fmt: impl CodeFormatter + 'static) -> Self {
        self.code_formatters.push(Box::new(fmt));
        self
    }

    /// Format a Markdown string with all registered plugins.
    pub fn format_str(&self, input: &str) -> String {
        use comrak::Arena;

        // Run pre-processing hooks
        let mut processed = input.to_string();
        for ext in &self.extensions {
            processed = ext.pre_process(&processed);
        }

        let arena = Arena::new();
        let root = crate::parser::parse_with_options(&processed, &self.build_options(), &arena);
        let mut output =
            crate::renderer::render_with_plugins(root, &self.config, &self.extensions, &self.code_formatters);

        // Run post-processing hooks
        for ext in &self.extensions {
            output = ext.post_process(output);
        }

        output
    }

    /// Build comrak options by merging all plugin configurations.
    fn build_options(&self) -> Options<'_> {
        let mut options = Options::default();
        options.parse.smart = false;
        for ext in &self.extensions {
            ext.configure_options(&mut options);
        }
        options
    }
}

impl Default for FormatterBuilder {
    fn default() -> Self {
        Self::new()
    }
}
