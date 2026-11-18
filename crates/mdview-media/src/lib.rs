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
mod math_source;
mod mermaid;
mod pdf;
mod raster_anim;
mod svg;
pub mod svg_anim;

use std::path::{Path, PathBuf};

use thiserror::Error;

pub use cache::{CacheStats, MediaCache};
pub use image::load_bytes;
pub use math::{MATH_PADDING, MATH_UNITS_PER_EM, math_baseline, render_math, render_math_at};
pub use math_source::{MathHighlight, highlight_math_at, math_spans_at};
pub use mermaid::render_mermaid;
pub use pdf::{MAX_PAGE_PIXELS, PdfDocument};
pub use raster_anim::{MAX_ANIMATION_BYTES, MAX_FRAMES, RasterAnimation, decode_animation};
pub use svg::rasterize_svg;
pub use svg_anim::{Animation, frame_svg, parse as parse_animation};

/// Image extensions that the application can open as an image document.
///
/// Raster formats decode with pure-Rust codecs: `image` for the common, game, HDR, and
/// scientific formats, `jxl-oxide` for JPEG XL. AVIF and HEIC are not listed: their decoders
/// need native libraries (dav1d, libheif).
pub const SUPPORTED_IMAGE_EXTENSIONS: &[&str] = &[
    "png", "apng", "jpg", "jpeg", "jfif", "gif", "webp", "bmp", "dib", "tif", "tiff", "svg", "ico", "cur", "tga",
    "dds", "hdr", "exr", "pbm", "pgm", "ppm", "pnm", "pam", "qoi", "ff", "farbfeld", "jxl",
];

/// Whether a path has a supported image extension and should open as an image document.
pub fn is_image_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| SUPPORTED_IMAGE_EXTENSIONS.contains(&extension.as_str()))
}

/// The media kind used to display an image document.
pub fn image_kind_for_path(path: &Path) -> MediaKind {
    if path
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| extension.eq_ignore_ascii_case("svg"))
    {
        MediaKind::Svg
    } else {
        MediaKind::Image
    }
}

/// Natural display size of a media request before layout scaling.
///
/// Images and SVG report their intrinsic pixel size; formulas and diagrams use the
/// same rasterizer and stable request width as the preview so the reserved box can
/// match the finished texture.
pub fn intrinsic_size(kind: MediaKind, source: &str, base: Option<&Path>, dark: bool) -> Option<(f32, f32)> {
    let lower = source.trim().to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") || lower.starts_with("data:") {
        return None;
    }
    let mut request = MediaRequest::new(kind, source);
    request.base = base.map(Path::to_path_buf);
    request.max_width = match kind {
        MediaKind::Image | MediaKind::Svg => 1_600,
        MediaKind::Math => 1_024,
        MediaKind::Mermaid => 1_200,
    };
    request.max_height = 4_096;
    request.dark = dark;
    match kind {
        MediaKind::Image | MediaKind::Svg => {
            let bytes = image::read_source(&request).ok()?;
            let text = String::from_utf8_lossy(&bytes[..bytes.len().min(256)]);
            if kind == MediaKind::Svg || text.trim_start().starts_with("<svg") || text.trim_start().starts_with("<?xml")
            {
                svg::intrinsic_size(&bytes)
            } else {
                let reader = ::image::ImageReader::new(std::io::Cursor::new(bytes))
                    .with_guessed_format()
                    .ok()?;
                let (width, height) = reader.into_dimensions().ok()?;
                Some((width as f32, height as f32))
            }
        }
        MediaKind::Math => render_math(source, dark, 1_024)
            .ok()
            .map(|raster| (raster.width as f32, raster.height as f32)),
        MediaKind::Mermaid => render_mermaid(source, dark, 1_200)
            .ok()
            .map(|raster| (raster.width as f32, raster.height as f32)),
    }
}

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

/// The source of an animation, kept to render its later frames.
#[derive(Clone, Debug)]
pub enum AnimatedSource {
    /// Animated SVG: each frame is the SVG patched for its time, then rasterized.
    Svg(String),
    /// Animated GIF, WebP, or APNG: frames decoded once, picked by time.
    Raster(std::sync::Arc<RasterAnimation>),
}

impl AnimatedSource {
    /// The frame shown at `time` seconds.
    pub fn render_frame(&self, time: f32, request: &MediaRequest) -> Result<Raster, MediaError> {
        match self {
            Self::Svg(source) => render_svg_frame(source, time, request),
            Self::Raster(animation) => Ok(animation.frame_at(time).clone()),
        }
    }

    /// A frame identity for change detection: decoded frames change only at their delays, so a
    /// caller can skip times that show the same frame. SVG frames change continuously.
    pub fn frame_index(&self, time: f32) -> Option<usize> {
        match self {
            Self::Svg(_) => None,
            Self::Raster(animation) => Some(animation.index_at(time)),
        }
    }
}

/// A loaded media item, with animation metadata when the source is animated.
#[derive(Clone, Debug)]
pub struct LoadedMedia {
    /// The first frame or decoded image.
    pub raster: Raster,
    /// Animation duration in seconds.
    pub animation_duration: Option<f32>,
    /// Whether the animation uses unsupported features.
    pub animation_unsupported: bool,
    /// The animation source retained for later frames.
    pub animated: Option<AnimatedSource>,
}

impl MediaRequest {
    /// Load a media item and detect whether it is animated (SVG, GIF, WebP, or APNG).
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
                            animated: Some(AnimatedSource::Svg(source)),
                        });
                    }
                    let raster = load_bytes(&bytes, self)?;
                    return Ok(LoadedMedia {
                        raster,
                        animation_duration: None,
                        animation_unsupported: false,
                        animated: None,
                    });
                }
                // A broken animation still shows its first frame through the still decoder.
                if let Ok(Some(animation)) = decode_animation(&bytes, self) {
                    return Ok(LoadedMedia {
                        raster: animation.first().clone(),
                        animation_duration: Some(animation.duration()),
                        animation_unsupported: animation.truncated,
                        animated: Some(AnimatedSource::Raster(std::sync::Arc::new(animation))),
                    });
                }
                let raster = load_bytes(&bytes, self)?;
                Ok(LoadedMedia {
                    raster,
                    animation_duration: None,
                    animation_unsupported: false,
                    animated: None,
                })
            }
            _ => self.load().map(|raster| LoadedMedia {
                raster,
                animation_duration: None,
                animation_unsupported: false,
                animated: None,
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
    fn image_paths_are_classified_by_extension() {
        assert!(is_image_path(Path::new("photo.PNG")));
        assert!(is_image_path(Path::new("mark.svg")));
        assert!(is_image_path(Path::new("animation.gif")));
        assert!(!is_image_path(Path::new("README.md")));
        assert!(!is_image_path(Path::new("archive.png.txt")));
        assert_eq!(image_kind_for_path(Path::new("mark.svg")), MediaKind::Svg);
        assert_eq!(image_kind_for_path(Path::new("photo.webp")), MediaKind::Image);
    }

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
