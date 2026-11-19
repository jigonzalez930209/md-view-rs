//! Video probing and decoding in pure Rust.
//!
//! MD View runs no external program for video and links no C codec. MP4/MOV/3GP, MKV/WebM,
//! AVI, MPEG transport and program streams, and FLV are demuxed in Rust (see [`demux`]); the
//! video and audio codecs are listed in the `decode` and `audio` modules. [`VideoReader`]
//! turns the video track into RGBA frames at a requested size, and [`AudioReader`] turns the
//! soundtrack into PCM for the output device.

mod audio;
mod decode;
mod demux;
mod track;
mod yuv;

use std::collections::VecDeque;
use std::path::Path;

pub use audio::AudioReader;
use decode::{Picture, VideoDecoder};
use demux::Demuxer;
use track::{Codec, Kind};

use crate::MediaError;

/// Extensions the video viewer opens. Files MD View cannot decode (WMV, OGV) open too and
/// explain why.
pub const VIDEO_EXTENSIONS: &[&str] = &[
    "mp4", "m4v", "mov", "mkv", "webm", "avi", "divx", "wmv", "flv", "mpg", "mpeg", "m2ts", "mts", "ts", "3gp", "3g2",
    "ogv", "vob", "asf", "f4v",
];

/// Whether a path opens in the video viewer.
pub fn is_video_path(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .is_some_and(|extension| VIDEO_EXTENSIONS.contains(&extension.as_str()))
}

/// What the container says about a video file.
#[derive(Clone, Debug, PartialEq)]
pub struct VideoInfo {
    /// Frame width in pixels, after rotation metadata.
    pub width: u32,
    /// Frame height in pixels, after rotation metadata.
    pub height: u32,
    /// Duration in seconds, when the container knows it.
    pub duration: Option<f64>,
    /// Frames per second.
    pub fps: f64,
    /// Whether the file has a soundtrack MD View can play.
    pub has_audio: bool,
    /// Video codec name, for the status bar.
    pub codec: String,
}

/// Run third-party demuxing or decoding, turning a panic on damaged data into an error, so a
/// bad file stops its own playback and nothing else.
pub(crate) fn guarded<T>(what: &str, run: impl FnOnce() -> T) -> Result<T, MediaError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
        .map_err(|_| MediaError::Decode(format!("The {what} failed on damaged data in this file.")))
}

/// Read a video file's layout without decoding a frame.
pub fn probe(path: &Path) -> Result<VideoInfo, MediaError> {
    VideoReader::open(path).map(|(_, info)| info)
}

/// The decode size for a video shown at most `max_width` pixels wide: the native size, or
/// smaller with the same aspect, rounded to even numbers.
pub fn decode_size(info: &VideoInfo, max_width: u32) -> (u32, u32) {
    let width = info.width.min(max_width.max(16));
    let height = (f64::from(info.height) * f64::from(width) / f64::from(info.width)).round() as u32;
    (width & !1, (height & !1).max(2))
}

/// A decoded frame ready for a texture.
pub struct VideoFrame {
    /// Presentation time in seconds.
    pub at: f64,
    /// Width in pixels.
    pub width: u32,
    /// Height in pixels.
    pub height: u32,
    /// Straight RGBA, `width * height * 4` bytes.
    pub rgba: Vec<u8>,
}

/// Decodes the video track of a file into RGBA frames, from any position.
pub struct VideoReader {
    demuxer: Demuxer,
    codec: Codec,
    fps: f64,
    /// The container times packets in decode order (AVI).
    decode_order: bool,
    decoder: VideoDecoder,
    rotation: u32,
    /// Decoded pictures waiting to be shown, in display order.
    ready: VecDeque<Picture>,
    /// Pictures before this time are decoded but not returned (after a seek).
    from: f64,
    /// Half a frame, the tolerance when comparing with `from`.
    slack: f64,
    /// The latest picture skipped before `from`, shown when a seek lands past the last frame.
    skipped: Option<Picture>,
    flushed: bool,
}

impl VideoReader {
    /// Open the video track of `path`, positioned at the start.
    pub fn open(path: &Path) -> Result<(Self, VideoInfo), MediaError> {
        let (demuxer, track) = guarded("container reader", || Demuxer::open(path, Kind::Video))??
            .ok_or_else(|| MediaError::Decode("The file has no video stream.".to_owned()))?;
        let fps = track.fps.filter(|fps| (1.0..=240.0).contains(fps)).unwrap_or(30.0);
        let decoder = VideoDecoder::new(&track.codec, fps, track.decode_order)?;
        let has_audio = guarded("container reader", || Demuxer::open(path, Kind::Audio))
            .ok()
            .and_then(Result::ok)
            .flatten()
            .is_some_and(|(_, audio)| audio.codec.is_playable_audio());
        let mut reader = Self {
            demuxer,
            codec: track.codec.clone(),
            fps,
            decode_order: track.decode_order,
            decoder,
            rotation: track.rotation,
            ready: VecDeque::new(),
            from: 0.0,
            slack: 0.5 / fps,
            skipped: None,
            flushed: false,
        };
        let (mut width, mut height) = track.size;
        if width == 0 || height == 0 {
            // The container does not say (MPEG streams, FLV): the first picture does.
            (width, height) = reader
                .first_picture_size()?
                .ok_or_else(|| MediaError::Decode("The video stream has no pictures.".to_owned()))?;
        }
        if track.rotation % 180 == 90 {
            std::mem::swap(&mut width, &mut height);
        }
        let info = VideoInfo {
            width,
            height,
            duration: track
                .duration
                .filter(|duration| duration.is_finite() && *duration > 0.0),
            fps,
            has_audio,
            codec: track.codec.name(),
        };
        Ok((reader, info))
    }

    /// Decode up to the first picture for its size, then go back to the start.
    fn first_picture_size(&mut self) -> Result<Option<(u32, u32)>, MediaError> {
        let size = loop {
            if let Some(picture) = self.ready.front() {
                break Some((picture.width, picture.height));
            }
            match guarded("container reader", || self.demuxer.next())?? {
                Some(packet) => self
                    .ready
                    .extend(guarded("video decoder", || self.decoder.decode(&packet))?),
                None => {
                    self.ready.extend(guarded("video decoder", || self.decoder.flush())?);
                    break self.ready.front().map(|picture| (picture.width, picture.height));
                }
            }
        };
        self.seek(0.0)?;
        Ok(size)
    }

    /// Continue from the frame shown at `at` seconds.
    pub fn seek(&mut self, at: f64) -> Result<(), MediaError> {
        guarded("container reader", || self.demuxer.seek(at))??;
        self.decoder = VideoDecoder::new(&self.codec, self.fps, self.decode_order)?;
        self.ready.clear();
        self.from = at;
        self.skipped = None;
        self.flushed = false;
        Ok(())
    }

    /// The next frame at `size` (after rotation), or `Ok(None)` at the end of the video. A
    /// seek past the last frame returns the last frame.
    pub fn next_frame(&mut self, size: (u32, u32)) -> Result<Option<VideoFrame>, MediaError> {
        loop {
            while let Some(picture) = self.ready.pop_front() {
                if picture.pts + self.slack < self.from {
                    self.skipped = Some(picture);
                    continue;
                }
                self.skipped = None;
                return Ok(Some(self.frame(&picture, size)));
            }
            if self.flushed {
                return Ok(self.skipped.take().map(|picture| self.frame(&picture, size)));
            }
            match guarded("container reader", || self.demuxer.next())?? {
                Some(packet) => self
                    .ready
                    .extend(guarded("video decoder", || self.decoder.decode(&packet))?),
                None => {
                    self.ready.extend(guarded("video decoder", || self.decoder.flush())?);
                    self.flushed = true;
                }
            }
        }
    }

    fn frame(&self, picture: &Picture, size: (u32, u32)) -> VideoFrame {
        VideoFrame {
            at: picture.pts,
            width: size.0,
            height: size.1,
            rgba: yuv::to_rgba(picture, size, self.rotation),
        }
    }
}

#[cfg(test)]
mod tests;
