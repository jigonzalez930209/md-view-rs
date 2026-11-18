//! Worker and cache payloads for the media runtime.

use std::time::Instant;

use mdview_media::{AnimatedSource, MediaRequest, Raster};

pub(super) struct Job {
    pub(super) key: String,
    pub(super) request: MediaRequest,
    pub(super) generation: u64,
    pub(super) mode: JobMode,
}

pub(super) enum JobMode {
    Load,
    Frame { source: AnimatedSource, time: f32 },
}

pub(super) struct Completed {
    pub(super) key: String,
    pub(super) request: MediaRequest,
    pub(super) result: Result<CompletedMedia, String>,
    pub(super) generation: u64,
    pub(super) is_frame: bool,
}

pub(super) struct CompletedMedia {
    pub(super) raster: Raster,
    pub(super) animation: Option<AnimationInfo>,
    pub(super) source: Option<AnimatedSource>,
}

pub(super) struct AnimationInfo {
    pub(super) duration: f32,
    pub(super) unsupported: bool,
}

pub(super) struct Playback {
    pub(super) request: MediaRequest,
    pub(super) source: AnimatedSource,
    /// The decoded frame on screen, for raster animations: ticks that keep it are skipped.
    pub(super) frame_index: Option<usize>,
    pub(super) duration: f32,
    pub(super) unsupported: bool,
    pub(super) frame_time: f32,
    pub(super) last_tick: Instant,
    pub(super) visible: bool,
    pub(super) frame_pending: bool,
    pub(super) failed: bool,
}

pub(super) struct TextureEntry {
    pub(super) texture: egui::TextureHandle,
    pub(super) bytes: usize,
}
