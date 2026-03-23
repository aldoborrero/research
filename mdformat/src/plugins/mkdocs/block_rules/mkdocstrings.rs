use super::{BlockMatch, BlockRule};

/// Recognizes mkdocstrings injection blocks.
///
/// Syntax:
/// ```markdown
/// ::: module.path.ClassName
///     handler: python
///     options:
///       show_source: true
/// ```
pub struct MkdocstringsRule;

impl BlockRule for MkdocstringsRule {
    fn name(&self) -> &'static str {
        "mkdocstrings"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        let first = lines.get(index)?;
        let trimmed = first.trim_start();

        if !trimmed.starts_with("::: ") && trimmed != ":::" {
            return None;
        }

        let base_indent = first.len() - trimmed.len();

        // Consume indented option lines
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
    fn test_basic_injection() {
        let input = "::: module.Class\n    handler: python";
        let lines: Vec<&str> = input.lines().collect();
        let rule = MkdocstringsRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 2);
    }

    #[test]
    fn test_bare_injection() {
        let input = "::: module.func";
        let lines: Vec<&str> = input.lines().collect();
        let rule = MkdocstringsRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 1);
    }
}
