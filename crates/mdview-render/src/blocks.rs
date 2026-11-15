//! Block painting for the Markdown preview.

use std::path::PathBuf;

use egui::{Align, Align2, Color32, FontId, Label, Layout, RichText, TextFormat, Ui};
mod list;
mod table;

use mdview_markdown::{Block, BlockKind, ListItem, QuoteKind};
use mdview_media::MediaKind;
use mdview_syntax::Highlighter;
use mdview_theme::Theme;

use crate::PreviewOutput;
use crate::inline::{show_inlines, text_of};
use crate::{MediaContext, MediaLightbox, MediaState};

/// Shared painting state.
pub(crate) struct BlockCtx<'a> {
    pub theme: &'a Theme,
    pub light: bool,
    pub highlighter: &'a Highlighter,
    pub output: &'a mut PreviewOutput,
    pub typography: mdview_theme::ReadingTypography,
    pub media_sizes: &'a std::collections::HashMap<String, (f32, f32)>,
    pub search: Option<&'a str>,
}

pub(crate) fn heading_size(level: u8, base: f32) -> f32 {
    let factor = match level {
        1 => 1.0,
        2 => 0.82,
        3 => 0.68,
        4 => 0.59,
        5 => 0.52,
        _ => 0.47,
    };
    (base * factor).max(12.0)
}

pub(crate) fn show_blocks(
    ui: &mut Ui,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    show_blocks_spacing(ui, blocks, ctx, media, 6.0);
}

pub(crate) fn show_blocks_spacing(
    ui: &mut Ui,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
    gap: f32,
) {
    for (index, block) in blocks.iter().enumerate() {
        // Headings need breathing room before them, especially after a code block or diagram.
        if index > 0 && matches!(block.kind, BlockKind::Heading { .. }) {
            ui.add_space(5.0);
        }
        show_block(ui, block, ctx, media);
        ui.add_space(gap);
    }
}

pub(crate) fn show_block(
    ui: &mut Ui,
    block: &Block,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    match &block.kind {
        BlockKind::Paragraph(inlines) => {
            show_inlines(
                ui,
                inlines,
                ctx.theme,
                ctx.output,
                media.as_mut().map(|context| &mut **context),
                &crate::inline::InlineStyle {
                    typography: ctx.typography,
                    media_sizes: ctx.media_sizes,
                },
                ctx.search,
            );
        }
        BlockKind::Heading { level, anchor, inlines } => {
            let text = text_of(inlines);
            let size = heading_size(*level, ctx.typography.headings);
            if let Some(query) = ctx.search {
                for (run, matched) in crate::highlight::highlight_runs(&text, query) {
                    let mut rich = RichText::new(run).size(size).strong().color(ctx.theme.text);
                    if matched {
                        rich = rich.background_color(ctx.theme.selection);
                    }
                    ui.add(Label::new(rich).selectable(true));
                }
            } else {
                ui.add(Label::new(RichText::new(text).size(size).strong().color(ctx.theme.text)).selectable(true));
            }
            let _ = anchor;
        }
        BlockKind::Code { language, text, math } => {
            show_code(ui, language.as_deref(), text, *math, ctx, media);
        }
        BlockKind::Html(html) => {
            egui::Frame::new()
                .fill(ctx.theme.sunken)
                .stroke(egui::Stroke::new(1.0, ctx.theme.border))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.label(
                        RichText::new(html.trim_end())
                            .monospace()
                            .size((ctx.typography.code - 1.0).max(9.0))
                            .color(ctx.theme.text_muted),
                    );
                });
        }
        BlockKind::BlockQuote { kind, blocks } => {
            egui::Frame::new()
                .fill(ctx.theme.sunken)
                .stroke(egui::Stroke::new(1.0, ctx.theme.border))
                .inner_margin(10.0)
                .show(ui, |ui| {
                    if let Some(title) = kind.alert_title() {
                        ui.label(
                            RichText::new(title)
                                .size(ctx.typography.body)
                                .strong()
                                .color(alert_color(*kind, ctx.theme)),
                        );
                        ui.add_space(2.0);
                    }
                    show_blocks(ui, blocks, ctx, media);
                });
        }
        BlockKind::List { ordered, start, items } => {
            list::show_list(ui, *ordered, *start, items, ctx, media);
        }
        BlockKind::Table { alignments, head, rows } => table::show_table(ui, alignments, head, rows, ctx, media),
        BlockKind::Rule => {
            ui.separator();
        }
        BlockKind::FootnoteDefinition { label, blocks } => {
            let _ = label;
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, ctx.theme.border))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("[^{label}]"))
                                .size(ctx.typography.body)
                                .strong()
                                .color(ctx.theme.text_muted),
                        );
                        if ui.small_button("back").on_hover_text("Back to reference").clicked() {
                            ctx.output.anchor = Some(format!("fnref-{label}"));
                        }
                    });
                    show_blocks(ui, blocks, ctx, media);
                });
        }
    }
}

fn show_code(
    ui: &mut Ui,
    language: Option<&str>,
    text: &str,
    math: bool,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    if math && show_media_block(ui, MediaKind::Math, text, ctx, media) {
        return;
    }
    if language.is_some_and(|language| language.eq_ignore_ascii_case("mermaid")) {
        if show_media_block(ui, MediaKind::Mermaid, text, ctx, media) {
            return;
        }
        ui.label(
            RichText::new("Could not render this Mermaid diagram; showing the source.")
                .small()
                .color(ctx.theme.danger),
        );
    }
    show_plain_code(ui, language, text, math, ctx);
}

fn show_media_block(
    ui: &mut Ui,
    kind: MediaKind,
    source: &str,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) -> bool {
    let Some(media_context) = media.as_mut().map(|context| &mut **context) else {
        return false;
    };
    let width = ui.available_width().max(64.0);
    // Vector media are rasterized once at a stable width. Split resizing then rescales
    // the existing texture instead of re-rendering diagrams and formulas.
    let request_width = if kind == MediaKind::Mermaid { 1_200.0 } else { width };
    let request = media_context.request(ui.ctx(), kind, source, request_width, ctx.theme.is_dark());
    match media_context.provider.resolve(ui.ctx(), &request) {
        MediaState::Loading => {
            let height = crate::layout::reserved_media_height(kind, source, width, ctx.typography, ctx.media_sizes);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            ui.painter().text(
                rect.center(),
                Align2::CENTER_CENTER,
                "Rendering...",
                FontId::proportional(12.0),
                ctx.theme.text_muted,
            );
            true
        }
        MediaState::Ready(texture) => {
            let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0);
            // Reserve one stable box per media kind. Both the placeholder and the
            // finished texture occupy the same height, so lazy rasterization cannot
            // reflow the document or shift the reader's position.
            let reserved = crate::layout::reserved_media_height(kind, source, width, ctx.typography, ctx.media_sizes);
            let scale = (width / points.x).min(reserved / points.y).min(1.0);
            let size = points * scale;
            let response = ui
                .allocate_ui_with_layout(egui::vec2(width, reserved), Layout::top_down(Align::Center), |ui| {
                    ui.add_space(((reserved - size.y) * 0.5).max(0.0));
                    ui.add(
                        egui::Image::new(&texture)
                            .fit_to_exact_size(size)
                            .sense(egui::Sense::click()),
                    )
                    .on_hover_cursor(egui::CursorIcon::ZoomIn)
                })
                .inner;
            if response.clicked() {
                ctx.output.lightbox = Some(MediaLightbox {
                    request,
                    alt: format!("{} view", kind.label()),
                });
            }
            true
        }
        MediaState::Error(error) => {
            ui.label(RichText::new(error).small().color(ctx.theme.danger));
            false
        }
    }
}

fn show_plain_code(ui: &mut Ui, language: Option<&str>, text: &str, math: bool, ctx: &mut BlockCtx<'_>) {
    let code_size = ctx.typography.code;
    let line_height = code_size * 1.46;
    egui::Frame::new()
        .fill(ctx.theme.code_background)
        .stroke(egui::Stroke::new(1.0, ctx.theme.border))
        .corner_radius(6.0)
        .inner_margin(egui::Margin::symmetric(10, 6))
        .show(ui, |ui| {
            ui.spacing_mut().item_spacing.y = 0.0;
            ui.horizontal(|ui| {
                let label = if math {
                    "Math".to_owned()
                } else {
                    language.unwrap_or("Code").to_owned()
                };
                ui.label(
                    RichText::new(label)
                        .size((code_size - 2.0).max(9.0))
                        .color(ctx.theme.text_muted)
                        .monospace(),
                );
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Copy").on_hover_text("Copy code").clicked() {
                        ui.ctx().copy_text(text.to_owned());
                        ctx.output.copied = true;
                    }
                });
            });
            ui.separator();
            if math {
                ui.add_space(4.0);
                ui.label(
                    RichText::new(text.trim_end())
                        .monospace()
                        .size(code_size)
                        .italics()
                        .color(ctx.theme.accent),
                );
                return;
            }
            ui.add_space(4.0);
            let path = language_path(language);
            let highlighted = ctx.highlighter.highlight(text, path.as_deref(), ctx.light);
            let mut job = egui::text::LayoutJob::default();
            // Code lines keep their indentation and scroll horizontally, like GitHub and
            // desktop editors, instead of soft-wrapping in the middle of a token.
            job.wrap.max_width = f32::INFINITY;
            for (index, line) in highlighted.lines.iter().enumerate() {
                for span in line {
                    let color = span
                        .color
                        .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                        .unwrap_or(ctx.theme.text);
                    let color = if mdview_theme::contrast_ratio(color, ctx.theme.code_background) < 3.0 {
                        ctx.theme.text
                    } else {
                        color
                    };
                    job.append(
                        &span.text,
                        0.0,
                        TextFormat {
                            font_id: FontId::monospace(code_size),
                            color,
                            line_height: Some(line_height),
                            ..Default::default()
                        },
                    );
                }
                if index + 1 < highlighted.lines.len() {
                    job.append(
                        "\n",
                        0.0,
                        TextFormat {
                            font_id: FontId::monospace(code_size),
                            color: ctx.theme.text,
                            line_height: Some(line_height),
                            ..Default::default()
                        },
                    );
                }
            }
            egui::ScrollArea::horizontal()
                .id_salt((language, text.len(), text.as_ptr()))
                .auto_shrink([false, true])
                .scroll_source(egui::scroll_area::ScrollSource {
                    scroll_bar: true,
                    drag: egui::scroll_area::DragScroll::Always,
                    mouse_wheel: false,
                })
                .show(ui, |ui| {
                    ui.add(Label::new(job).selectable(true).wrap_mode(egui::TextWrapMode::Extend));
                });
        });
}

fn language_path(language: Option<&str>) -> Option<PathBuf> {
    let language = language?;
    let extension = match language.to_ascii_lowercase().as_str() {
        "rust" | "rs" => "rs",
        "javascript" | "js" => "js",
        "typescript" | "ts" => "ts",
        "python" | "py" => "py",
        "json" => "json",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "html" => "html",
        "css" => "css",
        "scss" => "scss",
        "xml" => "xml",
        "sql" => "sql",
        "sh" | "bash" | "shell" => "sh",
        "c" => "c",
        "cpp" | "c++" => "cpp",
        "csharp" | "cs" => "cs",
        "go" => "go",
        "java" => "java",
        "kotlin" | "kt" => "kt",
        "swift" => "swift",
        "php" => "php",
        "ruby" | "rb" => "rb",
        "lua" => "lua",
        "dockerfile" => "dockerfile",
        "makefile" => "makefile",
        "ini" => "ini",
        "diff" | "patch" => "diff",
        "graphql" | "gql" => "graphql",
        "markdown" | "md" => "md",
        _ => return None,
    };
    Some(PathBuf::from(format!("code.{extension}")))
}

fn alert_color(kind: QuoteKind, theme: &Theme) -> Color32 {
    match kind {
        QuoteKind::Caution | QuoteKind::Warning => theme.danger,
        QuoteKind::Important => theme.accent,
        QuoteKind::Note | QuoteKind::Tip | QuoteKind::Quote => theme.text,
    }
}
