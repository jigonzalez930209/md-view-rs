//! Block source mapping for scroll synchronization and anchor preservation.

use mdview_markdown::MarkdownDocument;

/// Where one top-level block was painted in the preview content.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct BlockAnchor {
    /// Parser block identifier.
    pub block_id: usize,
    /// One-based first source line.
    pub start_line: usize,
    /// One-based last source line.
    pub end_line: usize,
    /// Content-space top in points.
    pub top: f32,
    /// Painted height in points.
    pub height: f32,
}

impl BlockAnchor {
    /// Height used for interpolation. Zero-height blocks fall back to the line height.
    pub fn height(&self) -> f32 {
        self.height.max(1.0)
    }
}

/// The top-level block layout produced by the last paint.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct SourceMap {
    /// Blocks in document order.
    pub blocks: Vec<BlockAnchor>,
    /// Total painted content height.
    pub content_height: f32,
}

impl SourceMap {
    /// Build the map from ordered anchors.
    pub fn from_anchors(blocks: Vec<BlockAnchor>, content_height: f32) -> Self {
        Self { blocks, content_height }
    }

    /// Find the anchor covering a zero-based source line.
    ///
    /// Blocks without an exact match use the nearest preceding block, so footnotes rendered at the
    /// end and gaps between blocks still map to a stable location.
    pub fn for_line(&self, line: usize) -> Option<&BlockAnchor> {
        let line = line + 1;
        if let Some(anchor) = self
            .blocks
            .iter()
            .find(|anchor| (anchor.start_line..=anchor.end_line).contains(&line))
        {
            return Some(anchor);
        }
        self.blocks
            .iter()
            .take_while(|anchor| anchor.start_line <= line)
            .last()
            .or_else(|| self.blocks.first())
    }

    /// The content-space offset for a zero-based source line and an intra-line fraction.
    pub fn offset_for_line(&self, line: usize, fraction: f32) -> Option<f32> {
        let anchor = self.for_line(line)?;
        let fraction = fraction.clamp(0.0, 1.0);
        Some(anchor.top + anchor.height() * fraction)
    }

    /// The anchor intersecting a content-space offset.
    pub fn anchor_at_offset(&self, offset: f32) -> Option<&BlockAnchor> {
        if self.blocks.is_empty() {
            return None;
        }
        self.blocks
            .iter()
            .find(|anchor| offset >= anchor.top && offset < anchor.top + anchor.height())
            .or_else(|| self.blocks.last())
    }

    /// Convert a content-space offset to a zero-based source line and fraction.
    pub fn line_at_offset(&self, offset: f32) -> Option<(usize, f32)> {
        let anchor = self.anchor_at_offset(offset)?;
        let fraction = ((offset - anchor.top) / anchor.height()).clamp(0.0, 1.0);
        let span = anchor.end_line.saturating_sub(anchor.start_line);
        let line = anchor.start_line + (fraction * span as f32).round() as usize;
        Some((line.saturating_sub(1), fraction))
    }

    /// The content-space top of a block.
    pub fn block_top(&self, block_id: usize) -> Option<f32> {
        self.blocks
            .iter()
            .find(|anchor| anchor.block_id == block_id)
            .map(|anchor| anchor.top)
    }

    /// An anchor that survives a re-render at a different height.
    pub fn preserve(&self, block_id: usize, fraction: f32) -> Option<f32> {
        self.blocks
            .iter()
            .find(|anchor| anchor.block_id == block_id)
            .map(|anchor| anchor.top + anchor.height() * fraction.clamp(0.0, 1.0))
    }
}

/// The preview scroll anchor.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviewAnchor {
    /// Block identifier.
    pub block_id: usize,
    /// Fraction inside the block, 0.0–1.0.
    pub fraction: f32,
}

impl PreviewAnchor {
    /// Derive an anchor from a source map and scroll offset.
    pub fn from_offset(map: &SourceMap, offset: f32) -> Option<Self> {
        let anchor = map.anchor_at_offset(offset)?;
        let fraction = ((offset - anchor.top) / anchor.height()).clamp(0.0, 1.0);
        Some(Self {
            block_id: anchor.block_id,
            fraction,
        })
    }

    /// The offset of this anchor in a (possibly re-rendered) source map.
    pub fn offset(&self, map: &SourceMap) -> Option<f32> {
        map.preserve(self.block_id, self.fraction)
    }
}

/// Validate that a parsed document and a source map agree on block order.
pub fn validate_map(document: &MarkdownDocument, map: &SourceMap) -> bool {
    if document.blocks.is_empty() {
        return map.blocks.is_empty();
    }
    if document.blocks.len() != map.blocks.len() {
        return false;
    }
    document
        .blocks
        .iter()
        .zip(&map.blocks)
        .all(|(block, anchor)| block.id == anchor.block_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map() -> SourceMap {
        SourceMap::from_anchors(
            vec![
                BlockAnchor {
                    block_id: 0,
                    start_line: 1,
                    end_line: 2,
                    top: 0.0,
                    height: 40.0,
                },
                BlockAnchor {
                    block_id: 1,
                    start_line: 3,
                    end_line: 4,
                    top: 40.0,
                    height: 100.0,
                },
                BlockAnchor {
                    block_id: 2,
                    start_line: 5,
                    end_line: 5,
                    top: 140.0,
                    height: 20.0,
                },
            ],
            160.0,
        )
    }

    #[test]
    fn line_offsets_round_trip() {
        let map = map();
        let offset = map.offset_for_line(2, 0.5).expect("offset");
        assert!((offset - 90.0).abs() < 0.01);
        let (line, fraction) = map.line_at_offset(offset).expect("line");
        assert_eq!(line, 3);
        assert!((fraction - 0.5).abs() < 0.01);
    }

    #[test]
    fn anchors_preserve_position_across_rerender() {
        let map = map();
        let anchor = PreviewAnchor::from_offset(&map, 60.0).expect("anchor");
        assert_eq!(anchor.block_id, 1);
        let reflowed = SourceMap::from_anchors(
            vec![
                BlockAnchor {
                    block_id: 0,
                    start_line: 1,
                    end_line: 2,
                    top: 0.0,
                    height: 30.0,
                },
                BlockAnchor {
                    block_id: 1,
                    start_line: 3,
                    end_line: 4,
                    top: 30.0,
                    height: 200.0,
                },
                BlockAnchor {
                    block_id: 2,
                    start_line: 5,
                    end_line: 5,
                    top: 230.0,
                    height: 20.0,
                },
            ],
            260.0,
        );
        let offset = anchor.offset(&reflowed).expect("offset");
        assert!((offset - 70.0).abs() < 0.01);
    }

    #[test]
    fn gaps_and_trailing_lines_use_the_nearest_block() {
        let map = map();
        assert_eq!(map.for_line(10).map(|anchor| anchor.block_id), Some(2));
        assert_eq!(map.for_line(0).map(|anchor| anchor.block_id), Some(0));
    }
}
