//! Inline Markdown flattening and painting.

use egui::{Label, RichText, Sense, Ui};
use mdview_markdown::{Inline, LinkKind, plain_text};
use mdview_media::MediaKind;
use mdview_theme::Theme;

use crate::{AnimationStatus, LinkActivation, MediaContext, MediaLightbox, MediaState, PreviewOutput};

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
    pub display_math: bool,
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
                segment.display_math = *display;
                segment.text.clone_from(tex);
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
pub(crate) fn show_inlines(
    ui: &mut Ui,
    inlines: &[Inline],
    theme: &Theme,
    output: &mut PreviewOutput,
    mut media: Option<&mut MediaContext<'_>>,
    size: f32,
    search: Option<&str>,
) {
    let segments = segments(inlines);
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        for segment in &segments {
            let next = media.as_deref_mut();
            show_segment(ui, segment, theme, output, next, size, search);
        }
    });
}

fn show_segment(
    ui: &mut Ui,
    segment: &Segment,
    theme: &Theme,
    output: &mut PreviewOutput,
    media: Option<&mut MediaContext<'_>>,
    size: f32,
    search: Option<&str>,
) {
    if segment.math {
        show_math(ui, segment, theme, media, size);
        return;
    }
    if segment.image {
        show_image(ui, segment, theme, output, media);
        return;
    }
    if let Some(footnote) = &segment.footnote {
        let text = RichText::new(&segment.text)
            .color(theme.link)
            .size((size - 3.0).max(9.0))
            .strong();
        let response = ui.add(Label::new(text).sense(Sense::click())).on_hover_text("Footnote");
        if response.clicked() {
            output.anchor = Some(format!("fn-{footnote}"));
        }
        return;
    }
    let runs = search
        .map(|query| highlight_runs(&segment.text, query))
        .unwrap_or_else(|| vec![(segment.text.clone(), false)]);
    for (run, matched) in runs {
        let mut text = styled_text(&run, segment, theme, size);
        if matched {
            text = text.background_color(theme.selection);
        }
        if segment.link.is_some() {
            let mut response = ui
                .add(Label::new(text).sense(Sense::click()))
                .on_hover_cursor(egui::CursorIcon::PointingHand);
            if let Some(target) = &segment.link {
                response = response.on_hover_text(&target.destination);
            }
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
    }
}

fn styled_text(text: &str, segment: &Segment, theme: &Theme, size: f32) -> RichText {
    let mut text = RichText::new(text).size(size).color(theme.text);
    if segment.strong {
        text = text.strong();
    }
    if segment.emphasis {
        text = text.italics();
    }
    if segment.strike {
        text = text.strikethrough();
    }
    if segment.monospace || segment.keyboard {
        text = text
            .monospace()
            .size((size - 2.0).max(9.0))
            .background_color(theme.table_header)
            .color(theme.text);
    }
    if segment.mark {
        text = text.background_color(theme.selection);
    }
    if segment.subscript || segment.superscript {
        text = text.size((size - 2.0).max(9.0));
    }
    if segment.link.is_some() {
        text = text.color(theme.link).underline();
    }
    text
}

/// Split text into plain and query-matching runs, case-insensitively.
pub(crate) fn highlight_runs(text: &str, query: &str) -> Vec<(String, bool)> {
    let query = query.trim();
    if query.is_empty() || text.is_empty() {
        return vec![(text.to_owned(), false)];
    }
    let lower = text.to_lowercase();
    let needle = query.to_lowercase();
    let mut runs = Vec::new();
    let mut start = 0usize;
    while start < text.len() {
        let Some(relative) = lower[start..].find(&needle) else {
            break;
        };
        let hit = start + relative;
        if !text.is_char_boundary(hit) || !text.is_char_boundary(hit + needle.len()) {
            break;
        }
        if hit > start {
            runs.push((text[start..hit].to_owned(), false));
        }
        runs.push((text[hit..hit + needle.len()].to_owned(), true));
        start = hit + needle.len();
    }
    if start < text.len() {
        runs.push((text[start..].to_owned(), false));
    }
    if runs.is_empty() {
        runs.push((text.to_owned(), false));
    }
    runs
}

fn show_image(
    ui: &mut Ui,
    segment: &Segment,
    theme: &Theme,
    output: &mut PreviewOutput,
    media: Option<&mut MediaContext<'_>>,
) {
    let Some(target) = &segment.link else {
        return;
    };
    let max_width = ui.available_width().clamp(32.0, 720.0);
    let alt = segment.text.clone();
    let Some(media_context) = media else {
        ui.label(RichText::new(&alt).italics().color(theme.text_muted));
        return;
    };
    // A stable raster width lets the same texture survive a split resize; the displayed
    // size is still derived from the pane width.
    let request = media_context.request(
        ui.ctx(),
        MediaKind::Image,
        &target.destination,
        1_600.0,
        theme.is_dark(),
    );
    let key = request.cache_key();
    let status = media_context.provider.animation_status(&key);
    match media_context.provider.resolve(ui.ctx(), &request) {
        MediaState::Loading => {
            ui.spinner();
            ui.label(RichText::new(&alt).italics().color(theme.text_muted));
        }
        MediaState::Ready(texture) => {
            let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0);
            let scale = (max_width / points.x).min(1.0);
            let response = ui
                .vertical(|ui| {
                    let response = ui
                        .add(
                            egui::Image::new(&texture)
                                .fit_to_exact_size(points * scale)
                                .sense(Sense::click()),
                        )
                        .on_hover_text(if alt.is_empty() {
                            target.destination.clone()
                        } else {
                            alt.clone()
                        });
                    if let Some(label) = animation_label(status) {
                        ui.label(RichText::new(label).small().color(theme.text_muted));
                    }
                    response
                })
                .inner;
            media_context
                .provider
                .mark_visible(&key, ui.is_rect_visible(response.rect));
            if response.clicked() {
                output.lightbox = Some(MediaLightbox {
                    request: request.clone(),
                    alt,
                });
            }
        }
        MediaState::Error(error) => {
            egui::Frame::new()
                .fill(theme.sunken)
                .stroke(egui::Stroke::new(1.0, theme.danger))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&alt).italics().color(theme.text_muted));
                        ui.label(RichText::new(error).small().color(theme.danger));
                    });
                });
        }
    }
}

fn animation_label(status: AnimationStatus) -> Option<&'static str> {
    match status {
        AnimationStatus::None => None,
        AnimationStatus::Playing => Some("Animated"),
        AnimationStatus::Paused => Some("Animation paused"),
        AnimationStatus::Unsupported => Some("Animation not fully supported"),
    }
}

fn show_math(ui: &mut Ui, segment: &Segment, theme: &Theme, media: Option<&mut MediaContext<'_>>, body_size: f32) {
    let Some(media_context) = media else {
        ui.label(RichText::new(&segment.text).italics().color(theme.accent));
        return;
    };
    let max_width = ui.available_width().clamp(32.0, 720.0);
    // Render the formula once at a generous resolution. Display scaling then follows the
    // reading typography, so split-width changes do not rasterize the same formula again.
    let request_width = 1024.0;
    match media_context.load(ui.ctx(), MediaKind::Math, &segment.text, request_width, theme.is_dark()) {
        MediaState::Loading => {
            ui.spinner();
        }
        MediaState::Ready(texture) => {
            let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0);
            // RaTeX lays formulas out at 40 px. Scale inline formulas to the body size and
            // display formulas to a heading-sized block; fractions and integrals keep their
            // vertical detail because the aspect ratio is preserved.
            let target_height = if segment.display_math {
                (body_size * 2.6).max(32.0)
            } else {
                (body_size * 1.25).max(15.0)
            };
            let scale = (max_width / points.x).min(target_height / points.y).min(1.0);
            let size = points * scale;
            if segment.display_math {
                let free = (ui.available_width() - size.x).max(0.0);
                ui.add_space(free / 2.0);
            }
            ui.add(egui::Image::new(&texture).fit_to_exact_size(size));
        }
        MediaState::Error(error) => {
            ui.label(
                RichText::new(format!("{}\n{error}", segment.text))
                    .italics()
                    .small()
                    .color(theme.danger),
            );
        }
    }
}

/// Plain text of an inline sequence, used by tooltips and snapshots.
pub(crate) fn text_of(inlines: &[Inline]) -> String {
    plain_text(inlines)
}
