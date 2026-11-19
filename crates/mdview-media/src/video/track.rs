//! What the demuxers hand to the decoders: codecs with their setup data, track layout, and
//! timed packets.

/// The codecs a track can hold, with the setup data their decoders need.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Codec {
    /// H.264 with its `avcC` record, or empty for an Annex B stream (start codes in band).
    H264(Vec<u8>),
    /// HEVC with its `hvcC` record, or empty for an Annex B stream.
    Hevc(Vec<u8>),
    /// AV1 with the configuration OBUs stored outside the packets (may be empty).
    Av1(Vec<u8>),
    Vp8,
    Vp9,
    /// MPEG-1 or MPEG-2 video, with any sequence header stored outside the packets.
    Mpeg12(Vec<u8>),
    /// MPEG-4 Part 2 (Xvid, DivX 4 and later), with any VOL headers stored outside the packets.
    Mpeg4(Vec<u8>),
    H263,
    /// Motion JPEG: every packet is a JPEG image.
    Mjpeg,
    /// AAC with its AudioSpecificConfig, or empty for ADTS frames.
    Aac(Vec<u8>),
    /// Opus with the samples to drop at the start (pre-skip).
    Opus {
        pre_skip: u64,
    },
    /// MPEG audio layers I, II, and III (MP3).
    MpegAudio,
    /// Vorbis with its identification and setup headers (Xiph-laced, or one after the other).
    Vorbis(Vec<u8>),
    /// FLAC with its STREAMINFO block.
    Flac(Vec<u8>),
    /// AC-3 (Dolby Digital).
    Ac3,
    /// Uncompressed audio.
    Pcm(PcmFormat),
    /// A codec MD View has no decoder for, by name.
    Other(String),
}

/// The sample layout of uncompressed audio.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct PcmFormat {
    pub bits: u16,
    pub float: bool,
    pub big_endian: bool,
}

impl Codec {
    /// A short name for the status bar.
    pub(crate) fn name(&self) -> String {
        let name = match self {
            Self::H264(_) => "h264",
            Self::Hevc(_) => "hevc",
            Self::Av1(_) => "av1",
            Self::Vp8 => "vp8",
            Self::Vp9 => "vp9",
            Self::Mpeg12(_) => "mpeg",
            Self::Mpeg4(_) => "mpeg4",
            Self::H263 => "h263",
            Self::Mjpeg => "mjpeg",
            Self::Aac(_) => "aac",
            Self::Opus { .. } => "opus",
            Self::MpegAudio => "mp3",
            Self::Vorbis(_) => "vorbis",
            Self::Flac(_) => "flac",
            Self::Ac3 => "ac3",
            Self::Pcm(_) => "pcm",
            Self::Other(name) => name,
        };
        name.to_owned()
    }

    /// Whether this is an audio codec MD View decodes.
    pub(crate) fn is_playable_audio(&self) -> bool {
        matches!(
            self,
            Self::Aac(_)
                | Self::Opus { .. }
                | Self::MpegAudio
                | Self::Vorbis(_)
                | Self::Flac(_)
                | Self::Ac3
                | Self::Pcm(_)
        )
    }
}

/// The video codec of a Video for Windows FourCC (AVI `biCompression`, Matroska
/// `V_MS/VFW/FOURCC`), with the setup bytes that follow the bitmap header.
pub(crate) fn fourcc_codec(fourcc: [u8; 4], extra: &[u8]) -> Codec {
    let upper = fourcc.map(|byte| byte.to_ascii_uppercase());
    match &upper {
        b"H264" | b"X264" | b"AVC1" | b"DAVC" | b"VSSH" => Codec::H264(Vec::new()),
        b"HEVC" | b"H265" | b"HVC1" | b"HEV1" | b"X265" => Codec::Hevc(Vec::new()),
        b"XVID" | b"DIVX" | b"DX50" | b"FMP4" | b"MP4V" | b"M4S2" | b"3IV2" | b"DXGM" => Codec::Mpeg4(extra.to_vec()),
        b"MJPG" | b"AVRN" | b"LJPG" | b"JPGL" | b"MJPA" | b"DMB1" => Codec::Mjpeg,
        b"MPG1" | b"MPG2" | b"MPEG" | b"PIM1" | b"PIM2" | b"MX5P" => Codec::Mpeg12(Vec::new()),
        b"VP80" => Codec::Vp8,
        b"VP90" => Codec::Vp9,
        b"AV01" => Codec::Av1(Vec::new()),
        b"H263" | b"S263" | b"U263" | b"M263" => Codec::H263,
        b"DIV3" | b"DIV4" | b"MP43" | b"MP42" | b"MP41" | b"MPG3" | b"DVX3" | b"AP41" => {
            Codec::Other("msmpeg4".to_owned())
        }
        b"WMV1" | b"WMV2" | b"WMV3" | b"WVC1" | b"WMVA" => Codec::Other("wmv".to_owned()),
        _ => Codec::Other(String::from_utf8_lossy(&fourcc).trim().to_ascii_lowercase()),
    }
}

/// The audio codec of a `WAVEFORMATEX` format tag (AVI, Matroska `A_MS/ACM`).
pub(crate) fn wave_codec(tag: u16, bits: u16, extra: &[u8]) -> Codec {
    match tag {
        0x0001 => Codec::Pcm(PcmFormat {
            bits,
            float: false,
            big_endian: false,
        }),
        0x0003 => Codec::Pcm(PcmFormat {
            bits,
            float: true,
            big_endian: false,
        }),
        0x0050 | 0x0055 => Codec::MpegAudio,
        0x2000 => Codec::Ac3,
        0x00FF | 0x1600 | 0x1610 | 0x4143 | 0x706D => {
            Codec::Aac(if tag == 0x1600 { Vec::new() } else { extra.to_vec() })
        }
        0x0161..=0x0163 => Codec::Other("wma".to_owned()),
        other => Codec::Other(format!("audio format {other:#06x}")),
    }
}

/// What a track holds, before any decoding.
#[derive(Clone, Debug)]
pub(crate) struct TrackInfo {
    pub codec: Codec,
    /// Coded size in pixels (video), or zero when the container does not say.
    pub size: (u32, u32),
    /// Clockwise rotation in degrees.
    pub rotation: u32,
    /// Sample rate and channels (audio), or zero when the container does not say.
    pub rate: u32,
    pub channels: usize,
    /// Average frames per second (video).
    pub fps: Option<f64>,
    pub duration: Option<f64>,
    /// Packet times count packets in decode order rather than giving when each is shown
    /// (AVI, which has no presentation times).
    pub decode_order: bool,
}

impl TrackInfo {
    /// A track of `codec` with nothing else known yet.
    pub(crate) fn of(codec: Codec) -> Self {
        Self {
            codec,
            size: (0, 0),
            rotation: 0,
            rate: 0,
            channels: 0,
            fps: None,
            duration: None,
            decode_order: false,
        }
    }
}

/// One compressed frame.
pub(crate) struct Packet {
    pub data: Vec<u8>,
    pub pts: f64,
    pub key: bool,
}

#[derive(Clone, Copy, PartialEq)]
pub(crate) enum Kind {
    Video,
    Audio,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fourccs_name_their_codecs_in_any_case() {
        assert_eq!(fourcc_codec(*b"xvid", b"vol"), Codec::Mpeg4(b"vol".to_vec()));
        assert_eq!(fourcc_codec(*b"H264", &[]), Codec::H264(Vec::new()));
        assert_eq!(fourcc_codec(*b"MJPG", &[]), Codec::Mjpeg);
        assert_eq!(fourcc_codec(*b"WMV3", &[]).name(), "wmv");
        assert_eq!(fourcc_codec(*b"cvid", &[]).name(), "cvid");
    }

    #[test]
    fn wave_tags_name_their_codecs() {
        assert_eq!(wave_codec(0x55, 0, &[]), Codec::MpegAudio);
        assert!(wave_codec(1, 16, &[]).is_playable_audio());
        assert!(!wave_codec(0x161, 16, &[]).is_playable_audio());
    }
}
