//! The video worker thread: decoding, frame timing, and the soundtrack.
//!
//! The worker owns a [`VideoReader`] and decodes in-process. While playing it decodes the
//! next frame, waits for its presentation time on the order channel (so a pause or seek
//! interrupts the wait), and hands it over; frames that are already late by more than two
//! frame intervals are dropped to catch up. The clock starts once the first frame after a
//! seek is decoded, together with the soundtrack.

use std::path::Path;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;
use mdview_media::{VideoFrame, VideoInfo, VideoReader};

use super::video_audio::AudioPlayer;

pub(super) enum Order {
    Play { from: f64, width: u32, audio: bool },
    Pause,
    Still { at: f64, width: u32 },
}

pub(super) enum Reply {
    Info(Result<VideoInfo, String>),
    Frame {
        at: f64,
        width: u32,
        height: u32,
        rgba: Vec<u8>,
    },
    Ended,
}

/// Playback in progress: when the clock started, at which video time, and the sound.
struct Playing {
    started: Instant,
    from: f64,
    size: (u32, u32),
    _audio: Option<AudioPlayer>,
}

pub(super) fn worker(path: &Path, orders: &Receiver<Order>, replies: &Sender<Reply>, ctx: &egui::Context) {
    let send = |reply: Reply| {
        let sent = replies.send(reply).is_ok();
        ctx.request_repaint();
        sent
    };
    let (mut reader, info) = match VideoReader::open(path) {
        Ok(opened) => opened,
        Err(error) => {
            send(Reply::Info(Err(error.to_string())));
            return;
        }
    };
    if !send(Reply::Info(Ok(info.clone()))) {
        return;
    }
    let interval = 1.0 / info.fps.max(1.0);
    let mut playing: Option<Playing> = None;
    let mut waiting: Option<Order> = None;
    loop {
        let order = match (waiting.take(), &playing) {
            (Some(order), _) => Some(order),
            (None, Some(_)) => match orders.try_recv() {
                Ok(order) => Some(order),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            },
            (None, None) => match orders.recv() {
                Ok(order) => Some(order),
                Err(_) => return,
            },
        };
        match order {
            Some(Order::Pause) => playing = None,
            Some(Order::Still { at, width }) => {
                playing = None;
                let size = mdview_media::decode_size(&info, width);
                if let Some(frame) = first_frame(&mut reader, at, size)
                    && !send(frame_reply(frame))
                {
                    return;
                }
            }
            Some(Order::Play { from, width, audio }) => {
                // Stop the old sound before the new one starts.
                playing = None;
                let size = mdview_media::decode_size(&info, width);
                let Some(frame) = first_frame(&mut reader, from, size) else {
                    send(Reply::Ended);
                    continue;
                };
                let sound = (audio && info.has_audio)
                    .then(|| AudioPlayer::start(path, frame.at))
                    .flatten();
                playing = Some(Playing {
                    started: Instant::now(),
                    from: frame.at,
                    size,
                    _audio: sound,
                });
                if !send(frame_reply(frame)) {
                    return;
                }
                continue;
            }
            None => {}
        }
        let Some(state) = &playing else {
            continue;
        };
        let frame = match reader.next_frame(state.size) {
            Ok(Some(frame)) => frame,
            Ok(None) | Err(_) => {
                playing = None;
                send(Reply::Ended);
                continue;
            }
        };
        let due = state.started + Duration::from_secs_f64((frame.at - state.from).max(0.0));
        let now = Instant::now();
        if let Some(wait) = due.checked_duration_since(now) {
            match orders.recv_timeout(wait) {
                // An order cancels this frame; it is handled on the next turn.
                Ok(order) => {
                    waiting = Some(order);
                    continue;
                }
                Err(RecvTimeoutError::Timeout) => {}
                Err(RecvTimeoutError::Disconnected) => return,
            }
        } else if now.duration_since(due).as_secs_f64() > 2.0 * interval {
            continue;
        }
        if !send(frame_reply(frame)) {
            return;
        }
    }
}

/// Seek to `at` and decode the frame shown there.
fn first_frame(reader: &mut VideoReader, at: f64, size: (u32, u32)) -> Option<VideoFrame> {
    if let Err(error) = reader.seek(at) {
        log::warn!("video seek to {at:.3}: {error}");
        return None;
    }
    match reader.next_frame(size) {
        Ok(frame) => frame,
        Err(error) => {
            log::warn!("video decode at {at:.3}: {error}");
            None
        }
    }
}

fn frame_reply(frame: VideoFrame) -> Reply {
    Reply::Frame {
        at: frame.at,
        width: frame.width,
        height: frame.height,
        rgba: frame.rgba,
    }
}
