//! Inline Markdown flattening and painting.

use egui::{Label, RichText, Sense, Ui};
use mdview_markdown::{Inline, LinkKind, plain_text};
use mdview_theme::Theme;

use crate::{LinkActivation, PreviewOutput};

/// One styled run of inline text.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Segment {
    pub text: String,
    pub strong: bool,
    pub emphasis: bool,
    pub strike: bool,
    pub monospace: bool,
    pub mark: bool,
    pub subscript: bool,
    pub superscript: bool,
    pub keyboard: bool,
    pub link: Option<LinkTarget>,
    pub image: bool,
    pub math: bool,
    pub footnote: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct LinkTarget {
    pub destination: String,
    pub kind: LinkKind,
}

#[derive(Clone, Debug, Default)]
struct HtmlState {
    strong: bool,
    emphasis: bool,
    strike: bool,
    mark: bool,
    subscript: bool,
    superscript: bool,
    keyboard: bool,
    suppress: bool,
}

/// Flatten inline nodes into styled segments, applying the safe HTML subset.
pub(crate) fn segments(inlines: &[Inline]) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut html = HtmlState::default();
    flatten(inlines, &Segment::default(), &mut html, &mut out);
    out
}

fn flatten(inlines: &[Inline], style: &Segment, html: &mut HtmlState, out: &mut Vec<Segment>) {
    for inline in inlines {
        match inline {
            Inline::Text(text) => push_text(text, style, html, out),
            Inline::Code(code) => {
                let mut segment = style.clone();
                segment.monospace = true;
                segment.text.clone_from(code);
                push_segment(segment, html, out);
            }
            Inline::Emphasis(children) => {
                let mut nested = style.clone();
                nested.emphasis = true;
                flatten(children, &nested, html, out);
            }
            Inline::Strong(children) => {
                let mut nested = style.clone();
                nested.strong = true;
                flatten(children, &nested, html, out);
            }
            Inline::Strikethrough(children) => {
                let mut nested = style.clone();
                nested.strike = true;
                flatten(children, &nested, html, out);
            }
            Inline::Link {
                destination,
                children,
                kind,
                ..
            } => {
                let mut nested = style.clone();
                nested.link = Some(LinkTarget {
                    destination: destination.clone(),
                    kind: *kind,
                });
                flatten(children, &nested, html, out);
            }
            Inline::Image { source, alt, kind, .. } => {
                let mut segment = style.clone();
                segment.image = true;
                segment.text = if alt.is_empty() { source.clone() } else { alt.clone() };
                segment.link = Some(LinkTarget {
                    destination: source.clone(),
                    kind: *kind,
                });
                push_segment(segment, html, out);
            }
            Inline::Math { tex, display } => {
                let mut segment = style.clone();
                segment.math = true;
                segment.text = if *display { format!("Math: {tex}") } else { tex.clone() };
                push_segment(segment, html, out);
            }
            Inline::FootnoteReference(label) => {
                let mut segment = style.clone();
                segment.superscript = true;
                segment.text = format!("[^{label}]");
                segment.footnote = Some(label.clone());
                push_segment(segment, html, out);
            }
            Inline::SoftBreak => push_text(" ", style, html, out),
            Inline::HardBreak => push_text("\n", style, html, out),
            Inline::Html(raw) => apply_html(raw, html, style, out),
        }
    }
}

fn push_text(text: &str, style: &Segment, html: &HtmlState, out: &mut Vec<Segment>) {
    if html.suppress || text.is_empty() {
        return;
    }
    let mut segment = style.clone();
    segment.strong |= html.strong;
    segment.emphasis |= html.emphasis;
    segment.strike |= html.strike;
    segment.mark |= html.mark;
    segment.subscript |= html.subscript;
    segment.superscript |= html.superscript;
    segment.keyboard |= html.keyboard;
    segment.text = text.to_owned();
    out.push(segment);
}

fn push_segment(segment: Segment, html: &HtmlState, out: &mut Vec<Segment>) {
    if !html.suppress && !segment.text.is_empty() {
        out.push(segment);
    }
}

fn apply_html(raw: &str, html: &mut HtmlState, style: &Segment, out: &mut Vec<Segment>) {
    let tag = raw.trim().trim_start_matches('<').trim_end_matches('>').trim();
    let closing = tag.starts_with('/');
    let name = tag
        .trim_start_matches('/')
        .split(|character: char| character.is_whitespace() || character == '/')
        .next()
        .unwrap_or("")
        .to_ascii_lowercase();
    match name.as_str() {
        "br" => push_text("\n", style, html, out),
        "mark" => html.mark = !closing,
        "sub" => html.subscript = !closing,
        "sup" => html.superscript = !closing,
        "kbd" => html.keyboard = !closing,
        "strong" | "b" => html.strong = !closing,
        "em" | "i" => html.emphasis = !closing,
        "del" | "s" | "strike" => html.strike = !closing,
        "script" | "style" | "iframe" | "object" | "embed" => html.suppress = !closing,
        _ => {}
    }
}

/// Paint one inline sequence in the current layout.
pub(crate) fn show_inlines(ui: &mut Ui, inlines: &[Inline], theme: &Theme, output: &mut PreviewOutput) {
    let segments = segments(inlines);
    let mut first = true;
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for segment in &segments {
            if !first && !segment.monospace {
                ui.add_space(0.0);
            }
            first = false;
            show_segment(ui, segment, theme, output);
        }
    });
}

fn show_segment(ui: &mut Ui, segment: &Segment, theme: &Theme, output: &mut PreviewOutput) {
    let mut text = RichText::new(&segment.text).color(theme.text);
    if segment.strong {
        text = text.strong();
    }
    if segment.emphasis {
        text = text.italics();
    }
    if segment.strike {
        text = text.strikethrough();
    }
    if segment.monospace {
        text = text.monospace().background_color(theme.sunken).color(theme.text);
    }
    if segment.mark {
        text = text.background_color(theme.selection);
    }
    if segment.keyboard {
        text = text.monospace().background_color(theme.sunken).size(12.0);
    }
    if segment.subscript {
        text = text.size(11.0);
    }
    if segment.superscript {
        text = text.size(11.0);
    }
    if segment.image {
        text = text.italics().color(theme.text_muted);
    }
    if segment.math {
        text = text.italics().color(theme.accent);
    }
    if let Some(footnote) = &segment.footnote {
        text = text.color(theme.link).size(11.0).strong();
        let response = ui.add(Label::new(text).sense(Sense::click())).on_hover_text("Footnote");
        if response.clicked() {
            output.anchor = Some(format!("fn-{footnote}"));
        }
        return;
    }
    if segment.link.is_some() {
        text = text.color(theme.link).underline();
    }
    if segment.link.is_some() || segment.image {
        let response = ui
            .add(Label::new(text).sense(Sense::click()))
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked()
            && let Some(target) = &segment.link
        {
            output.link = Some(LinkActivation {
                destination: target.destination.clone(),
                kind: target.kind,
            });
        }
    } else {
        ui.add(Label::new(text));
    }
    if segment.image {
        ui.add_space(2.0);
    }
}

/// Plain text of an inline sequence, used by tooltips and snapshots.
pub(crate) fn text_of(inlines: &[Inline]) -> String {
    plain_text(inlines)
}
