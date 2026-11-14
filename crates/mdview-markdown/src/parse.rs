//! `pulldown-cmark` adapter that builds the spanned AST.

use std::ops::Range;

use pulldown_cmark::{CodeBlockKind, Event, Parser, Tag, TagEnd};

use crate::ast::{Block, BlockKind, FrontMatter, Heading, Inline, Link, ListItem, MarkdownDocument, Span, plain_text};
use crate::slug::Slugger;
use lines::LineIndex;

mod convert;
mod lines;
mod links;
mod nesting;
mod options;
mod table;

pub use options::options;

use convert::{alignment, empty_to_none, heading_level, quote_kind};

pub use links::classify;

/// Parse Markdown into a spanned document.
pub fn parse(source: &str) -> MarkdownDocument {
    let events = Parser::new_ext(source, options())
        .into_offset_iter()
        .collect::<Vec<_>>();
    let mut parser = EventParser {
        events,
        cursor: 0,
        source,
        lines: LineIndex::new(source),
        slugs: Slugger::default(),
        next_id: 0,
        headings: Vec::new(),
        links: Vec::new(),
        front_matter: None,
        depth: 0,
    };
    let blocks = parser.parse_blocks(BlockEnd::Document);
    MarkdownDocument {
        blocks,
        headings: parser.headings,
        links: parser.links,
        front_matter: parser.front_matter,
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BlockEnd {
    Document,
    BlockQuote,
    Item,
    Footnote,
    Any,
}

impl BlockEnd {
    fn matches(self, end: &TagEnd) -> bool {
        match self {
            Self::Document => false,
            Self::BlockQuote => matches!(end, TagEnd::BlockQuote(_)),
            Self::Item => matches!(end, TagEnd::Item),
            Self::Footnote => matches!(end, TagEnd::FootnoteDefinition),
            Self::Any => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InlineEnd {
    Paragraph,
    Heading,
    Emphasis,
    Strong,
    Strikethrough,
    Link,
    Image,
    Cell,
    Any,
}

impl InlineEnd {
    fn matches(self, end: &TagEnd) -> bool {
        match self {
            Self::Paragraph => matches!(end, TagEnd::Paragraph),
            Self::Heading => matches!(end, TagEnd::Heading(_)),
            Self::Emphasis => matches!(end, TagEnd::Emphasis),
            Self::Strong => matches!(end, TagEnd::Strong),
            Self::Strikethrough => matches!(end, TagEnd::Strikethrough),
            Self::Link => matches!(end, TagEnd::Link),
            Self::Image => matches!(end, TagEnd::Image),
            Self::Cell => matches!(end, TagEnd::TableCell),
            Self::Any => true,
        }
    }
}

struct EventParser<'a> {
    events: Vec<(Event<'a>, Range<usize>)>,
    cursor: usize,
    source: &'a str,
    lines: LineIndex,
    slugs: Slugger,
    next_id: usize,
    headings: Vec<Heading>,
    links: Vec<Link>,
    front_matter: Option<FrontMatter>,
    depth: usize,
}

/// Maximum nested containers before the parser flattens the remainder as text.
const MAX_NESTING: usize = 64;

impl<'a> EventParser<'a> {
    fn peek(&self) -> Option<&(Event<'a>, Range<usize>)> {
        self.events.get(self.cursor)
    }

    fn span(&self, range: Range<usize>) -> Span {
        let start = range.start.min(self.source.len());
        let last = range.end.saturating_sub(1).max(start).min(self.source.len());
        Span {
            start,
            end: range.end.min(self.source.len()),
            start_line: self.lines.line(start) + 1,
            end_line: self.lines.line(last) + 1,
        }
    }

    /// The range of the event most recently consumed, used to close container spans.
    fn consumed_end(&self) -> Range<usize> {
        self.cursor
            .checked_sub(1)
            .and_then(|index| self.events.get(index))
            .map(|(_, range)| range.clone())
            .unwrap_or(0..0)
    }

    fn push_block(&mut self, blocks: &mut Vec<Block>, span: Span, kind: BlockKind) {
        let id = self.next_id;
        self.next_id += 1;
        blocks.push(Block { id, span, kind });
    }

    fn parse_blocks(&mut self, end: BlockEnd) -> Vec<Block> {
        let mut blocks = Vec::new();
        while let Some((event, range)) = self.peek().cloned() {
            if let Event::End(tag_end) = &event
                && end.matches(tag_end)
            {
                self.cursor += 1;
                break;
            }
            match event {
                Event::Start(Tag::Paragraph) => {
                    self.cursor += 1;
                    let (inlines, end_range) = self.parse_inlines(InlineEnd::Paragraph);
                    self.push_block(
                        &mut blocks,
                        self.span(range.start..end_range.end),
                        BlockKind::Paragraph(inlines),
                    );
                }
                Event::Start(Tag::Heading { level, id, .. }) => {
                    self.cursor += 1;
                    let (inlines, end_range) = self.parse_inlines(InlineEnd::Heading);
                    let text = plain_text(&inlines);
                    let requested = id
                        .as_deref()
                        .map(|id| id.trim_start_matches('#').trim())
                        .filter(|id| !id.is_empty())
                        .map(str::to_owned);
                    let anchor = match requested {
                        Some(id) => self.slugs.unique(id),
                        None => self.slugs.slug(&text),
                    };
                    let span = self.span(range.start..end_range.end);
                    let level = heading_level(level);
                    self.headings.push(Heading {
                        level,
                        anchor: anchor.clone(),
                        text,
                        span,
                    });
                    self.push_block(&mut blocks, span, BlockKind::Heading { level, anchor, inlines });
                }
                Event::Start(Tag::BlockQuote(kind)) => {
                    self.cursor += 1;
                    let inner = if self.depth >= MAX_NESTING {
                        self.collect_block_text()
                    } else {
                        self.depth += 1;
                        let inner = self.parse_blocks(BlockEnd::BlockQuote);
                        self.depth -= 1;
                        inner
                    };
                    let end_range = self.consumed_end();
                    let span = self.span(range.start..end_range.end);
                    self.push_block(
                        &mut blocks,
                        span,
                        BlockKind::BlockQuote {
                            kind: quote_kind(kind),
                            blocks: inner,
                        },
                    );
                }
                Event::Start(Tag::List(start)) => {
                    self.cursor += 1;
                    let mut items = Vec::new();
                    loop {
                        match self.peek().cloned() {
                            Some((Event::Start(Tag::Item), _)) => {
                                self.cursor += 1;
                                let task = match self.peek().cloned() {
                                    Some((Event::TaskListMarker(checked), _)) => {
                                        self.cursor += 1;
                                        Some(checked)
                                    }
                                    _ => None,
                                };
                                let item_blocks = if self.depth >= MAX_NESTING {
                                    self.collect_until_block_end(BlockEnd::Item)
                                } else {
                                    self.depth += 1;
                                    let blocks = self.parse_blocks(BlockEnd::Item);
                                    self.depth -= 1;
                                    blocks
                                };
                                items.push(ListItem {
                                    task,
                                    blocks: item_blocks,
                                });
                            }
                            Some((Event::End(TagEnd::List(_)), _)) => {
                                self.cursor += 1;
                                break;
                            }
                            Some(_) => self.cursor += 1,
                            None => break,
                        }
                    }
                    let span = self.span(range.start..self.consumed_end().end);
                    self.push_block(
                        &mut blocks,
                        span,
                        BlockKind::List {
                            ordered: start.is_some(),
                            start: start.unwrap_or(1),
                            items,
                        },
                    );
                }
                Event::Start(Tag::FootnoteDefinition(label)) => {
                    self.cursor += 1;
                    let inner = if self.depth >= MAX_NESTING {
                        self.collect_block_text()
                    } else {
                        self.depth += 1;
                        let inner = self.parse_blocks(BlockEnd::Footnote);
                        self.depth -= 1;
                        inner
                    };
                    let span = self.span(range.start..self.consumed_end().end);
                    self.push_block(
                        &mut blocks,
                        span,
                        BlockKind::FootnoteDefinition {
                            label: label.into_string(),
                            blocks: inner,
                        },
                    );
                }
                Event::Start(Tag::Table(alignments)) => {
                    self.cursor += 1;
                    let (head, rows) = self.parse_table();
                    let span = self.span(range.start..self.consumed_end().end);
                    self.push_block(
                        &mut blocks,
                        span,
                        BlockKind::Table {
                            alignments: alignments.into_iter().map(alignment).collect(),
                            head,
                            rows,
                        },
                    );
                }
                Event::Start(Tag::CodeBlock(kind)) => {
                    self.cursor += 1;
                    let text = self.collect_literal(BlockEnd::Any);
                    let span = self.span(range.start..self.consumed_end().end);
                    let language = match kind {
                        CodeBlockKind::Fenced(info) => info
                            .split_whitespace()
                            .next()
                            .filter(|word| !word.is_empty())
                            .map(str::to_owned),
                        CodeBlockKind::Indented => None,
                    };
                    let math = language.as_deref() == Some("math");
                    self.push_block(&mut blocks, span, BlockKind::Code { language, text, math });
                }
                Event::Start(Tag::HtmlBlock) => {
                    self.cursor += 1;
                    let text = self.collect_literal(BlockEnd::Any);
                    let span = self.span(range.start..self.consumed_end().end);
                    self.push_block(&mut blocks, span, BlockKind::Html(text));
                }
                Event::Start(Tag::MetadataBlock(_)) => {
                    self.cursor += 1;
                    let _ = self.collect_literal(BlockEnd::Any);
                    let span = self.span(range.start..self.consumed_end().end);
                    self.front_matter = Some(FrontMatter {
                        source: self.source[range.start..span.end].to_owned(),
                        span,
                    });
                }
                Event::Rule => {
                    self.cursor += 1;
                    let span = self.span(range);
                    self.push_block(&mut blocks, span, BlockKind::Rule);
                }
                Event::Start(_) => {
                    // An unsupported container (for example a definition list). Consume it and
                    // keep its rendered text as a plain paragraph so content is never lost.
                    self.cursor += 1;
                    let (inlines, end_range) = self.parse_inlines(InlineEnd::Any);
                    if !inlines.is_empty() {
                        self.push_block(
                            &mut blocks,
                            self.span(range.start..end_range.end),
                            BlockKind::Paragraph(inlines),
                        );
                    }
                }
                Event::Text(text) | Event::Code(text) => {
                    self.cursor += 1;
                    self.push_block(
                        &mut blocks,
                        self.span(range),
                        BlockKind::Paragraph(vec![Inline::Text(text.into_string())]),
                    );
                }
                Event::End(_) => {
                    self.cursor += 1;
                }
                _ => self.cursor += 1,
            }
        }
        blocks
    }

    fn parse_inlines(&mut self, end: InlineEnd) -> (Vec<Inline>, Range<usize>) {
        let mut inlines = Vec::new();
        let mut end_range = 0..0;
        while let Some((event, range)) = self.peek().cloned() {
            if let Event::End(tag_end) = &event
                && end.matches(tag_end)
            {
                self.cursor += 1;
                end_range = range;
                break;
            }
            match event {
                Event::Text(text) => {
                    self.cursor += 1;
                    inlines.push(Inline::Text(text.into_string()));
                }
                Event::Code(text) => {
                    self.cursor += 1;
                    inlines.push(Inline::Code(text.into_string()));
                }
                Event::InlineMath(tex) => {
                    self.cursor += 1;
                    inlines.push(Inline::Math {
                        tex: tex.into_string(),
                        display: false,
                    });
                }
                Event::DisplayMath(tex) => {
                    self.cursor += 1;
                    inlines.push(Inline::Math {
                        tex: tex.into_string(),
                        display: true,
                    });
                }
                Event::SoftBreak => {
                    self.cursor += 1;
                    inlines.push(Inline::SoftBreak);
                }
                Event::HardBreak => {
                    self.cursor += 1;
                    inlines.push(Inline::HardBreak);
                }
                Event::FootnoteReference(label) => {
                    self.cursor += 1;
                    inlines.push(Inline::FootnoteReference(label.into_string()));
                }
                Event::Html(raw) | Event::InlineHtml(raw) => {
                    self.cursor += 1;
                    inlines.push(Inline::Html(raw.into_string()));
                }
                Event::Start(Tag::Emphasis) => {
                    self.cursor += 1;
                    let children = self.nested_inlines(InlineEnd::Emphasis);
                    inlines.push(Inline::Emphasis(children));
                }
                Event::Start(Tag::Strong) => {
                    self.cursor += 1;
                    let children = self.nested_inlines(InlineEnd::Strong);
                    inlines.push(Inline::Strong(children));
                }
                Event::Start(Tag::Strikethrough) => {
                    self.cursor += 1;
                    let children = self.nested_inlines(InlineEnd::Strikethrough);
                    inlines.push(Inline::Strikethrough(children));
                }
                Event::Start(Tag::Link { dest_url, title, .. }) => {
                    self.cursor += 1;
                    let (children, inner_end) = if self.depth >= MAX_NESTING {
                        (self.collect_inline_text(), range.clone())
                    } else {
                        self.depth += 1;
                        let result = self.parse_inlines(InlineEnd::Link);
                        self.depth -= 1;
                        result
                    };
                    let span = self.span(range.start..inner_end.end);
                    let destination = dest_url.into_string();
                    let title = empty_to_none(title.into_string());
                    let kind = classify(&destination);
                    self.links.push(Link {
                        destination: destination.clone(),
                        title: title.clone(),
                        label: plain_text(&children),
                        span,
                        kind,
                    });
                    inlines.push(Inline::Link {
                        destination,
                        title,
                        children,
                        kind,
                    });
                }
                Event::Start(Tag::Image { dest_url, title, .. }) => {
                    self.cursor += 1;
                    let (children, inner_end) = if self.depth >= MAX_NESTING {
                        (self.collect_inline_text(), range.clone())
                    } else {
                        self.depth += 1;
                        let result = self.parse_inlines(InlineEnd::Image);
                        self.depth -= 1;
                        result
                    };
                    let span = self.span(range.start..inner_end.end);
                    let source_url = dest_url.into_string();
                    let title = empty_to_none(title.into_string());
                    let alt = plain_text(&children);
                    let kind = classify(&source_url);
                    self.links.push(Link {
                        destination: source_url.clone(),
                        title: title.clone(),
                        label: alt.clone(),
                        span,
                        kind,
                    });
                    inlines.push(Inline::Image {
                        source: source_url,
                        title,
                        alt,
                        kind,
                    });
                }
                Event::TaskListMarker(_checked) => {
                    // The list renderer draws the checkbox from `ListItem::task`.
                    self.cursor += 1;
                }
                Event::Start(Tag::Paragraph) => {
                    // A paragraph nested inside an inline container is malformed; flatten it.
                    self.cursor += 1;
                    let (children, _) = self.parse_inlines(InlineEnd::Paragraph);
                    inlines.extend(children);
                }
                Event::Start(_) => {
                    self.cursor += 1;
                    let (children, _) = self.parse_inlines(InlineEnd::Any);
                    inlines.extend(children);
                }
                Event::End(_) => {
                    self.cursor += 1;
                }
                _ => self.cursor += 1,
            }
        }
        (inlines, end_range)
    }
}
