//! Delimited text tables (CSV, TSV, and similar) for MD View's grid view.
//!
//! The grid is a view of the document's text, like the Markdown preview, but editable. This
//! crate parses the text into rows and cells while keeping each field's exact byte range, and
//! turns grid edits into minimal text replacements, so the document's own undo, save, encoding,
//! and conflict handling apply unchanged. Untouched fields keep their original bytes, quoting
//! included; only edited fields are re-quoted, and only when they need it.
//!
//! ```
//! use mdview_table::{Dialect, Table};
//!
//! let text = "name,age\nAda,36\n";
//! let table = Table::parse(text, Dialect::sniff(text, None));
//! assert_eq!(table.value(1, 0), Some("Ada"));
//! let edits = table.set_cell(text, 1, 1, "37");
//! assert_eq!(mdview_table::apply(text, &edits), "name,age\nAda,37\n");
//! ```

#![deny(missing_docs)]

mod dialect;
mod edit;
mod parse;

pub use dialect::{Dialect, is_table_path};
pub use edit::{TextEdit, apply};
pub use parse::{Cell, Row, Table};

/// Largest document the grid parses, in bytes; larger files stay in the text editor.
pub const MAX_GRID_BYTES: usize = 32 * 1024 * 1024;
