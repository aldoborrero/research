use super::{BlockMatch, BlockRule};

/// Recognizes Python-Markdown caption blocks.
///
/// Syntax:
/// ```markdown
/// /// caption
/// Some caption text here.
///
/// | Table | Here |
/// |-------|------|
/// | data  | data |
/// ///
/// ```
pub struct CaptionRule;

impl BlockRule for CaptionRule {
    fn name(&self) -> &'static str {
        "caption"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        let first = lines.get(index)?;
        let trimmed = first.trim();

        if !trimmed.starts_with("/// ") && trimmed != "///" {
            return None;
        }

        // `/// caption` is a block opening — consume until `///` closing or
        // until a line that doesn't match the caption block pattern.
        // Caption blocks contain the caption text and then the captioned element.
        let mut end = index + 1;

        // If it's just "///", this is a closing marker — match single line
        if trimmed == "///" {
            return Some(BlockMatch {
                line_count: 1,
                raw_content: first.to_string(),
            });
        }

        // Otherwise consume until we see a closing "///" or run out of content
        while end < lines.len() {
            let line = lines[end].trim();
            end += 1;
            if line == "///" {
                break;
            }
        }

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
    fn test_caption_block() {
        let input = "/// caption\nMy Table Caption\n///";
        let lines: Vec<&str> = input.lines().collect();
        let rule = CaptionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 3);
    }

    #[test]
    fn test_closing_marker_only() {
        let input = "///";
        let lines: Vec<&str> = input.lines().collect();
        let rule = CaptionRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 1);
    }
}
