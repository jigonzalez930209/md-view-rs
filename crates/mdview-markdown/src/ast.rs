//! Spanned Markdown abstract syntax tree.
//!
//! Every block carries the byte range it came from and the source lines that range covers. The
//! renderer uses the line range for scroll synchronization and the byte range for internal links
//! and diagnostics. Inline nodes carry text, styles, and link targets.

/// A byte range in the source with its one-based line numbers.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Span {
    /// First byte offset.
    pub start: usize,
    /// Byte offset one past the end.
    pub end: usize,
    /// One-based first line.
    pub start_line: usize,
    /// One-based last line.
    pub end_line: usize,
}

impl Span {
    /// Whether a zero-based line falls inside the span.
    pub fn contains_line(&self, line: usize) -> bool {
        let line = line + 1;
        (self.start_line..=self.end_line).contains(&line)
    }

    /// The number of source lines covered, at least one.
    pub fn line_count(&self) -> usize {
        self.end_line.saturating_sub(self.start_line) + 1
    }
}

/// A parsed Markdown document.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MarkdownDocument {
    /// Top-level blocks in document order.
    pub blocks: Vec<Block>,
    /// Headings with stable anchors.
    pub headings: Vec<Heading>,
    /// Link and image destinations in document order.
    pub links: Vec<Link>,
    /// Front matter hidden from the rendered view.
    pub front_matter: Option<FrontMatter>,
}

impl MarkdownDocument {
    /// Whether the document has no blocks.
    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    /// Find a heading by its anchor.
    pub fn heading(&self, anchor: &str) -> Option<&Heading> {
        self.headings.iter().find(|heading| heading.anchor == anchor)
    }

    /// The top-level block whose source lines contain a zero-based line.
    pub fn block_at_line(&self, line: usize) -> Option<&Block> {
        self.blocks.iter().find(|block| block.span.contains_line(line))
    }
}

/// Front matter at the start of the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FrontMatter {
    /// The raw block, including its delimiters.
    pub source: String,
    /// The source span.
    pub span: Span,
}

/// One block with a stable identifier.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    /// Stable identifier, unique within the document, in parse order.
    pub id: usize,
    /// Source coverage.
    pub span: Span,
    /// Block contents.
    pub kind: BlockKind,
}

/// Block-level Markdown.
#[derive(Clone, Debug, PartialEq)]
pub enum BlockKind {
    /// A paragraph.
    Paragraph(Vec<Inline>),
    /// A heading with its level and resolved anchor.
    Heading {
        /// Heading level, 1–6.
        level: u8,
        /// Stable anchor.
        anchor: String,
        /// Inline content.
        inlines: Vec<Inline>,
    },
    /// A fenced or indented code block. `math` fences are parsed as math blocks.
    Code {
        /// The fence language, when present.
        language: Option<String>,
        /// Literal code.
        text: String,
        /// Whether this is a `math` fence.
        math: bool,
    },
    /// Raw HTML. It is shown as text or discarded; it is never executed.
    Html(String),
    /// A block quote, optionally a GFM alert.
    BlockQuote {
        /// Alert kind.
        kind: QuoteKind,
        /// Quoted blocks.
        blocks: Vec<Block>,
    },
    /// A list.
    List {
        /// Whether the list is ordered.
        ordered: bool,
        /// The first number of an ordered list.
        start: u64,
        /// Items in order.
        items: Vec<ListItem>,
        /// Whether the list is loose (items separated by blank lines). Loose items
        /// are spaced like paragraphs.
        loose: bool,
    },
    /// A GFM table.
    Table {
        /// Column alignments.
        alignments: Vec<Alignment>,
        /// Header cells, each containing inline content.
        head: Vec<Vec<Inline>>,
        /// Body rows.
        rows: Vec<Vec<Vec<Inline>>>,
    },
    /// A thematic break.
    Rule,
    /// A footnote definition.
    FootnoteDefinition {
        /// The reference label, without brackets.
        label: String,
        /// Definition blocks.
        blocks: Vec<Block>,
    },
    /// An HTML `<details>` element whose content is Markdown between its opening and
    /// closing HTML blocks, as GitHub renders it.
    Details {
        /// The raw HTML of the opening block, including `<summary>`.
        summary: String,
        /// Whether the element has the `open` attribute.
        open: bool,
        /// The Markdown blocks inside the element.
        blocks: Vec<Block>,
    },
}

/// A list item.
#[derive(Clone, Debug, PartialEq)]
pub struct ListItem {
    /// Task state for a GFM task-list item.
    pub task: Option<bool>,
    /// Item blocks.
    pub blocks: Vec<Block>,
}

/// A GFM alert kind.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum QuoteKind {
    /// Ordinary block quote.
    #[default]
    Quote,
    /// Note alert.
    Note,
    /// Tip alert.
    Tip,
    /// Important alert.
    Important,
    /// Warning alert.
    Warning,
    /// Caution alert.
    Caution,
}

impl QuoteKind {
    /// The alert title, if this is an alert.
    pub fn alert_title(self) -> Option<&'static str> {
        match self {
            Self::Quote => None,
            Self::Note => Some("Note"),
            Self::Tip => Some("Tip"),
            Self::Important => Some("Important"),
            Self::Warning => Some("Warning"),
            Self::Caution => Some("Caution"),
        }
    }
}

/// Table column alignment.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Alignment {
    /// No explicit alignment.
    #[default]
    None,
    /// Left aligned.
    Left,
    /// Centered.
    Center,
    /// Right aligned.
    Right,
}

/// A heading entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Heading {
    /// Level, 1–6.
    pub level: u8,
    /// Stable anchor, unique in the document.
    pub anchor: String,
    /// Plain heading text.
    pub text: String,
    /// Source span.
    pub span: Span,
}

/// One link or image destination.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    /// The destination as written.
    pub destination: String,
    /// Optional title.
    pub title: Option<String>,
    /// Plain text of the link label or image alt text.
    pub label: String,
    /// Source span.
    pub span: Span,
    /// Classification for rendering and validation.
    pub kind: LinkKind,
}

/// How a link destination is interpreted.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LinkKind {
    /// `#anchor` within the current document.
    Anchor,
    /// A relative or absolute local file.
    LocalFile,
    /// A `file:` URL.
    FileUrl,
    /// A remote `http(s)` or `mailto` target.
    External,
    /// A smaller-than-remote scheme such as `data:`.
    Data,
    /// A destination the parser could not classify.
    Invalid,
}

impl LinkKind {
    /// Whether the link stays inside the workspace.
    pub fn is_internal(self) -> bool {
        matches!(self, Self::Anchor | Self::LocalFile | Self::FileUrl)
    }
}

/// Inline Markdown.
#[derive(Clone, Debug, PartialEq)]
pub enum Inline {
    /// Plain text.
    Text(String),
    /// Inline code.
    Code(String),
    /// Emphasis.
    Emphasis(Vec<Inline>),
    /// Strong emphasis.
    Strong(Vec<Inline>),
    /// GFM strikethrough.
    Strikethrough(Vec<Inline>),
    /// A link.
    Link {
        /// Destination as written.
        destination: String,
        /// Optional title.
        title: Option<String>,
        /// Child inlines.
        children: Vec<Inline>,
        /// Classification.
        kind: LinkKind,
    },
    /// An image.
    Image {
        /// Image source.
        source: String,
        /// Optional title.
        title: Option<String>,
        /// Alternative text.
        alt: String,
        /// Classification.
        kind: LinkKind,
        /// Display width in CSS pixels from an HTML `width` attribute.
        width: Option<u32>,
        /// Display height in CSS pixels from an HTML `height` attribute.
        height: Option<u32>,
    },
    /// Inline or display math.
    Math {
        /// TeX source.
        tex: String,
        /// Whether this is display math.
        display: bool,
    },
    /// A footnote reference.
    FootnoteReference(String),
    /// A soft line break that may reflow.
    SoftBreak,
    /// A hard line break.
    HardBreak,
    /// Inline HTML, kept verbatim and never executed.
    Html(String),
}

impl Inline {
    /// Append the plain text of this node and its children.
    pub fn plain_text(&self, out: &mut String) {
        match self {
            Self::Text(text) | Self::Code(text) | Self::Html(text) => out.push_str(text),
            Self::Emphasis(children) | Self::Strong(children) | Self::Strikethrough(children) => {
                for child in children {
                    child.plain_text(out);
                }
            }
            Self::Link { children, .. } => {
                for child in children {
                    child.plain_text(out);
                }
            }
            Self::Image { alt, .. } => out.push_str(alt),
            Self::Math { tex, .. } => out.push_str(tex),
            Self::FootnoteReference(label) => {
                out.push('[');
                out.push('^');
                out.push_str(label);
                out.push(']');
            }
            Self::SoftBreak | Self::HardBreak => out.push(' '),
        }
    }
}

/// Plain text of a range of inline nodes.
pub fn plain_text(inlines: &[Inline]) -> String {
    let mut text = String::new();
    for inline in inlines {
        inline.plain_text(&mut text);
    }
    text
}
