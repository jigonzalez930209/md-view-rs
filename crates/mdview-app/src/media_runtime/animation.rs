//! Animation state transitions and frame scheduling.

use std::time::{Duration, Instant};

use eframe::egui;

use super::MediaRuntime;
use super::types::{Job, JobMode};

impl MediaRuntime {
    pub(super) fn mark_frame_failed(&mut self, key: &str) {
        if let Some(playback) = self.playbacks.get_mut(key) {
            playback.frame_pending = false;
            playback.failed = true;
        }
    }

    pub(super) fn tick_animations(&mut self, ctx: &egui::Context) {
        if !self.playing || self.reduced_motion {
            return;
        }
        let now = Instant::now();
        let Some(sender) = self.sender.clone() else {
            return;
        };
        let mut active = false;
        for (key, playback) in &mut self.playbacks {
            if playback.unsupported || playback.failed || !playback.visible || playback.frame_pending {
                continue;
            }
            let elapsed = now.duration_since(playback.last_tick);
            if elapsed < Duration::from_millis(33) {
                continue;
            }
            playback.frame_time = (playback.frame_time + elapsed.as_secs_f32()) % playback.duration;
            playback.last_tick = now;
            playback.frame_pending = true;
            active = true;
            let _ = sender.send(Job {
                key: key.clone(),
                request: playback.request.clone(),
                generation: self.generation,
                mode: JobMode::Frame {
                    source: playback.source.clone(),
                    time: playback.frame_time,
                },
            });
        }
        if active || self.playbacks.values().any(|playback| playback.visible) {
            ctx.request_repaint_after(Duration::from_millis(16));
        }
    }
}
