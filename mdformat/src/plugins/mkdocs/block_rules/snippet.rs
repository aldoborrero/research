use super::{BlockMatch, BlockRule};

/// Recognizes Python-Markdown snippet includes.
///
/// Syntax:
/// ```markdown
/// --8<-- "path/to/file.md"
/// --8<--
/// path/to/file1.md
/// path/to/file2.md
/// --8<--
/// ```
pub struct SnippetRule;

impl BlockRule for SnippetRule {
    fn name(&self) -> &'static str {
        "snippet"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        let first = lines.get(index)?;
        let trimmed = first.trim();

        if !trimmed.starts_with("--8<--") {
            return None;
        }

        // Single-line snippet: --8<-- "file.md"
        let after = trimmed.strip_prefix("--8<--").unwrap().trim();
        if !after.is_empty() {
            return Some(BlockMatch {
                line_count: 1,
                raw_content: first.to_string(),
            });
        }

        // Multi-line snippet block: --8<-- / files / --8<--
        let mut end = index + 1;
        while end < lines.len() {
            if lines[end].trim() == "--8<--" {
                end += 1; // Include closing marker
                break;
            }
            end += 1;
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
    fn test_single_line_snippet() {
        let input = "--8<-- \"file.md\"";
        let lines: Vec<&str> = input.lines().collect();
        let rule = SnippetRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn test_multi_line_snippet() {
        let input = "--8<--\nfile1.md\nfile2.md\n--8<--";
        let lines: Vec<&str> = input.lines().collect();
        let rule = SnippetRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 4);
    }
}
