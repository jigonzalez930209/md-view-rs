//! The video viewer's sound: FFmpeg decodes the soundtrack to PCM and cpal plays it.
//!
//! FFmpeg resamples to the output device's own rate and channel count, so the device plays
//! the samples as they come. A bounded queue between the pipe reader and the device callback
//! keeps FFmpeg at playback speed: when the queue is full the reader stops reading and the
//! pipe's backpressure pauses decoding. An empty queue plays silence instead of blocking the
//! audio thread. Dropping the player stops the device, FFmpeg, and the reader.

use std::collections::VecDeque;
use std::io::Read;
use std::path::Path;
use std::process::{Child, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};

/// Queued audio, in seconds, before the reader waits for the device to catch up.
const QUEUE_SECONDS: usize = 1;

type Queue = Arc<Mutex<VecDeque<f32>>>;

pub(super) struct AudioPlayer {
    child: Child,
    stop: Arc<AtomicBool>,
    reader: Option<JoinHandle<()>>,
    _stream: Stream,
}

impl AudioPlayer {
    /// Start the soundtrack of `path` at `from` seconds on the default output device.
    /// Returns `None` when there is no device or FFmpeg cannot start; the video still plays.
    pub(super) fn start(path: &Path, from: f64) -> Option<Self> {
        let device = cpal::default_host().default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let mut child = mdview_media::ffmpeg_command()
            .args(mdview_media::audio_args(
                path,
                from,
                config.sample_rate,
                config.channels,
            ))
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .ok()?;
        let stdout = child.stdout.take()?;
        let capacity = config.sample_rate as usize * usize::from(config.channels) * QUEUE_SECONDS;
        let queue: Queue = Arc::new(Mutex::new(VecDeque::with_capacity(capacity)));
        let stop = Arc::new(AtomicBool::new(false));
        let stream = match format {
            SampleFormat::F32 => build::<f32>(&device, config, &queue),
            SampleFormat::F64 => build::<f64>(&device, config, &queue),
            SampleFormat::I16 => build::<i16>(&device, config, &queue),
            SampleFormat::I32 => build::<i32>(&device, config, &queue),
            SampleFormat::U16 => build::<u16>(&device, config, &queue),
            SampleFormat::U8 => build::<u8>(&device, config, &queue),
            _ => None,
        };
        let Some(stream) = stream.filter(|stream| stream.play().is_ok()) else {
            let _ = child.kill();
            let _ = child.wait();
            return None;
        };
        let reader = {
            let (queue, stop) = (Arc::clone(&queue), Arc::clone(&stop));
            std::thread::Builder::new()
                .name("video-audio".to_owned())
                .spawn(move || read_pcm(stdout, &queue, &stop, capacity))
                .ok()
        };
        Some(Self {
            child,
            stop,
            reader,
            _stream: stream,
        })
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        let _ = self.child.kill();
        let _ = self.child.wait();
        if let Some(reader) = self.reader.take() {
            let _ = reader.join();
        }
    }
}

fn build<T>(device: &cpal::Device, config: StreamConfig, queue: &Queue) -> Option<Stream>
where
    T: SizedSample + FromSample<f32>,
{
    let queue = Arc::clone(queue);
    device
        .build_output_stream(
            config,
            move |output: &mut [T], _: &cpal::OutputCallbackInfo| {
                let mut queue = queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
                for sample in output.iter_mut() {
                    *sample = T::from_sample(queue.pop_front().unwrap_or(0.0));
                }
            },
            |error| log::warn!("video audio stream: {error}"),
            None,
        )
        .ok()
}

/// Move PCM from FFmpeg's pipe into the queue, waiting while the queue is full.
fn read_pcm(mut stdout: impl Read, queue: &Queue, stop: &AtomicBool, capacity: usize) {
    let mut bytes = vec![0u8; 8192];
    let mut carry: Option<u8> = None;
    while !stop.load(Ordering::Relaxed) {
        let read = match stdout.read(&mut bytes) {
            Ok(0) | Err(_) => return,
            Ok(read) => read,
        };
        let samples = pcm_samples(&mut carry, &bytes[..read]);
        loop {
            if stop.load(Ordering::Relaxed) {
                return;
            }
            let mut queue = queue.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
            if queue.len() + samples.len() <= capacity.max(samples.len()) {
                queue.extend(samples);
                break;
            }
            drop(queue);
            std::thread::sleep(Duration::from_millis(5));
        }
    }
}

/// Convert little-endian signed 16-bit PCM to `f32`, keeping an odd trailing byte for the
/// next read so a sample split across reads is not lost.
fn pcm_samples(carry: &mut Option<u8>, bytes: &[u8]) -> Vec<f32> {
    let mut joined;
    let bytes = match carry.take() {
        Some(first) => {
            joined = Vec::with_capacity(bytes.len() + 1);
            joined.push(first);
            joined.extend_from_slice(bytes);
            &joined[..]
        }
        None => bytes,
    };
    let pairs = bytes.chunks_exact(2);
    if let [last] = pairs.remainder() {
        *carry = Some(*last);
    }
    pairs
        .map(|pair| f32::from(i16::from_le_bytes([pair[0], pair[1]])) / 32768.0)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pcm_split_across_reads_is_reassembled() {
        let samples: Vec<i16> = vec![0, 16384, -32768, 32767, -1];
        let bytes: Vec<u8> = samples.iter().flat_map(|sample| sample.to_le_bytes()).collect();
        let mut carry = None;
        let mut out = pcm_samples(&mut carry, &bytes[..3]);
        assert_eq!(carry, Some(bytes[2]), "the odd byte waits for the next read");
        out.extend(pcm_samples(&mut carry, &bytes[3..]));
        assert_eq!(carry, None);
        let expected: Vec<f32> = samples.iter().map(|sample| f32::from(*sample) / 32768.0).collect();
        assert_eq!(out, expected);
    }

    /// A pipe that hands over at most 40 bytes (20 samples) per read.
    struct Trickle(Vec<u8>);

    impl Read for Trickle {
        fn read(&mut self, buffer: &mut [u8]) -> std::io::Result<usize> {
            let count = self.0.len().min(buffer.len()).min(40);
            buffer[..count].copy_from_slice(&self.0[..count]);
            self.0.drain(..count);
            Ok(count)
        }
    }

    #[test]
    fn a_full_queue_holds_the_reader_until_it_drains() {
        let queue: Queue = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let pipe = Trickle(vec![0u8; 400]);
        let reader = {
            let (queue, stop) = (Arc::clone(&queue), Arc::clone(&stop));
            std::thread::spawn(move || read_pcm(pipe, &queue, &stop, 100))
        };
        std::thread::sleep(Duration::from_millis(50));
        assert!(
            queue.lock().unwrap().len() <= 100,
            "the queue never exceeds its capacity"
        );
        let mut drained = 0;
        while drained < 200 {
            drained += queue.lock().unwrap().drain(..).count();
            std::thread::sleep(Duration::from_millis(5));
        }
        reader.join().unwrap();
        assert_eq!(drained, 200, "every sample arrives once the device drains the queue");
    }
}
