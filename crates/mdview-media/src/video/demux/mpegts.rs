//! MPEG transport streams (TS, M2TS, MTS) and program streams (MPG, VOB), indexed with
//! `filmcraft-mpegts`.

use filmcraft_mpegts::{Codec as TsCodec, File, Kind as TsKind};

use super::{FileSource, last_key_at};
use crate::MediaError;
use crate::video::track::{Codec, Kind, Packet, TrackInfo};

pub(crate) struct TsTrack {
    source: FileSource,
    file: File,
    stream: usize,
    /// Presentation time of every unit; units without a timestamp follow the one before.
    times: Vec<f64>,
    /// Whether each unit carries its own timestamp (only those are seek targets).
    stamped: Vec<bool>,
    next: usize,
}

fn ts_error(error: filmcraft_mpegts::Error) -> MediaError {
    MediaError::Decode(format!("Not a readable MPEG stream: {error}"))
}

impl TsTrack {
    pub(super) fn open(source: FileSource, kind: Kind) -> Result<Option<(Self, TrackInfo)>, MediaError> {
        let file = filmcraft_mpegts::open(&source).map_err(ts_error)?;
        let stream = match kind {
            Kind::Video => file
                .find(TsKind::Video, |codec| {
                    matches!(
                        codec,
                        TsCodec::Mpeg1Video | TsCodec::Mpeg2Video | TsCodec::H264 | TsCodec::Hevc
                    )
                })
                .or_else(|| file.find(TsKind::Video, |_| true)),
            Kind::Audio => file
                .find(TsKind::Audio, |codec| {
                    matches!(codec, TsCodec::MpegAudio | TsCodec::AacAdts | TsCodec::Ac3)
                })
                .or_else(|| file.find(TsKind::Audio, |_| true)),
        };
        let Some(stream) = stream else {
            return Ok(None);
        };
        // Every track of the file counts from the same origin, so sound and picture agree.
        let origin = file
            .streams
            .iter()
            .filter(|stream| matches!(stream.kind(), TsKind::Video | TsKind::Audio))
            .filter_map(|stream| stream.pts_range().map(|(first, _)| first))
            .min()
            .unwrap_or(0);
        let units = &file.streams[stream].units;
        let range = file.streams[stream]
            .pts_range()
            .map(|(first, last)| (last - first) as f64 / 90_000.0);
        let fps = range
            .filter(|range| *range > 0.0)
            .map(|range| (units.len().saturating_sub(1)) as f64 / range)
            .filter(|fps| (1.0..=240.0).contains(fps));
        let step = if kind == Kind::Video {
            1.0 / fps.unwrap_or(25.0)
        } else {
            0.0
        };
        let mut times = Vec::with_capacity(units.len());
        let mut previous: Option<f64> = None;
        for unit in units {
            let time = match unit.pts.or(unit.dts) {
                Some(pts) => (pts - origin) as f64 / 90_000.0,
                None => previous.map_or(0.0, |previous| previous + step),
            };
            times.push(time);
            previous = Some(time);
        }
        let stamped = units.iter().map(|unit| unit.pts.is_some()).collect();
        let codec = match &file.streams[stream].codec {
            TsCodec::Mpeg1Video | TsCodec::Mpeg2Video => Codec::Mpeg12(Vec::new()),
            TsCodec::H264 => Codec::H264(Vec::new()),
            TsCodec::Hevc => Codec::Hevc(Vec::new()),
            TsCodec::MpegAudio => Codec::MpegAudio,
            TsCodec::AacAdts => Codec::Aac(Vec::new()),
            TsCodec::Ac3 => Codec::Ac3,
            other => Codec::Other(other.name().to_ascii_lowercase()),
        };
        let mut info = TrackInfo::of(codec);
        info.fps = fps;
        info.duration = range.map(|range| range + step).filter(|duration| *duration > 0.0);
        let track = Self {
            source,
            file,
            stream,
            times,
            stamped,
            next: 0,
        };
        Ok(Some((track, info)))
    }

    pub(super) fn next(&mut self) -> Result<Option<Packet>, MediaError> {
        let units = &self.file.streams[self.stream].units;
        let Some(unit) = units.get(self.next) else {
            return Ok(None);
        };
        let index = self.next;
        self.next += 1;
        let data = self
            .file
            .read_unit(&self.source, self.stream, index)
            .map_err(ts_error)?;
        Ok(Some(Packet {
            data,
            pts: self.times[index],
            key: unit.key,
        }))
    }

    pub(super) fn seek(&mut self, at: f64) {
        let units = &self.file.streams[self.stream].units;
        self.next = last_key_at(
            units
                .iter()
                .zip(&self.times)
                .zip(&self.stamped)
                .map(|((unit, time), stamped)| (*time, unit.key && *stamped)),
            at,
        );
    }
}
