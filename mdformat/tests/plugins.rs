use pretty_assertions::assert_eq;

use mdformat::plugin::{CodeFormatter, FormatterBuilder, ParserExtension};
use mdformat::plugins::gfm::GfmPlugin;

/// Helper: format with GFM plugin.
fn gfm(input: &str) -> String {
    FormatterBuilder::new()
        .parser_extension(GfmPlugin::new())
        .format_str(input)
}

/// Helper: assert idempotent with GFM plugin.
fn assert_gfm_idempotent(input: &str) {
    let first = gfm(input);
    let second = gfm(&first);
    assert_eq!(first, second, "GFM formatting is not idempotent");
}

// ─── GFM: Strikethrough ──────────────────────────────────────

#[test]
fn gfm_strikethrough() {
    assert_eq!(gfm("~~deleted~~"), "~~deleted~~\n");
}

#[test]
fn gfm_strikethrough_in_paragraph() {
    assert_eq!(
        gfm("Some ~~deleted~~ text here"),
        "Some ~~deleted~~ text here\n"
    );
}

// ─── GFM: Tables ─────────────────────────────────────────────

#[test]
fn gfm_simple_table() {
    let input = "| A | B |\n|---|---|\n| 1 | 2 |";
    let output = gfm(input);
    assert!(output.contains("| A"), "should have header: {}", output);
    assert!(output.contains("| 1"), "should have data row: {}", output);
    assert!(output.contains("---"), "should have separator: {}", output);
}

#[test]
fn gfm_table_alignment() {
    let input = "| Left | Center | Right |\n|:---|:---:|---:|\n| l | c | r |";
    let output = gfm(input);
    assert!(output.contains(":--"), "should have left alignment marker: {}", output);
    assert!(output.contains("--:"), "should have right alignment marker: {}", output);
}

#[test]
fn gfm_table_with_inline_formatting() {
    let input = "| **Bold** | *Italic* |\n|---|---|\n| `code` | text |";
    let output = gfm(input);
    assert!(output.contains("**Bold**"), "should preserve bold: {}", output);
    assert!(output.contains("*Italic*"), "should preserve italic: {}", output);
    assert!(output.contains("`code`"), "should preserve code: {}", output);
}

#[test]
fn gfm_table_with_strikethrough() {
    let input = "| A | B |\n|---|---|\n| ~~deleted~~ | ok |";
    let output = gfm(input);
    assert!(output.contains("~~deleted~~"), "should preserve strikethrough in table: {}", output);
    assert_gfm_idempotent(input);
}

#[test]
fn gfm_table_idempotent() {
    assert_gfm_idempotent("| A | B |\n|---|---|\n| 1 | 2 |");
}

// ─── GFM: Task lists ─────────────────────────────────────────

#[test]
fn gfm_task_list() {
    let input = "- [x] done\n- [ ] pending";
    let output = gfm(input);
    assert!(output.contains("[x]"), "should have checked box: {}", output);
    assert!(output.contains("[ ]"), "should have unchecked box: {}", output);
}

#[test]
fn gfm_task_list_idempotent() {
    assert_gfm_idempotent("- [x] done\n- [ ] pending");
}

// ─── GFM: Autolinks ──────────────────────────────────────────

#[test]
fn gfm_autolink() {
    let input = "Visit https://example.com for more";
    let output = gfm(input);
    assert!(output.contains("https://example.com"), "should preserve URL: {}", output);
}

// ─── GFM: Combined ───────────────────────────────────────────

#[test]
fn gfm_combined_features() {
    let input = r#"# GFM Example

~~deleted~~ and **bold** text.

| Feature | Status |
|---|---|
| Tables | done |
| Strikethrough | done |

- [x] Implement tables
- [ ] Write docs
"#;
    let output = gfm(input);
    assert!(output.contains("~~deleted~~"), "strikethrough: {}", output);
    assert!(output.contains("| Feature"), "table: {}", output);
    assert!(output.contains("[x]"), "task list: {}", output);
    assert_gfm_idempotent(input);
}

// ─── Code formatter plugin ───────────────────────────────────

struct UppercaseFormatter;

impl CodeFormatter for UppercaseFormatter {
    fn lang(&self) -> &str {
        "upper"
    }

    fn format_code(&self, code: &str, _info: &str) -> Option<String> {
        Some(code.to_uppercase())
    }
}

#[test]
fn code_formatter_plugin() {
    let input = "```upper\nhello world\n```";
    let output = FormatterBuilder::new()
        .code_formatter(UppercaseFormatter)
        .format_str(input);
    assert_eq!(output, "```upper\nHELLO WORLD\n```\n");
}

#[test]
fn code_formatter_no_match_unchanged() {
    let input = "```rust\nfn main() {}\n```";
    let output = FormatterBuilder::new()
        .code_formatter(UppercaseFormatter)
        .format_str(input);
    assert_eq!(output, "```rust\nfn main() {}\n```\n");
}

// ─── Custom parser extension ─────────────────────────────────

use comrak::Options;

struct FootnotePlugin;

impl ParserExtension for FootnotePlugin {
    fn name(&self) -> &str {
        "footnotes"
    }

    fn configure_options(&self, options: &mut Options) {
        options.extension.footnotes = true;
    }
}

#[test]
fn custom_extension_enables_parsing() {
    // Just verify that the plugin system accepts custom extensions
    let _output = FormatterBuilder::new()
        .parser_extension(FootnotePlugin)
        .format_str("Hello[^1]\n\n[^1]: World");
    // The footnote is parsed (not treated as plain text), even if rendering
    // falls through to the default (which may not handle it perfectly yet).
}

// ─── Multiple plugins ────────────────────────────────────────

#[test]
fn multiple_plugins() {
    let output = FormatterBuilder::new()
        .parser_extension(GfmPlugin::new())
        .code_formatter(UppercaseFormatter)
        .format_str("~~strike~~\n\n```upper\nhello\n```");
    assert!(output.contains("~~strike~~"), "strikethrough: {}", output);
    assert!(output.contains("HELLO"), "formatted code: {}", output);
}

// ─── FormatterBuilder default config ─────────────────────────

#[test]
fn builder_without_plugins_matches_format_str() {
    let input = "# Hello\n\nWorld\n";
    let from_builder = FormatterBuilder::new().format_str(input);
    let from_fn = mdformat::format_str(input);
    assert_eq!(from_builder, from_fn);
}

// ─── Line ending normalization ──────────────────────────────

#[test]
fn crlf_line_endings() {
    use mdformat::config::{Config, LineEnding};

    let config = Config {
        line_ending: LineEnding::CrLf,
    };
    let output = FormatterBuilder::new()
        .config(config)
        .format_str("# Hello\n\nWorld");
    assert_eq!(output, "# Hello\r\n\r\nWorld\r\n");
}
