//! HTML blocks rendered like GitHub's sanitized HTML.

use mdview_markdown::Inline;

use super::*;
use crate::html::{HtmlLine, LineKind, to_lines};

/// A `<details>` element with Markdown content: GitHub shows only the summary until
/// it is opened, unless the element has the `open` attribute.
pub(super) fn show_details(
    ui: &mut Ui,
    id: usize,
    summary_html: &str,
    open_attribute: bool,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let lines = to_lines(summary_html);
    let state = ui.make_persistent_id(("mdview-details", id, crate::layout::hash(summary_html)));
    let open = ui
        .ctx()
        .data(|data| data.get_temp::<bool>(state))
        .unwrap_or(open_attribute);
    let fallback = HtmlLine {
        kind: LineKind::Summary,
        align: Align::Min,
        details: 1,
        inlines: vec![Inline::Text("Details".to_owned())],
        table: None,
    };
    let summary_line = lines
        .iter()
        .find(|line| line.kind == LineKind::Summary)
        .unwrap_or(&fallback);
    if summary(ui, summary_line, open, ctx, media) {
        ui.ctx().data_mut(|data| data.insert_temp(state, !open));
    }
    if open {
        // HTML after the summary in the opening block, then the Markdown content.
        let rest: Vec<HtmlLine> = lines
            .iter()
            .filter(|line| line.kind != LineKind::Summary)
            .cloned()
            .collect();
        if !rest.is_empty() {
            ui.add_space(block_gap(ctx.typography));
            show_lines(ui, &rest, state, ctx, media);
        }
        if !blocks.is_empty() {
            ui.add_space(block_gap(ctx.typography));
            show_blocks(ui, blocks, ctx, media);
        }
    }
}

/// Paint an HTML block: aligned paragraphs and headings, images at their attribute
/// size, bullets for `<li>`, rules, and `<details>` that start closed.
pub(super) fn show_html(ui: &mut Ui, html: &str, ctx: &mut BlockCtx<'_>, media: &mut Option<&mut MediaContext<'_>>) {
    let lines = to_lines(html);
    let base = ui.make_persistent_id(("mdview-html", crate::layout::hash(html)));
    show_lines(ui, &lines, base, ctx, media);
}

fn show_lines(
    ui: &mut Ui,
    lines: &[HtmlLine],
    base: egui::Id,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    // Depth of the innermost closed `<details>`; its content stays hidden.
    let mut hidden_from: Option<usize> = None;
    let mut previous: Option<LineKind> = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(depth) = hidden_from {
            if line.details >= depth {
                continue;
            }
            hidden_from = None;
        }
        if let Some(previous) = previous {
            let tight = previous == LineKind::Item && line.kind == LineKind::Item;
            ui.add_space(if tight {
                (ctx.typography.body * 0.25).round()
            } else {
                block_gap(ctx.typography)
            });
        }
        previous = Some(line.kind);
        match line.kind {
            LineKind::Rule => {
                let body = ctx.typography.body;
                let height = (body * 0.25).round().max(2.0);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), height), egui::Sense::hover());
                ui.painter().rect_filled(rect, 0.0, ctx.theme.border);
            }
            LineKind::Heading(level) if !has_image(&line.inlines) => {
                show_heading_aligned(ui, level, &line.inlines, ctx, line.align);
            }
            LineKind::Summary => {
                let id = base.with(index);
                let open = ui.ctx().data(|data| data.get_temp::<bool>(id)).unwrap_or(false);
                if summary(ui, line, open, ctx, media) {
                    ui.ctx().data_mut(|data| data.insert_temp(id, !open));
                }
                if !open {
                    hidden_from = Some(line.details);
                }
            }
            LineKind::Item => {
                ui.horizontal_top(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    let indent = (ctx.typography.body * 2.0).round();
                    let line_height = crate::inline::line_height(ctx.typography);
                    let (marker, _) = ui.allocate_exact_size(egui::vec2(indent, line_height), egui::Sense::hover());
                    let radius = (ctx.typography.body * 0.18).max(2.2);
                    ui.painter().circle_filled(
                        egui::pos2(marker.right() - ctx.typography.body * 0.72, marker.center().y),
                        radius,
                        ctx.theme.text,
                    );
                    ui.vertical(|ui| paragraph(ui, line, ctx, media));
                });
            }
            LineKind::Paragraph | LineKind::Heading(_) => paragraph(ui, line, ctx, media),
            LineKind::Table => {
                if let Some(table) = &line.table {
                    super::table::show_table(ui, &table.alignments, &table.head, &table.rows, ctx, media);
                }
            }
        }
    }
}

fn has_image(inlines: &[Inline]) -> bool {
    inlines.iter().any(|inline| match inline {
        Inline::Image { .. } => true,
        Inline::Link { children, .. }
        | Inline::Strong(children)
        | Inline::Emphasis(children)
        | Inline::Strikethrough(children) => has_image(children),
        _ => false,
    })
}

/// Draw a `<summary>` row with GitHub's disclosure triangle; returns `true` on click.
fn summary(
    ui: &mut Ui,
    line: &HtmlLine,
    open: bool,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) -> bool {
    let body = ctx.typography.body;
    let row = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let line_height = crate::inline::line_height(ctx.typography);
        let (marker, response) = ui.allocate_exact_size(egui::vec2(body * 1.1, line_height), egui::Sense::click());
        let center = marker.center() - egui::vec2(body * 0.15, 0.0);
        let size = body * 0.3;
        let points = if open {
            vec![
                center + egui::vec2(-size, -size * 0.6),
                center + egui::vec2(size, -size * 0.6),
                center + egui::vec2(0.0, size * 0.8),
            ]
        } else {
            vec![
                center + egui::vec2(-size * 0.6, -size),
                center + egui::vec2(size * 0.8, 0.0),
                center + egui::vec2(-size * 0.6, size),
            ]
        };
        ui.painter()
            .add(egui::Shape::convex_polygon(points, ctx.theme.text, egui::Stroke::NONE));
        let text = ui.vertical(|ui| paragraph(ui, line, ctx, media)).response;
        response.clicked()
            || ui
                .interact(text.rect, text.id.with("summary"), egui::Sense::click())
                .clicked()
    });
    row.inner
}

/// An inline line. Aligned lines center (or right-align) each `<br>`-separated row.
fn paragraph(ui: &mut Ui, line: &HtmlLine, ctx: &mut BlockCtx<'_>, media: &mut Option<&mut MediaContext<'_>>) {
    if line.align == Align::Min {
        row(ui, &line.inlines, line.align, ctx, media);
        return;
    }
    for part in line.inlines.split(|inline| matches!(inline, Inline::HardBreak)) {
        ui.scope(|ui| row(ui, part, line.align, ctx, media));
    }
}

/// One row of inlines, offset for its alignment when it fits on one line.
fn row(
    ui: &mut Ui,
    inlines: &[Inline],
    align: Align,
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let style = crate::inline::InlineStyle {
        typography: ctx.typography,
        media_sizes: ctx.media_sizes,
        footnotes: &ctx.footnotes.numbers,
    };
    let available = ui.available_width();
    let offset = match align {
        Align::Min => 0.0,
        align => {
            let width = estimate_width(ui, inlines, ctx);
            if width <= available {
                let free = available - width;
                if align == Align::Center { free * 0.5 } else { free }
            } else {
                0.0
            }
        }
    };
    ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        ui.add_space(offset.floor());
        ui.vertical(|ui| {
            crate::inline::show_inlines(
                ui,
                inlines,
                ctx.theme,
                ctx.output,
                media.as_mut().map(|context| &mut **context),
                &style,
                ctx.search,
            );
        });
    });
}

/// Width of a line laid out on one row: text at the body font, images at their size.
fn estimate_width(ui: &Ui, inlines: &[Inline], ctx: &BlockCtx<'_>) -> f32 {
    let mut width: f32 = 0.0;
    for (index, inline) in inlines.iter().enumerate() {
        width += match inline {
            Inline::Image {
                source,
                width: w,
                height: h,
                ..
            } => ctx
                .media_sizes
                .get(source)
                .map(|(natural_w, natural_h)| {
                    crate::inline::image_display_size(
                        egui::vec2(*natural_w, *natural_h),
                        (*w, *h),
                        ui.available_width(),
                    )
                    .x
                })
                .or_else(|| w.map(|w| w as f32))
                .unwrap_or(0.0),
            // A `<br>` starts a new row: the line is as wide as its widest row.
            Inline::HardBreak => return width.max(estimate_width(ui, &inlines[index + 1..], ctx)),
            Inline::Link { children, .. }
            | Inline::Strong(children)
            | Inline::Emphasis(children)
            | Inline::Strikethrough(children) => estimate_width(ui, children, ctx),
            other => {
                let mut text = String::new();
                other.plain_text(&mut text);
                ui.painter()
                    .layout_no_wrap(
                        text,
                        mdview_theme::ReadingFace::Regular.font(ctx.typography.body),
                        ctx.theme.text,
                    )
                    .size()
                    .x
            }
        };
    }
    width
}
