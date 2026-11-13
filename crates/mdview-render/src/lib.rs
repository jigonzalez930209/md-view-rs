//! Markdown preview rendering for MD View.
//!
//! The preview parses a document with [`mdview_markdown`], paints it with egui tokens from
//! [`mdview_theme`], and exposes a block source map for scroll synchronization. It performs no I/O:
//! media and link resolution belong to the application layer.
//!
//! ```
//! use mdview_render::Preview;
//!
//! let mut preview = Preview::default();
//! preview.set_width(680.0);
//! assert_eq!(preview.width(), 680.0);
//! ```

#![deny(missing_docs)]

mod blocks;
mod inline;
mod source_map;

use std::collections::{BTreeMap, hash_map::DefaultHasher};
use std::hash::{Hash, Hasher};
use std::path::PathBuf;

use egui::{Ui, scroll_area::ScrollAreaOutput};
use mdview_markdown::{LinkKind, MarkdownDocument, parse};
use mdview_media::{MediaKind, MediaRequest};
use mdview_syntax::Highlighter;
use mdview_theme::Theme;

pub use source_map::{BlockAnchor, PreviewAnchor, SourceMap, validate_map};

/// A link the user activated in the preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LinkActivation {
    /// Destination exactly as written in Markdown.
    pub destination: String,
    /// Classification produced by the parser.
    pub kind: LinkKind,
}

/// The state of one media item requested by the preview.
#[derive(Clone)]
pub enum MediaState {
    /// The item is being loaded.
    Loading,
    /// The item is ready as a texture.
    Ready(egui::TextureHandle),
    /// The item could not be produced.
    Error(String),
}

/// A media resolver supplied by the application.
pub trait MediaProvider {
    /// Resolve one media request. The application may return `Loading` and complete later.
    fn resolve(&mut self, ctx: &egui::Context, request: &MediaRequest) -> MediaState;
}

/// Per-paint media inputs.
pub struct MediaContext<'a> {
    /// Directory of the current document for relative paths.
    pub base: Option<PathBuf>,
    /// Application resolver.
    pub provider: &'a mut dyn MediaProvider,
}

impl MediaContext<'_> {
    /// Build a request with the preview's limits.
    pub fn request(
        &self,
        ctx: &egui::Context,
        kind: MediaKind,
        source: &str,
        max_width: f32,
        dark: bool,
    ) -> MediaRequest {
        let pixels_per_point = ctx.pixels_per_point().max(1.0);
        let mut request = MediaRequest::new(kind, source);
        request.base.clone_from(&self.base);
        request.max_width = (max_width * pixels_per_point).clamp(32.0, 4096.0) as u32;
        request.max_height = 4096;
        request.dark = dark;
        request
    }

    /// Resolve a media item with the preview's limits.
    pub fn load(
        &mut self,
        ctx: &egui::Context,
        kind: MediaKind,
        source: &str,
        max_width: f32,
        dark: bool,
    ) -> MediaState {
        let request = self.request(ctx, kind, source, max_width, dark);
        self.provider.resolve(ctx, &request)
    }
}

/// A request to open the image lightbox.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaLightbox {
    /// The resolved request for the image.
    pub request: MediaRequest,
    /// Alternative text shown in the window title and placeholder.
    pub alt: String,
}

/// What the preview reported during one paint.
#[derive(Clone, Debug, Default)]
pub struct PreviewOutput {
    /// A link the user clicked.
    pub link: Option<LinkActivation>,
    /// An in-document anchor requested from a footnote or backlink.
    pub anchor: Option<String>,
    /// Whether the user copied a code block.
    pub copied: bool,
    /// An image the user asked to inspect.
    pub lightbox: Option<MediaLightbox>,
    pub(crate) anchors_screen: Vec<(String, f32)>,
}

struct Cached {
    key: u64,
    document: MarkdownDocument,
}

struct BlockScreen {
    block_id: usize,
    start_line: usize,
    end_line: usize,
    top: f32,
    bottom: f32,
}

/// The Markdown preview widget and its scroll/cache state.
pub struct Preview {
    cached: Option<Cached>,
    highlighter: Highlighter,
    scroll: f32,
    pending_scroll: Option<f32>,
    pending_line: Option<(usize, f32)>,
    pending_anchor: Option<String>,
    source_map: SourceMap,
    anchors: BTreeMap<String, f32>,
    width: f32,
    user_scrolled: bool,
    viewport: egui::Rect,
}

impl Default for Preview {
    fn default() -> Self {
        Self {
            cached: None,
            highlighter: Highlighter::new(),
            scroll: 0.0,
            pending_scroll: None,
            pending_line: None,
            pending_anchor: None,
            source_map: SourceMap::default(),
            anchors: BTreeMap::new(),
            width: 820.0,
            user_scrolled: false,
            viewport: egui::Rect::NOTHING,
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

    /// Request a scroll offset on the next paint.
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.pending_scroll = Some(offset.max(0.0));
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

    /// Paint the preview and report link and copy interactions.
    pub fn show(&mut self, ui: &mut Ui, source: &str, theme: &Theme, light: bool) -> PreviewOutput {
        self.show_internal(ui, source, theme, light, None)
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
        let mut media = MediaContext { base, provider };
        self.show_internal(ui, source, theme, light, Some(&mut media))
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
        }

        let mut output = PreviewOutput::default();
        let mut block_screens = Vec::new();
        let mut media = media;

        let mut area = egui::ScrollArea::vertical()
            .id_salt("mdview-markdown-preview")
            .auto_shrink([false, false]);
        let programmatic = self.pending_scroll.is_some();
        if let Some(offset) = self.pending_scroll.take() {
            area = area.vertical_scroll_offset(offset);
        }

        let area_output: ScrollAreaOutput<()> = area.show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_space(8.0);
            ui.scope(|ui| {
                ui.set_max_width(self.width.min(ui.available_width()));
                let Some(cached) = &self.cached else {
                    return;
                };
                let mut ctx = blocks::BlockCtx {
                    theme,
                    light,
                    highlighter: &self.highlighter,
                    output: &mut output,
                };
                for block in &cached.document.blocks {
                    let response = ui
                        .scope(|ui| blocks::show_block(ui, block, &mut ctx, &mut media))
                        .response;
                    block_screens.push(BlockScreen {
                        block_id: block.id,
                        start_line: block.span.start_line,
                        end_line: block.span.end_line,
                        top: response.rect.top(),
                        bottom: response.rect.bottom(),
                    });
                    ui.add_space(6.0);
                }
            });
            ui.add_space(12.0);
        });

        let scroll = area_output.state.clamped_offset().y;
        let content_origin = area_output.inner_rect.top() - scroll;
        let mut mapped = Vec::with_capacity(block_screens.len());
        let mut content_height: f32 = 0.0;
        for block in &block_screens {
            let top = block.top - content_origin;
            let bottom = block.bottom - content_origin;
            content_height = content_height.max(bottom);
            mapped.push(BlockAnchor {
                block_id: block.block_id,
                start_line: block.start_line,
                end_line: block.end_line,
                top,
                height: (bottom - top).max(1.0),
            });
        }
        self.source_map = SourceMap::from_anchors(mapped, content_height);

        let mut anchor_map = BTreeMap::new();
        for (anchor, screen_y) in &output.anchors_screen {
            anchor_map.insert(anchor.clone(), (screen_y - content_origin).max(0.0));
        }
        self.anchors = anchor_map;
        if !programmatic && (scroll - self.scroll).abs() > 0.5 {
            self.user_scrolled = true;
        }
        self.scroll = scroll;
        self.viewport = area_output.inner_rect;

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

fn hash(text: &str) -> u64 {
    let mut hasher = DefaultHasher::new();
    text.hash(&mut hasher);
    hasher.finish()
}
