//! Background media loading, texture cache, and animation playback (SVG, GIF, WebP, APNG).

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::{Duration, Instant};

use eframe::egui;
use mdview_media::MediaRequest;

mod animation;
mod cache;
mod media_policy;
mod types;
use mdview_render::{AnimationStatus, MediaProvider, MediaState};
pub use media_policy::MediaPolicy;
use media_policy::media_source;
use types::{AnimationInfo, Completed, CompletedMedia, Job, JobMode, Playback, TextureEntry};

/// The main-thread media service: worker channel plus texture cache.
pub struct MediaRuntime {
    sender: Option<Sender<Job>>,
    receiver: Receiver<Completed>,
    textures: HashMap<String, TextureEntry>,
    order: VecDeque<String>,
    bytes: usize,
    budget: usize,
    pending: HashSet<String>,
    errors: HashMap<String, String>,
    policy: MediaPolicy,
    cache_dir: Option<PathBuf>,
    generation: u64,
    playbacks: HashMap<String, Playback>,
    playing: bool,
    reduced_motion: bool,
}

impl MediaRuntime {
    /// Start the media worker. The worker gets a large stack because the SVG parser is recursive.
    pub fn new(ctx: &egui::Context) -> Self {
        let (jobs, job_receiver) = mpsc::channel::<Job>();
        let (completed, receiver) = mpsc::channel::<Completed>();
        let repaint = ctx.clone();
        let sender = thread::Builder::new()
            .name("mdview-media".to_owned())
            .stack_size(16 * 1024 * 1024)
            .spawn(move || {
                while let Ok(job) = job_receiver.recv() {
                    let is_frame = matches!(job.mode, JobMode::Frame { .. });
                    let started = Instant::now();
                    let request = job.request.clone();
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        match job.mode {
                            JobMode::Load => job.request.load_with_animation().map(|loaded| CompletedMedia {
                                raster: loaded.raster,
                                animation: loaded.animation_duration.map(|duration| AnimationInfo {
                                    duration,
                                    unsupported: loaded.animation_unsupported,
                                }),
                                source: loaded.animated,
                            }),
                            JobMode::Frame { source, time } => {
                                source.render_frame(time, &job.request).map(|raster| CompletedMedia {
                                    raster,
                                    animation: None,
                                    source: None,
                                })
                            }
                        }
                        .map_err(|error| error.to_string())
                    }))
                    .unwrap_or_else(|_| Err("The media renderer failed unexpectedly.".to_owned()));
                    if !is_frame {
                        let elapsed = started.elapsed().as_secs_f64() * 1000.0;
                        if elapsed >= 250.0 {
                            log::warn!(
                                "slow media load key={} source={} elapsed_ms={elapsed:.1}",
                                job.key,
                                media_source(&request.source)
                            );
                        } else {
                            log::debug!(
                                "media load key={} source={} elapsed_ms={elapsed:.1}",
                                job.key,
                                media_source(&request.source)
                            );
                        }
                    }
                    if completed
                        .send(Completed {
                            key: job.key,
                            request,
                            result,
                            generation: job.generation,
                            is_frame,
                        })
                        .is_err()
                    {
                        break;
                    }
                    repaint.request_repaint();
                }
            })
            .ok()
            .map(|_| jobs);
        Self {
            sender,
            receiver,
            textures: HashMap::new(),
            order: VecDeque::new(),
            bytes: 0,
            budget: 64 * 1024 * 1024,
            pending: HashSet::new(),
            errors: HashMap::new(),
            policy: MediaPolicy::default(),
            cache_dir: dirs::cache_dir().map(|directory| directory.join("mdview").join("media")),
            generation: 0,
            playbacks: HashMap::new(),
            playing: true,
            reduced_motion: false,
        }
    }

    /// Update the current policy and retry anything that was blocked by the old one.
    pub fn set_policy(&mut self, policy: MediaPolicy) {
        if self.policy == policy {
            return;
        }
        self.policy = policy;
        self.generation = self.generation.wrapping_add(1);
        self.pending.clear();
        self.errors.clear();
    }

    /// Enable or pause all SVG animations.
    pub fn set_playing(&mut self, playing: bool) {
        self.playing = playing;
    }

    /// Show only the first animation frame when reduced motion is requested.
    pub fn set_reduced_motion(&mut self, reduced: bool) {
        self.reduced_motion = reduced;
    }

    /// Drop every cached texture and animation. Called when a media consumer closes.
    pub fn clear_cache(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.textures.clear();
        self.order.clear();
        self.bytes = 0;
        self.errors.clear();
        self.pending.clear();
        self.playbacks.clear();
    }

    /// Upload completed rasters, advance animations, and schedule the next frame.
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(completed) = self.receiver.try_recv() {
            if completed.generation != self.generation {
                continue;
            }
            self.pending.remove(&completed.key);
            match completed.result {
                Ok(media) if media.raster.is_consistent() && media.raster.width > 0 && media.raster.height > 0 => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [media.raster.width as usize, media.raster.height as usize],
                        &media.raster.rgba,
                    );
                    let texture = ctx.load_texture(
                        format!("mdview-media-{}", completed.key),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.insert_texture(completed.key.clone(), texture, media.raster.byte_len());
                    if let (Some(animation), Some(source)) = (media.animation, media.source) {
                        self.playbacks.insert(
                            completed.key.clone(),
                            Playback {
                                request: completed.request.clone(),
                                frame_index: source.frame_index(0.0),
                                source,
                                duration: animation.duration.max(0.05),
                                unsupported: animation.unsupported,
                                frame_time: 0.0,
                                last_tick: Instant::now(),
                                visible: false,
                                frame_pending: false,
                                failed: false,
                            },
                        );
                    }
                    if completed.is_frame
                        && let Some(playback) = self.playbacks.get_mut(&completed.key)
                    {
                        playback.frame_pending = false;
                    }
                }
                Ok(_) => {
                    self.errors
                        .insert(completed.key.clone(), "Media has no visible pixels.".to_owned());
                    self.mark_frame_failed(&completed.key);
                }
                Err(error) => {
                    log::warn!(
                        "media load failed key={} source={} error={error}",
                        completed.key,
                        media_source(&completed.request.source)
                    );
                    self.errors.insert(completed.key.clone(), error);
                    self.mark_frame_failed(&completed.key);
                }
            }
        }
        self.tick_animations(ctx);
        if !self.pending.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }
}

impl MediaProvider for MediaRuntime {
    fn resolve(&mut self, _ctx: &egui::Context, request: &MediaRequest) -> MediaState {
        let key = request.cache_key();
        if let Some(texture) = self.cached(&key).cloned() {
            if let Some(playback) = self.playbacks.get_mut(&key) {
                playback.visible = true;
            }
            return MediaState::Ready(texture);
        }
        if let Some(error) = self.errors.get(&key) {
            return MediaState::Error(error.clone());
        }
        if self.pending.contains(&key) {
            return MediaState::Loading;
        }
        let mut request = request.clone();
        request.allow_remote = self.policy.remote_images;
        request.allow_private_networks = self.policy.private_networks;
        request.cache_dir.clone_from(&self.cache_dir);
        let Some(sender) = &self.sender else {
            return MediaState::Error("The media worker is unavailable.".to_owned());
        };
        if sender
            .send(Job {
                key: key.clone(),
                request,
                generation: self.generation,
                mode: JobMode::Load,
            })
            .is_err()
        {
            return MediaState::Error("The media worker stopped.".to_owned());
        }
        self.pending.insert(key);
        MediaState::Loading
    }

    fn mark_visible(&mut self, key: &str, visible: bool) {
        if let Some(playback) = self.playbacks.get_mut(key) {
            playback.visible = visible;
        }
    }

    fn animation_status(&mut self, key: &str) -> AnimationStatus {
        let Some(playback) = self.playbacks.get(key) else {
            return AnimationStatus::None;
        };
        if playback.unsupported || playback.failed {
            return AnimationStatus::Unsupported;
        }
        if !self.playing || self.reduced_motion || !playback.visible {
            AnimationStatus::Paused
        } else {
            AnimationStatus::Playing
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn policy_defaults_are_closed_for_private_networks() {
        let policy = MediaPolicy::default();
        assert!(policy.remote_images);
        assert!(!policy.private_networks);
    }

    #[test]
    fn the_worker_loads_a_local_image() {
        let directory = tempfile::tempdir().expect("tempdir");
        let path = directory.path().join("pixel.png");
        image::RgbaImage::from_pixel(2, 2, image::Rgba([10, 20, 30, 255]))
            .save(&path)
            .expect("save png");
        let ctx = egui::Context::default();
        let mut runtime = MediaRuntime::new(&ctx);
        let mut request = MediaRequest::new(mdview_media::MediaKind::Image, "pixel.png");
        request.base = Some(directory.path().to_path_buf());
        assert!(matches!(runtime.resolve(&ctx, &request), MediaState::Loading));
        wait_until(&ctx, &mut runtime, |runtime| runtime.cached_bytes() > 0);
        assert!(runtime.cached_bytes() > 0);
        assert!(matches!(runtime.resolve(&ctx, &request), MediaState::Ready(_)));
    }

    fn wait_until(ctx: &egui::Context, runtime: &mut MediaRuntime, predicate: impl Fn(&mut MediaRuntime) -> bool) {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            runtime.poll(ctx);
            if predicate(runtime) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        panic!("media worker did not finish before the deadline");
    }

    #[test]
    fn animated_svgs_are_detected_and_advance() {
        let directory = tempfile::tempdir().expect("tempdir");
        std::fs::write(
            directory.path().join("pulse.svg"),
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="20" height="20"><rect width="20" height="20" fill="#ff0000"><animate attributeName="opacity" values="0;1" dur="0.2s" repeatCount="indefinite"/></rect></svg>"##,
        )
        .expect("svg");
        let ctx = egui::Context::default();
        let mut runtime = MediaRuntime::new(&ctx);
        let mut request = MediaRequest::new(mdview_media::MediaKind::Image, "pulse.svg");
        request.base = Some(directory.path().to_path_buf());
        assert!(matches!(runtime.resolve(&ctx, &request), MediaState::Loading));
        wait_until(&ctx, &mut runtime, |runtime| {
            runtime.animation_status(&request.cache_key()) != AnimationStatus::None
        });
        let key = request.cache_key();
        runtime.mark_visible(&key, true);
        assert_eq!(runtime.animation_status(&key), AnimationStatus::Playing);
        runtime.mark_visible(&key, false);
        assert_eq!(runtime.animation_status(&key), AnimationStatus::Paused);
        runtime.mark_visible(&key, true);
        runtime.set_playing(false);
        assert_eq!(runtime.animation_status(&key), AnimationStatus::Paused);
        runtime.set_playing(true);
        runtime.set_reduced_motion(true);
        assert_eq!(runtime.animation_status(&key), AnimationStatus::Paused);
        runtime.clear_cache();
        assert_eq!(runtime.animation_status(&key), AnimationStatus::None);
    }

    #[test]
    fn the_texture_cache_evicts_oldest_entries() {
        let ctx = egui::Context::default();
        let mut runtime = MediaRuntime::new(&ctx);
        runtime.budget = 16;
        for (index, color) in [[255, 0, 0, 255], [0, 255, 0, 255], [0, 0, 255, 255]]
            .into_iter()
            .enumerate()
        {
            let image = egui::ColorImage::from_rgba_unmultiplied([1, 1], &color);
            let texture = ctx.load_texture(format!("test-{index}"), image, Default::default());
            runtime.insert_texture(format!("test-{index}"), texture, 8);
        }
        assert!(runtime.cached_bytes() <= 16);
        assert!(runtime.cached("test-0").is_none());
    }
}
