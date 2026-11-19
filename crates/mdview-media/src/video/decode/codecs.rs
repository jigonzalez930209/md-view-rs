//! One adapter per video codec: setup from the container's codec record, packets in, and
//! pictures converted to [`Picture`] planes.

use oximedia_codec::traits::{DecoderConfig, VideoDecoder as _};

use super::{Picture, Plane, threads};
use crate::MediaError;
use crate::video::track::Codec;

pub(super) enum Inner {
    H264 {
        decoder: Box<rust_h264::threading::ThreadedDecoder>,
        /// Bytes of each NAL unit's length prefix, or 0 for start codes (Annex B).
        length_size: usize,
    },
    Hevc {
        decoder: Box<rust_h265::Decoder>,
        length_size: u8,
    },
    Av1(Box<filmcraft_av1::Decoder>),
    Vp8(Box<filmcraft_vp8::Decoder>),
    Vp9(Box<oximedia_codec::vp9::Vp9Decoder>),
    Mpeg12(Box<filmcraft_mpeg2v::Decoder>),
    /// MPEG-4 Part 2, and H.263 (its short-header subset).
    Mpeg4(Box<oxideav_mpeg4video::decoder::Mpeg4VideoDecoder>),
    Mjpeg,
}

fn setup_error(codec: &str) -> impl Fn(String) -> MediaError + '_ {
    move |error| MediaError::Decode(format!("{codec} setup: {error}"))
}

impl Inner {
    /// The decoder for `codec`, and bytes to put in front of its first packet.
    pub(super) fn new(codec: &Codec) -> Result<(Self, Vec<u8>), MediaError> {
        let inner = match codec {
            Codec::H264(config) => {
                let mut decoder = rust_h264::threading::ThreadedDecoder::new(threads());
                let mut length_size = 0;
                if !config.is_empty() {
                    let record = rust_h264::nal::parse_avcc_config(config)
                        .map_err(|error| setup_error("H.264")(error.to_owned()))?;
                    for nal in record.sps_nals.iter().chain(&record.pps_nals) {
                        decoder
                            .decode_nal(nal)
                            .map_err(|error| setup_error("H.264")(format!("{error:?}")))?;
                    }
                    length_size = record.length_size;
                }
                Self::H264 {
                    decoder: Box::new(decoder),
                    length_size,
                }
            }
            Codec::Hevc(config) => {
                let mut decoder = rust_h265::Decoder::new().with_display_order().with_threads(threads());
                let mut length_size = 0;
                if !config.is_empty() {
                    let (size, nals) = hvcc_nals(config).ok_or_else(|| setup_error("HEVC")("bad hvcC".to_owned()))?;
                    for nal in nals.iter().filter_map(|nal| rust_h265::parse_nal(nal)) {
                        decoder
                            .decode_nal(&nal)
                            .map_err(|error| setup_error("HEVC")(format!("{error:?}")))?;
                    }
                    length_size = size;
                }
                Self::Hevc {
                    decoder: Box::new(decoder),
                    length_size,
                }
            }
            Codec::Av1(config) => {
                let decoder = filmcraft_av1::Decoder::with_threads(threads());
                return Ok((Self::Av1(Box::new(decoder)), config.clone()));
            }
            Codec::Vp8 => Self::Vp8(Box::new(filmcraft_vp8::Decoder::new())),
            Codec::Vp9 => {
                let config = DecoderConfig {
                    codec: oximedia_core::CodecId::Vp9,
                    threads: 0,
                    ..DecoderConfig::default()
                };
                let decoder = oximedia_codec::vp9::Vp9Decoder::new(config)
                    .map_err(|error| setup_error("VP9")(error.to_string()))?;
                Self::Vp9(Box::new(decoder))
            }
            Codec::Mpeg12(header) => {
                let mut decoder = filmcraft_mpeg2v::Decoder::new();
                decoder.set_threads(true);
                return Ok((Self::Mpeg12(Box::new(decoder)), header.clone()));
            }
            Codec::Mpeg4(config) => {
                let decoder = oxideav_mpeg4video::decoder::Mpeg4VideoDecoder::new();
                return Ok((Self::Mpeg4(Box::new(decoder)), config.clone()));
            }
            Codec::H263 => Self::Mpeg4(Box::new(oxideav_mpeg4video::decoder::Mpeg4VideoDecoder::new())),
            Codec::Mjpeg => Self::Mjpeg,
            other => return Err(MediaError::Decode(super::unsupported(other))),
        };
        Ok((inner, Vec::new()))
    }

    /// Whether the pictures carry the time of the packet they came in (see
    /// [`Timing`](super::Timing)).
    pub(super) fn carries_time(&self) -> bool {
        matches!(
            self,
            Self::Av1(_) | Self::Vp8(_) | Self::Mpeg12(_) | Self::Mpeg4(_) | Self::Mjpeg
        )
    }

    /// Whether the decoder holds back exactly one reference picture for reordering (see
    /// [`Timing::Sequential`](super::Timing::Sequential)).
    pub(super) fn holds_one_anchor(&self) -> bool {
        matches!(self, Self::Mpeg12(_) | Self::Mpeg4(_))
    }

    /// Decode one packet; `None` stands for a picture that cannot be shown (a hidden frame,
    /// or a format the converter does not handle).
    pub(super) fn decode(&mut self, data: &[u8], pts: f64) -> Vec<Option<Picture>> {
        let micros = (pts * 1e6) as i64;
        match self {
            Self::H264 { decoder, length_size } => {
                let nals = if *length_size == 0 {
                    rust_h264::nal::parse_annex_b(data)
                } else {
                    rust_h264::nal::parse_avcc(data, *length_size)
                };
                let mut frames = Vec::new();
                for nal in nals {
                    match decoder.decode_nal(&nal) {
                        Ok(out) => frames.extend(out),
                        Err(error) => log::debug!("H.264 packet at {pts:.3}: {error:?}"),
                    }
                }
                frames.into_iter().map(from_h264).collect()
            }
            Self::Hevc { decoder, length_size } => {
                let nals = if *length_size == 0 {
                    rust_h265::parse_annex_b(data)
                } else {
                    rust_h265::parse_hvcc(data, *length_size)
                };
                for nal in nals {
                    if let Err(error) = decoder.decode_nal(&nal) {
                        log::debug!("HEVC packet at {pts:.3}: {error:?}");
                    }
                }
                std::iter::from_fn(|| decoder.next_output()).map(from_h265).collect()
            }
            Self::Av1(decoder) => match decoder.decode_pts(data, micros) {
                Ok(pictures) => pictures.into_iter().map(from_av1).collect(),
                Err(error) => {
                    log::debug!("AV1 packet at {pts:.3}: {error}");
                    Vec::new()
                }
            },
            Self::Vp8(decoder) => match decoder.decode(data, micros) {
                Ok(picture) => vec![picture.map(from_vp8)],
                Err(error) => {
                    log::debug!("VP8 packet at {pts:.3}: {error}");
                    vec![None]
                }
            },
            Self::Vp9(decoder) => {
                if let Err(error) = decoder.send_packet(data, 0) {
                    log::debug!("VP9 packet at {pts:.3}: {error}");
                }
                drain_vp9(decoder)
            }
            Self::Mpeg12(decoder) => match decoder.decode(data, micros) {
                Ok(pictures) => pictures.into_iter().map(from_mpeg12).collect(),
                Err(error) => {
                    log::debug!("MPEG video packet at {pts:.3}: {error}");
                    Vec::new()
                }
            },
            Self::Mpeg4(decoder) => match decoder.decode_with_pts(data, Some(micros)) {
                Ok(frames) => {
                    let size = display_size(decoder);
                    frames.iter().map(|frame| from_mpeg4(frame, size)).collect()
                }
                Err(error) => {
                    log::debug!("MPEG-4 packet at {pts:.3}: {error:?}");
                    Vec::new()
                }
            },
            Self::Mjpeg => vec![from_jpeg(data).map(|picture| timed(picture, micros))],
        }
    }

    /// The pictures still held for reordering, at the end of the stream.
    pub(super) fn flush(&mut self) -> Vec<Option<Picture>> {
        match self {
            Self::H264 { decoder, .. } => decoder.flush().into_iter().map(from_h264).collect(),
            Self::Hevc { decoder, .. } => std::iter::from_fn(|| decoder.flush()).map(from_h265).collect(),
            Self::Av1(decoder) => decoder.flush().into_iter().map(from_av1).collect(),
            Self::Vp9(decoder) => {
                let _ = decoder.flush();
                drain_vp9(decoder)
            }
            Self::Mpeg12(decoder) => decoder.flush().into_iter().map(from_mpeg12).collect(),
            Self::Mpeg4(decoder) => {
                let size = display_size(decoder);
                decoder.flush().iter().map(|frame| from_mpeg4(frame, size)).collect()
            }
            Self::Vp8(_) | Self::Mjpeg => Vec::new(),
        }
    }
}

/// The NAL length size and parameter set NAL units of an `hvcC` record.
fn hvcc_nals(record: &[u8]) -> Option<(u8, Vec<&[u8]>)> {
    let length_size = (record.get(21)? & 3) + 1;
    let arrays = *record.get(22)?;
    let mut position = 23;
    let mut nals = Vec::new();
    for _ in 0..arrays {
        let count = u16::from_be_bytes([*record.get(position + 1)?, *record.get(position + 2)?]);
        position += 3;
        for _ in 0..count {
            let length = usize::from(u16::from_be_bytes([*record.get(position)?, *record.get(position + 1)?]));
            nals.push(record.get(position + 2..position + 2 + length)?);
            position += 2 + length;
        }
    }
    Some((length_size, nals))
}

/// Planes from tight 8-bit sample buffers.
fn tight(width: u32, height: u32, chroma_shift: (u32, u32), planes: [Vec<u8>; 3]) -> Picture {
    let chroma_width = width.div_ceil(1 << chroma_shift.0) as usize;
    let [y, u, v] = planes;
    Picture {
        width,
        height,
        planes: [
            Plane {
                data: y,
                stride: width as usize,
            },
            Plane {
                data: u,
                stride: chroma_width,
            },
            Plane {
                data: v,
                stride: chroma_width,
            },
        ],
        chroma_shift,
        bit_depth: 8,
        full_range: false,
        pts: 0.0,
    }
}

/// Microseconds, as the time-carrying decoders are given them, to seconds.
fn seconds(micros: i64) -> f64 {
    micros as f64 / 1e6
}

fn timed(mut picture: Picture, micros: i64) -> Picture {
    picture.pts = seconds(micros);
    picture
}

fn wide_bytes(samples: &[u16]) -> Vec<u8> {
    samples.iter().flat_map(|sample| sample.to_le_bytes()).collect()
}

fn from_h264(frame: rust_h264::decoder::Frame) -> Option<Picture> {
    let wide = frame.bytes_per_sample();
    let chroma_height = frame.chroma_height().max(1) as usize;
    let chroma_stride = frame.u.len() / chroma_height;
    let chroma_shift = match frame.chroma_format {
        rust_h264::decoder::ChromaFormat::Yuv422 => (1, 0),
        _ => (1, 1),
    };
    let mut picture = tight(frame.width, frame.height, chroma_shift, [frame.y, frame.u, frame.v]);
    picture.planes[0].stride = frame.width as usize * wide;
    picture.planes[1].stride = chroma_stride;
    picture.planes[2].stride = chroma_stride;
    picture.bit_depth = frame.bit_depth;
    Some(picture)
}

fn from_h265(frame: rust_h265::Frame) -> Option<Picture> {
    let chroma_shift = match frame.chroma_format_idc {
        1 => (1, 1),
        2 => (1, 0),
        3 => (0, 0),
        _ => return None,
    };
    let bytes = |data: rust_h265::PixelData| match data {
        rust_h265::PixelData::U8(samples) => samples,
        rust_h265::PixelData::U16(samples) => wide_bytes(&samples),
    };
    let wide = if frame.bit_depth > 8 { 2 } else { 1 };
    let (width, height, bit_depth) = (frame.width, frame.height, frame.bit_depth);
    let mut picture = tight(
        width,
        height,
        chroma_shift,
        [bytes(frame.y), bytes(frame.u), bytes(frame.v)],
    );
    for plane in &mut picture.planes {
        plane.stride *= wide;
    }
    picture.bit_depth = bit_depth;
    Some(picture)
}

fn from_av1(picture: filmcraft_av1::Picture) -> Option<Picture> {
    let chroma_shift = (u32::from(picture.subsampling_x), u32::from(picture.subsampling_y));
    let wide = picture.bit_depth > 8;
    let plane = |index: usize| -> Plane {
        let width = picture.plane_width(index) as usize;
        let samples = &picture.planes[index];
        if index > 0 && picture.mono_chrome {
            // No chroma: mid-grey at the subsampled size.
            let height = picture.plane_height(index) as usize;
            let middle = 1u16 << (picture.bit_depth - 1);
            let grey = vec![middle; width * height];
            return Plane {
                data: if wide {
                    wide_bytes(&grey)
                } else {
                    grey.iter().map(|&v| v as u8).collect()
                },
                stride: width * if wide { 2 } else { 1 },
            };
        }
        Plane {
            data: if wide {
                wide_bytes(samples)
            } else {
                samples.iter().map(|&v| v as u8).collect()
            },
            stride: width * if wide { 2 } else { 1 },
        }
    };
    Some(Picture {
        width: picture.width,
        height: picture.height,
        planes: [plane(0), plane(1), plane(2)],
        chroma_shift,
        bit_depth: picture.bit_depth,
        full_range: picture.full_range,
        pts: seconds(picture.pts),
    })
}

fn from_vp8(picture: filmcraft_vp8::Picture) -> Picture {
    let pts = picture.pts;
    timed(
        tight(picture.width, picture.height, (1, 1), [picture.y, picture.u, picture.v]),
        pts,
    )
}

fn from_mpeg12(picture: filmcraft_mpeg2v::Picture) -> Option<Picture> {
    let chroma_shift = match picture.chroma {
        filmcraft_mpeg2v::ChromaFormat::Yuv420 => (1, 1),
        filmcraft_mpeg2v::ChromaFormat::Yuv422 => (1, 0),
        filmcraft_mpeg2v::ChromaFormat::Yuv444 => (0, 0),
    };
    let pts = picture.pts;
    Some(timed(
        tight(
            picture.width,
            picture.height,
            chroma_shift,
            [picture.y, picture.cb, picture.cr],
        ),
        pts,
    ))
}

/// The shown size of MPEG-4 pictures: the VOL's, or the coded one for H.263.
fn display_size(decoder: &oxideav_mpeg4video::decoder::Mpeg4VideoDecoder) -> Option<(u32, u32)> {
    decoder.vol().map(|vol| (u32::from(vol.width), u32::from(vol.height)))
}

fn from_mpeg4(frame: &oxideav_mpeg4video::framestore::DecodedFrame, size: Option<(u32, u32)>) -> Option<Picture> {
    let stride = frame.width();
    let (width, height) = size.unwrap_or((stride as u32, frame.height() as u32));
    let mut picture = tight(
        width.min(stride as u32),
        height.min(frame.height() as u32),
        (1, 1),
        [
            frame.luma_samples().to_vec(),
            frame.cb_samples().to_vec(),
            frame.cr_samples().to_vec(),
        ],
    );
    picture.planes[0].stride = stride;
    picture.planes[1].stride = stride / 2;
    picture.planes[2].stride = stride / 2;
    picture.pts = seconds(frame.pts().unwrap_or_default());
    Some(picture)
}

fn from_jpeg(data: &[u8]) -> Option<Picture> {
    use zune_core::bytestream::ZCursor;
    use zune_core::colorspace::ColorSpace;
    use zune_core::options::DecoderOptions;
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::YCbCr);
    let mut decoder = zune_jpeg::JpegDecoder::new_with_options(ZCursor::new(data), options);
    let pixels = decoder
        .decode()
        .map_err(|error| log::debug!("Motion JPEG frame: {error:?}"))
        .ok()?;
    let info = decoder.info()?;
    let (width, height) = (u32::from(info.width), u32::from(info.height));
    let count = (width * height) as usize;
    let channels = pixels.len() / count.max(1);
    let mut planes = [Vec::with_capacity(count), vec![128; count], vec![128; count]];
    if channels >= 3 {
        planes[1].clear();
        planes[2].clear();
        for pixel in pixels.chunks_exact(channels) {
            planes[0].push(pixel[0]);
            planes[1].push(pixel[1]);
            planes[2].push(pixel[2]);
        }
    } else {
        planes[0] = pixels.iter().step_by(channels.max(1)).copied().collect();
    }
    let mut picture = tight(width, height, (0, 0), planes);
    picture.full_range = true;
    Some(picture)
}

fn drain_vp9(decoder: &mut oximedia_codec::vp9::Vp9Decoder) -> Vec<Option<Picture>> {
    use oximedia_core::PixelFormat;
    let mut pictures = Vec::new();
    while let Ok(Some(frame)) = decoder.receive_frame() {
        let (chroma_shift, bit_depth) = match frame.format {
            PixelFormat::Yuv420p => ((1, 1), 8),
            PixelFormat::Yuv422p => ((1, 0), 8),
            PixelFormat::Yuv444p => ((0, 0), 8),
            PixelFormat::Yuv420p10le => ((1, 1), 10),
            _ => {
                pictures.push(None);
                continue;
            }
        };
        let mut planes = frame.planes.into_iter().map(|plane| Plane {
            data: plane.data,
            stride: plane.stride,
        });
        let (Some(y), Some(u), Some(v)) = (planes.next(), planes.next(), planes.next()) else {
            pictures.push(None);
            continue;
        };
        pictures.push(Some(Picture {
            width: frame.width,
            height: frame.height,
            planes: [y, u, v],
            chroma_shift,
            bit_depth,
            full_range: false,
            pts: 0.0,
        }));
    }
    pictures
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hvcc_records_give_their_parameter_sets() {
        let mut record = vec![0u8; 21];
        record.extend([0xFF, 2]); // four-byte lengths, two arrays
        record.extend([0x20, 0, 1, 0, 2, 0x40, 0x01]); // VPS array: one NAL of two bytes
        record.extend([0x21, 0, 1, 0, 1, 0x42]); // SPS array: one NAL of one byte
        let (length_size, nals) = hvcc_nals(&record).expect("parsed");
        assert_eq!(length_size, 4);
        assert_eq!(nals, vec![&[0x40, 0x01][..], &[0x42][..]]);
        assert!(hvcc_nals(&record[..25]).is_none(), "truncated");
    }
}
