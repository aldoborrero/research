//! mdformat — A CommonMark-compliant, opinionated Markdown formatter.
//!
//! Rust port of [hukkin/mdformat](https://github.com/hukkin/mdformat).

pub mod config;
pub mod parser;
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
