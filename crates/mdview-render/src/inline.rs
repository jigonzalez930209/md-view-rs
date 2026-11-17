//! Inline Markdown flattening and painting.

use egui::{Label, RichText, Sense, Ui};
use mdview_markdown::{Inline, LinkKind};
use mdview_theme::Theme;

use crate::{LinkActivation, MediaContext, PreviewOutput};

mod code;
mod image;

pub(crate) use image::display_size as image_display_size;
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
    /// `<ins>` or `<u>`: underlined, as GitHub shows inserted text.
    pub underline: bool,
    pub monospace: bool,
    pub mark: bool,
    pub subscript: bool,
    pub superscript: bool,
    pub keyboard: bool,
    pub link: Option<LinkTarget>,
    pub image: bool,
    /// Image source, kept apart from `link` so a linked image keeps its link.
    pub image_source: Option<LinkTarget>,
    /// Display size from HTML `width`/`height` attributes.
    pub image_size: (Option<u32>, Option<u32>),
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
    underline: bool,
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
            Inline::Image {
                source,
                alt,
                kind,
                width,
                height,
                ..
            } => {
                let mut segment = style.clone();
                segment.image = true;
                segment.text = if alt.is_empty() { source.clone() } else { alt.clone() };
                segment.image_source = Some(LinkTarget {
                    destination: source.clone(),
                    kind: *kind,
                });
                segment.image_size = (*width, *height);
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
            Inline::Html(raw) => match crate::html::image_tag(raw) {
                // GitHub renders inline `<img>` tags in paragraphs.
                Some(image) => flatten(std::slice::from_ref(&image), style, html, out),
                None => apply_html(raw, html, style, out),
            },
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
    segment.underline |= html.underline;
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
        "ins" | "u" => html.underline = !closing,
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
    let mut segments = segments(inlines);
    // egui measures each word with its trailing space; the extra row width below
    // compensates. The final word gets a trailing space too, so it is measured the
    // same way instead of fitting into the tolerance (browsers would wrap it).
    if let Some(last) = segments.last_mut()
        && !last.math
        && !last.image
        && !last.monospace
        && !last.keyboard
        && last.footnote.is_none()
        && !last.text.ends_with(char::is_whitespace)
    {
        last.text.push(' ');
    }
    // GitHub's Markdown body uses a 1.5 line height. epaint centers glyphs inside the
    // line box, so the row center is also the visual center of the text.
    let body_row = line_height(style.typography);
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);
        // The wrapped layout reads its first row height from the parent spacing.
        ui.spacing_mut().interact_size.y = body_row;
        // Rows with images align on the bottom, which with the image's descent pad
        // puts images on the text baseline like CSS `vertical-align: baseline`.
        let layout = if segments.iter().any(|segment| segment.image) {
            egui::Layout::left_to_right(egui::Align::Max).with_main_wrap(true)
        } else {
            egui::Layout::left_to_right(egui::Align::Center).with_main_wrap(true)
        };
        // Same sizing as `horizontal_wrapped`, with the chosen cross alignment. A row
        // with images starts at the tallest image's height, so text placed before an
        // image is already bottom-aligned with it (egui does not re-place items).
        // egui counts a line's trailing space when deciding whether a word fits;
        // browsers let that space hang past the edge. One extra space of width
        // lets the same words share a line as on GitHub without any ink overflow.
        let space = ui
            .painter()
            .layout_no_wrap(
                " ".to_owned(),
                mdview_theme::ReadingFace::Regular.font(style.typography.body),
                theme.text,
            )
            .size()
            .x;
        let width = ui.available_size_before_wrap().x + space;
        let descent = body_row - math::text_baseline(ui, style.typography);
        let image_row = segments
            .iter()
            .filter_map(|segment| {
                let source = segment.image_source.as_ref()?;
                let (natural_width, natural_height) = style.media_sizes.get(&source.destination)?;
                let size = image::display_size(
                    egui::vec2(*natural_width, *natural_height),
                    segment.image_size,
                    ui.max_rect().width().max(32.0),
                );
                Some(size.y + descent)
            })
            .fold(body_row, f32::max);
        let initial = egui::vec2(width, image_row);
        ui.allocate_ui_with_layout(initial, layout, |ui| {
            let mut first = None;
            for (index, segment) in segments.iter().enumerate() {
                let next = media.as_deref_mut();
                // Browsers cannot break between inline code and text glued to it (a
                // following comma or period); that text must fit with the chip.
                let glue = match segments.get(index + 1) {
                    Some(following) if (segment.monospace || segment.keyboard) && !following.monospace => {
                        let prefix: String = following
                            .text
                            .chars()
                            .take_while(|character| !character.is_whitespace())
                            .collect();
                        if prefix.is_empty() {
                            0.0
                        } else {
                            ui.painter()
                                .layout_no_wrap(
                                    prefix,
                                    mdview_theme::ReadingFace::Regular.font(style.typography.body),
                                    theme.text,
                                )
                                .size()
                                .x
                        }
                    }
                    _ => 0.0,
                };
                let rect = show_segment(ui, segment, theme, output, next, style, (search, glue));
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
    (search, glue): (Option<&str>, f32),
) -> Option<egui::Rect> {
    let typography = style.typography;
    let media_sizes = style.media_sizes;
    let size = typography.body;
    if segment.math {
        return Some(math::show_math(ui, segment, theme, media, typography, media_sizes));
    }
    if segment.image {
        let before = ui.cursor().min;
        image::show_image(ui, segment, theme, output, media, typography, media_sizes);
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
    if (segment.subscript || segment.superscript) && segment.link.is_none() && !segment.monospace {
        return Some(code::show_script(ui, segment, theme, size, typography));
    }
    let runs = search
        .map(|query| crate::highlight::highlight_runs(&segment.text, query))
        .unwrap_or_else(|| vec![(segment.text.clone(), false)]);
    let mut first = None;
    for (run, matched) in runs {
        let code = segment.monospace || segment.keyboard;
        // Every run shares the body line box, so wrapped rows keep GitHub's 1.5 rhythm.
        // In GitHub a line holding inline code is 0.125em taller: the monospace font's
        // own 1.5 line box, aligned on the baseline, extends the line.
        let row = line_height(typography) + if code { (size * 0.125).round() } else { 0.0 };
        let mut text = styled_text(&run, segment, theme, size, typography.code).line_height(Some(row));
        if matched {
            text = text.background_color(theme.selection);
        }
        let chip_text = |part: &str| {
            let mut part_text = styled_text(part, segment, theme, size, typography.code).line_height(Some(row));
            if matched {
                part_text = part_text.background_color(theme.selection);
            }
            part_text
        };
        if segment.link.is_some() {
            let mut response = if code {
                // Linked code keeps GitHub's chip, in the link color.
                let rect = code::show_code_chip(ui, &chip_text, &run, theme, (size, glue), typography);
                ui.interact(
                    rect,
                    ui.make_persistent_id(("mdview-code-link", &run, rect.min.x as i32, rect.min.y as i32)),
                    Sense::click(),
                )
            } else {
                ui.add(Label::new(text).sense(Sense::click()))
            }
            .on_hover_cursor(egui::CursorIcon::PointingHand);
            if let Some(target) = &segment.link {
                response = response.on_hover_text(&target.destination);
            }
            if response.clicked()
                && let Some(target) = &segment.link
                && let Some(label) = target.destination.strip_prefix("#fnref-")
            {
                // A footnote's `↩` scrolls back to its reference inside the preview.
                output.anchor = Some(format!("fnref-{label}"));
            } else if response.clicked()
                && let Some(target) = &segment.link
            {
                output.link = Some(LinkActivation {
                    destination: target.destination.clone(),
                    kind: target.kind,
                });
            }
            first.get_or_insert(response.rect);
        } else if segment.keyboard {
            let rect = code::show_kbd(ui, &run, theme, size, typography);
            first.get_or_insert(rect);
        } else if code {
            let rect = code::show_code_chip(ui, &chip_text, &run, theme, (size, glue), typography);
            first.get_or_insert(rect);
        } else if ui.layout().vertical_align() == egui::Align::Max && !matched {
            // Rows with images are bottom-aligned. A wrapping label would center its
            // first row regardless, so text goes word by word as non-wrapping labels
            // and the row layout breaks lines between words.
            let row = line_height(typography);
            for word in run.split_inclusive(' ') {
                let word_text = styled_text(word, segment, theme, size, typography.code).line_height(Some(row));
                let rect = ui.add(Label::new(word_text).wrap_mode(egui::TextWrapMode::Extend)).rect;
                first.get_or_insert(rect);
            }
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
    if segment.underline {
        text = text.underline();
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

#[cfg(test)]
#[path = "inline_tests.rs"]
mod underline_tests;
