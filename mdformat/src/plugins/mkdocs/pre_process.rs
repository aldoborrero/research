//! Pre-processor: scans input for MkDocs block syntax and replaces them
//! with HTML comment placeholders that comrak passes through untouched.
//!
//! The original block content is stored in a `CapturedBlock` list, keyed
//! by index. The post-processor restores these blocks after rendering.

use super::block_rules::BlockRule;

/// A block captured during pre-processing.
#[derive(Debug, Clone)]
pub struct CapturedBlock {
    /// Index into the captured blocks list.
    pub index: usize,
    /// Name of the block rule that matched (e.g., "admonition").
    pub rule_name: &'static str,
    /// The original raw content of the block.
    pub raw_content: String,
}

/// Unique prefix for placeholder HTML comments. Chosen to be unlikely
/// to appear in real documents.
const PLACEHOLDER_PREFIX: &str = "<!-- __mkdocs_fmt_";
const PLACEHOLDER_SUFFIX: &str = " -->";

/// Generate a placeholder comment for a captured block.
pub fn placeholder(index: usize, rule_name: &str) -> String {
    format!("{PLACEHOLDER_PREFIX}{index}:{rule_name}{PLACEHOLDER_SUFFIX}")
}

/// Parse a placeholder comment, returning `(index, rule_name)`.
pub fn parse_placeholder(line: &str) -> Option<(usize, &str)> {
    let inner = line
        .trim()
        .strip_prefix(PLACEHOLDER_PREFIX)?
        .strip_suffix(PLACEHOLDER_SUFFIX)?;
    let (idx_str, name) = inner.split_once(':')?;
    let index = idx_str.parse().ok()?;
    Some((index, name))
}

/// Pre-process input markdown, replacing MkDocs blocks with placeholders.
///
/// Returns the modified input and the list of captured blocks.
pub fn pre_process(input: &str, rules: &[Box<dyn BlockRule>]) -> (String, Vec<CapturedBlock>) {
    let lines: Vec<&str> = input.lines().collect();
    let mut output_lines: Vec<String> = Vec::with_capacity(lines.len());
    let mut captured: Vec<CapturedBlock> = Vec::new();
    let mut i = 0;

    while i < lines.len() {
        let mut matched = false;

        for rule in rules {
            if let Some(block_match) = rule.try_match(&lines, i) {
                let index = captured.len();
                let placeholder_line = placeholder(index, rule.name());

                captured.push(CapturedBlock {
                    index,
                    rule_name: rule.name(),
                    raw_content: block_match.raw_content,
                });

                output_lines.push(placeholder_line);
                i += block_match.line_count;
                matched = true;
                break;
            }
        }

        if !matched {
            output_lines.push(lines[i].to_string());
            i += 1;
        }
    }

    // Preserve trailing newline if original had one
    let mut result = output_lines.join("\n");
    if input.ends_with('\n') {
        result.push('\n');
    }

    (result, captured)
}

/// Pre-process inline math expressions ($...$ and $$...$$) by wrapping
/// them in HTML tags that comrak won't mangle.
///
/// This prevents comrak from interpreting `*`, `_`, etc. inside math as
/// emphasis markers.
pub fn protect_inline_math(input: &str) -> String {
    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Check for $$ (display math)
        if i + 1 < len && chars[i] == '$' && chars[i + 1] == '$' {
            // Find closing $$
            if let Some(close) = find_closing_dollars(&chars, i + 2, true) {
                result.push_str("<span class=\"mkdocs-math-display\">");
                for &ch in &chars[i..close + 2] {
                    result.push(ch);
                }
                result.push_str("</span>");
                i = close + 2;
                continue;
            }
        }

        // Check for $ (inline math) — not preceded by $ or \
        if chars[i] == '$'
            && (i == 0 || (chars[i - 1] != '$' && chars[i - 1] != '\\'))
        {
            if let Some(close) = find_closing_dollars(&chars, i + 1, false) {
                result.push_str("<span class=\"mkdocs-math-inline\">");
                for &ch in &chars[i..close + 1] {
                    result.push(ch);
                }
                result.push_str("</span>");
                i = close + 1;
                continue;
            }
        }

        result.push(chars[i]);
        i += 1;
    }

    result
}

/// Find the position of closing dollar sign(s).
fn find_closing_dollars(chars: &[char], start: usize, double: bool) -> Option<usize> {
    let mut i = start;
    while i < chars.len() {
        if double {
            if i + 1 < chars.len() && chars[i] == '$' && chars[i + 1] == '$' {
                return Some(i);
            }
        } else if chars[i] == '$' && chars[i] != '\\' {
            // Don't match $$ as single $
            if i + 1 < chars.len() && chars[i + 1] == '$' {
                i += 2;
                continue;
            }
            return Some(i);
        }
        // Skip escaped characters
        if chars[i] == '\\' {
            i += 2;
            continue;
        }
        // Don't cross newlines for inline math
        if !double && chars[i] == '\n' {
            return None;
        }
        i += 1;
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::mkdocs::block_rules::admonition::AdmonitionRule;

    #[test]
    fn test_pre_process_admonition() {
        let input = "# Title\n\n!!! note\n    Body text.\n\nMore content.\n";
        let rules: Vec<Box<dyn BlockRule>> = vec![Box::new(AdmonitionRule)];
        let (output, captured) = pre_process(input, &rules);

        assert_eq!(captured.len(), 1);
        assert_eq!(captured[0].rule_name, "admonition");
        assert_eq!(captured[0].raw_content, "!!! note\n    Body text.");
        assert!(output.contains("<!-- __mkdocs_fmt_0:admonition -->"));
        assert!(output.contains("# Title"));
        assert!(output.contains("More content."));
    }

    #[test]
    fn test_pre_process_no_matches() {
        let input = "# Just a heading\n\nRegular paragraph.\n";
        let rules: Vec<Box<dyn BlockRule>> = vec![Box::new(AdmonitionRule)];
        let (output, captured) = pre_process(input, &rules);

        assert!(captured.is_empty());
        assert_eq!(output, input);
    }

    #[test]
    fn test_parse_placeholder_roundtrip() {
        let ph = placeholder(42, "admonition");
        let (idx, name) = parse_placeholder(&ph).unwrap();
        assert_eq!(idx, 42);
        assert_eq!(name, "admonition");
    }

    #[test]
    fn test_protect_inline_math() {
        let input = "The formula $x^2 + y^2$ is important.";
        let output = protect_inline_math(input);
        assert!(output.contains("<span class=\"mkdocs-math-inline\">$x^2 + y^2$</span>"));
    }

    #[test]
    fn test_protect_display_math() {
        let input = "Here: $$a^2 + b^2 = c^2$$ done.";
        let output = protect_inline_math(input);
        assert!(output.contains("<span class=\"mkdocs-math-display\">$$a^2 + b^2 = c^2$$</span>"));
    }
}
