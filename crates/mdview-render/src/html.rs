//! The HTML subset GitHub keeps after sanitizing, converted to Markdown inlines.
//!
//! GitHub renders raw HTML in Markdown through an allow-list: comments and scripts
//! disappear, `align` centers paragraphs and headings, `<img>` honors `width` and
//! `height`, and `<details>` collapses its content. This module tokenizes that subset
//! and turns it into lines of [`Inline`] nodes so the normal paragraph, heading, and
//! image painters draw it. Nothing is executed and unknown tags are ignored.

use mdview_markdown::{Inline, classify};

mod tokens;

use tokens::{Tag, Token, tokenize};

/// What a converted line is.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum LineKind {
    /// A paragraph-like run of inline content.
    Paragraph,
    /// A heading of the given level.
    Heading(u8),
    /// A list item; drawn with a bullet.
    Item,
    /// A `<summary>` that toggles the rest of its `<details>`.
    Summary,
    /// A horizontal rule.
    Rule,
}

/// One block-level line of HTML content.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct HtmlLine {
    pub kind: LineKind,
    pub align: egui::Align,
    /// Nesting depth of `<details>` that contains the line (0 = top level).
    pub details: usize,
    pub inlines: Vec<Inline>,
}

#[derive(Clone, Debug, Default)]
struct Style {
    strong: bool,
    emphasis: bool,
    strike: bool,
    code: bool,
    link: Option<String>,
}

fn alignment(tag: &Tag<'_>) -> Option<egui::Align> {
    match tag.attribute("align")?.to_ascii_lowercase().as_str() {
        "center" | "middle" => Some(egui::Align::Center),
        "right" => Some(egui::Align::Max),
        "left" => Some(egui::Align::Min),
        _ => None,
    }
}

fn styled(text: String, style: &Style) -> Inline {
    let mut inline = if style.code {
        Inline::Code(text)
    } else {
        Inline::Text(text)
    };
    if style.strike {
        inline = Inline::Strikethrough(vec![inline]);
    }
    if style.emphasis {
        inline = Inline::Emphasis(vec![inline]);
    }
    if style.strong {
        inline = Inline::Strong(vec![inline]);
    }
    if let Some(destination) = &style.link {
        inline = Inline::Link {
            destination: destination.clone(),
            title: None,
            children: vec![inline],
            kind: classify(destination),
        };
    }
    inline
}

struct Builder {
    lines: Vec<HtmlLine>,
    current: HtmlLine,
    aligns: Vec<(String, egui::Align)>,
    details: usize,
}

impl Builder {
    fn align(&self) -> egui::Align {
        self.aligns.last().map_or(egui::Align::Min, |(_, align)| *align)
    }

    fn flush(&mut self) {
        let kind = self.current.kind;
        let empty = self
            .current
            .inlines
            .iter()
            .all(|inline| matches!(inline, Inline::Text(text) if text.trim().is_empty()));
        if !empty || kind == LineKind::Rule {
            let next = HtmlLine {
                kind: LineKind::Paragraph,
                align: self.align(),
                details: self.details,
                inlines: Vec::new(),
            };
            let mut line = std::mem::replace(&mut self.current, next);
            trim_edges(&mut line.inlines);
            self.lines.push(line);
        } else {
            self.current.kind = LineKind::Paragraph;
            self.current.inlines.clear();
        }
        self.current.align = self.align();
        self.current.details = self.details;
    }

    fn start(&mut self, kind: LineKind) {
        self.flush();
        self.current.kind = kind;
    }
}

fn trim_edges(inlines: &mut Vec<Inline>) {
    if let Some(Inline::Text(text)) = inlines.first_mut() {
        *text = text.trim_start().to_owned();
    }
    if let Some(Inline::Text(text)) = inlines.last_mut() {
        *text = text.trim_end().to_owned();
    }
    inlines.retain(|inline| !matches!(inline, Inline::Text(text) if text.is_empty()));
}

/// Convert an HTML block to lines of inline content.
pub(crate) fn to_lines(html: &str) -> Vec<HtmlLine> {
    let mut builder = Builder {
        lines: Vec::new(),
        current: HtmlLine {
            kind: LineKind::Paragraph,
            align: egui::Align::Min,
            details: 0,
            inlines: Vec::new(),
        },
        aligns: Vec::new(),
        details: 0,
    };
    let mut style = Style::default();
    let mut suppress = 0usize;
    for token in tokenize(html) {
        match token {
            Token::Text(text) => {
                if suppress == 0 {
                    // HTML collapses whitespace runs to one space.
                    let collapsed = text.split_whitespace().collect::<Vec<_>>().join(" ");
                    let lead = text.starts_with(char::is_whitespace) && !collapsed.is_empty();
                    let trail = text.ends_with(char::is_whitespace);
                    let mut value = String::new();
                    if lead {
                        value.push(' ');
                    }
                    value.push_str(&collapsed);
                    if trail && !collapsed.is_empty() {
                        value.push(' ');
                    }
                    if collapsed.is_empty() && !text.is_empty() {
                        value.push(' ');
                    }
                    builder.current.inlines.push(styled(value, &style));
                }
            }
            Token::Tag(tag) => {
                let name = tag.name.as_str();
                match (name, tag.closing) {
                    ("script" | "style" | "iframe" | "object" | "embed" | "noscript", closing) => {
                        suppress = if closing {
                            suppress.saturating_sub(1)
                        } else {
                            suppress + 1
                        };
                    }
                    ("b" | "strong", closing) => style.strong = !closing,
                    ("i" | "em", closing) => style.emphasis = !closing,
                    ("s" | "del" | "strike", closing) => style.strike = !closing,
                    ("code" | "kbd" | "samp" | "tt", closing) => style.code = !closing,
                    ("a", false) => style.link = tag.attribute("href").map(str::to_owned),
                    ("a", true) => style.link = None,
                    ("br", _) => builder.current.inlines.push(Inline::HardBreak),
                    ("img", false) if suppress == 0 => {
                        if let Some(source) = tag.attribute("src") {
                            let image = Inline::Image {
                                source: source.to_owned(),
                                title: tag.attribute("title").map(str::to_owned),
                                alt: tag.attribute("alt").unwrap_or_default().to_owned(),
                                kind: classify(source),
                                width: tag.attribute("width").and_then(|value| value.trim().parse().ok()),
                                height: tag.attribute("height").and_then(|value| value.trim().parse().ok()),
                            };
                            let image = match &style.link {
                                Some(destination) => Inline::Link {
                                    destination: destination.clone(),
                                    title: None,
                                    children: vec![image],
                                    kind: classify(destination),
                                },
                                None => image,
                            };
                            builder.current.inlines.push(image);
                        }
                    }
                    ("hr", _) => {
                        builder.start(LineKind::Rule);
                        builder.flush();
                    }
                    ("h1" | "h2" | "h3" | "h4" | "h5" | "h6", false) => {
                        let level = name[1..].parse().unwrap_or(1);
                        if let Some(align) = alignment(&tag) {
                            builder.aligns.push((tag.name.clone(), align));
                        }
                        builder.start(LineKind::Heading(level));
                    }
                    ("p" | "div" | "center" | "section" | "picture" | "figure" | "tr", false) => {
                        let align = if name == "center" {
                            Some(egui::Align::Center)
                        } else {
                            alignment(&tag)
                        };
                        if let Some(align) = align {
                            builder.aligns.push((tag.name.clone(), align));
                        }
                        builder.start(LineKind::Paragraph);
                    }
                    ("li", false) => builder.start(LineKind::Item),
                    ("details", false) => {
                        builder.flush();
                        builder.details += 1;
                        builder.flush();
                    }
                    ("details", true) => {
                        builder.flush();
                        builder.details = builder.details.saturating_sub(1);
                        builder.flush();
                    }
                    ("summary", false) => builder.start(LineKind::Summary),
                    ("td" | "th", false) => builder.current.inlines.push(Inline::Text(" ".to_owned())),
                    (
                        "p" | "div" | "center" | "section" | "picture" | "figure" | "tr" | "li" | "summary" | "h1"
                        | "h2" | "h3" | "h4" | "h5" | "h6" | "ul" | "ol" | "table",
                        true,
                    ) => {
                        builder.flush();
                        if builder.aligns.last().is_some_and(|(open, _)| open == name) {
                            builder.aligns.pop();
                            builder.current.align = builder.align();
                        }
                    }
                    _ => {}
                }
            }
        }
    }
    builder.flush();
    builder.lines
}

/// Whether the last element of an HTML block is a margin-less container (`<div>`,
/// `<center>`, …) rather than a `<p>`, heading, list, or rule.
pub(crate) fn ends_without_margin(html: &str) -> bool {
    let mut last = None;
    for token in tokenize(html) {
        if let Token::Tag(tag) = token
            && tag.closing
        {
            last = Some(tag.name);
        }
    }
    matches!(
        last.as_deref(),
        Some("div" | "center" | "section" | "picture" | "figure" | "a" | "span")
    )
}

/// The image an inline `<img>` tag describes, if `raw` is one.
pub(crate) fn image_tag(raw: &str) -> Option<Inline> {
    if !raw.trim_start().to_ascii_lowercase().starts_with("<img") {
        return None;
    }
    to_lines(raw)
        .into_iter()
        .flat_map(|line| line.inlines)
        .find(|inline| matches!(inline, Inline::Image { .. }))
}

#[cfg(test)]
mod tests {
    use super::tokens::decode_entities;
    use super::*;

    #[test]
    fn comments_scripts_and_unknown_tags_disappear() {
        let lines = to_lines("<!-- hidden -->\n<script>alert(1)</script><span>Shown</span>");
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].inlines, vec![Inline::Text("Shown".to_owned())]);
    }

    #[test]
    fn centered_paragraph_with_sized_linked_image() {
        let lines = to_lines(
            r#"<p align="center"><a href="https://example.com"><img src="logo.png" width="120" alt="Logo"></a><br>Caption &amp; more</p>"#,
        );
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].align, egui::Align::Center);
        let Inline::Link { children, .. } = &lines[0].inlines[0] else {
            panic!("expected a link");
        };
        assert!(matches!(
            &children[0],
            Inline::Image {
                width: Some(120),
                height: None,
                ..
            }
        ));
        assert_eq!(lines[0].inlines[1], Inline::HardBreak);
        assert_eq!(lines[0].inlines[2], Inline::Text("Caption & more".to_owned()));
    }

    #[test]
    fn headings_details_and_items() {
        let lines = to_lines(
            "<h1 align=\"center\">Title</h1>\n<details><summary>More</summary>\n<ul><li><b>One</b></li></ul></details>",
        );
        let kinds: Vec<_> = lines.iter().map(|line| (line.kind, line.details)).collect();
        assert_eq!(
            kinds,
            vec![(LineKind::Heading(1), 0), (LineKind::Summary, 1), (LineKind::Item, 1)]
        );
        assert_eq!(lines[0].align, egui::Align::Center);
    }

    #[test]
    fn only_paragraph_like_endings_keep_their_margin() {
        assert!(ends_without_margin("<div align=\"right\">Text</div>"));
        assert!(!ends_without_margin("<p align=\"center\">Text</p>"));
        assert!(!ends_without_margin("<h3>Title</h3>"));
    }

    #[test]
    fn entities_decode() {
        assert_eq!(
            decode_entities("a &lt;b&gt; &#169; &#x263A; &bogus; &"),
            "a <b> © ☺ &bogus; &"
        );
    }
}
