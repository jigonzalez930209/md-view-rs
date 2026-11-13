//! Media loading and rasterization for MD View.
//!
//! The crate is egui-free: it validates local and remote sources, decodes images, rasterizes
//! static SVG, lays out LaTeX with RaTeX, and renders Mermaid with `mermaid-rs-renderer`. Results
//! are plain RGBA buffers that the application uploads as textures. All work is bounded and
//! returns typed errors; nothing here executes scripts or loads external resources from SVG.

#![deny(missing_docs)]

mod cache;
mod image;
mod math;
mod mermaid;
mod svg;
pub mod svg_anim;

use std::path::PathBuf;

use thiserror::Error;

pub use cache::{CacheStats, MediaCache};
pub use image::load_bytes;
pub use math::render_math;
pub use mermaid::render_mermaid;
pub use svg::rasterize_svg;
pub use svg_anim::{Animation, frame_svg, parse as parse_animation};

/// The kind of media to produce.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MediaKind {
    /// A raster image (PNG, JPEG, GIF, WebP, BMP, TIFF).
    Image,
    /// A static SVG document.
    Svg,
    /// A LaTeX formula.
    Math,
    /// A Mermaid diagram.
    Mermaid,
}

impl MediaKind {
    /// A short stable label for cache keys and diagnostics.
    pub fn label(self) -> &'static str {
        match self {
            Self::Image => "image",
            Self::Svg => "svg",
            Self::Math => "math",
            Self::Mermaid => "mermaid",
        }
    }
}

/// A decoded RGBA image, ready for texture upload.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Raster {
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight RGBA bytes, row-major.
    pub rgba: Vec<u8>,
}

impl Raster {
    /// Number of bytes held by the pixel buffer.
    pub fn byte_len(&self) -> usize {
        self.rgba.len()
    }

    /// Validate that the buffer matches the dimensions.
    pub fn is_consistent(&self) -> bool {
        self.rgba.len() == self.width as usize * self.height as usize * 4
    }
}

/// Everything needed to load one media item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MediaRequest {
    /// Source as written in the document: a path, URL, TeX expression, or Mermaid source.
    pub source: String,
    /// Document directory for relative local paths.
    pub base: Option<PathBuf>,
    /// What to produce.
    pub kind: MediaKind,
    /// Maximum output width in pixels.
    pub max_width: u32,
    /// Maximum output height in pixels.
    pub max_height: u32,
    /// Whether the active theme is dark.
    pub dark: bool,
    /// Whether remote images may be fetched.
    pub allow_remote: bool,
    /// Whether private-network URLs may be fetched.
    pub allow_private_networks: bool,
    /// Optional disk cache folder for remote responses.
    pub cache_dir: Option<PathBuf>,
}

impl MediaRequest {
    /// Build a request with conservative limits.
    pub fn new(kind: MediaKind, source: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            base: None,
            kind,
            max_width: 1600,
            max_height: 1600,
            dark: false,
            allow_remote: true,
            allow_private_networks: false,
            cache_dir: None,
        }
    }

    /// A stable cache key for the request.
    pub fn cache_key(&self) -> String {
        let mut hasher = blake3::Hasher::new();
        hasher.update(self.kind.label().as_bytes());
        hasher.update(b"\0");
        hasher.update(self.source.as_bytes());
        hasher.update(b"\0");
        if let Some(base) = &self.base {
            hasher.update(base.to_string_lossy().as_bytes());
        }
        hasher.update(b"\0");
        hasher.update(&self.max_width.to_le_bytes());
        hasher.update(&self.max_height.to_le_bytes());
        hasher.update(&[u8::from(self.dark)]);
        hasher.finalize().to_hex().to_string()
    }

    /// Load and rasterize the request.
    pub fn load(&self) -> Result<Raster, MediaError> {
        match self.kind {
            MediaKind::Image | MediaKind::Svg => {
                let bytes = image::read_source(self)?;
                load_bytes(&bytes, self)
            }
            MediaKind::Math => render_math(&self.source, self.dark, self.max_width),
            MediaKind::Mermaid => render_mermaid(&self.source, self.dark, self.max_width),
        }
    }
}

/// A typed media failure with an actionable message.
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum MediaError {
    /// The request was empty.
    #[error("The media source is empty.")]
    Empty,
    /// A local path is not local or escapes the document.
    #[error("The media path is not a local file.")]
    RefusedPath,
    /// Reading the source failed.
    #[error("Could not read media: {0}")]
    Io(String),
    /// Remote fetching is disabled.
    #[error("Remote images are disabled in settings.")]
    RemoteDisabled,
    /// The URL points at a private network address.
    #[error("Private-network image URLs are blocked.")]
    PrivateNetwork,
    /// The HTTP request failed.
    #[error("Could not download media: {0}")]
    Network(String),
    /// The response or file is larger than the limit.
    #[error("Media exceeds the size limit.")]
    TooLarge,
    /// Decoding failed.
    #[error("Could not decode media: {0}")]
    Decode(String),
    /// The SVG is invalid.
    #[error("Could not render SVG: {0}")]
    Svg(String),
    /// The LaTeX expression is invalid.
    #[error("Could not render math: {0}")]
    Math(String),
    /// The Mermaid diagram is invalid or unsupported.
    #[error("Could not render Mermaid: {0}")]
    Mermaid(String),
    /// The requested output has no pixels.
    #[error("Media has no visible pixels.")]
    EmptyImage,
}

/// A loaded media item, with animation metadata when the source is animated SVG.
#[derive(Clone, Debug)]
pub struct LoadedMedia {
    /// The first frame or decoded image.
    pub raster: Raster,
    /// Animation duration in seconds.
    pub animation_duration: Option<f32>,
    /// Whether the animation uses unsupported features.
    pub animation_unsupported: bool,
    /// The SVG source retained for later frames.
    pub svg_source: Option<String>,
}

impl MediaRequest {
    /// Load a media item and detect whether an SVG is animated.
    pub fn load_with_animation(&self) -> Result<LoadedMedia, MediaError> {
        match self.kind {
            MediaKind::Image | MediaKind::Svg => {
                let bytes = image::read_source(self)?;
                if image::looks_like_svg(&bytes) {
                    let source = String::from_utf8_lossy(&bytes).into_owned();
                    if let Some(animation) = parse_animation(&source).map_err(MediaError::Svg)? {
                        let first = frame_svg(&source, &animation, 0.0);
                        let raster = rasterize_svg(first.as_bytes(), self)?;
                        return Ok(LoadedMedia {
                            raster,
                            animation_duration: Some(animation.duration),
                            animation_unsupported: animation.unsupported,
                            svg_source: Some(source),
                        });
                    }
                    let raster = load_bytes(&bytes, self)?;
                    return Ok(LoadedMedia {
                        raster,
                        animation_duration: None,
                        animation_unsupported: false,
                        svg_source: None,
                    });
                }
                let raster = load_bytes(&bytes, self)?;
                Ok(LoadedMedia {
                    raster,
                    animation_duration: None,
                    animation_unsupported: false,
                    svg_source: None,
                })
            }
            _ => self.load().map(|raster| LoadedMedia {
                raster,
                animation_duration: None,
                animation_unsupported: false,
                svg_source: None,
            }),
        }
    }
}

/// Render one frame of an animated SVG source at a given time.
pub fn render_svg_frame(source: &str, time: f32, request: &MediaRequest) -> Result<Raster, MediaError> {
    let animation = parse_animation(source)
        .map_err(MediaError::Svg)?
        .ok_or_else(|| MediaError::Svg("The SVG has no animation.".to_owned()))?;
    let frame = frame_svg(source, &animation, time);
    rasterize_svg(frame.as_bytes(), request)
}

/// Maximum bytes accepted for one image or SVG source.
pub const MAX_SOURCE_BYTES: usize = 32 * 1024 * 1024;
/// Maximum decoded image side.
pub const MAX_IMAGE_SIDE: u32 = 16_384;
/// Maximum SVG element depth handled by the security guard.
pub const MAX_SVG_DEPTH: usize = 128;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cache_keys_change_with_content_and_size() {
        let first = MediaRequest::new(MediaKind::Image, "a.png");
        let mut second = first.clone();
        second.max_width = 800;
        assert_ne!(first.cache_key(), second.cache_key());
        let third = MediaRequest::new(MediaKind::Image, "b.png");
        assert_ne!(first.cache_key(), third.cache_key());
    }

    #[test]
    fn raster_consistency_is_checked() {
        let raster = Raster {
            width: 2,
            height: 1,
            rgba: vec![0; 8],
        };
        assert!(raster.is_consistent());
        assert_eq!(raster.byte_len(), 8);
    }

    #[test]
    fn empty_requests_fail() {
        let request = MediaRequest::new(MediaKind::Image, "  ");
        assert_eq!(request.load(), Err(MediaError::Empty));
    }
}
