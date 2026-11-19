//! The video viewer's sound: the soundtrack decoded in-process and played by cpal.
//!
//! An [`AudioReader`] decodes on its own thread, already resampled to the output device's
//! rate and channel count, into a bounded queue that the device callback drains. A full queue
//! makes the decoder wait, which keeps it at playback speed; an empty queue plays silence
//! instead of blocking the audio thread. Dropping the player stops the device and the decoder.

use std::collections::VecDeque;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample, Stream, StreamConfig};
use mdview_media::AudioReader;

/// Queued audio, in seconds, before the decoder waits for the device to catch up.
const QUEUE_SECONDS: usize = 1;

type Queue = Arc<Mutex<VecDeque<f32>>>;

pub(super) struct AudioPlayer {
    stop: Arc<AtomicBool>,
    decoder: Option<JoinHandle<()>>,
    _stream: Stream,
}

impl AudioPlayer {
    /// Start the soundtrack of `path` at `from` seconds on the default output device.
    /// Returns `None` when there is no device or no playable soundtrack; the video still plays.
    pub(super) fn start(path: &Path, from: f64) -> Option<Self> {
        let device = cpal::default_host().default_output_device()?;
        let supported = device.default_output_config().ok()?;
        let format = supported.sample_format();
        let config: StreamConfig = supported.config();
        let reader = match AudioReader::open(path, from, config.sample_rate, config.channels) {
            Ok(Some(reader)) => reader,
            Ok(None) => return None,
            Err(error) => {
                log::warn!("video audio: {error}");
                return None;
            }
        };
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
        let stream = stream.filter(|stream| stream.play().is_ok())?;
        let decoder = {
            let (queue, stop) = (Arc::clone(&queue), Arc::clone(&stop));
            let mut reader = reader;
            std::thread::Builder::new()
                .name("video-audio".to_owned())
                .spawn(move || {
                    feed(
                        || match reader.next_chunk() {
                            Ok(chunk) => chunk,
                            Err(error) => {
                                log::warn!("video audio: {error}");
                                None
                            }
                        },
                        &queue,
                        &stop,
                        capacity,
                    );
                })
                .ok()
        };
        Some(Self {
            stop,
            decoder,
            _stream: stream,
        })
    }
}

impl Drop for AudioPlayer {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Relaxed);
        if let Some(decoder) = self.decoder.take() {
            let _ = decoder.join();
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

/// Move decoded chunks from `next` into the queue, waiting while the queue is full, until
/// the soundtrack ends or `stop` is set.
fn feed(mut next: impl FnMut() -> Option<Vec<f32>>, queue: &Queue, stop: &AtomicBool, capacity: usize) {
    while !stop.load(Ordering::Relaxed) {
        let Some(samples) = next() else {
            return;
        };
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_full_queue_holds_the_decoder_until_it_drains() {
        let queue: Queue = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let (queue, stop) = (Arc::clone(&queue), Arc::clone(&stop));
            let mut chunks = (0..10).map(|_| vec![0.25f32; 20]);
            std::thread::spawn(move || feed(|| chunks.next(), &queue, &stop, 100))
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

    #[test]
    fn stopping_ends_a_waiting_decoder() {
        let queue: Queue = Arc::new(Mutex::new(VecDeque::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let reader = {
            let (queue, stop) = (Arc::clone(&queue), Arc::clone(&stop));
            std::thread::spawn(move || feed(|| Some(vec![0.0; 50]), &queue, &stop, 100))
        };
        std::thread::sleep(Duration::from_millis(30));
        stop.store(true, Ordering::Relaxed);
        reader.join().unwrap();
    }
}
