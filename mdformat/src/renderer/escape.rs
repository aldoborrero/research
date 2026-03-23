//! Markdown-aware character escaping.
//!
//! When emitting text content, certain characters must be escaped to
//! prevent them from being interpreted as Markdown syntax. The escaping
//! is context-sensitive — e.g., `#` only needs escaping at the start
//! of a line.

/// Escape Markdown-significant characters in a text string.
///
/// This performs context-aware escaping: only characters that would
/// actually trigger Markdown syntax in their current position are
/// escaped with a backslash.
pub fn escape_markdown(text: &str) -> String {
    let mut result = String::with_capacity(text.len() + 16);
    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();

    for (i, &ch) in chars.iter().enumerate() {
        let at_line_start = i == 0 || (i > 0 && chars[i - 1] == '\n');
        let next = if i + 1 < len { Some(chars[i + 1]) } else { None };

        if needs_escape(ch, at_line_start, next, &chars, i) {
            result.push('\\');
        }
        result.push(ch);
    }

    result
}

/// Check whether a character needs escaping at the given position.
fn needs_escape(ch: char, at_line_start: bool, next: Option<char>, chars: &[char], pos: usize) -> bool {
    match ch {
        '\\' => true,
        '*' | '_' => true,
        '[' | ']' => true,
        // `#` only at start of line followed by space or end
        '#' => at_line_start,
        // `-`, `+`, `*` as list markers at start of line followed by space
        '-' | '+' => at_line_start && next.map_or(false, |c| c == ' '),
        // Ordered list marker: digit(s) followed by `.` or `)` at line start
        '.' | ')' => {
            if !at_line_start {
                // Check if preceded by digits from line start
                if pos > 0 && chars[pos - 1].is_ascii_digit() {
                    // Walk back to see if all preceding chars on this line are digits
                    let mut j = pos - 1;
                    loop {
                        if j == 0 || chars[j - 1] == '\n' {
                            return next.map_or(true, |c| c == ' ' || c == '\n');
                        }
                        if !chars[j - 1].is_ascii_digit() {
                            return false;
                        }
                        if j == 0 {
                            break;
                        }
                        j -= 1;
                    }
                }
            }
            false
        }
        '`' => true,
        '<' => true,
        '>' => at_line_start,
        '!' => next == Some('['),
        '|' => true,
        '~' => next == Some('~') || (pos > 0 && chars[pos - 1] == '~'),
        _ => false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_escape_backslash() {
        assert_eq!(escape_markdown(r"a\b"), r"a\\b");
    }

    #[test]
    fn test_escape_emphasis_markers() {
        assert_eq!(escape_markdown("a*b*c"), r"a\*b\*c");
        assert_eq!(escape_markdown("a_b_c"), r"a\_b\_c");
    }

    #[test]
    fn test_escape_brackets() {
        assert_eq!(escape_markdown("[link]"), r"\[link\]");
    }

    #[test]
    fn test_escape_hash_at_line_start() {
        assert_eq!(escape_markdown("# heading"), r"\# heading");
        // Not at line start — no escape
        assert_eq!(escape_markdown("a # b"), "a # b");
    }

    #[test]
    fn test_escape_backtick() {
        assert_eq!(escape_markdown("a`b`c"), r"a\`b\`c");
    }

    #[test]
    fn test_escape_image_bang() {
        assert_eq!(escape_markdown("![alt]"), r"\!\[alt\]");
        // `!` not before `[` — no escape
        assert_eq!(escape_markdown("hello!"), "hello!");
    }

    #[test]
    fn test_escape_pipe() {
        assert_eq!(escape_markdown("a|b"), r"a\|b");
    }

    #[test]
    fn test_escape_tilde() {
        assert_eq!(escape_markdown("~~strike~~"), r"\~\~strike\~\~");
        assert_eq!(escape_markdown("a~b"), "a~b");
    }

    #[test]
    fn test_escape_angle_bracket() {
        assert_eq!(escape_markdown("<html>"), r"\<html>");
        assert_eq!(escape_markdown("> quote"), r"\> quote");
    }

    #[test]
    fn test_plain_text_unchanged() {
        assert_eq!(escape_markdown("hello world"), "hello world");
    }
}
