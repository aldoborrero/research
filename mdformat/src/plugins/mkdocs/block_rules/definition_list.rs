use super::{BlockMatch, BlockRule};

/// Recognizes Python-Markdown definition lists.
///
/// Syntax:
/// ```markdown
/// Term
/// :   Definition text with 4-space indent after colon.
///
///     Continuation paragraph.
/// ```
pub struct DefinitionListRule;

impl BlockRule for DefinitionListRule {
    fn name(&self) -> &'static str {
        "definition_list"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        // Need at least 2 lines: term + definition
        if index + 1 >= lines.len() {
            return None;
        }

        let term_line = lines[index];

        // Term line must be non-empty, non-indented, and not a block marker
        if term_line.trim().is_empty() || term_line.starts_with(' ') || term_line.starts_with('\t')
        {
            return None;
        }

        // Next line must start with `:` followed by spaces
        let def_line = lines[index + 1];
        let def_trimmed = def_line.trim_start();
        if !def_trimmed.starts_with(':') {
            return None;
        }
        let after_colon = &def_trimmed[1..];
        if !after_colon.starts_with("   ") && !after_colon.starts_with('\t') {
            return None;
        }

        // Consume definition and continuation lines
        let mut end = index + 2;
        while end < lines.len() {
            let line = lines[end];
            if line.trim().is_empty() {
                end += 1;
                continue;
            }
            let trimmed = line.trim_start();
            // Another definition for the same or new term
            if trimmed.starts_with(':') {
                let after = &trimmed[1..];
                if after.starts_with("   ") || after.starts_with('\t') {
                    end += 1;
                    continue;
                }
            }
            // Continuation with 4+ space indent
            let indent = line.len() - trimmed.len();
            if indent >= 4 {
                end += 1;
                continue;
            }
            break;
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
    fn test_basic_deflist() {
        let input = "Term\n:   Definition here.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = DefinitionListRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 2);
    }

    #[test]
    fn test_no_match_no_colon() {
        let input = "Term\nNot a definition.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = DefinitionListRule;
        assert!(rule.try_match(&lines, 0).is_none());
    }
}
