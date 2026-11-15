//! Height estimates for offscreen preview blocks.

use mdview_markdown::{BlockKind, Inline};
use mdview_theme::ReadingTypography;

/// Estimate the painted height of a block before it is measured.
///
/// The estimate uses the reading width and typography so offscreen media-heavy
/// documents reserve a realistic amount of space. Inline images and display math
/// are the largest source of underestimation, so they get a block-sized floor.
pub(crate) fn estimate_height(kind: &BlockKind, width: f32, typography: ReadingTypography) -> f32 {
    let body = typography.body.max(10.0);
    let line_height = body * 1.5;
    match kind {
        BlockKind::Heading { level, .. } => crate::blocks::heading_size(*level, typography.headings) * 1.45 + 10.0,
        BlockKind::Paragraph(inlines) => {
            let text = mdview_markdown::plain_text(inlines);
            let chars_per_line = (width / (body * 0.52)).max(8.0);
            let lines = (text.chars().count() as f32 / chars_per_line).ceil().max(1.0);
            let text_height = lines * line_height + 8.0;
            let media = inline_media_height(inlines, width, typography);
            text_height.max(media)
        }
        BlockKind::Code { text, .. } => 50.0 + text.lines().count().max(1) as f32 * typography.code.max(9.0) * 1.5,
        BlockKind::Html(text) => 24.0 + text.lines().count().max(1) as f32 * line_height,
        BlockKind::BlockQuote { blocks, .. } => {
            24.0 + blocks
                .iter()
                .map(|block| estimate_height(&block.kind, width, typography))
                .sum::<f32>()
        }
        BlockKind::List { items, .. } => {
            8.0 + items
                .iter()
                .map(|item| {
                    line_height
                        + item
                            .blocks
                            .iter()
                            .map(|block| estimate_height(&block.kind, width - 34.0, typography))
                            .sum::<f32>()
                })
                .sum::<f32>()
        }
        BlockKind::Table { rows, .. } => 40.0 + rows.len() as f32 * (typography.tables.max(9.0) * 2.6).max(30.0),
        BlockKind::Rule => 14.0,
        BlockKind::FootnoteDefinition { blocks, .. } => {
            34.0 + blocks
                .iter()
                .map(|block| estimate_height(&block.kind, width, typography))
                .sum::<f32>()
        }
    }
}

fn inline_media_height(inlines: &[Inline], width: f32, typography: ReadingTypography) -> f32 {
    let mut height = 0.0_f32;
    for inline in inlines {
        match inline {
            Inline::Image { .. } => height = height.max((width * 0.55).clamp(120.0, 360.0)),
            Inline::Math { display, .. } if *display => height = height.max(typography.math * 2.4),
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Link { children, .. } => {
                height = height.max(inline_media_height(children, width, typography));
            }
            _ => {}
        }
    }
    height
}

/// Stable hash of Markdown source for the preview cache.
pub(crate) fn hash(text: &str) -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}

impl crate::Preview {
    /// Set the reading width preset and per-area text sizes.
    pub fn set_reading_style(
        &mut self,
        margins: mdview_theme::ReadingMargins,
        typography: mdview_theme::ReadingTypography,
    ) {
        let typography = typography.sanitized();
        if self.margins != margins || self.typography != typography {
            self.margins = margins;
            self.typography = typography;
            self.heights.clear();
            self.layout_width = 0.0;
        }
    }
}
