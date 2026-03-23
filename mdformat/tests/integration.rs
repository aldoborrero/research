use pretty_assertions::assert_eq;

/// Helper: format input and assert the expected output.
fn fmt(input: &str) -> String {
    mdformat::format_str(input)
}

/// Helper: assert that formatting is idempotent.
fn assert_idempotent(input: &str) {
    let first = fmt(input);
    let second = fmt(&first);
    assert_eq!(first, second, "formatting is not idempotent");
}

// ─── Headings ────────────────────────────────────────────────

#[test]
fn atx_headings() {
    assert_eq!(fmt("# H1"), "# H1\n");
    assert_eq!(fmt("## H2"), "## H2\n");
    assert_eq!(fmt("###### H6"), "###### H6\n");
}

#[test]
fn setext_heading_normalized_to_atx() {
    assert_eq!(fmt("Heading\n======="), "# Heading\n");
    assert_eq!(fmt("Heading\n-------"), "## Heading\n");
}

#[test]
fn heading_with_inline_formatting() {
    assert_eq!(fmt("# Hello **world**"), "# Hello **world**\n");
}

// ─── Paragraphs ──────────────────────────────────────────────

#[test]
fn simple_paragraph() {
    assert_eq!(fmt("Hello world"), "Hello world\n");
}

#[test]
fn two_paragraphs_separated_by_blank_line() {
    assert_eq!(fmt("Para one\n\nPara two"), "Para one\n\nPara two\n");
}

#[test]
fn trailing_newline_normalized() {
    assert_eq!(fmt("Hello\n\n\n"), "Hello\n");
}

// ─── Emphasis and strong ─────────────────────────────────────

#[test]
fn emphasis_uses_asterisk() {
    // comrak parses *italic* as Emph node; we always render with *
    assert_eq!(fmt("*italic*"), "*italic*\n");
    // Underscores are also rendered as asterisks
    assert_eq!(fmt("_italic_"), "*italic*\n");
}

#[test]
fn emphasis_rendering() {
    assert_eq!(fmt("This is *emphasized* text"), "This is *emphasized* text\n");
}

#[test]
fn strong_rendering() {
    assert_eq!(fmt("This is **strong** text"), "This is **strong** text\n");
}

#[test]
fn nested_emphasis() {
    assert_eq!(
        fmt("***bold and italic***"),
        "***bold and italic***\n"
    );
}

// ─── Code spans ──────────────────────────────────────────────

#[test]
fn inline_code() {
    assert_eq!(fmt("Use `fmt` here"), "Use `fmt` here\n");
}

#[test]
fn inline_code_with_backticks() {
    assert_eq!(
        fmt("Use `` `backtick` `` here"),
        "Use `` `backtick` `` here\n"
    );
}

// ─── Links and images ────────────────────────────────────────

#[test]
fn inline_link() {
    assert_eq!(
        fmt("[click](https://example.com)"),
        "[click](https://example.com)\n"
    );
}

#[test]
fn link_with_title() {
    assert_eq!(
        fmt("[click](https://example.com \"Title\")"),
        "[click](https://example.com \"Title\")\n"
    );
}

#[test]
fn image() {
    assert_eq!(
        fmt("![alt text](image.png)"),
        "![alt text](image.png)\n"
    );
}

#[test]
fn image_with_title() {
    assert_eq!(
        fmt("![alt](pic.jpg \"Photo\")"),
        "![alt](pic.jpg \"Photo\")\n"
    );
}

// ─── Code blocks ─────────────────────────────────────────────

#[test]
fn fenced_code_block() {
    assert_eq!(
        fmt("```\ncode\n```"),
        "```\ncode\n```\n"
    );
}

#[test]
fn fenced_code_block_with_language() {
    assert_eq!(
        fmt("```rust\nfn main() {}\n```"),
        "```rust\nfn main() {}\n```\n"
    );
}

#[test]
fn indented_code_block_becomes_fenced() {
    assert_eq!(
        fmt("    indented code"),
        "```\nindented code\n```\n"
    );
}

#[test]
fn code_block_with_backticks_in_content() {
    let input = "````\ncode with ``` backticks\n````";
    let output = fmt(input);
    assert!(output.starts_with("````\n"));
    assert!(output.contains("code with ``` backticks"));
}

// ─── Lists ───────────────────────────────────────────────────

#[test]
fn unordered_list_tight() {
    assert_eq!(
        fmt("- a\n- b\n- c"),
        "- a\n- b\n- c\n"
    );
}

#[test]
fn unordered_list_bullet_normalization() {
    // All bullet styles normalized to `-`
    assert_eq!(fmt("* item"), "- item\n");
    assert_eq!(fmt("+ item"), "- item\n");
    assert_eq!(fmt("- item"), "- item\n");
}

#[test]
fn ordered_list_tight() {
    assert_eq!(
        fmt("1. a\n2. b\n3. c"),
        "1. a\n2. b\n3. c\n"
    );
}

#[test]
fn ordered_list_renumbered() {
    // comrak preserves start number; items are sequential from there
    assert_eq!(
        fmt("1. a\n1. b\n1. c"),
        "1. a\n2. b\n3. c\n"
    );
}

#[test]
fn loose_list() {
    assert_eq!(
        fmt("- a\n\n- b\n\n- c"),
        "- a\n\n- b\n\n- c\n"
    );
}

// ─── Block quotes ────────────────────────────────────────────

#[test]
fn block_quote_simple() {
    assert_eq!(
        fmt("> Hello world"),
        "> Hello world\n"
    );
}

#[test]
fn block_quote_multiple_paragraphs() {
    assert_eq!(
        fmt("> Para 1\n>\n> Para 2"),
        "> Para 1\n>\n> Para 2\n"
    );
}

#[test]
fn nested_block_quotes() {
    assert_eq!(
        fmt("> > Nested"),
        "> > Nested\n"
    );
}

// ─── Thematic breaks ────────────────────────────────────────

#[test]
fn thematic_break_normalized() {
    assert_eq!(fmt("---"), "___\n");
    assert_eq!(fmt("***"), "___\n");
    assert_eq!(fmt("___"), "___\n");
}

// ─── Hard and soft breaks ────────────────────────────────────

#[test]
fn hard_break_uses_backslash() {
    // Trailing spaces become backslash hard break
    assert_eq!(
        fmt("line one  \nline two"),
        "line one\\\nline two\n"
    );
}

#[test]
fn soft_break_preserved() {
    assert_eq!(
        fmt("line one\nline two"),
        "line one\nline two\n"
    );
}

// ─── HTML ────────────────────────────────────────────────────

#[test]
fn inline_html_passthrough() {
    assert_eq!(
        fmt("Hello <em>world</em>"),
        "Hello <em>world</em>\n"
    );
}

#[test]
fn html_block_passthrough() {
    assert_eq!(
        fmt("<div>\nHello\n</div>"),
        "<div>\nHello\n</div>\n"
    );
}

// ─── Block separation ────────────────────────────────────────

#[test]
fn blocks_separated_by_blank_lines() {
    let input = "# Heading\n\nParagraph\n\n- list item";
    let output = fmt(input);
    assert_eq!(output, "# Heading\n\nParagraph\n\n- list item\n");
}

// ─── Idempotency ─────────────────────────────────────────────

#[test]
fn idempotent_headings() {
    assert_idempotent("# Hello\n\n## World");
}

#[test]
fn idempotent_paragraphs() {
    assert_idempotent("Para one\n\nPara two\n\nPara three");
}

#[test]
fn idempotent_lists() {
    assert_idempotent("- a\n- b\n- c");
    assert_idempotent("1. a\n2. b\n3. c");
    assert_idempotent("- a\n\n- b\n\n- c");
}

#[test]
fn idempotent_block_quotes() {
    assert_idempotent("> Hello\n>\n> World");
}

#[test]
fn idempotent_code_blocks() {
    assert_idempotent("```rust\nfn main() {}\n```");
}

#[test]
fn idempotent_complex_document() {
    let input = r#"# Title

Paragraph with **bold**, *italic*, and `code`.

- item 1
- item 2

1. first
2. second

> A quote
>
> With two paragraphs

```rust
fn main() {}
```

___

[link](https://example.com "title") and ![img](pic.png)
"#;
    assert_idempotent(input);
}
