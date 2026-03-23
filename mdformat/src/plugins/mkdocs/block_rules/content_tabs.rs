use super::{BlockMatch, BlockRule};

/// Recognizes MkDocs Material content tabs.
///
/// Syntax:
/// ```markdown
/// === "Tab 1"
///     Tab 1 content.
///
/// ===! "Tab 2"
///     Tab 2 content (forced new set).
///
/// ===+ "Tab 3"
///     Tab 3 content (forced select).
/// ```
pub struct ContentTabsRule;

const MARKERS: &[&str] = &["===!", "===+", "==="];

impl BlockRule for ContentTabsRule {
    fn name(&self) -> &'static str {
        "content_tabs"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        let first = lines.get(index)?;
        let trimmed = first.trim_start();

        let _marker = MARKERS.iter().find(|m| trimmed.starts_with(**m))?;

        // Must be followed by a space and quoted title
        let after = trimmed.trim_start_matches(|c: char| c == '=' || c == '!' || c == '+');
        if !after.starts_with(' ') {
            return None;
        }

        let base_indent = first.len() - trimmed.len();

        // Consume indented body lines
        let mut end = index + 1;
        while end < lines.len() {
            let line = lines[end];
            if line.trim().is_empty() {
                end += 1;
                continue;
            }
            let line_indent = line.len() - line.trim_start().len();
            if line_indent > base_indent {
                end += 1;
            } else {
                break;
            }
        }

        // Trim trailing blank lines
        while end > index + 1 && lines[end - 1].trim().is_empty() {
            end -= 1;
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
    fn test_basic_tab() {
        let input = "=== \"Tab 1\"\n    Content here.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = ContentTabsRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 2);
    }

    #[test]
    fn test_no_match_without_space() {
        let input = "===no space";
        let lines: Vec<&str> = input.lines().collect();
        let rule = ContentTabsRule;
        assert!(rule.try_match(&lines, 0).is_none());
    }
}
