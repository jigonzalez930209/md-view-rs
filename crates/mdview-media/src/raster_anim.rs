//! Animated raster images: GIF, animated WebP, and APNG.
//!
//! Frames are decoded once into composited RGBA rasters with their delays, so playback only
//! picks a frame; nothing is decoded again while an animation runs. Decoding is bounded by a
//! frame count and a total pixel budget: an animation over the budget keeps its first frames,
//! and one that cannot be decoded falls back to the static first frame.

use std::io::Cursor;
use std::time::Duration;

use image::codecs::gif::GifDecoder;
use image::codecs::png::PngDecoder;
use image::codecs::webp::WebPDecoder;
use image::{AnimationDecoder, Frames, ImageFormat};

use crate::{MediaError, MediaRequest, Raster};

/// Most frames kept from one animation.
pub const MAX_FRAMES: usize = 1_000;
/// Most decoded bytes kept from one animation (RGBA), about 256 MiB.
pub const MAX_ANIMATION_BYTES: usize = 256 * 1024 * 1024;
/// Shortest frame delay honoured; browsers treat smaller GIF delays as 100 ms, but 20 ms is
/// the GIF minimum that real encoders use, so anything shorter is raised to it.
const MIN_DELAY: f32 = 0.02;

/// A decoded animation: frames with their display durations.
#[derive(Clone, Debug)]
pub struct RasterAnimation {
    frames: Vec<(Raster, f32)>,
    duration: f32,
    /// Whether frames were dropped to stay within the budget.
    pub truncated: bool,
}

impl RasterAnimation {
    /// Total duration of one loop in seconds.
    pub fn duration(&self) -> f32 {
        self.duration
    }

    /// Number of frames kept.
    pub fn frame_count(&self) -> usize {
        self.frames.len()
    }

    /// The first frame.
    pub fn first(&self) -> &Raster {
        &self.frames[0].0
    }

    /// The frame shown at `time` seconds, looping.
    pub fn frame_at(&self, time: f32) -> &Raster {
        let mut remaining = time.rem_euclid(self.duration.max(MIN_DELAY));
        for (raster, delay) in &self.frames {
            if remaining < *delay {
                return raster;
            }
            remaining -= delay;
        }
        &self.frames[self.frames.len() - 1].0
    }

    /// Index of the frame shown at `time`, for change detection.
    pub fn index_at(&self, time: f32) -> usize {
        let mut remaining = time.rem_euclid(self.duration.max(MIN_DELAY));
        for (index, (_, delay)) in self.frames.iter().enumerate() {
            if remaining < *delay {
                return index;
            }
            remaining -= delay;
        }
        self.frames.len() - 1
    }
}

/// Decode an animated GIF, WebP, or APNG. Returns `Ok(None)` for a still image.
pub fn decode_animation(bytes: &[u8], request: &MediaRequest) -> Result<Option<RasterAnimation>, MediaError> {
    let decode_error = |error: image::ImageError| MediaError::Decode(error.to_string());
    let frames = match image::guess_format(bytes) {
        Ok(ImageFormat::Gif) => GifDecoder::new(Cursor::new(bytes)).map_err(decode_error)?.into_frames(),
        Ok(ImageFormat::WebP) => {
            let decoder = WebPDecoder::new(Cursor::new(bytes)).map_err(decode_error)?;
            if !decoder.has_animation() {
                return Ok(None);
            }
            decoder.into_frames()
        }
        Ok(ImageFormat::Png) => {
            let decoder = PngDecoder::new(Cursor::new(bytes)).map_err(decode_error)?;
            if !decoder.is_apng().map_err(decode_error)? {
                return Ok(None);
            }
            decoder.apng().map_err(decode_error)?.into_frames()
        }
        _ => return Ok(None),
    };
    collect(frames, request)
}

fn collect(frames: Frames<'_>, request: &MediaRequest) -> Result<Option<RasterAnimation>, MediaError> {
    let mut kept = Vec::new();
    let mut bytes = 0usize;
    let mut truncated = false;
    for frame in frames {
        let frame = frame.map_err(|error| MediaError::Decode(error.to_string()))?;
        if kept.len() >= MAX_FRAMES {
            truncated = true;
            break;
        }
        let (numerator, denominator) = frame.delay().numer_denom_ms();
        let delay = Duration::from_secs_f64(f64::from(numerator) / f64::from(denominator.max(1)) / 1000.0);
        let mut buffer = frame.into_buffer();
        let (width, height) = buffer.dimensions();
        if width > request.max_width.max(1) || height > request.max_height.max(1) {
            buffer = image::DynamicImage::ImageRgba8(buffer)
                .thumbnail(request.max_width.max(1), request.max_height.max(1))
                .to_rgba8();
        }
        let size = buffer.as_raw().len();
        if bytes + size > MAX_ANIMATION_BYTES && !kept.is_empty() {
            truncated = true;
            break;
        }
        bytes += size;
        kept.push((
            Raster {
                width: buffer.width(),
                height: buffer.height(),
                rgba: buffer.into_raw(),
            },
            delay.as_secs_f32().max(MIN_DELAY),
        ));
    }
    if kept.len() < 2 {
        return Ok(None);
    }
    let duration = kept.iter().map(|(_, delay)| delay).sum();
    Ok(Some(RasterAnimation {
        frames: kept,
        duration,
        truncated,
    }))
}

#[cfg(test)]
mod tests {
    use image::codecs::gif::{GifEncoder, Repeat};
    use image::{Delay, Frame, Rgba, RgbaImage};

    use super::*;
    use crate::MediaKind;

    fn gif(frames: &[(u8, u32)]) -> Vec<u8> {
        let mut bytes = Vec::new();
        {
            let mut encoder = GifEncoder::new(&mut bytes);
            encoder.set_repeat(Repeat::Infinite).expect("repeat");
            for (shade, delay_ms) in frames {
                let image = RgbaImage::from_pixel(8, 8, Rgba([*shade, 0, 0, 255]));
                encoder
                    .encode_frame(Frame::from_parts(image, 0, 0, Delay::from_numer_denom_ms(*delay_ms, 1)))
                    .expect("frame");
            }
        }
        bytes
    }

    fn request() -> MediaRequest {
        MediaRequest::new(MediaKind::Image, "test.gif")
    }

    #[test]
    fn animated_gif_keeps_frames_and_delays() {
        let animation = decode_animation(&gif(&[(10, 100), (200, 300)]), &request())
            .expect("decode")
            .expect("animated");
        assert_eq!(animation.frame_count(), 2);
        assert!((animation.duration() - 0.4).abs() < 0.001);
        assert_eq!(animation.index_at(0.05), 0);
        assert_eq!(animation.index_at(0.15), 1);
        assert_eq!(animation.index_at(0.45), 0, "loops");
        assert_eq!(animation.frame_at(0.2).rgba[0], 200);
    }

    #[test]
    fn single_frame_gif_is_a_still() {
        assert!(
            decode_animation(&gif(&[(10, 100)]), &request())
                .expect("decode")
                .is_none()
        );
    }

    #[test]
    fn zero_delays_are_raised_to_the_minimum() {
        let animation = decode_animation(&gif(&[(10, 0), (20, 0)]), &request())
            .expect("decode")
            .expect("animated");
        assert!((animation.duration() - 2.0 * MIN_DELAY).abs() < 0.001);
    }

    #[test]
    fn other_formats_are_not_animations() {
        let mut png = Vec::new();
        RgbaImage::from_pixel(4, 4, Rgba([1, 2, 3, 255]))
            .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
            .expect("png");
        assert!(decode_animation(&png, &request()).expect("decode").is_none());
    }
}
