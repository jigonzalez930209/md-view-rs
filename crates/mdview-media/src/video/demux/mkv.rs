//! Matroska and WebM tracks, read with `matroska-demuxer`.

use std::fs::File;
use std::io::BufReader;

use matroska_demuxer::{Frame, MatroskaFile, TrackEntry, TrackType};

use crate::MediaError;
use crate::video::track::{Codec, Kind, Packet, PcmFormat, TrackInfo, fourcc_codec, wave_codec};

pub(crate) struct MkvTrack {
    file: MatroskaFile<BufReader<File>>,
    track: u64,
    /// Seconds per timestamp unit.
    scale: f64,
    frame: Frame,
}

impl MkvTrack {
    pub(super) fn open(file: File, kind: Kind) -> Result<Option<(Self, TrackInfo)>, MediaError> {
        let file = MatroskaFile::open(BufReader::new(file))
            .map_err(|error| MediaError::Decode(format!("Not a readable Matroska/WebM file: {error}")))?;
        let wanted = match kind {
            Kind::Video => TrackType::Video,
            Kind::Audio => TrackType::Audio,
        };
        let Some(entry) = file.tracks().iter().find(|track| track.track_type() == wanted) else {
            return Ok(None);
        };
        let scale = file.info().timestamp_scale().get() as f64 / 1e9;
        let duration = file.info().duration().map(|duration| duration * scale);
        let info = mkv_info(entry, duration);
        let track = entry.track_number().get();
        Ok(Some((
            Self {
                file,
                track,
                scale,
                frame: Frame::default(),
            },
            info,
        )))
    }

    pub(super) fn next(&mut self) -> Result<Option<Packet>, MediaError> {
        loop {
            let more = self
                .file
                .next_frame(&mut self.frame)
                .map_err(|error| MediaError::Decode(format!("Matroska: {error}")))?;
            if !more {
                return Ok(None);
            }
            if self.frame.track == self.track {
                return Ok(Some(Packet {
                    data: std::mem::take(&mut self.frame.data),
                    pts: self.frame.timestamp as f64 * self.scale,
                    key: self.frame.is_keyframe.unwrap_or(false),
                }));
            }
        }
    }

    pub(super) fn seek(&mut self, at: f64) -> Result<(), MediaError> {
        // Cues point at clusters, not key frames: step back until the first frame of this
        // track found after the seek point is a key frame at or before `at`.
        for back in [0.0, 1.0, 3.0, 7.0, 15.0, 31.0, 63.0] {
            let target = ((at - back).max(0.0) / self.scale) as u64;
            self.file
                .seek(target)
                .map_err(|error| MediaError::Decode(format!("Matroska seek: {error}")))?;
            let position = self.position_of_next_key()?;
            if position.is_some_and(|(pts, _)| pts <= at + 1e-6) || target == 0 {
                return self
                    .file
                    .seek(position.map_or(0, |(_, timestamp)| timestamp))
                    .map_err(|error| MediaError::Decode(format!("Matroska seek: {error}")));
            }
        }
        self.file
            .seek(0)
            .map_err(|error| MediaError::Decode(format!("Matroska seek: {error}")))
    }

    /// The time and raw timestamp of the next key frame of this track.
    fn position_of_next_key(&mut self) -> Result<Option<(f64, u64)>, MediaError> {
        while let Some(packet) = self.next()? {
            if packet.key {
                return Ok(Some((packet.pts, (packet.pts / self.scale).round() as u64)));
            }
        }
        Ok(None)
    }
}

fn mkv_info(entry: &TrackEntry, duration: Option<f64>) -> TrackInfo {
    let private = entry.codec_private().unwrap_or_default();
    let audio = entry.audio();
    let bits = audio
        .and_then(|audio| audio.bit_depth())
        .map_or(16, |bits| bits.get() as u16);
    let pcm = |float: bool, big_endian: bool| {
        Codec::Pcm(PcmFormat {
            bits,
            float,
            big_endian,
        })
    };
    let codec = match entry.codec_id() {
        "V_MPEG4/ISO/AVC" => Codec::H264(private.to_vec()),
        "V_MPEGH/ISO/HEVC" => Codec::Hevc(private.to_vec()),
        // CodecPrivate is the av1C record: four bytes, then the configuration OBUs.
        "V_AV1" => Codec::Av1(private.get(4..).unwrap_or_default().to_vec()),
        "V_VP8" => Codec::Vp8,
        "V_VP9" => Codec::Vp9,
        "V_MPEG1" | "V_MPEG2" => Codec::Mpeg12(private.to_vec()),
        "V_MPEG4/ISO/ASP" | "V_MPEG4/ISO/SP" | "V_MPEG4/ISO/AP" => Codec::Mpeg4(private.to_vec()),
        "V_MJPEG" => Codec::Mjpeg,
        // A BITMAPINFOHEADER: the FourCC at 16, codec setup after the 40 bytes.
        "V_MS/VFW/FOURCC" => match private.get(16..20) {
            Some(&[a, b, c, d]) => fourcc_codec([a, b, c, d], private.get(40..).unwrap_or_default()),
            _ => Codec::Other("vfw".to_owned()),
        },
        "A_AAC" | "A_AAC/MPEG4/LC" | "A_AAC/MPEG2/LC" => Codec::Aac(private.to_vec()),
        "A_OPUS" => {
            // OpusHead keeps the pre-skip in bytes 10..12; CodecDelay (ns) says the same.
            let pre_skip = private
                .get(10..12)
                .map(|bytes| u64::from(u16::from_le_bytes([bytes[0], bytes[1]])))
                .or_else(|| entry.codec_delay().map(|delay| delay * 48_000 / 1_000_000_000))
                .unwrap_or(0);
            Codec::Opus { pre_skip }
        }
        "A_MPEG/L3" | "A_MPEG/L2" | "A_MPEG/L1" => Codec::MpegAudio,
        "A_VORBIS" => Codec::Vorbis(private.to_vec()),
        // "fLaC", then the STREAMINFO block behind its four-byte header.
        "A_FLAC" => Codec::Flac(private.get(8..42).unwrap_or_default().to_vec()),
        "A_AC3" => Codec::Ac3,
        "A_PCM/INT/LIT" => pcm(false, false),
        "A_PCM/INT/BIG" => pcm(false, true),
        "A_PCM/FLOAT/IEEE" => pcm(true, false),
        // A WAVEFORMATEX: the format tag first, codec setup after the 18 bytes.
        "A_MS/ACM" => match private.get(0..2) {
            Some(&[low, high]) => wave_codec(
                u16::from_le_bytes([low, high]),
                bits,
                private.get(18..).unwrap_or_default(),
            ),
            _ => Codec::Other("acm".to_owned()),
        },
        other => Codec::Other(other.trim_start_matches(['V', 'A', '_']).to_ascii_lowercase()),
    };
    let video = entry.video();
    TrackInfo {
        codec,
        size: video.map_or((0, 0), |video| {
            (video.pixel_width().get() as u32, video.pixel_height().get() as u32)
        }),
        rotation: 0,
        rate: audio.map_or(0, |audio| audio.sampling_frequency() as u32),
        channels: audio.map_or(0, |audio| audio.channels().get() as usize),
        fps: entry
            .default_duration()
            .map(|nanos| 1e9 / nanos.get() as f64)
            .filter(|fps| (1.0..=240.0).contains(fps)),
        duration,
        decode_order: false,
    }
}
