use super::{BlockMatch, BlockRule};

/// Recognizes Python-Markdown abbreviation definitions.
///
/// Syntax:
/// ```markdown
/// *[HTML]: Hyper Text Markup Language
/// *[W3C]: World Wide Web Consortium
/// ```
pub struct AbbreviationRule;

impl BlockRule for AbbreviationRule {
    fn name(&self) -> &'static str {
        "abbreviation"
    }

    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch> {
        let first = lines.get(index)?;
        let trimmed = first.trim();

        // Must match pattern: *[ABBR]: definition
        if !trimmed.starts_with("*[") {
            return None;
        }

        // Find closing bracket
        let close = trimmed.find("]:")?;
        if close <= 2 {
            return None; // Empty abbreviation
        }

        // Consume consecutive abbreviation lines
        let mut end = index + 1;
        while end < lines.len() {
            let line = lines[end].trim();
            if line.starts_with("*[") && line.contains("]:") {
                end += 1;
            } else {
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
    fn test_single_abbreviation() {
        let input = "*[HTML]: Hyper Text Markup Language";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AbbreviationRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 1);
    }

    #[test]
    fn test_consecutive_abbreviations() {
        let input = "*[HTML]: Hyper Text Markup Language\n*[CSS]: Cascading Style Sheets";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AbbreviationRule;
        let m = rule.try_match(&lines, 0).unwrap();
        assert_eq!(m.line_count, 2);
    }

    #[test]
    fn test_no_match() {
        let input = "Normal text.";
        let lines: Vec<&str> = input.lines().collect();
        let rule = AbbreviationRule;
        assert!(rule.try_match(&lines, 0).is_none());
    }
}
