//! Block painting for the Markdown preview.

use std::path::PathBuf;

use egui::{Align, Align2, Color32, FontId, Label, Layout, RichText, TextFormat, Ui};
mod code;
mod footnote;
mod list;
mod octicon;
mod quote;
mod table;

pub(crate) use footnote::{Footnotes, block_anchors, footnote_numbers};

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
    pub list: ListState,
    /// Footnote numbers by label, in order of first reference.
    pub footnotes: &'a Footnotes,
}

/// List layout state shared by nested blocks.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ListState {
    /// Nesting depth of the list being painted; selects the bullet and number style.
    pub depth: usize,
    /// When set, the next paragraph records its first text row center in `first_line`.
    pub capture_first_line: bool,
    /// Vertical center of the first text row of the last captured paragraph.
    pub first_line: Option<f32>,
}

impl BlockCtx<'_> {
    /// A nested context that shares the output, for containers that change the
    /// theme or typography of their children.
    pub(crate) fn reborrow(&mut self) -> BlockCtx<'_> {
        BlockCtx {
            theme: self.theme,
            light: self.light,
            highlighter: self.highlighter,
            output: &mut *self.output,
            typography: self.typography,
            media_sizes: self.media_sizes,
            search: self.search,
            list: ListState {
                depth: self.list.depth,
                ..ListState::default()
            },
            footnotes: self.footnotes,
        }
    }
}

pub(crate) fn heading_size(level: u8, base: f32) -> f32 {
    // GitHub: h1 2em, h2 1.5em, h3 1.25em, h4 1em, h5 0.875em, h6 0.85em. The
    // heading setting is the h1 size, so every level keeps GitHub's ratio to h1.
    let factor = match level {
        1 => 1.0,
        2 => 0.75,
        3 => 0.625,
        4 => 0.5,
        5 => 0.4375,
        _ => 0.425,
    };
    (base * factor).max(12.0)
}

pub(crate) fn show_blocks(
    ui: &mut Ui,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let gap = block_gap(ctx.typography);
    show_blocks_spacing(ui, blocks, ctx, media, gap);
}

/// Space after a block: GitHub uses a 16 px margin at a 16 px body, one em.
pub(crate) fn block_gap(typography: mdview_theme::ReadingTypography) -> f32 {
    typography.body.round()
}

pub(crate) fn show_blocks_spacing(
    ui: &mut Ui,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
    gap: f32,
) {
    for (index, block) in blocks.iter().enumerate() {
        show_block(ui, block, ctx, media);
        if index + 1 < blocks.len() {
            ui.add_space(gap);
        }
    }
}

pub(crate) fn show_block(
    ui: &mut Ui,
    block: &Block,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let capture = std::mem::take(&mut ctx.list.capture_first_line);
    match &block.kind {
        BlockKind::Paragraph(inlines) => {
            let first = show_inlines(
                ui,
                inlines,
                ctx.theme,
                ctx.output,
                media.as_mut().map(|context| &mut **context),
                &crate::inline::InlineStyle {
                    typography: ctx.typography,
                    media_sizes: ctx.media_sizes,
                    footnotes: &ctx.footnotes.numbers,
                },
                ctx.search,
            );
            if capture {
                ctx.list.first_line = first;
            }
        }
        BlockKind::Heading { level, anchor, inlines } => {
            let _ = anchor;
            show_heading(ui, *level, inlines, ctx);
        }
        BlockKind::Code { language, text, math } => {
            code::show_code(ui, language.as_deref(), text, *math, ctx, media);
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
        BlockKind::BlockQuote { kind, blocks } => quote::show_quote(ui, *kind, blocks, ctx, media),
        BlockKind::List {
            ordered,
            start,
            items,
            loose,
        } => {
            list::show_list(ui, (*ordered, *start, *loose), items, ctx, media);
        }
        BlockKind::Table { alignments, head, rows } => table::show_table(ui, alignments, head, rows, ctx, media),
        BlockKind::Rule => {
            // GitHub: a 0.25em bar in the border color with 24 px of space around it.
            let body = ctx.typography.body;
            let height = (body * 0.25).round().max(2.0);
            let (rect, _) = ui.allocate_exact_size(
                egui::vec2(ui.available_width(), body * 0.5 + height + body * 0.5),
                egui::Sense::hover(),
            );
            let bar = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), height));
            ui.painter().rect_filled(bar, 0.0, ctx.theme.border);
        }
        BlockKind::FootnoteDefinition { label, blocks } => footnote::show_footnote(ui, label, blocks, ctx, media),
    }
}

/// A heading with GitHub's 24 px top margin, 1.25 line height, and the bottom rule
/// under h1 and h2.
fn show_heading(ui: &mut Ui, level: u8, inlines: &[mdview_markdown::Inline], ctx: &mut BlockCtx<'_>) {
    let text = text_of(inlines);
    let size = heading_size(level, ctx.typography.headings);
    let body = ctx.typography.body;
    let color = if level >= 6 {
        ctx.theme.text_muted
    } else {
        ctx.theme.text
    };
    ui.add_space((body * 0.5).round());
    let line = (size * 1.25).round();
    let runs = match ctx.search {
        Some(query) => crate::highlight::highlight_runs(&text, query),
        None => vec![(text, false)],
    };
    let mut job = egui::text::LayoutJob::default();
    for (run, matched) in runs {
        job.append(
            &run,
            0.0,
            TextFormat {
                font_id: mdview_theme::ReadingFace::SemiBold.font(size),
                color,
                line_height: Some(line),
                background: if matched {
                    ctx.theme.selection
                } else {
                    Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    ui.add(Label::new(job).selectable(true));
    if level <= 2 {
        ui.add_space((size * 0.3).round());
        let (rule, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(rule, 0.0, ctx.theme.border);
    }
}
