use mdformat::plugin::FormatterBuilder;
use mdformat::plugins::mkdocs::config::MkDocsConfig;
use mdformat::plugins::mkdocs::MkDocsPlugin;

fn mkdocs_format(input: &str) -> String {
    FormatterBuilder::new()
        .parser_extension(MkDocsPlugin::default())
        .format_str(input)
}

fn mkdocs_format_with_config(input: &str, config: MkDocsConfig) -> String {
    FormatterBuilder::new()
        .parser_extension(MkDocsPlugin::new(config))
        .format_str(input)
}

// ─── Admonitions ────────────────────────────────────────────

#[test]
fn mkdocs_admonition_preserved() {
    let input = "!!! note \"Title\"\n    Body text here.\n";
    let output = mkdocs_format(input);
    assert!(output.contains("!!! note \"Title\""), "header preserved");
    assert!(output.contains("    Body text here."), "body preserved");
}

#[test]
fn mkdocs_collapsible_admonition() {
    let input = "??? warning\n    Hidden content.\n";
    let output = mkdocs_format(input);
    assert!(output.contains("??? warning"));
    assert!(output.contains("    Hidden content."));
}

#[test]
fn mkdocs_open_collapsible_admonition() {
    let input = "???+ tip \"Open\"\n    Visible content.\n";
    let output = mkdocs_format(input);
    assert!(output.contains("???+ tip \"Open\""));
    assert!(output.contains("    Visible content."));
}

#[test]
fn mkdocs_admonition_idempotent() {
    let input = "!!! note\n    Content here.\n\n    More content.\n";
    let first = mkdocs_format(input);
    let second = mkdocs_format(&first);
    assert_eq!(first, second, "admonition formatting is idempotent");
}

// ─── Content Tabs ───────────────────────────────────────────

#[test]
fn mkdocs_content_tab_preserved() {
    let input = "=== \"Tab 1\"\n    Tab 1 content.\n";
    let output = mkdocs_format(input);
    assert!(output.contains("=== \"Tab 1\""));
    assert!(output.contains("    Tab 1 content."));
}

// ─── Definition Lists ───────────────────────────────────────

#[test]
fn mkdocs_definition_list_preserved() {
    let input = "Term\n:   Definition text.\n";
    let output = mkdocs_format(input);
    assert!(output.contains("Term"));
    assert!(output.contains(":   Definition text."));
}

// ─── mkdocstrings ───────────────────────────────────────────

#[test]
fn mkdocs_mkdocstrings_injection_preserved() {
    let input = "::: module.Class\n    handler: python\n";
    let output = mkdocs_format(input);
    assert!(output.contains("::: module.Class"));
    assert!(output.contains("    handler: python"));
}

// ─── Snippets ───────────────────────────────────────────────

#[test]
fn mkdocs_snippet_preserved() {
    let input = "--8<-- \"path/to/file.md\"\n";
    let output = mkdocs_format(input);
    assert!(output.contains("--8<-- \"path/to/file.md\""));
}

// ─── Captions ───────────────────────────────────────────────

#[test]
fn mkdocs_caption_preserved() {
    let input = "/// caption\nMy Table\n///\n";
    let output = mkdocs_format(input);
    assert!(output.contains("/// caption"));
    assert!(output.contains("///"));
}

// ─── Abbreviations ──────────────────────────────────────────

#[test]
fn mkdocs_abbreviation_preserved() {
    let input = "*[HTML]: Hyper Text Markup Language\n";
    let output = mkdocs_format(input);
    assert!(output.contains("*[HTML]: Hyper Text Markup Language"));
}

// ─── Math ───────────────────────────────────────────────────

#[test]
fn mkdocs_inline_math_preserved() {
    let input = "The formula $x^2 + y^2$ is well known.\n";
    let output = mkdocs_format(input);
    assert!(
        output.contains("$x^2 + y^2$"),
        "inline math preserved, got: {output}"
    );
}

#[test]
fn mkdocs_display_math_preserved() {
    let input = "Here:\n\n$$a^2 + b^2 = c^2$$\n";
    let output = mkdocs_format(input);
    assert!(
        output.contains("$$a^2 + b^2 = c^2$$"),
        "display math preserved, got: {output}"
    );
}

#[test]
fn mkdocs_math_disabled() {
    let mut config = MkDocsConfig::default();
    config.no_math = true;
    let input = "The formula $x^2$ here.\n";
    let output = mkdocs_format_with_config(input, config);
    // With math disabled, $ is treated as regular text
    assert!(output.contains("$"));
}

// ─── List Indentation ───────────────────────────────────────

#[test]
fn mkdocs_list_4space_indent() {
    let input = "- Item 1\n  - Nested item\n";
    let output = mkdocs_format(input);
    // MkDocs plugin should use 4-space indent for continuation/nested
    assert!(output.contains("- Item 1"), "top-level item present");
    // The nested item should be indented with 4 spaces under the parent
    assert!(
        output.contains("    - Nested item"),
        "nested item has 4-space indent, got: {output}"
    );
}

#[test]
fn mkdocs_ordered_list_4space_indent() {
    let input = "1. First\n   - Sub item\n";
    let output = mkdocs_format(input);
    assert!(output.contains("1. First"));
    // Sub-item should be at 4-space indent
    assert!(
        output.contains("    - Sub item"),
        "ordered list sub-item has 4-space indent, got: {output}"
    );
}

#[test]
fn mkdocs_list_indent_idempotent() {
    let input = "- Item 1\n    - Nested\n        - Deep\n";
    let first = mkdocs_format(input);
    let second = mkdocs_format(&first);
    assert_eq!(first, second, "list indentation is idempotent");
}

// ─── Mixed Content ──────────────────────────────────────────

#[test]
fn mkdocs_mixed_document() {
    let input = "\
# My Doc

Some text with $math$ inline.

!!! note \"Important\"
    This is important.

- List item 1
- List item 2

::: module.func

*[API]: Application Programming Interface
";

    let output = mkdocs_format(input);
    assert!(output.contains("# My Doc"));
    assert!(output.contains("$math$"), "inline math preserved");
    assert!(output.contains("!!! note \"Important\""), "admonition preserved");
    assert!(output.contains("    This is important."), "admonition body preserved");
    assert!(output.contains("- List item 1"), "list preserved");
    assert!(output.contains("::: module.func"), "mkdocstrings preserved");
    assert!(output.contains("*[API]:"), "abbreviation preserved");
}

#[test]
fn mkdocs_mixed_document_idempotent() {
    let input = "\
# Title

!!! warning
    Be careful.

=== \"Tab A\"
    Content A.

- Item
    - Sub

::: my.module

*[URL]: Uniform Resource Locator
";

    let first = mkdocs_format(input);
    let second = mkdocs_format(&first);
    assert_eq!(first, second, "mixed document is idempotent");
}
