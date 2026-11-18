//! The video worker thread: probing, the decoder and audio processes, and frame timing.

use std::io::Read;
use std::path::Path;
use std::process::{Child, ChildStdout, Command, Stdio};
use std::sync::mpsc::{Receiver, Sender, TryRecvError};
use std::time::{Duration, Instant};

use eframe::egui;
use mdview_media::VideoInfo;

/// Shown when FFmpeg is missing.
const INSTALL_HINT: &str = "Video playback uses FFmpeg, which was not found. Install it (Debian/Ubuntu: \
`sudo apt install ffmpeg`, Fedora: `sudo dnf install ffmpeg`, macOS: `brew install ffmpeg`, Windows: \
`winget install ffmpeg`) and open the file again.";

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

/// Child processes that die with their owner, so closing a tab never leaves FFmpeg running.
struct Processes {
    decoder: Option<(Child, ChildStdout)>,
    audio: Option<Child>,
}

impl Processes {
    fn stop(&mut self) {
        for mut child in self
            .decoder
            .take()
            .map(|(child, _)| child)
            .into_iter()
            .chain(self.audio.take())
        {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

impl Drop for Processes {
    fn drop(&mut self) {
        self.stop();
    }
}

pub(super) fn worker(path: &Path, orders: &Receiver<Order>, replies: &Sender<Reply>, ctx: &egui::Context) {
    let send = |reply: Reply| {
        let sent = replies.send(reply).is_ok();
        ctx.request_repaint();
        sent
    };
    if !mdview_media::ffmpeg_available() {
        send(Reply::Info(Err(INSTALL_HINT.to_owned())));
        return;
    }
    let info = match mdview_media::probe_video(path) {
        Ok(info) => info,
        Err(error) => {
            send(Reply::Info(Err(error.to_string())));
            return;
        }
    };
    if !send(Reply::Info(Ok(info.clone()))) {
        return;
    }
    let mut processes = Processes {
        decoder: None,
        audio: None,
    };
    let mut clock: Option<(Instant, f64, u64)> = None;
    let mut size = (0, 0);
    loop {
        let order = if processes.decoder.is_some() {
            match orders.try_recv() {
                Ok(order) => Some(order),
                Err(TryRecvError::Empty) => None,
                Err(TryRecvError::Disconnected) => return,
            }
        } else {
            match orders.recv() {
                Ok(order) => Some(order),
                Err(_) => return,
            }
        };
        match order {
            Some(Order::Pause) => {
                processes.stop();
                clock = None;
                continue;
            }
            Some(Order::Still { at, width }) => {
                processes.stop();
                clock = None;
                size = mdview_media::decode_size(&info, width);
                if let Some(frame) = decode_still(path, at, size) {
                    send(Reply::Frame {
                        at,
                        width: size.0,
                        height: size.1,
                        rgba: frame,
                    });
                }
                continue;
            }
            Some(Order::Play { from, width, audio }) => {
                processes.stop();
                size = mdview_media::decode_size(&info, width);
                processes.decoder = spawn_decoder(path, from, size);
                if audio && info.has_audio {
                    processes.audio = Command::new("ffplay")
                        .args(mdview_media::audio_args(path, from))
                        .stdin(Stdio::null())
                        .stdout(Stdio::null())
                        .stderr(Stdio::null())
                        .spawn()
                        .ok();
                }
                clock = Some((Instant::now(), from, 0));
            }
            None => {}
        }
        // Playing: read the next frame and hand it over at its presentation time.
        let (Some((_, stdout)), Some((started, from, frames))) = (processes.decoder.as_mut(), clock.as_mut()) else {
            continue;
        };
        let mut frame = vec![0u8; (size.0 * size.1 * 4) as usize];
        if stdout.read_exact(&mut frame).is_err() {
            processes.stop();
            clock = None;
            send(Reply::Ended);
            continue;
        }
        let at = *from + *frames as f64 / info.fps;
        *frames += 1;
        let due = *started + Duration::from_secs_f64((at - *from).max(0.0));
        if let Some(wait) = due.checked_duration_since(Instant::now()) {
            std::thread::sleep(wait);
        }
        if !send(Reply::Frame {
            at,
            width: size.0,
            height: size.1,
            rgba: frame,
        }) {
            return;
        }
    }
}

fn spawn_decoder(path: &Path, from: f64, size: (u32, u32)) -> Option<(Child, ChildStdout)> {
    let mut child = Command::new("ffmpeg")
        .args(mdview_media::decode_args(path, from, size, None))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;
    let stdout = child.stdout.take()?;
    Some((child, stdout))
}

fn decode_still(path: &Path, at: f64, size: (u32, u32)) -> Option<Vec<u8>> {
    let output = Command::new("ffmpeg")
        .args(mdview_media::decode_args(path, at, size, Some(1)))
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .output()
        .ok()?;
    (output.stdout.len() == (size.0 * size.1 * 4) as usize).then_some(output.stdout)
}
