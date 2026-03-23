# MkDocs Plugin Implementation Plan

Architecture: **Pre/Post-Processing Pipeline** on top of comrak.

## Pipeline

```
Input
  → Pre-processor (protect MkDocs block syntax as HTML comments)
  → comrak parse → AST
  → Renderer (with MkDocs overrides for list indentation)
  → Post-processor (restore MkDocs blocks, handle inline extensions)
  → Output
```

## Module Structure

```
src/plugins/
├── mod.rs                      # pub mod gfm; pub mod mkdocs;
└── mkdocs/
    ├── mod.rs                  # MkDocsPlugin impl ParserExtension
    ├── config.rs               # MkDocsConfig (indent, math, semantic breaks)
    ├── pre_process.rs          # Pre-processor: scan + protect MkDocs blocks
    ├── post_process.rs         # Post-processor: restore blocks, inline fixups
    └── block_rules/
        ├── mod.rs              # BlockRule trait + registry
        ├── admonition.rs       # !!!, ???, ???+
        ├── content_tabs.rs     # ===, ===!, ===+
        ├── definition_list.rs  # :   (4-space colon definitions)
        ├── mkdocstrings.rs     # ::: injection blocks
        ├── snippet.rs          # --8<-- includes
        ├── caption.rs          # /// caption blocks
        └── abbreviation.rs     # *[ABBR]: definition
```

## Implementation Steps

### Step 1: Scaffold MkDocs plugin module + config

Create `src/plugins/mkdocs/mod.rs` with `MkDocsPlugin` implementing
`ParserExtension`. Add `MkDocsConfig` with:
- `indent_count: usize` (default 4)
- `align_semantic_breaks_in_lists: bool` (default false)
- `no_math: bool` (default false)
- `ignore_missing_references: bool` (default false)

Wire `MkDocsPlugin` into `FormatterBuilder` like GFM.

### Step 2: Pre-processor framework + admonitions

Implement the `BlockRule` trait and pre-processor scanner:

```rust
pub struct BlockMatch {
    pub line_count: usize,
    pub raw_content: String,
}

pub trait BlockRule {
    fn name(&self) -> &'static str;
    fn try_match(&self, lines: &[&str], index: usize) -> Option<BlockMatch>;
}
```

Pre-processor scans input line-by-line, replaces matched blocks with
HTML comment placeholders:
```
<!-- __mkdocs_block_0:admonition -->
```

The original content is stored in a side-table (`Vec<CapturedBlock>`)
keyed by index.

Implement admonitions first (`!!!`, `???`, `???+`) as the most common
MkDocs extension.

### Step 3: Override list indentation (AST-level)

In `MkDocsPlugin::render_node_enter` for `NodeValue::Item`, override
the default 2-space indent with 4-space:

```rust
// Instead of aligning to marker width, always use 4 spaces
ctx.push_prefix("    ".to_string());
```

Also change unordered list bullet to `-` (already default) and
normalize ordered list numbering to `1.` (single-digit).

This replaces the Python plugin's 400-line `_normalize_list.py` with
~20 lines of AST-level logic.

### Step 4: Content tabs, definition lists, mkdocstrings

Implement remaining block rules:
- **Content tabs** (`===`, `===!`, `===+`): same nesting pattern as admonitions
- **Definition lists** (`: ` with 4-space indent): detect term + definition pattern
- **mkdocstrings injection** (`:::`): detect `:::` open/close blocks
- **Snippets** (`--8<--`): single-line includes
- **Captions** (`/// caption`): detect `///` blocks
- **Abbreviations** (`*[ABBR]: definition`): detect at end of document

### Step 5: Post-processor — block restoration

After rendering, scan output for HTML comment placeholders and replace
them with the original MkDocs syntax, re-indented to match the
surrounding context.

Key: recursively format the *inner content* of MkDocs blocks so
admonition body text gets normalized too:

```rust
fn restore_block(&self, block: &CapturedBlock, formatter: &Formatter) -> String {
    let header = &block.header;
    let inner = formatter.format_str(&block.content);
    let indented = indent(inner.trim(), "    ");
    format!("{header}\n{indented}\n")
}
```

### Step 6: Inline extensions (math, cross-refs, attr lists)

Post-processor handles inline MkDocs syntax:
- **Arithmatex**: Protect `$...$` and `$$...$$` in pre-processor
  (comrak mangles `*` and `_` inside math). Restore in post-processor.
- **Cross-references**: `[text][ref]` passthrough
- **Attribute lists**: `{: .class #id}` passthrough
- **Autorefs**: Reference-style link preservation

### Step 7: CLI flags + config file

Add CLI arguments to `main.rs`:
- `--align-semantic-breaks-in-lists`
- `--ignore-missing-references`
- `--no-mkdocs-math`

Support `.mdformat.toml` `[plugin.mkdocs]` section.

### Step 8: Tests

- Unit tests per block rule (match/no-match)
- Integration tests: full pipeline with MkDocs syntax
- Idempotency tests: format twice → same result
- Canary tests against real MkDocs project markdown files
