//! GitHub-style footnotes: numbered references and a numbered, muted note list.

use std::collections::HashMap;

use mdview_markdown::Inline;

use super::*;

/// Footnote numbering and the top-level blocks that reference each note.
#[derive(Debug, Default)]
pub(crate) struct Footnotes {
    /// Number by label, in order of first reference, as GitHub numbers them.
    /// Definitions that are never referenced follow in document order.
    pub numbers: HashMap<String, usize>,
    /// Labels first referenced by each top-level block, keyed by block id. The
    /// block registers `fnref-<label>` so a note's back link can return to it.
    references: HashMap<usize, Vec<String>>,
}

pub(crate) fn footnote_numbers(blocks: &[Block]) -> Footnotes {
    let mut footnotes = Footnotes::default();
    for block in blocks {
        let before = footnotes.numbers.len();
        collect_block_references(block, &mut footnotes.numbers);
        if footnotes.numbers.len() > before {
            let mut labels: Vec<(usize, String)> = footnotes
                .numbers
                .iter()
                .filter(|(_, number)| **number > before)
                .map(|(label, number)| (*number, label.clone()))
                .collect();
            labels.sort();
            footnotes
                .references
                .insert(block.id, labels.into_iter().map(|(_, label)| label).collect());
        }
    }
    for block in blocks {
        if let BlockKind::FootnoteDefinition { label, .. } = &block.kind {
            let next = footnotes.numbers.len() + 1;
            footnotes.numbers.entry(label.clone()).or_insert(next);
        }
    }
    footnotes
}

/// Anchor names a top-level block registers for in-document navigation.
pub(crate) fn block_anchors(block: &Block, footnotes: &Footnotes) -> Vec<String> {
    let mut anchors = match &block.kind {
        BlockKind::Heading { anchor, .. } => vec![anchor.clone()],
        BlockKind::FootnoteDefinition { label, .. } => vec![format!("fn-{label}")],
        _ => Vec::new(),
    };
    if let Some(labels) = footnotes.references.get(&block.id) {
        anchors.extend(labels.iter().map(|label| format!("fnref-{label}")));
    }
    anchors
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
        .numbers
        .get(label)
        .map_or_else(|| label.to_owned(), usize::to_string);
    // The first note opens GitHub's footnote section with a top border.
    if ctx.footnotes.numbers.get(label) == Some(&1) {
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
            // GitHub ends the note's last paragraph with a `↩` link to the reference.
            let mut blocks = blocks.to_vec();
            if let Some(Block {
                kind: BlockKind::Paragraph(inlines),
                ..
            }) = blocks.last_mut()
            {
                inlines.push(Inline::Text(" ".to_owned()));
                inlines.push(Inline::Link {
                    destination: format!("#fnref-{label}"),
                    title: None,
                    children: vec![Inline::Text("\u{21A9}".to_owned())],
                    kind: mdview_markdown::LinkKind::Anchor,
                });
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn footnotes_number_by_first_reference_and_anchor_their_blocks() {
        let document = mdview_markdown::parse(
            "Second[^b] then first[^a].\n\nAgain[^b].\n\n[^a]: A.\n\n[^b]: B.\n\n[^c]: Unused.\n",
        );
        let footnotes = footnote_numbers(&document.blocks);
        assert_eq!(footnotes.numbers.get("b"), Some(&1));
        assert_eq!(footnotes.numbers.get("a"), Some(&2));
        assert_eq!(footnotes.numbers.get("c"), Some(&3));
        let anchors = block_anchors(&document.blocks[0], &footnotes);
        assert_eq!(anchors, vec!["fnref-b".to_owned(), "fnref-a".to_owned()]);
        assert!(block_anchors(&document.blocks[1], &footnotes).is_empty());
    }
}
