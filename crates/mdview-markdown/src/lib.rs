//! Markdown parsing for MD View.
//!
//! The parser wraps `pulldown-cmark` with CommonMark 0.31 + GFM and produces a spanned AST. The
//! renderer never inspects pulldown events directly: blocks, inlines, headings, links, front
//! matter, alerts, task lists, footnotes, tables, and math all arrive through this crate.
//!
//! ```
//! let document = mdview_markdown::parse("# Hello\n\nText with a [link](other.md).");
//! assert_eq!(document.headings[0].anchor, "hello");
//! assert_eq!(document.links[0].destination, "other.md");
//! ```

#![deny(missing_docs)]

mod ast;
mod math;
mod parse;
mod slug;

pub use ast::{
    Alignment, Block, BlockKind, FrontMatter, Heading, Inline, Link, LinkKind, ListItem, MarkdownDocument, QuoteKind,
    Span, plain_text,
};
pub use math::{MathDelimiter, MathSource, format_math, math_source_at, math_sources};
pub use parse::{classify, options, parse as parse_with_options};
pub use slug::{Slugger, github_slug};

/// Parse Markdown into a spanned document.
pub fn parse(source: &str) -> MarkdownDocument {
    parse::parse(source)
}

/// Whether a path should open in the Markdown view.
pub fn is_markdown_path(path: &std::path::Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("md") || extension.eq_ignore_ascii_case("markdown"))
}
