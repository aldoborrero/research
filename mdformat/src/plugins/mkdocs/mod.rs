//! MkDocs parser extension plugin.
//!
//! Extends mdformat to handle MkDocs-specific syntax:
//! - Material admonitions (`!!!`, `???`, `???+`)
//! - Content tabs (`===`, `===!`, `===+`)
//! - Definition lists (`:   `)
//! - mkdocstrings injection blocks (`:::`)
//! - Snippets (`--8<--`)
//! - Captions (`/// caption`)
//! - Abbreviations (`*[ABBR]: ...`)
//! - Arithmatex math (`$...$`, `$$...$$`)
//!
//! # Architecture
//!
//! Uses a pre/post-processing pipeline:
//! 1. **Pre-process**: Identify MkDocs blocks, replace with HTML comment placeholders
//! 2. **Parse + Render**: comrak handles standard Markdown; plugin overrides list indentation
//! 3. **Post-process**: Restore MkDocs blocks, fix inline math
//!
//! # Usage
//!
//! ```
//! use mdformat::plugin::FormatterBuilder;
//! use mdformat::plugins::mkdocs::MkDocsPlugin;
//!
//! let output = FormatterBuilder::new()
//!     .parser_extension(MkDocsPlugin::default())
//!     .format_str("!!! note\n    Hello world.\n");
//! assert!(output.contains("!!! note"));
//! ```

pub mod block_rules;
pub mod config;
pub mod post_process;
pub mod pre_process;

use comrak::nodes::{AstNode, NodeValue};
use comrak::Options;

use crate::plugin::{NodeHandled, ParserExtension};
use crate::renderer::RenderContext;

use self::block_rules::abbreviation::AbbreviationRule;
use self::block_rules::admonition::AdmonitionRule;
use self::block_rules::caption::CaptionRule;
use self::block_rules::content_tabs::ContentTabsRule;
use self::block_rules::definition_list::DefinitionListRule;
use self::block_rules::mkdocstrings::MkdocstringsRule;
use self::block_rules::snippet::SnippetRule;
use self::block_rules::BlockRule;
use self::config::MkDocsConfig;
use self::post_process::{restore_blocks, restore_math};
use self::pre_process::{pre_process, protect_inline_math, CapturedBlock};

use std::cell::RefCell;

/// MkDocs parser extension plugin.
///
/// Handles MkDocs-specific Markdown syntax through a pre/post-processing
/// pipeline layered on top of comrak's standard CommonMark parsing.
pub struct MkDocsPlugin {
    config: MkDocsConfig,
    rules: Vec<Box<dyn BlockRule>>,
    /// Blocks captured during pre-processing, used during post-processing.
    captured: RefCell<Vec<CapturedBlock>>,
}

impl MkDocsPlugin {
    /// Create a new MkDocs plugin with default configuration.
    pub fn new(config: MkDocsConfig) -> Self {
        let rules: Vec<Box<dyn BlockRule>> = vec![
            // Order matters: more specific markers first
            Box::new(AdmonitionRule),
            Box::new(ContentTabsRule),
            Box::new(MkdocstringsRule),
            Box::new(SnippetRule),
            Box::new(CaptionRule),
            Box::new(DefinitionListRule),
            Box::new(AbbreviationRule),
        ];

        Self {
            config,
            rules,
            captured: RefCell::new(Vec::new()),
        }
    }

    /// Build the default set of block rules.
    fn all_rules() -> Vec<Box<dyn BlockRule>> {
        vec![
            Box::new(AdmonitionRule),
            Box::new(ContentTabsRule),
            Box::new(MkdocstringsRule),
            Box::new(SnippetRule),
            Box::new(CaptionRule),
            Box::new(DefinitionListRule),
            Box::new(AbbreviationRule),
        ]
    }
}

impl Default for MkDocsPlugin {
    fn default() -> Self {
        Self {
            config: MkDocsConfig::default(),
            rules: Self::all_rules(),
            captured: RefCell::new(Vec::new()),
        }
    }
}

impl ParserExtension for MkDocsPlugin {
    fn name(&self) -> &str {
        "mkdocs"
    }

    fn configure_options(&self, _options: &mut Options) {
        // MkDocs plugin doesn't need comrak extensions — it works via
        // pre/post-processing rather than parser configuration.
    }

    /// Pre-process the input before comrak parsing.
    ///
    /// Replaces MkDocs block syntax with HTML comment placeholders and
    /// optionally protects inline math expressions.
    fn pre_process(&self, input: &str) -> String {
        // Protect inline math if enabled
        let input = if self.config.no_math {
            input.to_string()
        } else {
            protect_inline_math(input)
        };

        // Replace MkDocs blocks with placeholders
        let (output, captured) = pre_process(&input, &self.rules);
        *self.captured.borrow_mut() = captured;
        output
    }

    /// Override list item rendering to use 4-space indentation.
    fn render_node_enter(
        &self,
        node: &AstNode<'_>,
        ctx: &mut RenderContext<'_>,
    ) -> NodeHandled {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Item(_) => {
                drop(data);
                self.render_list_item_enter(node, ctx);
                NodeHandled::Handled
            }
            _ => NodeHandled::Unhandled,
        }
    }

    fn render_node_leave(
        &self,
        node: &AstNode<'_>,
        ctx: &mut RenderContext<'_>,
    ) -> NodeHandled {
        let data = node.data.borrow();
        match &data.value {
            NodeValue::Item(_) => {
                drop(data);
                ctx.pop_prefix();
                ctx.needs_blank_line = false;
                NodeHandled::Handled
            }
            _ => NodeHandled::Unhandled,
        }
    }

    /// Post-process rendered output: restore MkDocs blocks and math.
    fn post_process(&self, output: String) -> String {
        let captured = self.captured.borrow();

        // Restore MkDocs blocks from placeholders
        let output = if captured.is_empty() {
            output
        } else {
            restore_blocks(&output, &captured, None)
        };

        // Restore math if it was protected
        let output = if self.config.no_math {
            output
        } else {
            restore_math(&output)
        };

        output
    }
}

impl MkDocsPlugin {
    /// Render a list item with MkDocs-style 4-space indentation.
    fn render_list_item_enter(&self, node: &AstNode<'_>, ctx: &mut RenderContext<'_>) {
        let is_first_item = node.previous_sibling().is_none();

        let (ordered, tight) = ctx
            .list_stack
            .last()
            .map(|s| (s.ordered, s.tight))
            .unwrap_or((false, true));

        if !is_first_item && !tight {
            ctx.ensure_blank_line();
        }

        if ordered {
            let num = ctx.list_stack.last().map(|s| s.next_number).unwrap_or(1);
            let marker = format!("{}. ", num);
            ctx.write(&marker);

            // MkDocs uses 4-space indent regardless of marker width
            let indent = " ".repeat(self.config.indent_count);
            ctx.push_prefix(indent);

            if let Some(state) = ctx.list_stack.last_mut() {
                state.next_number += 1;
            }
        } else {
            ctx.write("- ");

            // MkDocs uses 4-space indent for unordered lists too
            let indent = " ".repeat(self.config.indent_count);
            ctx.push_prefix(indent);
        }

        ctx.needs_blank_line = false;
    }
}
