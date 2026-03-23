use super::{BlockMatch, BlockRule};

/// Recognizes MkDocs Material admonitions and Python-Markdown admonitions.
///
/// Syntax:
/// ```markdown
/// !!! note "Optional title"
///     Admonition body content.
///
/// ??? warning "Collapsible"
///     Collapsed by default.
///
/// ???+ tip "Open collapsible"
///     Open by default.
/// ```
pub struct AdmonitionRule;

/// Markers that start an admonition block.
const MARKERS: &[&str] = &["!!!", "???+", "???"];

impl BlockRule for AdmonitionRule {
    fn name(&self) -> &'static str {
        "admonition"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        let first = lines.get(index)?;
        let trimmed = first.trim_start();

        // Check if line starts with an admonition marker
        let marker = MARKERS.iter().find(|m| trimmed.starts_with(**m))?;

        // After the marker, expect a space then a type keyword
        let after_marker = &trimmed[marker.len()..];
        if !after_marker.is_empty() && !after_marker.starts_with(' ') {
            return None;
        }

        // Determine the indentation of the opening line
        let base_indent = first.len() - trimmed.len();

        // Consume all subsequent indented lines (indent > base_indent)
        let mut end = index + 1;
        let mut saw_content = false;
        while end < lines.len() {
            let line = lines[end];
            if line.trim().is_empty() {
                // Blank line — continue, might be separating paragraphs inside
                end += 1;
                continue;
            }
            let line_indent = line.len() - line.trim_start().len();
            if line_indent > base_indent {
                saw_content = true;
                end += 1;
            } else {
                break;
            }
        }

        // Trim trailing blank lines from the match
        while end > index + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
        }

        // An admonition with just a header and no body is still valid
        let _ = saw_content;

        let line_count = end - index;
        let raw_content = lines[index..end].join("\n");
        Some(BlockMatch {
            line_count,
            raw_content,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic_admonition() {
        let input = "!!! note \"Title\"\n    Body text here.\n    More body.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AdmonitionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 3);
        assert_eq!(m.raw_content, input);
    }

    #[test]
    fn test_collapsible_admonition() {
        let input = "??? warning\n    Hidden content.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AdmonitionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 2);
    }

    #[test]
    fn test_open_collapsible() {
        let input = "???+ tip \"Open\"\n    Visible content.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AdmonitionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 2);
    }

    #[test]
    fn test_no_match_regular_text() {
        let input = "This is normal text.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AdmonitionRule;
        assert!(rule.try_match(&lines, 0).is_none());
    }

    #[test]
    fn test_admonition_header_only() {
        let input = "!!! note";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AdmonitionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn test_admonition_with_blank_line_in_body() {
        let input = "!!! note\n    Para 1.\n\n    Para 2.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AdmonitionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 4);
    }
}
