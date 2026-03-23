# mdformat-rs: Rust Port of mdformat

A CommonMark-compliant, opinionated Markdown formatter — a Rust port of
[hukkin/mdformat](https://github.com/hukkin/mdformat).

## Parser / AST Crate: comrak

**Chosen:** [comrak](https://crates.io/crates/comrak) (v0.36+)

### Rationale

| Criterion | pulldown-cmark | comrak | markdown-it-rs |
|---|---|---|---|
| Parse output | Event stream | **Full AST (arena)** | Token list |
| CommonMark spec | ✅ | ✅ | ✅ |
| GFM extensions | Partial | **Full** | Via plugins |
| Source positions | Offset ranges | **Line:col on every node** | ✅ |
| Tree traversal API | ❌ (must build own) | **Native (traverse/children/descendants)** | Partial |
| Built-in CM re-emit | ❌ | **✅ (format_commonmark)** | ❌ |
| Maturity | Very high | **High (used by GitLab)** | Moderate |

**Why comrak wins for a formatter:**

1. **Full AST with tree structure** — parent/child/sibling links make it natural to
   traverse and emit formatted output with full context awareness.
2. **Built-in `format_commonmark()`** — proves the AST retains enough information to
   reconstruct Markdown. We use this as a reference and replace it with our opinionated
   renderer.
3. **Arena-based allocation** — cache-friendly traversal, no Rc/RefCell overhead.
4. **Source positions on every node** — useful for diagnostics and future
   partial-formatting features.
5. **GFM + extensions built in** — tables, task lists, strikethrough, footnotes
   supported without extra work.

**Why not pulldown-cmark?** Its streaming event model would require us to build our own
AST layer on top — comrak already provides exactly what we need.

**Why not markdown-it-rs?** Less mature, less actively maintained, and the token-based
model is awkward for tree-structured formatting logic.

> Note: The Python mdformat uses markdown-it-py (a token-stream parser). We deliberately
> choose a tree-based AST for Rust because it's more idiomatic and gives us better tools
> for the formatter use case. What matters is CommonMark compliance and output parity, not
> matching the parser implementation.

---

## Architecture

```
                ┌──────────┐
  input.md ───▶│  Parser   │──▶ AST (comrak Arena<AstNode>)
                └──────────┘
                      │
                      ▼
                ┌──────────┐
                │ Renderer │──▶ Normalized Markdown string
                └──────────┘
                      │
                      ▼
                ┌──────────┐
                │   CLI    │──▶ stdout / in-place file write
                └──────────┘
```

### Module Layout

```
mdformat/
├── Cargo.toml
├── PLAN.md
└── src/
    ├── main.rs          # CLI entry point (clap)
    ├── lib.rs           # Public API: format_str(), format_file()
    ├── parser.rs        # Thin wrapper around comrak parsing + options
    ├── renderer/
    │   ├── mod.rs       # Renderer orchestration: walk AST, dispatch to renderers
    │   ├── blocks.rs    # Block-level renderers (headings, lists, code blocks, etc.)
    │   ├── inlines.rs   # Inline renderers (emphasis, links, code spans, etc.)
    │   └── escape.rs    # Markdown-aware character escaping
    └── config.rs        # Configuration / options
```

### How the Renderer Works

1. **`format_str(input) → String`** parses `input` with comrak into an arena-allocated
   AST, then calls the renderer.
2. The renderer performs a **depth-first traversal** of the AST using comrak's
   `traverse()` iterator, which yields `NodeEdge::Start(node)` and
   `NodeEdge::End(node)` events.
3. On each node, we match on `NodeValue` and delegate to the appropriate block or
   inline rendering function.
4. Renderers write into a `String` buffer, managing indentation and inter-block spacing
   via a small `RenderContext` struct that tracks:
   - Current indentation prefix (for nested lists, blockquotes)
   - Whether we need a blank line before the next block
   - Tight vs. loose list state
5. The final string is returned (guaranteed to end with a single `\n`).

### Idempotency Guarantee

`format_str(format_str(input)) == format_str(input)` — this is enforced in tests by
running every test case through the formatter twice.

---

## Formatting Rules (v0.1)

### Block-Level

| Rule | Behavior | mdformat parity |
|---|---|---|
| **ATX headings** | Always use `#` style, never setext (`===`/`---`) | ✅ Full |
| **Heading spacing** | Single space after `#`: `## Heading` | ✅ Full |
| **Fenced code blocks** | Always use backtick fences (`` ``` ``), never indented blocks | ✅ Full |
| **Code fence info** | Preserve language info string, lowercase it | ✅ Full |
| **Thematic breaks** | Normalize to `___` (three underscores) | ✅ Full |
| **Block quotes** | Normalize `>` markers, one space after `>` | ✅ Full |
| **Unordered lists** | Use `-` as bullet marker | ✅ Full |
| **Ordered lists** | Start from original start number, sequential numbering | ✅ Full |
| **List indentation** | Content aligned after marker (2 for `-·`, variable for `N.·`) | ✅ Full |
| **Tight vs loose** | Preserve tight/loose semantics (blank lines between items) | ✅ Full |
| **Paragraphs** | No wrapping by default (preserve line breaks within) | ✅ Full |
| **HTML blocks** | Pass through unchanged | ✅ Full |

### Inline-Level

| Rule | Behavior | mdformat parity |
|---|---|---|
| **Emphasis** | Always use `*`, never `_` | ✅ Full |
| **Strong emphasis** | Always use `**`, never `__` | ✅ Full |
| **Inline code** | Normalize backtick count, trim internal spaces | ✅ Full |
| **Hard breaks** | Use backslash `\` at end of line (not trailing spaces) | ✅ Full |
| **Links** | Inline style `[text](url "title")` | ✅ Full |
| **Images** | `![alt](src "title")` | ✅ Full |
| **Autolinks** | Preserve `<url>` form where applicable | ⚠️ Partial |
| **Inline HTML** | Pass through unchanged | ✅ Full |
| **Escaping** | Escape markdown-significant chars in text nodes | ✅ Full |

### Document-Level

| Rule | Behavior | mdformat parity |
|---|---|---|
| **Trailing newline** | File ends with exactly one `\n` | ✅ Full |
| **Blank lines** | At most one blank line between blocks | ✅ Full |
| **Trailing whitespace** | Removed from all lines | ✅ Full |
| **Line endings** | Normalize to LF (configurable) | ✅ Full |

---

## Phased Roadmap

### v0.1 — Core Formatter (current)
- [x] Project scaffold + PLAN.md
- [ ] Parse input with comrak
- [ ] Implement block-level renderers (headings, paragraphs, code blocks, lists,
      blockquotes, thematic breaks, HTML blocks)
- [ ] Implement inline renderers (emphasis, strong, code, links, images, hard/soft
      breaks, text escaping, inline HTML)
- [ ] Document-level normalization (trailing newline, blank lines, whitespace)
- [ ] CLI: read from stdin/files, write to stdout or in-place
- [ ] Test suite: roundtrip tests, idempotency tests, comparison with mdformat output

### v0.2 — GFM Extensions
- [ ] Tables (pipe table formatting, column alignment)
- [ ] Strikethrough (`~~text~~`)
- [ ] Task lists (`- [x]` / `- [ ]`)
- [ ] Autolinks
- [ ] Footnotes

### v0.3 — Configuration
- [ ] `.mdformat.toml` config file support
- [ ] `--wrap <N|no|keep>` line wrapping
- [ ] `--number <consecutive|preserve>` ordered list numbering
- [ ] `--end-of-line <lf|crlf|keep>` line ending style
- [ ] `--check` mode (exit 1 if file would change, for CI)

### v0.4 — Plugin System
- [ ] Trait-based plugin API for custom renderers
- [ ] Code formatter plugins (format code inside fenced blocks)
- [ ] Dynamic plugin loading (optional, via dylib or feature flags)

### v1.0 — Production Ready
- [ ] Full CommonMark spec test suite passing
- [ ] Performance benchmarks vs Python mdformat
- [ ] Comprehensive documentation
- [ ] Stable public API
- [ ] Published to crates.io

---

## Known Gaps vs Python mdformat

| Feature | Python mdformat | mdformat-rs (planned) | Notes |
|---|---|---|---|
| **Parser** | markdown-it-py (tokens) | comrak (AST) | Different parser, same output goal |
| **Plugin ecosystem** | Rich (PyPI) | Not yet | v0.4 will add trait-based plugins |
| **Code formatters** | Via plugins (black, etc.) | Not yet | v0.4 |
| **Word wrap** | `--wrap` option | Not in v0.1 | v0.3 |
| **Config file** | `.mdformat.toml` | Not in v0.1 | v0.3 |
| **Check mode** | `--check` | Not in v0.1 | v0.3 |
| **Reference links** | Converts to inline | May differ | comrak may normalize differently |
| **Setext → ATX** | Converts | Will convert | Same behavior planned |
| **Indented → fenced** | Converts code blocks | Will convert | Same behavior planned |
| **Unicode escaping** | Comprehensive | Basic in v0.1 | Will expand |
