//! Height estimates for offscreen preview blocks.

use mdview_markdown::{BlockKind, Inline};
use mdview_media::MediaKind;
use mdview_theme::ReadingTypography;

/// Estimate the painted height of a block before it is measured.
///
/// The estimate uses the reading width and typography so offscreen media-heavy
/// documents reserve a realistic amount of space. Inline images and display math
/// are the largest source of underestimation, so they get a block-sized floor.
pub(crate) fn estimate_height(
    kind: &BlockKind,
    width: f32,
    typography: ReadingTypography,
    sizes: &std::collections::HashMap<String, (f32, f32)>,
) -> f32 {
    let body = typography.body.max(10.0);
    let line_height = body * 1.5;
    match kind {
        BlockKind::Heading { level, .. } => {
            let size = crate::blocks::heading_size(*level, typography.headings);
            let rule = if *level <= 2 { size * 0.3 + 1.0 } else { 0.0 };
            body * 0.5 + size * 1.25 + rule
        }
        BlockKind::Paragraph(inlines) => {
            let text = mdview_markdown::plain_text(inlines);
            let chars_per_line = (width / (body * 0.52)).max(8.0);
            let lines = (text.chars().count() as f32 / chars_per_line).ceil().max(1.0);
            let text_height = lines * line_height + 8.0;
            let media = inline_media_height(inlines, width, typography, sizes);
            text_height.max(media)
        }
        BlockKind::Code { language, text, math } => {
            if *math {
                reserved_media_height(MediaKind::Math, text, width, typography, sizes)
            } else if language
                .as_deref()
                .is_some_and(|language| language.eq_ignore_ascii_case("mermaid"))
            {
                reserved_media_height(MediaKind::Mermaid, text, width, typography, sizes)
            } else {
                50.0 + text.lines().count().max(1) as f32 * typography.code.max(9.0) * 1.5
            }
        }
        BlockKind::Html(text) => 24.0 + text.lines().count().max(1) as f32 * line_height,
        BlockKind::BlockQuote { blocks, .. } => {
            24.0 + blocks
                .iter()
                .map(|block| estimate_height(&block.kind, width, typography, sizes))
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
                            .map(|block| estimate_height(&block.kind, width - 34.0, typography, sizes))
                            .sum::<f32>()
                })
                .sum::<f32>()
        }
        BlockKind::Table { rows, .. } => 40.0 + rows.len() as f32 * (typography.tables.max(9.0) * 2.6).max(30.0),
        BlockKind::Rule => body * 1.25,
        // Collapsed by default, like GitHub: one summary row.
        BlockKind::Details { open, blocks, .. } => {
            line_height
                + if *open {
                    blocks
                        .iter()
                        .map(|block| estimate_height(&block.kind, width, typography, sizes))
                        .sum::<f32>()
                } else {
                    0.0
                }
        }
        BlockKind::FootnoteDefinition { blocks, .. } => {
            34.0 + blocks
                .iter()
                .map(|block| estimate_height(&block.kind, width, typography, sizes))
                .sum::<f32>()
        }
    }
}

/// Space reserved for a media block before its texture is ready.
///
/// The placeholder keeps the content height stable while the background worker
/// rasterizes images, SVG, math, or Mermaid. Without it, every media block first
/// measures as a one-line spinner and then jumps to its real height.
pub(crate) fn media_reserved_height(kind: MediaKind, width: f32, typography: ReadingTypography) -> f32 {
    match kind {
        MediaKind::Image | MediaKind::Svg => (width * 0.46).clamp(96.0, 320.0),
        MediaKind::Mermaid => (width * 0.85).clamp(220.0, 560.0),
        MediaKind::Math => math_em(typography) * 2.6,
    }
}

/// Font size of a display formula in points.
///
/// The factors are measured against GitHub's MathJax output at a 14 px body: at the
/// default Math size (20) display formulas render 98 px wide for
/// `\int_0^1 x^2 dx = \frac{1}{3}` and inline `E = mc^2` 71 px, as on GitHub.
pub(crate) fn math_em(typography: ReadingTypography) -> f32 {
    typography.math * 0.66
}

/// Scale from display formula raster pixels to points.
pub(crate) fn math_scale(typography: ReadingTypography) -> f32 {
    math_em(typography) / mdview_media::MATH_UNITS_PER_EM
}

/// Scale from inline formula raster pixels to points. Inline math is a little
/// smaller than display math so its x-height sits close to the body text.
pub(crate) fn inline_math_scale(typography: ReadingTypography) -> f32 {
    typography.math * 0.63 / mdview_media::MATH_UNITS_PER_EM
}

/// Media source used for an inline formula. Inline math uses TeX text style, so
/// fractions and limits stay compact on the line, while display math keeps the
/// display style.
pub(crate) fn math_source(tex: &str, display: bool) -> std::borrow::Cow<'_, str> {
    if display {
        std::borrow::Cow::Borrowed(tex)
    } else {
        std::borrow::Cow::Owned(format!("\\textstyle {tex}"))
    }
}

/// Height of a media block using the intrinsic size when it is known.
pub(crate) fn reserved_media_height(
    kind: MediaKind,
    source: &str,
    width: f32,
    typography: ReadingTypography,
    sizes: &std::collections::HashMap<String, (f32, f32)>,
) -> f32 {
    sizes
        .get(source)
        .map(|(natural_width, natural_height)| {
            display_media_height(kind, *natural_width, *natural_height, width, typography)
        })
        .unwrap_or_else(|| media_reserved_height(kind, width, typography))
}

/// Paint height for a media block with a known intrinsic size.
pub(crate) fn display_media_height(
    kind: MediaKind,
    natural_width: f32,
    natural_height: f32,
    width: f32,
    typography: ReadingTypography,
) -> f32 {
    let natural_width = natural_width.max(1.0);
    let natural_height = natural_height.max(1.0);
    match kind {
        MediaKind::Math => {
            let scale = math_scale(typography).min(width / natural_width);
            // MathJax lays display math out at its TeX box: the raster's transparent
            // padding overlaps the surrounding margins instead of adding to them.
            ((natural_height - 2.0 * mdview_media::MATH_PADDING) * scale).max(1.0)
        }
        MediaKind::Mermaid | MediaKind::Image | MediaKind::Svg => (width / natural_width).min(1.0) * natural_height,
    }
}

fn inline_media_height(
    inlines: &[Inline],
    width: f32,
    typography: ReadingTypography,
    sizes: &std::collections::HashMap<String, (f32, f32)>,
) -> f32 {
    let mut height = 0.0_f32;
    for inline in inlines {
        match inline {
            Inline::Image { source, .. } => {
                height = height.max(reserved_media_height(
                    MediaKind::Image,
                    source,
                    width,
                    typography,
                    sizes,
                ));
            }
            Inline::Math { tex, display } if *display => {
                let source = math_source(tex, true);
                height = height.max(reserved_media_height(
                    MediaKind::Math,
                    &source,
                    width,
                    typography,
                    sizes,
                ));
            }
            Inline::Emphasis(children)
            | Inline::Strong(children)
            | Inline::Strikethrough(children)
            | Inline::Link { children, .. } => {
                height = height.max(inline_media_height(children, width, typography, sizes));
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
