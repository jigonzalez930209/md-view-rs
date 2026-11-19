//! Flash Video (FLV) tracks: H.264 or HEVC with AAC or MP3, including the enhanced FLV
//! FourCC video tags. The tag headers are indexed when the file is opened.

use std::fs::File;
use std::io::{BufReader, Read, Seek, SeekFrom};

use super::{io_error, last_key_at};
use crate::MediaError;
use crate::video::track::{Codec, Kind, Packet, PcmFormat, TrackInfo};

/// Where a frame's bytes are in the file.
struct Tag {
    offset: u64,
    size: usize,
    pts: f64,
    key: bool,
}

pub(crate) struct FlvTrack {
    file: BufReader<File>,
    tags: Vec<Tag>,
    next: usize,
}

/// What one tag's payload holds, from its first bytes.
#[derive(Debug, PartialEq)]
enum Payload {
    /// The codec setup record (avcC, hvcC, av1C, AudioSpecificConfig) from `skip` on.
    Config { codec: Codec, skip: usize },
    /// A frame from `skip` on, shown `offset` milliseconds after the tag time.
    Frame {
        codec: Codec,
        skip: usize,
        offset: i32,
        key: bool,
    },
    /// Nothing to decode (end of sequence, metadata).
    Ignored,
}

/// The longest payload prefix [`classify`] reads.
const PREFIX: usize = 8;

fn si24(bytes: &[u8]) -> i32 {
    match bytes {
        [a, b, c, ..] => (i32::from_be_bytes([*a, *b, *c, 0])) >> 8,
        _ => 0,
    }
}

fn classify(kind: Kind, payload: &[u8]) -> Payload {
    let Some(&first) = payload.first() else {
        return Payload::Ignored;
    };
    match kind {
        Kind::Video if first & 0x80 != 0 => {
            // Enhanced FLV: frame type, packet type, then a FourCC.
            let key = (first >> 4) & 7 == 1;
            let codec = match payload.get(1..5) {
                Some(b"avc1") => Codec::H264(Vec::new()),
                Some(b"hvc1") => Codec::Hevc(Vec::new()),
                Some(b"av01") => Codec::Av1(Vec::new()),
                Some(b"vp09") => Codec::Vp9,
                Some(other) => Codec::Other(String::from_utf8_lossy(other).trim().to_ascii_lowercase()),
                None => return Payload::Ignored,
            };
            let composition = matches!(codec, Codec::H264(_) | Codec::Hevc(_));
            match first & 0x0F {
                0 => Payload::Config { codec, skip: 5 },
                1 if composition => Payload::Frame {
                    codec,
                    skip: 8,
                    offset: si24(payload.get(5..).unwrap_or_default()),
                    key,
                },
                1 | 3 => Payload::Frame {
                    codec,
                    skip: 5,
                    offset: 0,
                    key,
                },
                _ => Payload::Ignored,
            }
        }
        Kind::Video => {
            let key = first >> 4 == 1;
            let codec = match first & 0x0F {
                7 => Codec::H264(Vec::new()),
                12 => Codec::Hevc(Vec::new()),
                2 => Codec::Other("flv1".to_owned()),
                4 | 5 => Codec::Other("vp6".to_owned()),
                other => Codec::Other(format!("flv video codec {other}")),
            };
            if !matches!(codec, Codec::H264(_) | Codec::Hevc(_)) {
                return Payload::Frame {
                    codec,
                    skip: 1,
                    offset: 0,
                    key,
                };
            }
            match payload.get(1) {
                Some(0) => Payload::Config { codec, skip: 5 },
                Some(1) => Payload::Frame {
                    codec,
                    skip: 5,
                    offset: si24(payload.get(2..).unwrap_or_default()),
                    key,
                },
                _ => Payload::Ignored,
            }
        }
        Kind::Audio => {
            let bits = if first & 2 != 0 { 16 } else { 8 };
            let pcm = Codec::Pcm(PcmFormat {
                bits,
                float: false,
                big_endian: false,
            });
            let frame = |codec, skip| Payload::Frame {
                codec,
                skip,
                offset: 0,
                key: true,
            };
            match first >> 4 {
                10 => match payload.get(1) {
                    Some(0) => Payload::Config {
                        codec: Codec::Aac(Vec::new()),
                        skip: 2,
                    },
                    Some(1) => frame(Codec::Aac(Vec::new()), 2),
                    _ => Payload::Ignored,
                },
                2 | 14 => frame(Codec::MpegAudio, 1),
                0 | 3 => frame(pcm, 1),
                other => frame(Codec::Other(format!("flv audio format {other}")), 1),
            }
        }
    }
}

/// The codec with its setup record attached.
fn with_config(codec: Codec, config: Vec<u8>) -> Codec {
    match codec {
        Codec::H264(_) => Codec::H264(config),
        Codec::Hevc(_) => Codec::Hevc(config),
        // av1C: four bytes, then the configuration OBUs.
        Codec::Av1(_) => Codec::Av1(config.get(4..).unwrap_or_default().to_vec()),
        Codec::Aac(_) => Codec::Aac(config),
        other => other,
    }
}

impl FlvTrack {
    pub(super) fn open(file: File, kind: Kind) -> Result<Option<(Self, TrackInfo)>, MediaError> {
        let mut file = BufReader::new(file);
        let mut header = [0u8; 9];
        file.read_exact(&mut header).map_err(io_error)?;
        let start = u32::from_be_bytes([header[5], header[6], header[7], header[8]]);
        file.seek(SeekFrom::Start(u64::from(start) + 4)).map_err(io_error)?;
        let wanted = match kind {
            Kind::Video => 9,
            Kind::Audio => 8,
        };
        let mut position = u64::from(start) + 4;
        let mut tags = Vec::new();
        let mut codec: Option<Codec> = None;
        let mut config: Option<Vec<u8>> = None;
        let mut first_audio = 0u8;
        let mut tag = [0u8; 11];
        while file.read_exact(&mut tag).is_ok() {
            let size = u32::from_be_bytes([0, tag[1], tag[2], tag[3]]) as usize;
            let time = u32::from_be_bytes([tag[7], tag[4], tag[5], tag[6]]);
            let data = position + 11;
            let mut cursor = data;
            position = data + size as u64 + 4;
            // Bit 5 marks an encrypted tag.
            if tag[0] & 0x1F == wanted && tag[0] & 0x20 == 0 && size > 0 {
                let mut prefix = vec![0u8; size.min(PREFIX)];
                file.read_exact(&mut prefix).map_err(io_error)?;
                cursor += prefix.len() as u64;
                first_audio = if first_audio == 0 { prefix[0] } else { first_audio };
                match classify(kind, &prefix) {
                    Payload::Config { codec: found, skip } if config.is_none() => {
                        let mut record = vec![0u8; size.saturating_sub(skip)];
                        file.seek(SeekFrom::Start(data + skip as u64)).map_err(io_error)?;
                        file.read_exact(&mut record).map_err(io_error)?;
                        cursor = data + size as u64;
                        codec.get_or_insert(found);
                        config = Some(record);
                    }
                    Payload::Frame {
                        codec: found,
                        skip,
                        offset,
                        key,
                    } if size > skip => {
                        codec.get_or_insert(found);
                        tags.push(Tag {
                            offset: data + skip as u64,
                            size: size - skip,
                            pts: (f64::from(time) + f64::from(offset)) / 1000.0,
                            key,
                        });
                    }
                    _ => {}
                }
            }
            // A relative seek keeps the read buffer when the next tag is close.
            file.seek_relative((position - cursor) as i64).map_err(io_error)?;
        }
        let Some(codec) = codec else {
            return Ok(None);
        };
        let mut info = TrackInfo::of(with_config(codec, config.unwrap_or_default()));
        let first = tags.iter().map(|tag| tag.pts).fold(f64::INFINITY, f64::min);
        let last = tags.iter().map(|tag| tag.pts).fold(f64::NEG_INFINITY, f64::max);
        if kind == Kind::Video && last > first {
            let fps = (tags.len() - 1) as f64 / (last - first);
            info.fps = Some(fps).filter(|fps| (1.0..=240.0).contains(fps));
            info.duration = Some(last + 1.0 / fps);
        } else if kind == Kind::Audio {
            info.rate = [5_512, 11_025, 22_050, 44_100][usize::from((first_audio >> 2) & 3)];
            info.channels = usize::from(first_audio & 1) + 1;
        }
        Ok(Some((Self { file, tags, next: 0 }, info)))
    }

    pub(super) fn next(&mut self) -> Result<Option<Packet>, MediaError> {
        let Some(tag) = self.tags.get(self.next) else {
            return Ok(None);
        };
        self.next += 1;
        let mut data = vec![0u8; tag.size];
        self.file
            .seek(SeekFrom::Start(tag.offset))
            .and_then(|_| self.file.read_exact(&mut data))
            .map_err(io_error)?;
        Ok(Some(Packet {
            data,
            pts: tag.pts,
            key: tag.key,
        }))
    }

    pub(super) fn seek(&mut self, at: f64) {
        self.next = last_key_at(self.tags.iter().map(|tag| (tag.pts, tag.key)), at);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn video_tags_say_what_they_carry() {
        assert_eq!(
            classify(Kind::Video, &[0x17, 0, 0, 0, 0]),
            Payload::Config {
                codec: Codec::H264(Vec::new()),
                skip: 5
            }
        );
        assert_eq!(
            classify(Kind::Video, &[0x27, 1, 0, 0, 40]),
            Payload::Frame {
                codec: Codec::H264(Vec::new()),
                skip: 5,
                offset: 40,
                key: false
            }
        );
        assert_eq!(
            classify(Kind::Video, &[0x91, b'h', b'v', b'c', b'1', 0xFF, 0xFF, 0xFE]),
            Payload::Frame {
                codec: Codec::Hevc(Vec::new()),
                skip: 8,
                offset: -2,
                key: true
            }
        );
        assert!(matches!(
            classify(Kind::Video, &[0x12]),
            Payload::Frame { codec: Codec::Other(name), .. } if name == "flv1"
        ));
    }

    #[test]
    fn audio_tags_say_what_they_carry() {
        assert!(matches!(
            classify(Kind::Audio, &[0xAF, 0]),
            Payload::Config {
                codec: Codec::Aac(_),
                skip: 2
            }
        ));
        assert!(matches!(
            classify(Kind::Audio, &[0xAF, 1]),
            Payload::Frame {
                codec: Codec::Aac(_),
                skip: 2,
                ..
            }
        ));
        assert!(matches!(
            classify(Kind::Audio, &[0x2F]),
            Payload::Frame {
                codec: Codec::MpegAudio,
                skip: 1,
                ..
            }
        ));
    }
}
