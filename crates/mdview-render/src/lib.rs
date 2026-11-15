//! Markdown preview rendering for MD View.
//!
//! The preview parses a document with [`mdview_markdown`], paints it with egui tokens from
//! [`mdview_theme`], and exposes a block source map for scroll synchronization. It performs no I/O:
//! media and link resolution belong to the application layer.
//!
//! ```text
//! use mdview_render::Preview;
//! let mut preview = Preview::default();
//! preview.set_width(680.0);
//! assert_eq!(preview.width(), 680.0);
//! ```
//!
//! Not a doctest: linking the media stack is expensive.

#![deny(missing_docs)]

mod blocks;
mod inline;
mod layout;
mod media;
mod source_map;

use layout::{estimate_height, hash};

use std::collections::BTreeMap;
use std::path::PathBuf;
use std::sync::Arc;

use egui::{Ui, scroll_area::ScrollAreaOutput};
use mdview_markdown::{MarkdownDocument, parse};
use mdview_syntax::Highlighter;
use mdview_theme::Theme;

pub use media::{
    AnimationStatus, LinkActivation, MediaContext, MediaLightbox, MediaProvider, MediaState, PreviewOutput,
};
pub use source_map::{BlockAnchor, PreviewAnchor, SourceMap, validate_map};

struct Cached {
    key: u64,
    document: MarkdownDocument,
}

struct BlockScreen {
    block_id: usize,
    start_line: usize,
    end_line: usize,
    top: f32,
    height: f32,
}

/// Scroll diagnostics reported by the last preview paint.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct PreviewScrollMetrics {
    /// Current vertical offset.
    pub offset: f32,
    /// Largest valid offset.
    pub max_offset: f32,
    /// Viewport height.
    pub viewport_height: f32,
    /// Total content height.
    pub content_height: f32,
    /// Bottom of the last top-level block.
    pub last_block_bottom: f32,
}

/// The Markdown preview widget and its scroll/cache state.
pub struct Preview {
    cached: Option<Cached>,
    highlighter: Arc<Highlighter>,
    scroll: f32,
    pending_scroll: Option<f32>,
    pending_line: Option<(usize, f32)>,
    pending_anchor: Option<String>,
    source_map: SourceMap,
    anchors: BTreeMap<String, f32>,
    width: f32,
    margins: mdview_theme::ReadingMargins,
    typography: mdview_theme::ReadingTypography,
    search_query: Option<String>,
    end_requested: bool,
    user_scrolled: bool,
    viewport: egui::Rect,
    heights: std::collections::HashMap<usize, f32>,
    content_height: f32,
    max_offset: f32,
    layout_width: f32,
    painted_blocks: usize,
    last_frame_time: std::time::Duration,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            cached: None,
            highlighter: Highlighter::shared(),
            scroll: 0.0,
            pending_scroll: None,
            pending_line: None,
            pending_anchor: None,
            source_map: SourceMap::default(),
            anchors: BTreeMap::new(),
            width: 760.0,
            margins: mdview_theme::ReadingMargins::default(),
            typography: mdview_theme::ReadingTypography::default(),
            search_query: None,
            end_requested: false,
            user_scrolled: false,
            viewport: egui::Rect::NOTHING,
            heights: std::collections::HashMap::new(),
            content_height: 0.0,
            max_offset: 0.0,
            layout_width: 0.0,
            painted_blocks: 0,
            last_frame_time: std::time::Duration::ZERO,
        }
    }
}

impl Preview {
    /// Build a new preview.
    pub fn new() -> Self {
        Self::default()
    }

    /// The last painted scroll offset in points.
    pub fn scroll_offset(&self) -> f32 {
        self.scroll
    }

    /// The largest valid scroll offset from the last paint.
    pub fn max_scroll_offset(&self) -> f32 {
        self.max_offset
    }

    /// Diagnostics for E2E assertions and the status bar.
    pub fn scroll_metrics(&self) -> PreviewScrollMetrics {
        PreviewScrollMetrics {
            offset: self.scroll,
            max_offset: self.max_offset,
            viewport_height: self.viewport.height(),
            content_height: self.content_height,
            last_block_bottom: self
                .source_map
                .blocks
                .last()
                .map(|block| block.top + block.height)
                .unwrap_or(0.0),
        }
    }

    /// Request a scroll offset on the next paint.
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.pending_scroll = Some(offset.max(0.0));
    }

    /// Scroll to the maximum offset on the next paint.
    pub fn scroll_to_end(&mut self) {
        self.end_requested = true;
        self.pending_scroll = Some(if self.max_offset > 0.0 {
            self.max_offset
        } else {
            f32::MAX
        });
    }

    /// Consume a completed end request from `End` or split synchronization.
    pub fn take_end_request(&mut self) -> bool {
        if self.end_requested && self.is_at_end() {
            self.end_requested = false;
            true
        } else {
            false
        }
    }

    /// Whether the preview is at the end of its scrollable range.
    pub fn is_at_end(&self) -> bool {
        self.max_offset > 0.0 && self.scroll >= self.max_offset - 2.0
    }

    /// Whether the user scrolled this frame, consuming the flag.
    pub fn take_user_scroll(&mut self) -> bool {
        std::mem::take(&mut self.user_scrolled)
    }

    /// The preview viewport rectangle from the last paint.
    pub fn viewport_rect(&self) -> egui::Rect {
        self.viewport
    }

    /// Scroll to a zero-based source line, preserving an intra-line fraction.
    pub fn scroll_to_line(&mut self, line: usize, fraction: f32) {
        if let Some(offset) = self.source_map.offset_for_line(line, fraction) {
            self.pending_scroll = Some(offset);
        } else {
            self.pending_line = Some((line, fraction));
        }
    }

    /// Scroll to a heading or footnote anchor.
    pub fn scroll_to_anchor(&mut self, anchor: &str) -> bool {
        self.pending_anchor = Some(anchor.to_owned());
        if let Some(offset) = self.anchors.get(anchor).copied() {
            self.pending_scroll = Some(offset);
            self.pending_anchor = None;
            true
        } else {
            false
        }
    }

    /// The heading and footnote anchors from the last paint.
    pub fn anchors(&self) -> &BTreeMap<String, f32> {
        &self.anchors
    }

    /// The block source map from the last paint.
    pub fn source_map(&self) -> &SourceMap {
        &self.source_map
    }

    /// Number of blocks painted in the last frame, for virtualization tests.
    pub fn painted_blocks(&self) -> usize {
        self.painted_blocks
    }

    /// CPU time spent parsing, laying out, and painting the last frame.
    pub fn last_frame_time(&self) -> std::time::Duration {
        self.last_frame_time
    }

    /// The anchor at the current scroll offset.
    pub fn anchor(&self) -> Option<PreviewAnchor> {
        PreviewAnchor::from_offset(&self.source_map, self.scroll)
    }

    /// Restore a previously captured anchor after a re-render or resize.
    pub fn restore_anchor(&mut self, anchor: PreviewAnchor) {
        if let Some(offset) = anchor.offset(&self.source_map) {
            self.pending_scroll = Some(offset);
        }
    }

    /// The parsed document from the last paint, if any.
    pub fn document(&self) -> Option<&MarkdownDocument> {
        self.cached.as_ref().map(|cached| &cached.document)
    }

    /// Set the maximum reading width in points.
    pub fn set_width(&mut self, width: f32) {
        self.width = width.max(280.0);
    }

    /// The maximum reading width in points.
    pub fn width(&self) -> f32 {
        self.width
    }

    /// Highlight a literal query in the reading view.
    pub fn set_search_query(&mut self, query: Option<String>) {
        let query = query.filter(|query| !query.trim().is_empty());
        if self.search_query != query {
            self.search_query = query;
        }
    }

    /// Paint the preview and report link and copy interactions.
    pub fn show(&mut self, ui: &mut Ui, source: &str, theme: &Theme, light: bool) -> PreviewOutput {
        let started = std::time::Instant::now();
        let output = self.show_internal(ui, source, theme, light, None);
        self.last_frame_time = started.elapsed();
        output
    }

    /// Paint the preview and resolve media through the application provider.
    pub fn show_with_media(
        &mut self,
        ui: &mut Ui,
        source: &str,
        theme: &Theme,
        light: bool,
        base: Option<PathBuf>,
        provider: &mut dyn MediaProvider,
    ) -> PreviewOutput {
        let started = std::time::Instant::now();
        let mut media = MediaContext { base, provider };
        let output = self.show_internal(ui, source, theme, light, Some(&mut media));
        self.last_frame_time = started.elapsed();
        output
    }

    fn show_internal(
        &mut self,
        ui: &mut Ui,
        source: &str,
        theme: &Theme,
        light: bool,
        media: Option<&mut MediaContext<'_>>,
    ) -> PreviewOutput {
        let key = hash(source);
        if self.cached.as_ref().map(|cached| cached.key) != Some(key) {
            if let Some((line, fraction)) = self.source_map.line_at_offset(self.scroll).or(self.pending_line) {
                self.pending_line = Some((line, fraction));
            }
            self.cached = Some(Cached {
                key,
                document: parse(source),
            });
            self.source_map = SourceMap::default();
            self.anchors.clear();
            self.heights.clear();
        }

        let outer_width = ui.available_width();
        if (outer_width - self.layout_width).abs() > 8.0 {
            self.heights.clear();
            self.layout_width = outer_width;
        }
        let mut output = PreviewOutput::default();
        let mut block_screens: Vec<BlockScreen> = Vec::new();
        let mut media = media;
        let search_query = self.search_query.clone();
        let mut heights = std::mem::take(&mut self.heights);
        let margin_fraction = self.margins.percent() / 100.0;
        let (top_gap, bottom_gap, block_gap) = (8.0, 12.0, 6.0);

        let anchor_before = self.anchor();
        let old_content_height = self.content_height;
        let old_max_offset = self.max_offset;
        let was_at_end = old_max_offset > 0.0 && self.scroll >= old_max_offset - 64.0;
        let mut area = egui::ScrollArea::vertical()
            .id_salt("mdview-markdown-preview")
            .animated(false)
            .auto_shrink([false, false]);
        let programmatic = self.pending_scroll.is_some();
        if let Some(offset) = self.pending_scroll.take() {
            area = area.vertical_scroll_offset(offset);
        }

        let area_output: ScrollAreaOutput<()> = area.show_viewport(ui, |ui, viewport| {
            ui.set_width(ui.available_width());
            ui.add_space(top_gap);
            let available = ui.available_width();
            let width = (available * (1.0 - 2.0 * margin_fraction))
                .max(240.0)
                .min((available - 24.0).max(64.0));
            let margin = ((available - width) * 0.5).max(4.0);
            ui.horizontal_top(|ui| {
                ui.add_space(margin);
                ui.vertical(|ui| {
                    ui.set_width(width);
                    ui.spacing_mut().item_spacing.y = 0.0;
                    let Some(cached) = &self.cached else {
                        return;
                    };
                    let mut anchor_points = Vec::new();
                    let mut painted = 0usize;
                    {
                        let mut ctx = blocks::BlockCtx {
                            theme,
                            light,
                            highlighter: &self.highlighter,
                            output: &mut output,
                            typography: self.typography,
                            search: search_query.as_deref(),
                        };
                        let overscan = 320.0;
                        let mut top = 0.0_f32;
                        for block in &cached.document.blocks {
                            let estimated = estimate_height(&block.kind, width, self.typography);
                            let height = heights.get(&block.id).copied().unwrap_or(estimated);
                            let visible = top + height >= viewport.min.y - overscan && top <= viewport.max.y + overscan;
                            if visible {
                                painted += 1;
                                let response = ui
                                    .scope(|ui| blocks::show_block(ui, block, &mut ctx, &mut media))
                                    .response;
                                let measured = response.rect.height().max(1.0);
                                heights.insert(block.id, measured);
                                block_screens.push(BlockScreen {
                                    block_id: block.id,
                                    start_line: block.span.start_line,
                                    end_line: block.span.end_line,
                                    top,
                                    height: measured,
                                });
                                ui.add_space(block_gap);
                                top += measured + block_gap;
                            } else {
                                ui.add_space(height + block_gap);
                                block_screens.push(BlockScreen {
                                    block_id: block.id,
                                    start_line: block.span.start_line,
                                    end_line: block.span.end_line,
                                    top,
                                    height,
                                });
                                top += height + block_gap;
                            }
                            match &block.kind {
                                mdview_markdown::BlockKind::Heading { anchor, .. } => {
                                    anchor_points.push((anchor.clone(), top - height - 6.0));
                                }
                                mdview_markdown::BlockKind::FootnoteDefinition { label, .. } => {
                                    anchor_points.push((format!("fn-{label}"), top - height - 6.0));
                                }
                                _ => {}
                            }
                        }
                    }
                    output.anchors_screen.extend(anchor_points);
                    self.painted_blocks = painted;
                    ui.add_space(bottom_gap);
                });
            });
        });
        self.heights = heights;

        let scroll = area_output.state.clamped_offset().y;
        let mut mapped = Vec::with_capacity(block_screens.len());
        let mut content_height: f32 = 0.0;
        for block in &block_screens {
            content_height = content_height.max(block.top + block.height);
            mapped.push(BlockAnchor {
                block_id: block.block_id,
                start_line: block.start_line,
                end_line: block.end_line,
                top: block.top,
                height: block.height.max(1.0),
            });
        }
        self.source_map = SourceMap::from_anchors(mapped, content_height + bottom_gap);
        let max_offset = (area_output.content_size.y - area_output.inner_rect.height()).max(0.0);
        self.content_height = self.source_map.content_height;
        self.max_offset = max_offset;

        let mut anchor_map = BTreeMap::new();
        for (anchor, y) in &output.anchors_screen {
            anchor_map.insert(anchor.clone(), *y);
        }
        self.anchors = anchor_map;
        if !programmatic && (scroll - self.scroll).abs() > 0.5 {
            self.user_scrolled = true;
        }
        self.scroll = scroll;
        self.viewport = area_output.inner_rect;

        // If the reader was at the end when an image, diagram, or table finished
        // measuring, stay pinned to the new end instead of stopping short.
        if !programmatic
            && ((self.scroll >= max_offset - 64.0 && self.scroll < max_offset - 0.5)
                || (was_at_end && max_offset > old_max_offset + 0.5))
        {
            self.pending_scroll = Some(max_offset);
            ui.ctx().request_repaint();
        } else if !programmatic
            && (self.source_map.content_height - old_content_height).abs() > 0.5
            && let Some(anchor) = anchor_before
            && let Some(offset) = anchor.offset(&self.source_map)
            && (offset - self.scroll).abs() > 0.5
        {
            self.pending_scroll = Some(offset);
            ui.ctx().request_repaint();
        }
        if let Some((line, fraction)) = self.pending_line.take()
            && let Some(offset) = self.source_map.offset_for_line(line, fraction)
        {
            self.pending_scroll = Some(offset);
            ui.ctx().request_repaint();
        }
        if let Some(anchor) = &output.anchor {
            self.scroll_to_anchor(anchor);
        }
        if let Some(anchor) = self.pending_anchor.clone()
            && let Some(offset) = self.anchors.get(&anchor).copied()
        {
            self.pending_scroll = Some(offset);
            self.pending_anchor = None;
            ui.ctx().request_repaint();
        }

        output
    }
}
