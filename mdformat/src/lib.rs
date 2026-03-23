//! mdformat — A CommonMark-compliant, opinionated Markdown formatter.
//!
//! Rust port of [hukkin/mdformat](https://github.com/hukkin/mdformat).
//!
//! # Plugin support
//!
//! mdformat supports two types of plugins:
//!
//! - **Parser extensions** ([`plugin::ParserExtension`]) — add support for syntax
//!   beyond CommonMark (e.g., GFM tables, strikethrough, admonitions).
//! - **Code formatters** ([`plugin::CodeFormatter`]) — format code inside fenced
//!   code blocks for specific languages.
//!
//! Use [`plugin::FormatterBuilder`] to configure plugins:
//!
//! ```
//! use mdformat::plugin::FormatterBuilder;
//! use mdformat::plugins::gfm::GfmPlugin;
//!
//! let output = FormatterBuilder::new()
//!     .parser_extension(GfmPlugin::new())
//!     .format_str("| A | B |\n|---|---|\n| 1 | 2 |");
//! ```

pub mod config;
pub mod parser;
pub mod plugin;
pub mod plugins;
pub mod renderer;

use comrak::Arena;
use config::Config;

/// Format a Markdown string, returning the normalized output.
///
/// The output is guaranteed to be idempotent: formatting the result
/// again produces the same string.
pub fn format_str(input: &str) -> String {
    format_str_with_config(input, &Config::default())
}

/// Format a Markdown string with the given configuration.
pub fn format_str_with_config(input: &str, config: &Config) -> String {
    let arena = Arena::new();
    let root = parser::parse_with_arena(input, config, &arena);
    renderer::render(root, config)
}
