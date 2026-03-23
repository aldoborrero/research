//! GFM (GitHub Flavored Markdown) parser extension plugin.
//!
//! Enables tables, strikethrough, task lists, and autolinks.
//!
//! # Usage
//!
//! ```
//! use mdformat::plugin::FormatterBuilder;
//! use mdformat::plugins::gfm::GfmPlugin;
//!
//! let output = FormatterBuilder::new()
//!     .parser_extension(GfmPlugin::new())
//!     .format_str("~~deleted~~ text");
//! assert_eq!(output, "~~deleted~~ text\n");
//! ```

use std::cell::RefCell;

use comrak::nodes::{AstNode, NodeTable, NodeValue, TableAlignment};
use comrak::Options;

use crate::plugin::{NodeHandled, ParserExtension};
use crate::renderer::escape::escape_markdown;
use crate::renderer::RenderContext;

/// Accumulated state for rendering a GFM table.
#[derive(Default)]
struct TableState {
    /// Column alignments.
    alignments: Vec<TableAlignment>,
    /// Accumulated rows. Each row is a vec of cell strings.
    rows: Vec<Vec<String>>,
    /// The current cell's content being accumulated.
    current_cell: String,
    /// Whether we are currently inside a table.
    in_table: bool,
}

/// GFM parser extension: tables, strikethrough, task lists, autolinks.
pub struct GfmPlugin {
    state: RefCell<TableState>,
}

impl GfmPlugin {
    pub fn new() -> Self {
        Self {
            state: RefCell::new(TableState::default()),
        }
    }
}

impl Default for GfmPlugin {
    fn default() -> Self {
        Self::new()
    }
}

impl ParserExtension for GfmPlugin {
    fn name(&self) -> &str {
        "gfm"
    }

    fn configure_options(&self, options: &mut Options) {
        options.extension.strikethrough = true;
        options.extension.table = true;
        options.extension.autolink = true;
        options.extension.tasklist = true;
    }

    fn render_node_enter(
        &self,
        node: &AstNode<'_>,
        ctx: &mut RenderContext<'_>,
    ) -> NodeHandled {
        let nv = node.data.borrow().value.clone();
        match nv {
            NodeValue::Table(NodeTable { ref alignments, .. }) => {
                let mut state = self.state.borrow_mut();
                state.in_table = true;
                state.alignments = alignments.clone();
                state.rows.clear();
                NodeHandled::Handled
            }
            NodeValue::TableRow(_) => {
                let mut state = self.state.borrow_mut();
                if state.in_table {
                    state.rows.push(Vec::new());
                }
                NodeHandled::Handled
            }
            NodeValue::TableCell => {
                let mut state = self.state.borrow_mut();
                if state.in_table {
                    state.current_cell.clear();
                }
                NodeHandled::Handled
            }
            NodeValue::Strikethrough => {
                let in_table = self.state.borrow().in_table;
                if in_table {
                    self.state.borrow_mut().current_cell.push_str("~~");
                } else {
                    ctx.write("~~");
                }
                NodeHandled::Handled
            }
            NodeValue::TaskItem(checked) => {
                task_item_enter(node, ctx, checked);
                NodeHandled::Handled
            }
            // Intercept inline nodes when inside a table cell
            NodeValue::Text(ref s) if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push_str(&escape_markdown(s));
                NodeHandled::Handled
            }
            NodeValue::Code(ref c) if self.state.borrow().in_table => {
                let mut state = self.state.borrow_mut();
                state.current_cell.push('`');
                state.current_cell.push_str(&c.literal);
                state.current_cell.push('`');
                NodeHandled::Handled
            }
            NodeValue::Emph if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push('*');
                NodeHandled::Handled
            }
            NodeValue::Strong if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push_str("**");
                NodeHandled::Handled
            }
            NodeValue::Link(_) if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push('[');
                NodeHandled::Handled
            }
            NodeValue::Image(_) if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push_str("![");
                NodeHandled::Handled
            }
            NodeValue::SoftBreak | NodeValue::LineBreak
                if self.state.borrow().in_table =>
            {
                self.state.borrow_mut().current_cell.push(' ');
                NodeHandled::Handled
            }
            _ => NodeHandled::Unhandled,
        }
    }

    fn render_node_leave(
        &self,
        node: &AstNode<'_>,
        ctx: &mut RenderContext<'_>,
    ) -> NodeHandled {
        let nv = node.data.borrow().value.clone();
        match nv {
            NodeValue::Table(_) => {
                let mut state = self.state.borrow_mut();
                if state.in_table {
                    render_table(ctx, &state.alignments, &state.rows);
                    state.in_table = false;
                }
                ctx.needs_blank_line = true;
                NodeHandled::Handled
            }
            NodeValue::TableRow(_) => NodeHandled::Handled,
            NodeValue::TableCell => {
                let mut state = self.state.borrow_mut();
                if state.in_table {
                    let cell = state.current_cell.clone();
                    if let Some(row) = state.rows.last_mut() {
                        row.push(cell);
                    }
                    state.current_cell.clear();
                }
                NodeHandled::Handled
            }
            NodeValue::Strikethrough => {
                let state = self.state.borrow();
                if state.in_table {
                    drop(state);
                    self.state.borrow_mut().current_cell.push_str("~~");
                } else {
                    ctx.write("~~");
                }
                NodeHandled::Handled
            }
            NodeValue::TaskItem(_) => {
                // TaskItem is like Item — pop the prefix and reset blank line
                ctx.pop_prefix();
                ctx.needs_blank_line = false;
                NodeHandled::Handled
            }
            // Close inline markers when inside a table cell
            NodeValue::Emph if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push('*');
                NodeHandled::Handled
            }
            NodeValue::Strong if self.state.borrow().in_table => {
                self.state.borrow_mut().current_cell.push_str("**");
                NodeHandled::Handled
            }
            NodeValue::Link(ref link) if self.state.borrow().in_table => {
                let mut state = self.state.borrow_mut();
                state.current_cell.push_str("](");
                state.current_cell.push_str(&link.url);
                if !link.title.is_empty() {
                    state.current_cell.push_str(" \"");
                    state.current_cell.push_str(&link.title);
                    state.current_cell.push('"');
                }
                state.current_cell.push(')');
                NodeHandled::Handled
            }
            NodeValue::Image(ref link) if self.state.borrow().in_table => {
                let mut state = self.state.borrow_mut();
                state.current_cell.push_str("](");
                state.current_cell.push_str(&link.url);
                if !link.title.is_empty() {
                    state.current_cell.push_str(" \"");
                    state.current_cell.push_str(&link.title);
                    state.current_cell.push('"');
                }
                state.current_cell.push(')');
                NodeHandled::Handled
            }
            _ => NodeHandled::Unhandled,
        }
    }
}

/// Render the accumulated table data.
fn render_table(
    ctx: &mut RenderContext<'_>,
    alignments: &[TableAlignment],
    rows: &[Vec<String>],
) {
    if ctx.needs_blank_line {
        ctx.ensure_blank_line();
    }

    let num_cols = alignments.len();

    // Calculate column widths (minimum 3 for separator dashes)
    let mut widths = vec![3usize; num_cols];
    for row in rows {
        for (i, cell) in row.iter().enumerate() {
            if i < num_cols {
                widths[i] = widths[i].max(cell.len());
            }
        }
    }

    // Render header row
    if let Some(header) = rows.first() {
        render_row(ctx, header, &widths, num_cols);
    }

    // Render separator row
    render_separator(ctx, alignments, &widths);

    // Render data rows
    for row in rows.iter().skip(1) {
        render_row(ctx, row, &widths, num_cols);
    }
}

fn render_row(
    ctx: &mut RenderContext<'_>,
    cells: &[String],
    widths: &[usize],
    num_cols: usize,
) {
    ctx.write("|");
    for i in 0..num_cols {
        let content = cells.get(i).map(|s| s.as_str()).unwrap_or("");
        ctx.write(&format!(" {:width$} |", content, width = widths[i]));
    }
    ctx.write("\n");
}

fn render_separator(
    ctx: &mut RenderContext<'_>,
    alignments: &[TableAlignment],
    widths: &[usize],
) {
    ctx.write("|");
    for (i, align) in alignments.iter().enumerate() {
        let w = widths[i];
        let sep = match align {
            TableAlignment::Left => format!(" :{} |", "-".repeat(w - 1)),
            TableAlignment::Right => format!(" {}: |", "-".repeat(w - 1)),
            TableAlignment::Center => {
                let inner = if w >= 2 { w - 2 } else { 1 };
                format!(" :{}: |", "-".repeat(inner))
            }
            TableAlignment::None => format!(" {} |", "-".repeat(w)),
        };
        ctx.write(&sep);
    }
    ctx.write("\n");
}

/// Render a task list item opening: list marker + checkbox.
fn task_item_enter(
    node: &AstNode<'_>,
    ctx: &mut RenderContext<'_>,
    checked: Option<char>,
) {
    // Reuse shared list marker logic
    crate::renderer::blocks::emit_list_marker(node, ctx);

    // Add checkbox after the marker
    match checked {
        Some('x') | Some('X') => ctx.write("[x] "),
        Some(_) => ctx.write("[x] "),
        None => ctx.write("[ ] "),
    }
}
