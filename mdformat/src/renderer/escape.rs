//! Markdown-aware character escaping.
//!
//! When emitting text content, certain characters must be escaped to
//! prevent them from being interpreted as Markdown syntax. The escaping
//! is context-sensitive — e.g., `#` only needs escaping at the start
//! of a line.

/// Characters that may need escaping in Markdown text.
const ESCAPABLE: &[char] = &[
    '\\', '*', '_', '[', ']', '(', ')', '#', '+', '-', '.', '!', '`', '|', '~', '<', '>',
];

/// Escape Markdown-significant characters in a text string.
///
/// This performs context-aware escaping: only characters that would
/// actually trigger Markdown syntax in their current position are
/// escaped with a backslash.
pub fn escape_markdown(_text: &str) -> String {
    todo!("implement context-aware Markdown escaping")
}

/// Check whether a character needs escaping at the given position.
pub fn needs_escape(_ch: char, _at_line_start: bool) -> bool {
    todo!("context-sensitive escape check")
}
