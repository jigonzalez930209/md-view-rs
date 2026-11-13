//! Block painting for the Markdown preview.

use std::path::PathBuf;

use egui::{Align, Color32, FontId, Label, Layout, RichText, TextFormat, Ui};
use mdview_markdown::{Alignment, Block, BlockKind, Inline, ListItem, QuoteKind};
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
}

pub(crate) fn heading_size(level: u8) -> f32 {
    match level {
        1 => 28.0,
        2 => 23.0,
        3 => 19.0,
        4 => 16.5,
        5 => 15.0,
        _ => 14.0,
    }
}

pub(crate) fn show_blocks(
    ui: &mut Ui,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    for block in blocks {
        show_block(ui, block, ctx, media);
        ui.add_space(6.0);
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
            );
        }
        BlockKind::Heading { level, anchor, inlines } => {
            let text = text_of(inlines);
            let response = ui.add(
                Label::new(
                    RichText::new(text)
                        .size(heading_size(*level))
                        .strong()
                        .color(ctx.theme.text),
                )
                .selectable(true),
            );
            ctx.output.anchors_screen.push((anchor.clone(), response.rect.top()));
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
                            .size(12.0)
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
                        ui.label(RichText::new(title).strong().color(alert_color(*kind, ctx.theme)));
                        ui.add_space(2.0);
                    }
                    show_blocks(ui, blocks, ctx, media);
                });
        }
        BlockKind::List { ordered, start, items } => {
            show_list(ui, *ordered, *start, items, ctx, media);
        }
        BlockKind::Table { alignments, head, rows } => show_table(ui, alignments, head, rows, ctx, media),
        BlockKind::Rule => {
            ui.separator();
        }
        BlockKind::FootnoteDefinition { label, blocks } => {
            ctx.output
                .anchors_screen
                .push((format!("fn-{label}"), ui.min_rect().top()));
            egui::Frame::new()
                .stroke(egui::Stroke::new(1.0, ctx.theme.border))
                .inner_margin(8.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("[^{label}]"))
                                .small()
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

fn show_list(
    ui: &mut Ui,
    ordered: bool,
    start: u64,
    items: &[ListItem],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    for (index, item) in items.iter().enumerate() {
        ui.horizontal_top(|ui| {
            let marker = match item.task {
                Some(true) => "[x]".to_owned(),
                Some(false) => "[ ]".to_owned(),
                None if ordered => format!("{}.", start + index as u64),
                None => "-".to_owned(),
            };
            ui.add_space(4.0);
            ui.label(RichText::new(marker).color(ctx.theme.text_muted).strong());
            ui.add_space(4.0);
            ui.vertical(|ui| {
                show_blocks(ui, &item.blocks, ctx, media);
            });
        });
    }
}

fn show_table(
    ui: &mut Ui,
    alignments: &[Alignment],
    head: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    egui::Frame::new()
        .stroke(egui::Stroke::new(1.0, ctx.theme.border))
        .inner_margin(4.0)
        .show(ui, |ui| {
            egui::Grid::new(ui.next_auto_id())
                .striped(true)
                .min_col_width(24.0)
                .show(ui, |ui| {
                    for (index, cell) in head.iter().enumerate() {
                        let align = alignments.get(index).copied().unwrap_or_default();
                        table_cell(ui, cell, align, ctx, media, true);
                    }
                    ui.end_row();
                    for row in rows {
                        for (index, cell) in row.iter().enumerate() {
                            let align = alignments.get(index).copied().unwrap_or_default();
                            table_cell(ui, cell, align, ctx, media, false);
                        }
                        ui.end_row();
                    }
                });
        });
}

fn table_cell(
    ui: &mut Ui,
    inlines: &[Inline],
    alignment: Alignment,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
    header: bool,
) {
    let layout = match alignment {
        Alignment::Right => Layout::right_to_left(Align::Center),
        Alignment::Center => Layout::centered_and_justified(egui::Direction::LeftToRight),
        Alignment::Left | Alignment::None => Layout::left_to_right(Align::Center),
    };
    ui.with_layout(layout, |ui| {
        if header {
            let text = text_of(inlines);
            ui.label(RichText::new(text).strong().color(ctx.theme.text));
        } else {
            show_inlines(
                ui,
                inlines,
                ctx.theme,
                ctx.output,
                media.as_mut().map(|context| &mut **context),
            );
        }
    });
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
    let request = media_context.request(ui.ctx(), kind, source, width, ctx.theme.is_dark());
    match media_context.provider.resolve(ui.ctx(), &request) {
        MediaState::Loading => {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label(RichText::new("Rendering...").small().color(ctx.theme.text_muted));
            });
            true
        }
        MediaState::Ready(texture) => {
            let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0);
            let scale = (width / points.x).min(1.0);
            let response = ui
                .add(
                    egui::Image::new(&texture)
                        .fit_to_exact_size(points * scale)
                        .sense(egui::Sense::click()),
                )
                .on_hover_cursor(egui::CursorIcon::ZoomIn);
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
    egui::Frame::new()
        .fill(ctx.theme.sunken)
        .stroke(egui::Stroke::new(1.0, ctx.theme.border))
        .corner_radius(4.0)
        .inner_margin(8.0)
        .show(ui, |ui| {
            ui.horizontal(|ui| {
                let label = if math {
                    "Math".to_owned()
                } else {
                    language.unwrap_or("Code").to_owned()
                };
                ui.label(RichText::new(label).small().color(ctx.theme.text_muted).monospace());
                ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
                    if ui.small_button("Copy").on_hover_text("Copy code").clicked() {
                        ui.ctx().copy_text(text.to_owned());
                        ctx.output.copied = true;
                    }
                });
            });
            if math {
                ui.label(
                    RichText::new(text.trim_end())
                        .monospace()
                        .size(13.0)
                        .italics()
                        .color(ctx.theme.accent),
                );
                return;
            }
            let path = language_path(language);
            let highlighted = ctx.highlighter.highlight(text, path.as_deref(), ctx.light);
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = ui.available_width();
            for line in &highlighted.lines {
                for span in line {
                    let color = span
                        .color
                        .map(|[r, g, b]| Color32::from_rgb(r, g, b))
                        .unwrap_or(ctx.theme.text);
                    job.append(
                        &span.text,
                        0.0,
                        TextFormat {
                            font_id: FontId::monospace(13.0),
                            color,
                            ..Default::default()
                        },
                    );
                }
                job.append(
                    "\n",
                    0.0,
                    TextFormat {
                        font_id: FontId::monospace(13.0),
                        color: ctx.theme.text,
                        ..Default::default()
                    },
                );
            }
            ui.add(Label::new(job).selectable(true));
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
