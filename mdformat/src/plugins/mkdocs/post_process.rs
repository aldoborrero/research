//! Post-processor: restores MkDocs blocks from placeholders and handles
//! inline extension cleanup after rendering.

use super::pre_process::{parse_placeholder, CapturedBlock};

/// Restore MkDocs blocks from HTML comment placeholders in rendered output.
///
/// For each placeholder found, the original block content is re-inserted.
/// The block's inner content can optionally be recursively formatted by
/// passing a formatting function.
pub fn restore_blocks(
    output: &str,
    captured: &[CapturedBlock],
    format_inner: Option<&dyn Fn(&str) -> String>,
) -> String {
    let mut result = String::with_capacity(output.len());

    for line in output.lines() {
        if let Some((index, _rule_name)) = parse_placeholder(line) {
            if let Some(block) = captured.get(index) {
                let restored = match format_inner {
                    Some(fmt) => reformat_block_content(&block.raw_content, fmt),
                    None => block.raw_content.clone(),
                };
                result.push_str(&restored);
                result.push('\n');
            } else {
                // Placeholder with unknown index — pass through
                result.push_str(line);
                result.push('\n');
            }
        } else {
            result.push_str(line);
            result.push('\n');
        }
    }

    // Trim extra trailing newline if output didn't end with one originally
    if !output.ends_with('\n') && result.ends_with('\n') {
        result.pop();
    }

    result
}

/// Reformat the inner content of a MkDocs block.
///
/// Splits the block into its header (first line) and body (indented content),
/// formats the body through the provided formatter, then reassembles.
fn reformat_block_content(raw: &str, format_fn: &dyn Fn(&str) -> String) -> String {
    let lines: Vec<&str> = raw.lines().collect();
    if lines.is_empty() {
        return raw.to_string();
    }

    let header = lines[0];

    // If there's no body, return header as-is
    if lines.len() == 1 {
        return header.to_string();
    }

    // Determine body indentation from the first non-blank body line
    let body_indent = lines[1..]
        .iter()
        .find(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .unwrap_or(4);

    // Strip body indentation
    let body: String = lines[1..]
        .iter()
        .map(|line| {
            if line.trim().is_empty() {
                ""
            } else if line.len() > body_indent {
                &line[body_indent..]
            } else {
                line.trim_start()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    // Format the body content
    let formatted_body = format_fn(&body);

    // Re-indent the formatted body
    let indent: String = std::iter::repeat(' ').take(body_indent).collect();
    let re_indented: String = formatted_body
        .trim_end_matches('\n')
        .lines()
        .map(|line| {
            if line.trim().is_empty() {
                String::new()
            } else {
                format!("{indent}{line}")
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!("{header}\n{re_indented}")
}

/// Remove math protection spans added by the pre-processor.
///
/// Converts `<span class="mkdocs-math-inline">$...$</span>` back to `$...$`.
pub fn restore_math(output: &str) -> String {
    output
        .replace("<span class=\"mkdocs-math-inline\">", "")
        .replace("<span class=\"mkdocs-math-display\">", "")
        .replace("</span>", "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::plugins::mkdocs::pre_process::CapturedBlock;

    #[test]
    fn test_restore_blocks_simple() {
        let output = "# Title\n\n<!-- __mkdocs_fmt_0:admonition -->\n\nMore text.\n";
        let captured = vec![CapturedBlock {
            index: 0,
            rule_name: "admonition",
            raw_content: "!!! note\n    Body content.".to_string(),
        }];

        let result = restore_blocks(output, &captured, None);
        assert!(result.contains("!!! note"));
        assert!(result.contains("    Body content."));
        assert!(result.contains("# Title"));
        assert!(result.contains("More text."));
    }

    #[test]
    fn test_restore_with_reformatter() {
        let output = "<!-- __mkdocs_fmt_0:admonition -->\n";
        let captured = vec![CapturedBlock {
            index: 0,
            rule_name: "admonition",
            raw_content: "!!! note\n    some  text".to_string(),
        }];

        // Simple formatter that uppercases
        let fmt = |s: &str| -> String { s.to_uppercase() };
        let result = restore_blocks(output, &captured, Some(&fmt));
        assert!(result.contains("!!! note"));
        assert!(result.contains("    SOME  TEXT"));
    }

    #[test]
    fn test_restore_math() {
        let input = "The formula <span class=\"mkdocs-math-inline\">$x^2$</span> works.";
        let result = restore_math(input);
        assert_eq!(result, "The formula $x^2$ works.");
    }
}
