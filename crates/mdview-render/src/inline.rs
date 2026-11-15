//! Inline Markdown flattening and painting.

use egui::{Label, RichText, Sense, Ui};
use mdview_markdown::{Inline, LinkKind, plain_text};
use mdview_media::MediaKind;
use mdview_theme::Theme;

use crate::{AnimationStatus, LinkActivation, MediaContext, MediaLightbox, MediaState, PreviewOutput};

mod code;
mod math;

/// Inline typography plus intrinsic media sizes.
pub(crate) struct InlineStyle<'a> {
    pub typography: mdview_theme::ReadingTypography,
    pub media_sizes: &'a std::collections::HashMap<String, (f32, f32)>,
    pub footnotes: &'a std::collections::HashMap<String, usize>,
}

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
///
/// Returns the vertical center of the first painted text row in screen
/// coordinates. List markers use it to sit on the same line as the text.
pub(crate) fn show_inlines(
    ui: &mut Ui,
    inlines: &[Inline],
    theme: &Theme,
    output: &mut PreviewOutput,
    mut media: Option<&mut MediaContext<'_>>,
    style: &InlineStyle<'_>,
    search: Option<&str>,
) -> Option<f32> {
    let segments = segments(inlines);
    // GitHub's Markdown body uses a 1.5 line height. epaint centers glyphs inside the
    // line box, so the row center is also the visual center of the text.
    let body_row = line_height(style.typography);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        // The wrapped layout reads its first row height from the parent spacing.
        ui.spacing_mut().interact_size.y = body_row;
        ui.horizontal_wrapped(|ui| {
            let mut first = None;
            for segment in &segments {
                let next = media.as_deref_mut();
                let rect = show_segment(ui, segment, theme, output, next, style, search);
                if first.is_none()
                    && let Some(rect) = rect
                {
                    // Items in a wrapped row are centered on the row. A label that
                    // wraps starts with a full body row at its top.
                    let center = if rect.height() > body_row * 1.5 && !segment.math && !segment.image {
                        rect.top() + body_row * 0.5
                    } else {
                        rect.center().y
                    };
                    first = Some(center);
                }
            }
            first
        })
        .inner
    })
    .inner
}

/// Body line height for the reading typography.
pub(crate) fn line_height(typography: mdview_theme::ReadingTypography) -> f32 {
    (typography.body * 1.5).round()
}

fn show_segment(
    ui: &mut Ui,
    segment: &Segment,
    theme: &Theme,
    output: &mut PreviewOutput,
    media: Option<&mut MediaContext<'_>>,
    style: &InlineStyle<'_>,
    search: Option<&str>,
) -> Option<egui::Rect> {
    let typography = style.typography;
    let media_sizes = style.media_sizes;
    let size = typography.body;
    if segment.math {
        return Some(math::show_math(ui, segment, theme, media, typography, media_sizes));
    }
    if segment.image {
        let before = ui.cursor().min;
        show_image(ui, segment, theme, output, media, typography, media_sizes);
        return Some(egui::Rect::from_min_max(before, ui.min_rect().max));
    }
    if let Some(footnote) = &segment.footnote {
        // GitHub renders a footnote reference as a raised, bracketed number.
        let number = style
            .footnotes
            .get(footnote)
            .map_or_else(|| footnote.clone(), usize::to_string);
        let galley = ui.painter().layout_no_wrap(
            format!("[{number}]"),
            mdview_theme::ReadingFace::Regular.font((size * 0.75).round().max(9.0)),
            theme.link,
        );
        let row = line_height(typography);
        let (rect, response) = ui.allocate_exact_size(egui::vec2(galley.size().x + 2.0, row), Sense::click());
        ui.painter().galley(
            egui::pos2(rect.left() + 1.0, rect.top() + row * 0.08),
            galley,
            theme.link,
        );
        let response = response
            .on_hover_text("Footnote")
            .on_hover_cursor(egui::CursorIcon::PointingHand);
        if response.clicked() {
            output.anchor = Some(format!("fn-{footnote}"));
        }
        return Some(response.rect);
    }
    let runs = search
        .map(|query| crate::highlight::highlight_runs(&segment.text, query))
        .unwrap_or_else(|| vec![(segment.text.clone(), false)]);
    let mut first = None;
    for (run, matched) in runs {
        let code = segment.monospace || segment.keyboard;
        // Every run shares the body line box, so wrapped rows keep GitHub's 1.5 rhythm.
        let mut text =
            styled_text(&run, segment, theme, size, typography.code).line_height(Some(line_height(typography)));
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
            first.get_or_insert(response.rect);
        } else if code {
            let rect = code::show_code_chip(ui, text, &run, theme, size, typography);
            first.get_or_insert(rect);
        } else {
            let rect = ui.add(Label::new(text)).rect;
            first.get_or_insert(rect);
        }
    }
    first
}

fn styled_text(text: &str, segment: &Segment, theme: &Theme, size: f32, code: f32) -> RichText {
    // Bold and italic use the real Noto Sans faces, as GitHub does on Linux.
    let face = mdview_theme::ReadingFace::from_style(segment.strong, segment.emphasis);
    let mut text = RichText::new(text).size(size).color(theme.text);
    if !(segment.monospace || segment.keyboard) {
        text = text.family(face.family());
    }
    if segment.strike {
        text = text.strikethrough();
    }
    if segment.monospace || segment.keyboard {
        // GitHub sets inline code at 85% of the body on a muted chip.
        text = text.monospace().size(code::code_size(size, code)).color(theme.text);
    }
    if segment.mark {
        text = text.background_color(theme.selection);
    }
    if segment.subscript || segment.superscript {
        text = text.size((size - 2.0).max(9.0));
    }
    if segment.link.is_some() {
        // Links are colored, not underlined, as in GitHub's Markdown body.
        text = text.color(theme.link);
    }
    text
}

fn show_image(
    ui: &mut Ui,
    segment: &Segment,
    theme: &Theme,
    output: &mut PreviewOutput,
    media: Option<&mut MediaContext<'_>>,
    typography: mdview_theme::ReadingTypography,
    media_sizes: &std::collections::HashMap<String, (f32, f32)>,
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
            let height = crate::layout::reserved_media_height(
                MediaKind::Image,
                &target.destination,
                max_width,
                typography,
                media_sizes,
            );
            let (rect, _) = ui.allocate_exact_size(egui::vec2(max_width, height), egui::Sense::hover());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &alt,
                egui::FontId::proportional(12.0),
                theme.text_muted,
            );
        }
        MediaState::Ready(texture) => {
            let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0);
            let reserved = crate::layout::reserved_media_height(
                MediaKind::Image,
                &target.destination,
                max_width,
                typography,
                media_sizes,
            );
            let scale = (max_width / points.x).min(reserved / points.y).min(1.0);
            let size = points * scale;
            let (rect, response) = ui.allocate_exact_size(egui::vec2(max_width, reserved), egui::Sense::click());
            let image_rect = egui::Rect::from_center_size(rect.center(), size);
            ui.painter().image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            if let Some(label) = animation_label(status) {
                ui.painter().text(
                    egui::pos2(rect.left() + 4.0, rect.bottom() - 4.0),
                    egui::Align2::LEFT_BOTTOM,
                    label,
                    egui::FontId::proportional(10.0),
                    theme.text_muted,
                );
            }
            let response = response.on_hover_text(if alt.is_empty() {
                target.destination.clone()
            } else {
                alt.clone()
            });
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

/// Plain text of an inline sequence, used by tooltips and snapshots.
pub(crate) fn text_of(inlines: &[Inline]) -> String {
    plain_text(inlines)
}
