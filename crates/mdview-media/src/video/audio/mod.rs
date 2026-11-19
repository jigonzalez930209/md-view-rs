//! A video's soundtrack as interleaved `f32` PCM at the output device's rate and channels.
//!
//! The decoded audio is mapped to the device's channel count and resampled linearly; samples
//! before the start position (seek pre-roll, AAC priming, Opus pre-skip) are dropped.

mod codecs;

use std::path::Path;

use super::demux::Demuxer;
use super::guarded;
use super::track::{Codec, Kind};
use crate::MediaError;
use codecs::AudioDecoder;

/// Decodes a soundtrack from a start time, in chunks ready for the output device.
pub struct AudioReader {
    demuxer: Demuxer,
    decoder: AudioDecoder,
    /// Seconds to subtract from the first packet's time: the Opus pre-skip.
    delay: f64,
    /// Presentation time of the next decoded source frame, once the first packet is read.
    clock: Option<f64>,
    start: f64,
    /// The output device's rate and channels.
    device: (u32, usize),
    /// The resampler for the source format decoded last.
    resampler: Option<(u32, Resampler)>,
}

impl AudioReader {
    /// Open the soundtrack of `path` at `start` seconds for a device playing `rate` Hz with
    /// `channels` channels. `Ok(None)` when there is no soundtrack MD View can decode.
    pub fn open(path: &Path, start: f64, rate: u32, channels: u16) -> Result<Option<Self>, MediaError> {
        let Some((mut demuxer, info)) = guarded("container reader", || Demuxer::open(path, Kind::Audio))?? else {
            return Ok(None);
        };
        let Some(decoder) = AudioDecoder::new(&info)? else {
            return Ok(None);
        };
        let delay = match info.codec {
            Codec::Opus { pre_skip } => pre_skip as f64 / 48_000.0,
            _ => 0.0,
        };
        // Start a little early so the decoder settles before the first sample that is kept.
        guarded("container reader", || demuxer.seek((start - 0.2).max(0.0)))??;
        Ok(Some(Self {
            demuxer,
            decoder,
            delay,
            clock: None,
            start,
            device: (rate.max(1), usize::from(channels.max(1))),
            resampler: None,
        }))
    }

    /// The next chunk of device-ready samples; `Ok(None)` at the end of the soundtrack.
    /// Chunks can be empty while the pre-roll before the start is dropped.
    pub fn next_chunk(&mut self) -> Result<Option<Vec<f32>>, MediaError> {
        let Some(packet) = guarded("container reader", || self.demuxer.next())?? else {
            return Ok(None);
        };
        let at = *self.clock.get_or_insert(packet.pts - self.delay);
        let Some(decoded) = guarded("audio decoder", || self.decoder.decode(packet.data, at))? else {
            return Ok(Some(Vec::new()));
        };
        let (rate, channels) = (decoded.rate.max(1), decoded.channels.max(1));
        let frames = decoded.samples.len() / channels;
        self.clock = Some(at + frames as f64 / f64::from(rate));
        // Drop what lies before the start position.
        let skip = (((self.start - at) * f64::from(rate)).ceil().max(0.0) as usize).min(frames);
        let kept = &decoded.samples[skip * channels..frames * channels];
        let device = self.device;
        let resampler = match &mut self.resampler {
            Some((from, resampler)) if *from == rate => resampler,
            slot => &mut slot.insert((rate, Resampler::new(rate, device.0, device.1))).1,
        };
        Ok(Some(resampler.process(kept, channels)))
    }
}

/// Linear resampling and channel mapping, continuous across chunks.
struct Resampler {
    /// Source frames per output frame.
    step: f64,
    /// Position of the next output frame, relative to `last`'s index (−1 is `last`).
    position: f64,
    /// The previous chunk's final frame, already mapped to output channels.
    last: Vec<f32>,
    channels: usize,
}

impl Resampler {
    fn new(from: u32, to: u32, channels: usize) -> Self {
        Self {
            step: f64::from(from) / f64::from(to),
            position: 0.0,
            last: Vec::new(),
            channels,
        }
    }

    fn process(&mut self, input: &[f32], input_channels: usize) -> Vec<f32> {
        let mapped = map_channels(input, input_channels, self.channels);
        let frames = mapped.len() / self.channels;
        if frames == 0 {
            return Vec::new();
        }
        let channels = self.channels;
        let frame = |index: isize| -> &[f32] {
            if index < 0 && !self.last.is_empty() {
                &self.last
            } else {
                let index = index.max(0) as usize;
                &mapped[index * channels..(index + 1) * channels]
            }
        };
        let mut output = Vec::with_capacity(((frames as f64 / self.step) as usize + 2) * channels);
        let mut position = self.position;
        while position <= (frames - 1) as f64 {
            let base = position.floor();
            let fraction = (position - base) as f32;
            let next = (base as isize + 1).min(frames as isize - 1);
            let (a, b) = (frame(base as isize), frame(next));
            for channel in 0..channels {
                output.push(a[channel] + (b[channel] - a[channel]) * fraction);
            }
            position += self.step;
        }
        self.position = position - frames as f64;
        self.last = mapped[(frames - 1) * channels..].to_vec();
        output
    }
}

/// Map interleaved audio from `from` channels to `to`: copies when equal, averages down to
/// mono, duplicates mono, and otherwise keeps the first channels (front left and right).
fn map_channels(input: &[f32], from: usize, to: usize) -> Vec<f32> {
    if from == to {
        return input.to_vec();
    }
    let mut output = Vec::with_capacity(input.len() / from.max(1) * to);
    for frame in input.chunks_exact(from.max(1)) {
        for channel in 0..to {
            output.push(match (from, to) {
                (_, 1) => frame.iter().take(2).sum::<f32>() / from.min(2) as f32,
                (1, _) => frame[0],
                _ => frame.get(channel).copied().unwrap_or(0.0),
            });
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channels_are_mapped_to_the_device() {
        assert_eq!(map_channels(&[0.5, 0.1], 1, 2), vec![0.5, 0.5, 0.1, 0.1]);
        assert_eq!(map_channels(&[0.2, 0.4], 2, 1), vec![0.3_f32]);
        assert_eq!(map_channels(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 6, 2), vec![1.0, 2.0]);
    }

    #[test]
    fn resampling_keeps_the_rate_ratio_across_chunks() {
        let mut resampler = Resampler::new(44_100, 48_000, 1);
        let input: Vec<f32> = (0..44_100).map(|index| (index as f32 * 0.01).sin()).collect();
        let output: usize = input.chunks(1000).map(|chunk| resampler.process(chunk, 1).len()).sum();
        assert!((output as i64 - 48_000).abs() <= 2, "{output} frames for one second");
    }

    #[test]
    fn resampling_interpolates_between_chunks() {
        let mut resampler = Resampler::new(1, 2, 1);
        let first = resampler.process(&[0.0, 1.0], 1);
        let second = resampler.process(&[2.0], 1);
        assert_eq!(first, vec![0.0, 0.5, 1.0]);
        assert_eq!(second, vec![1.5, 2.0]);
    }
}
