//! Inline event collection.

use super::*;

impl<'a> EventParser<'a> {
    pub(crate) fn parse_inlines(&mut self, end: InlineEnd) -> (Vec<Inline>, Range<usize>) {
        let mut inlines = Vec::new();
        let mut end_range = 0..0;
        while let Some((event, range)) = self.peek().cloned() {
            if let Event::End(tag_end) = &event
                && end.matches(tag_end)
            {
                // Leave End(Item) for parse_blocks, which owns the item boundary.
                if end != InlineEnd::Item {
                    self.cursor += 1;
                }
                end_range = range;
                break;
            }
            if matches!(end, InlineEnd::Item)
                && matches!(
                    event,
                    Event::Start(
                        Tag::Paragraph
                            | Tag::Heading { .. }
                            | Tag::List(_)
                            | Tag::Item
                            | Tag::BlockQuote(_)
                            | Tag::CodeBlock(_)
                            | Tag::HtmlBlock
                            | Tag::Table(_)
                            | Tag::FootnoteDefinition(_)
                    ) | Event::Rule
                )
            {
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
                        width: None,
                        height: None,
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
