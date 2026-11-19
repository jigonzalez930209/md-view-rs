//! MP4 and QuickTime (MOV, 3GP) tracks, read with `re_mp4`.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use super::{io_error, last_key_at};
use crate::MediaError;
use crate::video::track::{Codec, Kind, Packet, TrackInfo};

struct Sample {
    offset: u64,
    size: usize,
    pts: f64,
    key: bool,
}

pub(crate) struct Mp4Track {
    file: BufReader<File>,
    samples: Vec<Sample>,
    next: usize,
}

impl Mp4Track {
    pub(super) fn open(file: File, kind: Kind) -> Result<Option<(Self, TrackInfo)>, MediaError> {
        let size = file.metadata().map_err(io_error)?.len();
        let mut reader = BufReader::new(file);
        let mp4 = re_mp4::Mp4::read(&mut reader, size)
            .map_err(|error| MediaError::Decode(format!("Not a readable MP4/MOV file: {error}")))?;
        let (wanted, handler) = match kind {
            Kind::Video => (re_mp4::TrackKind::Video, b"vide"),
            Kind::Audio => (re_mp4::TrackKind::Audio, b"soun"),
        };
        // `re_mp4` only names the kind of codecs it knows; the handler says it for the rest
        // (H.263 and Motion JPEG video, MP3 and AC-3 audio).
        let Some(track) = mp4.tracks().values().find(|track| {
            track.kind == Some(wanted)
                || (track.kind.is_none() && &track.trak(&mp4).mdia.hdlr.handler_type.value == handler)
        }) else {
            return Ok(None);
        };
        let trak = track.trak(&mp4);
        let timescale = track.timescale.max(1) as f64;
        // The first edit that is not an empty one says where the presentation starts.
        let start = trak
            .edts
            .as_ref()
            .and_then(|edts| edts.elst.as_ref())
            .and_then(|elst| {
                elst.entries
                    .iter()
                    .find(|entry| entry.media_time != u64::from(u32::MAX) && entry.media_time != u64::MAX)
            })
            .map_or(0, |entry| entry.media_time as i64);
        // `re_mp4` moves the earliest composition time to zero. The edit list is on the
        // file's own media timeline, so undo that move first: the first sample's file
        // composition time is its decode time (zero) plus its first composition offset.
        let first_offset = trak
            .mdia
            .minf
            .stbl
            .ctts
            .as_ref()
            .and_then(|ctts| ctts.entries.first())
            .map_or(0, |entry| i64::from(entry.sample_offset));
        let moved = track
            .samples
            .first()
            .map_or(0, |sample| first_offset - sample.composition_timestamp);
        let start = start - moved;
        let samples: Vec<Sample> = track
            .samples
            .iter()
            .map(|sample| Sample {
                offset: sample.offset,
                size: sample.size as usize,
                pts: (sample.composition_timestamp - start) as f64 / timescale,
                key: sample.is_sync,
            })
            .collect();
        let duration = (track.duration > 0).then(|| track.duration as f64 / timescale);
        let matrix = &trak.tkhd.matrix;
        let contents = &trak.mdia.minf.stbl.stsd.contents;
        let (rate, channels) = match contents {
            re_mp4::StsdBoxContent::Mp4a(mp4a) => (u32::from(mp4a.samplerate.value()), usize::from(mp4a.channelcount)),
            _ => (0, 0),
        };
        let info = TrackInfo {
            codec: mp4_codec(contents),
            size: (u32::from(track.width), u32::from(track.height)),
            rotation: rotation(matrix.a, matrix.b),
            rate,
            channels,
            fps: duration
                .filter(|duration| *duration > 0.0)
                .map(|duration| samples.len() as f64 / duration),
            duration,
            decode_order: false,
        };
        let track = Self {
            file: reader,
            samples,
            next: 0,
        };
        Ok(Some((track, info)))
    }

    pub(super) fn next(&mut self) -> Result<Option<Packet>, MediaError> {
        let Some(sample) = self.samples.get(self.next) else {
            return Ok(None);
        };
        self.next += 1;
        let mut data = vec![0u8; sample.size];
        self.file
            .seek(SeekFrom::Start(sample.offset))
            .and_then(|_| self.file.read_exact(&mut data))
            .map_err(io_error)?;
        Ok(Some(Packet {
            data,
            pts: sample.pts,
            key: sample.key,
        }))
    }

    pub(super) fn seek(&mut self, at: f64) {
        // Samples are in decode order: start from the last key frame shown at or before `at`.
        self.next = last_key_at(self.samples.iter().map(|sample| (sample.pts, sample.key)), at);
    }
}

fn mp4_codec(contents: &re_mp4::StsdBoxContent) -> Codec {
    use re_mp4::StsdBoxContent;
    match contents {
        StsdBoxContent::Avc1(avc1) => Codec::H264(avc1.avcc.raw.clone()),
        StsdBoxContent::Hev1(hevc) | StsdBoxContent::Hvc1(hevc) => Codec::Hevc(hevc.hvcc.raw.clone()),
        StsdBoxContent::Av01(av01) => Codec::Av1(av01.av1c.contents.config_obus.clone()),
        StsdBoxContent::Vp08(_) => Codec::Vp8,
        StsdBoxContent::Vp09(_) => Codec::Vp9,
        StsdBoxContent::Mp4a(mp4a) => {
            let Some(esds) = &mp4a.esds else {
                return Codec::Other("mp4a".to_owned());
            };
            match esds.es_desc.dec_config.object_type_indication {
                0x40 | 0x66 | 0x67 | 0x68 => {
                    let specific = &esds.es_desc.dec_config.dec_specific;
                    // Rebuild the two-byte AudioSpecificConfig from its parsed fields.
                    let config = (u16::from(specific.profile) << 11)
                        | (u16::from(specific.freq_index) << 7)
                        | (u16::from(specific.chan_conf) << 3);
                    Codec::Aac(config.to_be_bytes().to_vec())
                }
                0x69 | 0x6B => Codec::MpegAudio,
                0xA5 => Codec::Ac3,
                other => Codec::Other(format!("mp4a object type {other:#04x}")),
            }
        }
        StsdBoxContent::Unknown(fourcc) => match &fourcc.value {
            b"s263" | b"h263" | b"H263" => Codec::H263,
            b"jpeg" | b"mjpa" | b"mjpb" | b"MJPG" => Codec::Mjpeg,
            b".mp3" => Codec::MpegAudio,
            b"ac-3" => Codec::Ac3,
            other => Codec::Other(String::from_utf8_lossy(other).trim().to_ascii_lowercase()),
        },
        StsdBoxContent::Tx3g(_) => Codec::Other("tx3g".to_owned()),
    }
}

/// Clockwise rotation, in degrees, of a track matrix whose first row is `(a, b)`.
fn rotation(a: i32, b: i32) -> u32 {
    match (a.signum(), b.signum()) {
        (0, 1) => 90,
        (-1, 0) => 180,
        (0, -1) => 270,
        _ => 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn track_matrices_map_to_clockwise_rotations() {
        assert_eq!(rotation(0x10000, 0), 0);
        assert_eq!(rotation(0, 0x10000), 90);
        assert_eq!(rotation(-0x10000, 0), 180);
        assert_eq!(rotation(0, -0x10000), 270);
    }
}
