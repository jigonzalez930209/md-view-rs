//! AVI tracks (including OpenDML files over 1 GB), indexed with `filmcraft-avi`.

use filmcraft_avi::{AviFile, StreamKind};

use super::{FileSource, last_key_at};
use crate::MediaError;
use crate::video::track::{Kind, Packet, TrackInfo, fourcc_codec, wave_codec};

pub(crate) struct AviTrack {
    source: FileSource,
    file: AviFile,
    stream: usize,
    /// Presentation time of every chunk of the stream.
    times: Vec<f64>,
    next: usize,
}

fn avi_error(error: filmcraft_avi::Error) -> MediaError {
    MediaError::Decode(format!("Not a readable AVI file: {error}"))
}

impl AviTrack {
    pub(super) fn open(source: FileSource, kind: Kind) -> Result<Option<(Self, TrackInfo)>, MediaError> {
        let file = filmcraft_avi::open(&source).map_err(avi_error)?;
        let wanted = match kind {
            Kind::Video => StreamKind::Video,
            Kind::Audio => StreamKind::Audio,
        };
        let Some(stream) = file
            .streams
            .iter()
            .position(|stream| stream.kind == wanted && !stream.chunks.is_empty())
        else {
            return Ok(None);
        };
        let header = &file.streams[stream];
        // Stream time runs in `scale / rate` second units.
        let unit = f64::from(header.scale.max(1)) / f64::from(header.rate.max(1));
        let start = f64::from(header.start);
        let times: Vec<f64> = if header.sample_size > 0 {
            // Fixed-size samples (PCM, constant bit rate MP3): time follows the bytes.
            let mut bytes = 0u64;
            header
                .chunks
                .iter()
                .map(|chunk| {
                    let at = (start + bytes as f64 / f64::from(header.sample_size)) * unit;
                    bytes += u64::from(chunk.size);
                    at
                })
                .collect()
        } else {
            (0..header.chunks.len())
                .map(|index| (start + index as f64) * unit)
                .collect()
        };
        let mut info = match (&header.video, &header.audio) {
            (Some(video), _) if kind == Kind::Video => {
                let mut info = TrackInfo::of(fourcc_codec(video.compression, &video.extra));
                info.size = (video.width.unsigned_abs(), video.height.unsigned_abs());
                info.fps = Some(1.0 / unit).filter(|fps| (1.0..=240.0).contains(fps));
                info.duration = Some(f64::from(header.length) * unit).filter(|duration| *duration > 0.0);
                // AVI has no presentation times: chunks are in decode order, one frame each.
                info.decode_order = true;
                info
            }
            (_, Some(audio)) if kind == Kind::Audio => {
                let mut info = TrackInfo::of(wave_codec(audio.format_tag, audio.bits_per_sample, &audio.extra));
                info.rate = audio.sample_rate;
                info.channels = usize::from(audio.channels);
                info
            }
            _ => return Ok(None),
        };
        if info.duration.is_none() && kind == Kind::Video {
            info.duration = times.last().map(|last| last + unit);
        }
        let track = Self {
            source,
            file,
            stream,
            times,
            next: 0,
        };
        Ok(Some((track, info)))
    }

    pub(super) fn next(&mut self) -> Result<Option<Packet>, MediaError> {
        let chunks = &self.file.streams[self.stream].chunks;
        while let Some(chunk) = chunks.get(self.next) {
            let index = self.next;
            self.next += 1;
            // An empty chunk is a dropped frame: the previous picture stays on screen.
            if chunk.size == 0 {
                continue;
            }
            let data = self
                .file
                .read_chunk(&self.source, self.stream, index)
                .map_err(avi_error)?;
            return Ok(Some(Packet {
                data,
                pts: self.times[index],
                key: chunk.key,
            }));
        }
        Ok(None)
    }

    pub(super) fn seek(&mut self, at: f64) {
        let chunks = &self.file.streams[self.stream].chunks;
        self.next = last_key_at(
            chunks
                .iter()
                .zip(&self.times)
                .map(|(chunk, time)| (*time, chunk.key && chunk.size > 0)),
            at,
        );
    }
}
