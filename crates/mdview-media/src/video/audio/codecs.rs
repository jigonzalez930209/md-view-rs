//! Audio decoders, all pure Rust: AAC, MP1/MP2/MP3, Vorbis, and FLAC with Symphonia, Opus
//! with `opus-decoder`, AC-3 with `filmcraft-ac3`, and uncompressed PCM. Codecs whose frames
//! a container may split or pack together (MPEG audio, ADTS AAC, AC-3) are re-framed from
//! their frame headers first.

use symphonia_core::codecs::audio::well_known::{
    CODEC_ID_AAC, CODEC_ID_FLAC, CODEC_ID_MP1, CODEC_ID_MP2, CODEC_ID_MP3, CODEC_ID_VORBIS,
};
use symphonia_core::codecs::audio::{AudioCodecParameters, AudioDecoder as Symphonia, AudioDecoderOptions};
use symphonia_core::packet::Packet;
use symphonia_core::units::{Duration, Timestamp};

use crate::MediaError;
use crate::video::track::{Codec, PcmFormat, TrackInfo};

/// Decoded interleaved samples.
pub(super) struct Decoded {
    pub samples: Vec<f32>,
    pub rate: u32,
    pub channels: usize,
}

pub(super) struct AudioDecoder {
    kind: Kind,
    /// Bytes of a frame that continues in the next packet.
    pending: Vec<u8>,
}

enum Kind {
    /// A Symphonia decoder for whole packets (AAC, Vorbis, FLAC).
    Symphonia(Box<dyn Symphonia>),
    /// MPEG audio, re-framed; the decoder is made for the layer of the first frame.
    Mpa(Option<(u8, Box<dyn Symphonia>)>),
    /// AAC in ADTS frames: the decoder is made from the first frame header.
    Adts(Option<Box<dyn Symphonia>>),
    Opus(Box<opus_decoder::OpusDecoder>, Vec<f32>, usize),
    Ac3(Box<filmcraft_ac3::Decoder>),
    Pcm {
        format: PcmFormat,
        rate: u32,
        channels: usize,
    },
}

fn symphonia(
    codec: symphonia_core::codecs::audio::AudioCodecId,
    extra: &[u8],
) -> Result<Box<dyn Symphonia>, MediaError> {
    let mut params = AudioCodecParameters::new();
    params.for_codec(codec);
    if !extra.is_empty() {
        params.with_extra_data(extra.to_vec().into_boxed_slice());
    }
    let options = AudioDecoderOptions::default();
    let decoder: Result<Box<dyn Symphonia>, _> = match codec {
        CODEC_ID_AAC => symphonia_codec_aac::AacDecoder::try_new(&params, &options).map(|d| Box::new(d) as _),
        CODEC_ID_MP1 | CODEC_ID_MP2 | CODEC_ID_MP3 => {
            symphonia_bundle_mp3::MpaDecoder::try_new(&params, &options).map(|d| Box::new(d) as _)
        }
        CODEC_ID_VORBIS => symphonia_codec_vorbis::VorbisDecoder::try_new(&params, &options).map(|d| Box::new(d) as _),
        _ => symphonia_bundle_flac::FlacDecoder::try_new(&params, &options).map(|d| Box::new(d) as _),
    };
    decoder.map_err(|error| MediaError::Decode(format!("Audio setup: {error}")))
}

impl AudioDecoder {
    /// A decoder for the track, or `None` when MD View cannot decode its codec.
    pub(super) fn new(info: &TrackInfo) -> Result<Option<Self>, MediaError> {
        let kind = match &info.codec {
            Codec::Aac(config) if config.is_empty() => Kind::Adts(None),
            Codec::Aac(config) => Kind::Symphonia(symphonia(CODEC_ID_AAC, config)?),
            Codec::MpegAudio => Kind::Mpa(None),
            Codec::Vorbis(headers) => Kind::Symphonia(symphonia(CODEC_ID_VORBIS, headers)?),
            Codec::Flac(info) => Kind::Symphonia(symphonia(CODEC_ID_FLAC, info)?),
            Codec::Opus { .. } => {
                let channels = info.channels.clamp(1, 2);
                let decoder = opus_decoder::OpusDecoder::new(48_000, channels)
                    .map_err(|error| MediaError::Decode(format!("Opus: {error:?}")))?;
                Kind::Opus(Box::new(decoder), vec![0.0; 5760 * channels], channels)
            }
            Codec::Ac3 => Kind::Ac3(Box::new(filmcraft_ac3::Decoder::new())),
            Codec::Pcm(format) if info.rate > 0 && info.channels > 0 && [8, 16, 24, 32, 64].contains(&format.bits) => {
                Kind::Pcm {
                    format: *format,
                    rate: info.rate,
                    channels: info.channels,
                }
            }
            _ => return Ok(None),
        };
        Ok(Some(Self {
            kind,
            pending: Vec::new(),
        }))
    }

    /// Decode a packet; damaged data is skipped (logged) rather than stopping the sound.
    pub(super) fn decode(&mut self, data: Vec<u8>, at: f64) -> Option<Decoded> {
        let mut out: Option<Decoded> = None;
        // Frames of one packet share a format; a change starts over with the new one.
        let mut add = |decoded: Option<Decoded>| match (&mut out, decoded) {
            (Some(all), Some(decoded)) if all.rate == decoded.rate && all.channels == decoded.channels => {
                all.samples.extend(decoded.samples);
            }
            (_, Some(decoded)) => out = Some(decoded),
            (_, None) => {}
        };
        match &mut self.kind {
            Kind::Symphonia(decoder) => add(run(decoder.as_mut(), data, at)),
            Kind::Mpa(decoder) => {
                for frame in frames(&mut self.pending, &data, mpa_frame_length) {
                    let layer = 4 - ((frame[1] >> 1) & 3);
                    if decoder.as_ref().is_none_or(|(current, _)| *current != layer) {
                        let codec = [CODEC_ID_MP1, CODEC_ID_MP2, CODEC_ID_MP3][usize::from(layer.clamp(1, 3) - 1)];
                        *decoder = symphonia(codec, &[]).ok().map(|made| (layer, made));
                    }
                    if let Some((_, decoder)) = decoder.as_mut() {
                        add(run(decoder.as_mut(), frame, at));
                    }
                }
            }
            Kind::Adts(decoder) => {
                for frame in frames(&mut self.pending, &data, adts_frame_length) {
                    let header = if frame[1] & 1 == 1 { 7 } else { 9 };
                    if decoder.is_none() {
                        // AudioSpecificConfig: object type, rate index, channel layout.
                        let config = (u16::from((frame[2] >> 6) + 1) << 11)
                            | (u16::from((frame[2] >> 2) & 0x0F) << 7)
                            | (u16::from(((frame[2] & 1) << 2) | (frame[3] >> 6)) << 3);
                        *decoder = symphonia(CODEC_ID_AAC, &config.to_be_bytes()).ok();
                    }
                    if let (Some(decoder), Some(payload)) = (decoder.as_mut(), frame.get(header..)) {
                        add(run(decoder.as_mut(), payload.to_vec(), at));
                    }
                }
            }
            Kind::Opus(decoder, buffer, channels) => match decoder.decode_float(&data, buffer, false) {
                Ok(count) => add(Some(Decoded {
                    samples: buffer[..count * *channels].to_vec(),
                    rate: 48_000,
                    channels: *channels,
                })),
                Err(error) => log::debug!("Opus packet at {at:.3}: {error:?}"),
            },
            Kind::Ac3(decoder) => {
                for frame in frames(&mut self.pending, &data, ac3_frame_length) {
                    match decoder.decode(&frame) {
                        Ok(decoded) => add(Some(interleave(&decoded.channels, decoded.header.sample_rate))),
                        Err(error) => log::debug!("AC-3 frame at {at:.3}: {error}"),
                    }
                }
            }
            Kind::Pcm { format, rate, channels } => add(Some(Decoded {
                samples: pcm(&data, *format),
                rate: *rate,
                channels: *channels,
            })),
        }
        out
    }
}

fn run(decoder: &mut dyn Symphonia, data: Vec<u8>, at: f64) -> Option<Decoded> {
    let packet = Packet::new(0, Timestamp::new(0), Duration::new(0), data);
    match decoder.decode(&packet) {
        Ok(buffer) => {
            let (rate, channels) = (buffer.spec().rate(), buffer.spec().channels().count());
            let mut samples: Vec<f32> = Vec::new();
            buffer.copy_to_vec_interleaved(&mut samples);
            Some(Decoded {
                samples,
                rate,
                channels: channels.max(1),
            })
        }
        Err(error) => {
            log::debug!("Audio packet at {at:.3}: {error}");
            None
        }
    }
}

fn interleave(planes: &[Vec<f32>], rate: u32) -> Decoded {
    let frames = planes.first().map_or(0, Vec::len);
    let mut samples = Vec::with_capacity(frames * planes.len());
    for index in 0..frames {
        samples.extend(planes.iter().map(|plane| plane.get(index).copied().unwrap_or(0.0)));
    }
    Decoded {
        samples,
        rate,
        channels: planes.len().max(1),
    }
}

/// Whole frames from `pending` plus `data`, found by their headers; what is left of a frame
/// that continues in the next packet stays in `pending`. Bytes between frames are skipped.
fn frames(pending: &mut Vec<u8>, data: &[u8], length: fn(&[u8]) -> Option<usize>) -> Vec<Vec<u8>> {
    pending.extend_from_slice(data);
    let mut found = Vec::new();
    let mut position = 0;
    while position + 8 <= pending.len() {
        match length(&pending[position..]) {
            Some(size) if size > 0 => {
                if position + size > pending.len() {
                    break;
                }
                found.push(pending[position..position + size].to_vec());
                position += size;
            }
            _ => position += 1,
        }
    }
    pending.drain(..position);
    found
}

/// The length of the MPEG audio frame (layer I, II, or III) starting at `header`.
fn mpa_frame_length(header: &[u8]) -> Option<usize> {
    const MPEG1: [[u32; 15]; 3] = [
        [0, 32, 64, 96, 128, 160, 192, 224, 256, 288, 320, 352, 384, 416, 448],
        [0, 32, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320, 384],
        [0, 32, 40, 48, 56, 64, 80, 96, 112, 128, 160, 192, 224, 256, 320],
    ];
    const MPEG2: [[u32; 15]; 2] = [
        [0, 32, 48, 56, 64, 80, 96, 112, 128, 144, 160, 176, 192, 224, 256],
        [0, 8, 16, 24, 32, 40, 48, 56, 64, 80, 96, 112, 128, 144, 160],
    ];
    let &[first, second, third, ..] = header else {
        return None;
    };
    if first != 0xFF || second & 0xE0 != 0xE0 {
        return None;
    }
    let version = (second >> 3) & 3; // 3: MPEG-1, 2: MPEG-2, 0: MPEG-2.5
    let layer = 4 - ((second >> 1) & 3); // 1, 2, or 3; 4 is reserved
    let (bitrate, rate) = (usize::from(third >> 4), usize::from((third >> 2) & 3));
    if version == 1 || layer == 4 || bitrate == 0 || bitrate == 15 || rate == 3 {
        return None;
    }
    let kbps = if version == 3 {
        MPEG1[usize::from(layer - 1)][bitrate]
    } else {
        MPEG2[usize::from(layer != 1)][bitrate]
    };
    let rate = [44_100, 48_000, 32_000][rate] >> [2, 0, 1, 0][usize::from(version)];
    let padding = u32::from((third >> 1) & 1);
    let bytes = match layer {
        1 => (12 * kbps * 1000 / rate + padding) * 4,
        3 if version != 3 => 72 * kbps * 1000 / rate + padding,
        _ => 144 * kbps * 1000 / rate + padding,
    };
    Some(bytes as usize)
}

/// The length of the ADTS frame starting at `header`.
fn adts_frame_length(header: &[u8]) -> Option<usize> {
    let &[0xFF, second, _, fourth, fifth, sixth, ..] = header else {
        return None;
    };
    if second & 0xF6 != 0xF0 {
        return None;
    }
    let length = (usize::from(fourth & 3) << 11) | (usize::from(fifth) << 3) | usize::from(sixth >> 5);
    (length > 9).then_some(length)
}

fn ac3_frame_length(header: &[u8]) -> Option<usize> {
    filmcraft_ac3::parse_header(header)
        .ok()
        .map(|header| header.frame_bytes)
}

/// Interleaved PCM as `f32` in −1..1. Eight-bit samples are unsigned, as in WAV.
fn pcm(data: &[u8], format: PcmFormat) -> Vec<f32> {
    let width = usize::from(format.bits / 8);
    data.chunks_exact(width)
        .map(|sample| {
            let mut bytes = [0u8; 8];
            for (index, byte) in sample.iter().enumerate() {
                // Little-endian order in `bytes`, whatever the source order.
                let target = if format.big_endian { width - 1 - index } else { index };
                bytes[target] = *byte;
            }
            match (format.float, width) {
                (true, 4) => f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]),
                (true, _) => f64::from_le_bytes(bytes) as f32,
                (false, 1) => (f32::from(bytes[0]) - 128.0) / 128.0,
                (false, _) => {
                    // Sign-extend from the top byte.
                    let shift = 64 - 8 * width as u32;
                    let value = (i64::from_le_bytes(bytes) << shift) >> shift;
                    value as f32 / (1u64 << (8 * width - 1)) as f32
                }
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mpeg_audio_frame_lengths_follow_the_header() {
        // MPEG-1 layer III, 128 kbit/s, 44.1 kHz, no padding: 417 bytes.
        assert_eq!(mpa_frame_length(&[0xFF, 0xFB, 0x90, 0x00]), Some(417));
        // MPEG-1 layer II, 192 kbit/s, 48 kHz: 576 bytes.
        assert_eq!(mpa_frame_length(&[0xFF, 0xFD, 0xA4, 0x00]), Some(576));
        // MPEG-2 layer III, 64 kbit/s, 24 kHz: 192 bytes.
        assert_eq!(mpa_frame_length(&[0xFF, 0xF3, 0x84, 0x00]), Some(192));
        assert_eq!(mpa_frame_length(&[0xFF, 0xFB, 0xF0, 0x00]), None, "bad bit rate");
        assert_eq!(mpa_frame_length(&[0x00, 0xFB, 0x90, 0x00]), None, "no sync");
    }

    #[test]
    fn frames_are_cut_across_packets() {
        let length = |header: &[u8]| (header[0] == 0xAA).then_some(10);
        let mut pending = Vec::new();
        let mut data = vec![0xAA; 10];
        data.extend([1, 2, 0xAA, 0, 0, 0]);
        assert_eq!(frames(&mut pending, &data, length).len(), 1);
        assert_eq!(pending.len(), 6, "garbage dropped, the started frame kept");
        let found = frames(&mut pending, &[0; 10], length);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].len(), 10);
    }

    #[test]
    fn pcm_samples_scale_to_unit_range() {
        let little = PcmFormat {
            bits: 16,
            float: false,
            big_endian: false,
        };
        assert_eq!(pcm(&[0x00, 0x80, 0xFF, 0x7F], little), vec![-1.0, 32_767.0 / 32_768.0]);
        let big = PcmFormat {
            big_endian: true,
            ..little
        };
        assert_eq!(pcm(&[0xC0, 0x00], big), vec![-0.5]);
        let unsigned = PcmFormat { bits: 8, ..little };
        assert_eq!(pcm(&[0, 128], unsigned), vec![-1.0, 0.0]);
        let float = PcmFormat {
            bits: 32,
            float: true,
            big_endian: false,
        };
        assert_eq!(pcm(&0.25f32.to_le_bytes(), float), vec![0.25]);
    }
}
