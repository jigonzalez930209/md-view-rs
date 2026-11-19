//! Video decoders, all pure Rust: H.264 (`rust_h264`), HEVC (`rust_h265`), AV1
//! (`filmcraft-av1`), VP8 (`filmcraft-vp8`), VP9 (`oximedia-codec`), MPEG-1/2
//! (`filmcraft-mpeg2v`), MPEG-4 Part 2 and H.263 (`oxideav-mpeg4video`), and Motion JPEG
//! (`zune-jpeg`).
//!
//! Each decoder takes packets in decode order and returns pictures in display order, timed in
//! one of three ways (see [`Timing`]).

mod codecs;

use std::cmp::Reverse;
use std::collections::BinaryHeap;

use super::track::{Codec, Packet};
use crate::MediaError;

/// One plane of samples, 8-bit, or 16-bit little-endian when the picture is deeper.
pub(crate) struct Plane {
    pub data: Vec<u8>,
    pub stride: usize,
}

/// A decoded YUV picture.
pub(crate) struct Picture {
    pub width: u32,
    pub height: u32,
    pub planes: [Plane; 3],
    /// Chroma subsampling as shifts: `(1, 1)` for 4:2:0, `(1, 0)` for 4:2:2.
    pub chroma_shift: (u32, u32),
    /// Bits per sample above 8 (stored in two bytes); 8 otherwise.
    pub bit_depth: u8,
    /// Samples use the whole range (JPEG) rather than 16–235.
    pub full_range: bool,
    pub pts: f64,
}

/// Packets in, pictures out, with presentation times.
pub(crate) struct VideoDecoder {
    inner: codecs::Inner,
    timing: Timing,
    /// Setup the container keeps apart from the packets, put in front of the first one.
    prefix: Vec<u8>,
    pending: BinaryHeap<Reverse<Time>>,
    /// The time of the first key frame fed, once there is one.
    key: Option<f64>,
    /// Whether a picture has come out yet, and the packets fed before it did.
    started: bool,
    fed: usize,
    /// The latest time given out and the frame interval, for pictures without a packet time.
    last: f64,
    step: f64,
}

/// How pictures get their presentation times.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Timing {
    /// The decoder hands back the time of the packet each picture came in.
    Decoder,
    /// The packet times are presentation times but the decoder does not carry them: they
    /// are kept in a min-heap and the smallest goes to each picture that comes out, as
    /// display order is presentation-time order. Leading pictures that cannot be decoded
    /// after a seek (open GOPs) never come out, so times before the first key frame are
    /// dropped when the first picture does.
    Heap,
    /// The packet times count packets in decode order (AVI) and the frame rate is
    /// constant: pictures are one frame interval apart. MPEG-1/2/4 decoders hold back one
    /// reference picture, so a picture comes out one packet after it goes in, and every
    /// further packet fed before the first picture comes out is a leading B picture that
    /// could not be decoded after a seek: the key frame is shown that many frames later.
    Sequential,
}

/// An `f64` time ordered for the heap; times are finite.
#[derive(PartialEq)]
struct Time(f64);

impl Eq for Time {}

impl PartialOrd for Time {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Time {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.total_cmp(&other.0)
    }
}

/// More packet times than any decoder holds back for reordering; older ones belong to
/// packets that gave no picture and are dropped.
const MAX_PENDING: usize = 32;

impl VideoDecoder {
    /// A decoder for `codec`, playing at about `fps` frames per second. `decode_order`
    /// says the container's packet times count packets in decode order (AVI).
    pub(crate) fn new(codec: &Codec, fps: f64, decode_order: bool) -> Result<Self, MediaError> {
        let (inner, prefix) = codecs::Inner::new(codec)?;
        let timing = match (decode_order, inner.holds_one_anchor(), inner.carries_time()) {
            (true, true, _) => Timing::Sequential,
            (true, false, _) | (false, _, false) => Timing::Heap,
            (false, _, true) => Timing::Decoder,
        };
        Ok(Self {
            inner,
            timing,
            prefix,
            pending: BinaryHeap::new(),
            key: None,
            started: false,
            fed: 0,
            last: f64::NEG_INFINITY,
            step: 1.0 / fps.max(1.0),
        })
    }

    /// Decode one packet. A packet the codec rejects is skipped: the pictures that depend on
    /// it come out damaged until the next key frame instead of stopping playback.
    pub(crate) fn decode(&mut self, packet: &Packet) -> Vec<Picture> {
        if packet.key && self.key.is_none() {
            self.key = Some(packet.pts);
        }
        if self.key.is_some() {
            self.fed += 1;
        }
        if self.timing == Timing::Heap {
            self.pending.push(Reverse(Time(packet.pts)));
        }
        let pictures = if self.prefix.is_empty() {
            self.inner.decode(&packet.data, packet.pts)
        } else {
            let mut data = std::mem::take(&mut self.prefix);
            data.extend_from_slice(&packet.data);
            self.inner.decode(&data, packet.pts)
        };
        self.stamp(pictures)
    }

    /// The pictures still held for reordering, at the end of the stream.
    pub(crate) fn flush(&mut self) -> Vec<Picture> {
        let pictures = self.inner.flush();
        self.stamp(pictures)
    }

    fn stamp(&mut self, pictures: Vec<Option<Picture>>) -> Vec<Picture> {
        if self.timing == Timing::Decoder {
            return pictures.into_iter().flatten().collect();
        }
        while self.pending.len() > MAX_PENDING + pictures.len() {
            self.pending.pop();
        }
        pictures
            .into_iter()
            .filter_map(|picture| {
                if !self.started {
                    self.started = true;
                    let key = self.key.unwrap_or(f64::NEG_INFINITY);
                    while self.pending.peek().is_some_and(|Reverse(Time(pts))| *pts < key - 1e-6) {
                        self.pending.pop();
                    }
                    if self.timing == Timing::Sequential {
                        let skipped = self.fed.saturating_sub(2) as f64;
                        self.last = key + (skipped - 1.0) * self.step;
                    }
                }
                // A picture in a format we cannot show (or a hidden frame) still uses up its
                // time; a picture with no packet time left follows the one before.
                let pts = match self.pending.pop() {
                    Some(Reverse(Time(pts))) => pts,
                    None if self.last.is_finite() => self.last + self.step,
                    None => 0.0,
                };
                self.last = pts;
                let mut picture = picture?;
                picture.pts = pts;
                Some(picture)
            })
            .collect()
    }
}

/// The message for a codec MD View cannot decode.
pub(crate) fn unsupported(codec: &Codec) -> String {
    let name = match codec {
        Codec::Other(name) if name == "wmv" => "Windows Media",
        Codec::Other(name) if name == "msmpeg4" => "Microsoft MPEG-4 (DivX 3)",
        Codec::Other(name) if name == "flv1" => "Sorenson Spark (FLV1)",
        Codec::Other(name) => name.as_str(),
        _ => "this",
    };
    format!(
        "MD View cannot decode {name} video yet. It plays H.264, HEVC, AV1, VP8, VP9, MPEG-1, MPEG-2, MPEG-4 Part 2, H.263, and Motion JPEG video."
    )
}

/// Decoder threads: the cores there are, up to eight.
pub(crate) fn threads() -> usize {
    std::thread::available_parallelism().map_or(2, |count| count.get().clamp(1, 8))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn picture() -> Option<Picture> {
        let plane = || Plane {
            data: vec![0],
            stride: 1,
        };
        Some(Picture {
            width: 1,
            height: 1,
            planes: [plane(), plane(), plane()],
            chroma_shift: (0, 0),
            bit_depth: 8,
            full_range: false,
            pts: 0.0,
        })
    }

    #[test]
    fn pictures_take_the_smallest_pending_time() {
        let mut decoder = VideoDecoder::new(&Codec::H263, 25.0, false).expect("decoder");
        decoder.timing = Timing::Heap;
        decoder.key = Some(0.0);
        for pts in [0.0, 0.12, 0.04, 0.08] {
            decoder.pending.push(Reverse(Time(pts)));
        }
        let times: Vec<f64> = decoder
            .stamp(vec![picture(), None, picture(), picture()])
            .iter()
            .map(|picture| picture.pts)
            .collect();
        assert_eq!(times, vec![0.0, 0.08, 0.12], "the dropped picture used up 0.04");
        let extra = decoder.stamp(vec![picture()]);
        assert!(
            (extra[0].pts - 0.16).abs() < 1e-9,
            "a picture without a packet time follows"
        );
    }

    #[test]
    fn leading_pictures_that_never_come_out_give_up_their_times() {
        let mut decoder = VideoDecoder::new(&Codec::H263, 25.0, false).expect("decoder");
        decoder.timing = Timing::Heap;
        decoder.key = Some(1.0);
        // The key frame at 1.0 is fed first; the two pictures shown before it cannot be
        // decoded after a seek, so only the key frame and what follows come out.
        for pts in [1.0, 0.92, 0.96, 1.04] {
            decoder.pending.push(Reverse(Time(pts)));
        }
        let times: Vec<f64> = decoder
            .stamp(vec![picture(), picture()])
            .iter()
            .map(|p| p.pts)
            .collect();
        assert_eq!(times, vec![1.0, 1.04]);
    }

    #[test]
    fn decode_order_times_count_frames_from_the_key_frame() {
        let mut decoder = VideoDecoder::new(&Codec::H263, 25.0, true).expect("decoder");
        assert_eq!(decoder.timing, Timing::Sequential);
        decoder.key = Some(0.88);
        // Key frame, two leading B pictures that cannot be decoded, then the next anchor,
        // which lets the key frame out: it is shown two frames after its packet time.
        decoder.fed = 4;
        let times: Vec<f64> = decoder
            .stamp(vec![picture(), None, picture()])
            .iter()
            .map(|p| p.pts)
            .collect();
        assert!(
            (times[0] - 0.96).abs() < 1e-9 && (times[1] - 1.04).abs() < 1e-9,
            "{times:?}"
        );
    }
}
