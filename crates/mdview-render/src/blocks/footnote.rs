//! GitHub-style footnotes: numbered references and a numbered, muted note list.

use std::collections::HashMap;

use mdview_markdown::Inline;

use super::*;

/// Footnote numbers in order of first reference, as GitHub numbers them. Definitions
/// that are never referenced follow in document order.
pub(crate) fn footnote_numbers(blocks: &[Block]) -> HashMap<String, usize> {
    let mut numbers = HashMap::new();
    for block in blocks {
        collect_block_references(block, &mut numbers);
    }
    for block in blocks {
        if let BlockKind::FootnoteDefinition { label, .. } = &block.kind {
            let next = numbers.len() + 1;
            numbers.entry(label.clone()).or_insert(next);
        }
    }
    numbers
}

/// Anchor name a block registers for in-document navigation.
pub(crate) fn block_anchor(kind: &BlockKind) -> Option<String> {
    match kind {
        BlockKind::Heading { anchor, .. } => Some(anchor.clone()),
        BlockKind::FootnoteDefinition { label, .. } => Some(format!("fn-{label}")),
        _ => None,
    }
}

fn collect_block_references(block: &Block, numbers: &mut HashMap<String, usize>) {
    match &block.kind {
        BlockKind::Paragraph(inlines) | BlockKind::Heading { inlines, .. } => collect_inlines(inlines, numbers),
        BlockKind::BlockQuote { blocks, .. } | BlockKind::FootnoteDefinition { blocks, .. } => {
            for block in blocks {
                collect_block_references(block, numbers);
            }
        }
        BlockKind::List { items, .. } => {
            for block in items.iter().flat_map(|item| &item.blocks) {
                collect_block_references(block, numbers);
            }
        }
        BlockKind::Table { head, rows, .. } => {
            for cell in head.iter().chain(rows.iter().flatten()) {
                collect_inlines(cell, numbers);
            }
        }
        BlockKind::Code { .. } | BlockKind::Html(_) | BlockKind::Rule => {}
    }
}

fn collect_inlines(inlines: &[Inline], numbers: &mut HashMap<String, usize>) {
    for inline in inlines {
        match inline {
            Inline::FootnoteReference(label) => {
                let next = numbers.len() + 1;
                numbers.entry(label.clone()).or_insert(next);
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Link { children, .. } => collect_inlines(children, numbers),
            _ => {}
        }
    }
}

/// A footnote definition laid out like GitHub's footnote list: an ordered marker in
/// a 2em gutter, 12/14 of the body size, muted text. The marker links back to the
/// reference.
pub(super) fn show_footnote(
    ui: &mut Ui,
    label: &str,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let size = (ctx.typography.body * 0.86).round();
    let indent = (ctx.typography.body * 2.0).round();
    let number = ctx
        .footnotes
        .get(label)
        .map_or_else(|| label.to_owned(), usize::to_string);
    // The first note opens GitHub's footnote section with a top border.
    if ctx.footnotes.get(label) == Some(&1) {
        let (rule, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 1.0), egui::Sense::hover());
        ui.painter().rect_filled(rule, 0.0, ctx.theme.border);
        ui.add_space(block_gap(ctx.typography));
    }
    let mut first_line = None;
    let row = ui.horizontal_top(|ui| {
        ui.spacing_mut().item_spacing.x = 0.0;
        let (marker, response) = ui.allocate_exact_size(egui::vec2(indent, size * 1.5), egui::Sense::click());
        ui.vertical(|ui| {
            let mut theme = *ctx.theme;
            theme.text = ctx.theme.text_muted;
            let mut nested = ctx.reborrow();
            nested.theme = &theme;
            nested.typography.body = size;
            nested.list.capture_first_line = true;
            if let Some((first, rest)) = blocks.split_first() {
                show_block(ui, first, &mut nested, media);
                first_line = nested.list.first_line.take();
                nested.list.capture_first_line = false;
                for block in rest {
                    ui.add_space(block_gap(nested.typography));
                    show_block(ui, block, &mut nested, media);
                }
            }
        });
        (marker, response)
    });
    let (marker, response) = row.inner;
    let center = first_line.unwrap_or(marker.center().y);
    ui.painter().text(
        egui::pos2(marker.right() - size * 0.3, center),
        Align2::RIGHT_CENTER,
        format!("{number}."),
        mdview_theme::ReadingFace::Regular.font(size),
        ctx.theme.text_muted,
    );
    if response
        .on_hover_text("Back to reference")
        .on_hover_cursor(egui::CursorIcon::PointingHand)
        .clicked()
    {
        ctx.output.anchor = Some(format!("fnref-{label}"));
    }
}
