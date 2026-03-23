# mdformat-rs: Rust Port of mdformat

A CommonMark-compliant Markdown formatter written in Rust, ported from
[hukkin/mdformat](https://github.com/hukkin/mdformat).

## Overview

mdformat is a Python tool that parses Markdown into an AST (via markdown-it-py),
then renders it back into consistently-formatted Markdown. The Rust port will
replicate this architecture using the `markdown-it` Rust crate (a port of
markdown-it) as the parser foundation.

---

## Phase 1: Project Scaffolding & Parser Foundation

**Goal:** Set up workspace, integrate markdown-it parser, build the token tree.

### Crates

```
mdformat-rs/
├── Cargo.toml              (workspace)
├── crates/
│   ├── mdformat-core/      # Core formatting library
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── tree.rs     # RenderTreeNode (AST wrapper)
│   │   │   ├── context.rs  # RenderContext
│   │   │   ├── renderer.rs # MDRenderer
│   │   │   ├── options.rs  # Formatting options
│   │   │   └── util.rs     # Escaping, codepoint helpers
│   │   └── Cargo.toml
│   └── mdformat-cli/       # CLI binary
│       ├── src/
│       │   └── main.rs
│       └── Cargo.toml
├── tests/                   # Integration tests (CommonMark fixtures)
├── PLAN.md
└── README.md
```

### Tasks

1. **Initialize Cargo workspace** with `mdformat-core` and `mdformat-cli` crates.
2. **Evaluate Rust markdown-it crates:**
   - [`markdown-it`](https://crates.io/crates/markdown-it) — Rust port of
     markdown-it, actively maintained. **Primary candidate.**
   - [`pulldown-cmark`](https://crates.io/crates/pulldown-cmark) — Event-based
     CommonMark parser. Faster but different model (no AST/token tree).
   - Decision: Use `markdown-it` crate for closest architectural alignment with
     the Python version. It produces a token tree we can walk, matching
     mdformat's `RenderTreeNode` approach.
3. **Define `Options` struct** mirroring mdformat's configuration:
   ```rust
   pub struct Options {
       pub number: bool,           // consecutive ordered list numbering
       pub wrap: Wrap,             // keep | no | Column(u32)
       pub end_of_line: EndOfLine, // lf | crlf | keep
       pub validate: bool,         // verify HTML equivalence
   }
   ```

### Key Dependencies

| Crate | Purpose |
|-------|---------|
| `markdown-it` | CommonMark parser (token tree) |
| `clap` | CLI argument parsing |
| `toml` | Config file (`.mdformat.toml`) parsing |
| `glob` | File pattern matching |
| `similar` | Diff output for `--check` mode |

---

## Phase 2: Render Tree & Context

**Goal:** Build the render tree walker and context system.

### Tasks

1. **`RenderTreeNode`** — Wrapper around markdown-it's AST nodes providing:
   - `.type` (token type string)
   - `.children` iteration
   - `.content` / `.markup` / `.info` accessors
   - `.render(context)` → produces formatted markdown string
2. **`RenderContext`** — Carries state through rendering:
   - `renderers: HashMap<String, RenderFn>` — per-token-type render functions
   - `postprocessors: HashMap<String, Vec<PostprocessFn>>` — chained post-processing
   - `options: Options`
   - `env: RenderEnv` — mutable state (indent stack, reference tracking)
   - `with_indent(width) -> Guard` — RAII-based indentation management
3. **Sentinel system** for word wrapping:
   - `WRAP_POINT` character to mark potential break points
   - `PRESERVE_CHAR` to protect intentional whitespace
   - Post-render wrap pass using `textwrap` crate or custom implementation

---

## Phase 3: Token Renderers

**Goal:** Implement rendering functions for all CommonMark token types.

Each renderer is a function: `fn(node: &RenderTreeNode, ctx: &mut RenderContext) -> String`

### Token Types to Implement

| Category | Tokens | Notes |
|----------|--------|-------|
| **Inline** | `text`, `softbreak`, `hardbreak`, `code_inline` | Escape sequences in code spans |
| **Emphasis** | `em_open/close`, `strong_open/close`, `s_open/close` | Preserve original `*` vs `_` |
| **Links** | `link_open/close`, `autolink`, `image` | Reference vs inline, autolink detection |
| **Blocks** | `paragraph_open/close`, `heading_open/close` | ATX headings, paragraph escaping |
| **Code** | `fence`, `code_block` | Fence char selection, info string |
| **Lists** | `bullet_list`, `ordered_list`, `list_item` | Marker style, indentation, numbering |
| **Quotes** | `blockquote_open/close` | Nested `>` prefixing |
| **Other** | `hr`, `html_block`, `html_inline`, `table` | Thematic breaks, raw HTML passthrough |
| **References** | `definition`, `footnote` | Reference link collection & output |

### Paragraph Safety (Critical)

Port mdformat's paragraph escaping logic that prevents accidental markdown
interpretation of leading characters (`#`, `>`, `-`, `*`, `1.`, etc.).

---

## Phase 4: CLI

**Goal:** Feature-complete command-line interface.

### Commands & Options

```
mdformat-rs [OPTIONS] [PATHS...]

Options:
  --check              Report if files need formatting (exit 1 if so)
  --no-validate        Skip HTML equivalence validation
  --number             Consecutive numbering for ordered lists
  --wrap <MODE>        Word wrap: keep | no | <INTEGER>
  --end-of-line <EOL>  Line endings: lf | crlf | keep
  --exclude <PATTERN>  Glob patterns to exclude
  --extensions <EXT>   Parser extensions to enable
  --codeformatters <L> Code formatters to enable
  -                    Read from stdin, write to stdout
```

### Tasks

1. **Argument parsing** with `clap` (derive API).
2. **Path expansion** — recursively find `*.md` files in directories.
3. **Config loading** — parse `.mdformat.toml` per directory, merge with CLI args.
4. **Formatting pipeline:**
   - Read file → `mdformat_core::format(text, options)` → compare → write if changed.
5. **Validation** — optionally compare HTML output before/after formatting.
6. **Exit codes** — 0 success, 1 formatting needed or error.
7. **Stdin/stdout mode** — pipe-friendly formatting.

---

## Phase 5: Plugin System

**Goal:** Extensible architecture for parser extensions and code formatters.

### Design

Unlike Python's entry-point discovery, Rust plugins will use a **trait-based**
approach with compile-time registration:

```rust
/// Parser extension plugin trait
pub trait ParserExtension: Send + Sync {
    fn name(&self) -> &str;
    fn update_parser(&self, md: &mut MarkdownIt);
    fn renderers(&self) -> HashMap<String, RenderFn>;
    fn postprocessors(&self) -> HashMap<String, Vec<PostprocessFn>> {
        HashMap::new()
    }
    fn changes_ast(&self) -> bool { false }
}

/// Code formatter plugin trait
pub trait CodeFormatter: Send + Sync {
    fn lang(&self) -> &str;
    fn format(&self, code: &str, info: &str) -> String;
}
```

### Plugin Loading Options

- **Static (v1):** Compile-time feature flags for built-in plugins (e.g.,
  `--features gfm` enables GFM tables/strikethrough).
- **Dynamic (v2, future):** `libloading`-based shared library plugins for
  runtime extensibility.

### Built-in Extensions (Phase 5b)

- **GFM** — tables, strikethrough, autolinks, task lists
  (via `markdown-it` GFM extensions)
- **TOML frontmatter** — preserve YAML/TOML frontmatter blocks

---

## Phase 6: Validation & Testing

**Goal:** Ensure correctness and CommonMark compliance.

### Tasks

1. **CommonMark spec tests** — Parse the CommonMark spec examples, format them,
   verify HTML output is preserved.
2. **Idempotency tests** — `format(format(input)) == format(input)` for all
   test cases.
3. **Round-trip fuzz testing** — Use `cargo-fuzz` with arbitrary markdown inputs.
4. **Comparison tests** — Run both Python mdformat and mdformat-rs on the same
   inputs, compare outputs.
5. **Benchmark suite** — `criterion` benchmarks against Python mdformat and
   other Rust formatters.

---

## Phase 7: Polish & Distribution

### Tasks

1. **Error messages** — Clear, actionable error output with file paths and line numbers.
2. **Parallel formatting** — Use `rayon` for concurrent file processing (improvement over Python's sequential approach).
3. **Pre-commit hook** — Provide `.pre-commit-hooks.yaml` for integration.
4. **Cross-compilation** — CI builds for Linux, macOS, Windows (x86_64 + aarch64).
5. **Packaging** — `cargo install`, GitHub releases with binaries, Homebrew formula, Nix flake.

---

## Architecture Decisions

| Decision | Choice | Rationale |
|----------|--------|-----------|
| Parser | `markdown-it` crate | Closest to Python mdformat's markdown-it-py; produces walkable AST |
| CLI | `clap` derive | Industry standard, excellent help generation |
| Config | `.mdformat.toml` | Compatible with Python mdformat configs |
| Plugin model | Trait-based + feature flags | Idiomatic Rust, zero-cost abstractions |
| Parallelism | `rayon` | Easy parallel file iteration, big perf win |
| Testing | CommonMark spec + fuzz | Ensures correctness parity |

## Non-Goals (Initial Release)

- 100% output-identical with Python mdformat (minor whitespace differences acceptable if both are valid CommonMark)
- Dynamic plugin loading (defer to v2)
- Language server protocol (future consideration)
