//! Media, link, and preview-output contracts.

use std::path::PathBuf;

use mdview_markdown::LinkKind;
use mdview_media::{MediaKind, MediaRequest};

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

/// The playback state of an animated item, shown as a badge.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AnimationStatus {
    /// Not animated.
    None,
    /// Playing.
    Playing,
    /// Paused by the user, reduced motion, or visibility.
    Paused,
    /// The animation uses unsupported features; the first frame is shown.
    Unsupported,
}

/// A media resolver supplied by the application.
pub trait MediaProvider {
    /// Resolve one media request. The application may return `Loading` and complete later.
    fn resolve(&mut self, ctx: &egui::Context, request: &MediaRequest) -> MediaState;

    /// Report whether a resolved item is visible in the viewport.
    fn mark_visible(&mut self, _key: &str, _visible: bool) {}

    /// The animation badge state for a cache key.
    fn animation_status(&mut self, _key: &str) -> AnimationStatus {
        AnimationStatus::None
    }
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

/// A formula the user asked to edit from the preview.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathActivation {
    /// The formula's TeX, as the preview received it.
    pub tex: String,
    /// Byte range of the top-level block that holds the formula in the source.
    pub block: Option<std::ops::Range<usize>>,
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
    /// A formula the user double-clicked or chose to edit.
    pub math: Option<MathActivation>,
    /// Translated label of the formula context menu, copied in before painting.
    pub(crate) math_edit_label: String,
    pub(crate) anchors_screen: Vec<(String, f32)>,
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

/// Report `response` as an edit request for `tex` on a double-click or the context menu.
pub(crate) fn math_edit_request(response: &egui::Response, tex: &str, output: &mut PreviewOutput) {
    let mut edit = response.double_clicked();
    response.context_menu(|ui| {
        if ui.button(&output.math_edit_label).clicked() {
            edit = true;
            ui.close();
        }
    });
    if edit {
        output.math = Some(MathActivation {
            tex: tex.to_owned(),
            block: None,
        });
    }
}
