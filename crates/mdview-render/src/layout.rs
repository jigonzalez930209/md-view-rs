//! Height estimates for offscreen preview blocks.

use mdview_markdown::BlockKind;

/// Estimate the painted height of a block before it is measured.
pub(crate) fn estimate_height(kind: &BlockKind) -> f32 {
    match kind {
        BlockKind::Heading { level, .. } => 28.0 + (*level as f32) * 3.0,
        BlockKind::Paragraph(inlines) => {
            let text = mdview_markdown::plain_text(inlines);
            let lines = (text.chars().count() / 72).max(1);
            22.0 * lines as f32 + 10.0
        }
        BlockKind::Code { text, .. } => 50.0 + text.lines().count().max(1) as f32 * 17.0,
        BlockKind::Html(text) => 24.0 + text.lines().count().max(1) as f32 * 16.0,
        BlockKind::BlockQuote { blocks, .. } => {
            24.0 + blocks.iter().map(|block| estimate_height(&block.kind)).sum::<f32>()
        }
        BlockKind::List { items, .. } => {
            8.0 + items
                .iter()
                .map(|item| {
                    26.0 + item
                        .blocks
                        .iter()
                        .map(|block| estimate_height(&block.kind))
                        .sum::<f32>()
                })
                .sum::<f32>()
        }
        BlockKind::Table { rows, .. } => 50.0 + rows.len() as f32 * 30.0,
        BlockKind::Rule => 14.0,
        BlockKind::FootnoteDefinition { blocks, .. } => {
            34.0 + blocks.iter().map(|block| estimate_height(&block.kind)).sum::<f32>()
        }
    }
}
