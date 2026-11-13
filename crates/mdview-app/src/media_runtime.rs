//! Background media loading and texture cache for the application.

use std::collections::{HashMap, HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use eframe::egui;
use mdview_media::{MediaRequest, Raster};
use mdview_render::{MediaProvider, MediaState};

/// Media policy from preferences.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MediaPolicy {
    /// Fetch remote images referenced by a document.
    pub remote_images: bool,
    /// Allow URLs that resolve to private network addresses.
    pub private_networks: bool,
}

impl Default for MediaPolicy {
    fn default() -> Self {
        Self {
            remote_images: true,
            private_networks: false,
        }
    }
}

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
}

struct Job {
    key: String,
    request: MediaRequest,
    generation: u64,
}

struct Completed {
    key: String,
    result: Result<Raster, String>,
    generation: u64,
}

struct TextureEntry {
    texture: egui::TextureHandle,
    bytes: usize,
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
                    let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                        job.request.load().map_err(|error| error.to_string())
                    }))
                    .unwrap_or_else(|_| Err("The media renderer failed unexpectedly.".to_owned()));
                    if completed
                        .send(Completed {
                            key: job.key,
                            result,
                            generation: job.generation,
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
        }
    }

    /// Update the current policy.
    pub fn set_policy(&mut self, policy: MediaPolicy) {
        self.policy = policy;
    }

    /// Drop every cached texture and error. Called when a media consumer closes.
    pub fn clear_cache(&mut self) {
        self.generation = self.generation.wrapping_add(1);
        self.textures.clear();
        self.order.clear();
        self.bytes = 0;
        self.errors.clear();
        self.pending.clear();
    }

    /// Upload completed rasters and drop stale entries.
    pub fn poll(&mut self, ctx: &egui::Context) {
        while let Ok(completed) = self.receiver.try_recv() {
            if completed.generation != self.generation {
                continue;
            }
            self.pending.remove(&completed.key);
            match completed.result {
                Ok(raster) if raster.is_consistent() && raster.width > 0 && raster.height > 0 => {
                    let image = egui::ColorImage::from_rgba_unmultiplied(
                        [raster.width as usize, raster.height as usize],
                        &raster.rgba,
                    );
                    let texture = ctx.load_texture(
                        format!("mdview-media-{}", completed.key),
                        image,
                        egui::TextureOptions::LINEAR,
                    );
                    self.insert_texture(completed.key, texture, raster.byte_len());
                }
                Ok(_) => {
                    self.errors
                        .insert(completed.key, "Media has no visible pixels.".to_owned());
                }
                Err(error) => {
                    self.errors.insert(completed.key, error);
                }
            }
        }
        if !self.pending.is_empty() {
            ctx.request_repaint_after(Duration::from_millis(40));
        }
    }

    /// Cached texture bytes.
    #[cfg(test)]
    pub fn cached_bytes(&self) -> usize {
        self.bytes
    }

    fn insert_texture(&mut self, key: String, texture: egui::TextureHandle, bytes: usize) {
        if let Some(previous) = self.textures.remove(&key) {
            self.bytes = self.bytes.saturating_sub(previous.bytes);
        }
        self.order.retain(|entry| entry != &key);
        self.bytes += bytes;
        self.textures.insert(key.clone(), TextureEntry { texture, bytes });
        self.order.push_back(key);
        while self.bytes > self.budget {
            let Some(oldest) = self.order.pop_front() else {
                break;
            };
            if let Some(entry) = self.textures.remove(&oldest) {
                self.bytes = self.bytes.saturating_sub(entry.bytes);
            }
        }
    }

    fn cached(&mut self, key: &str) -> Option<&egui::TextureHandle> {
        if !self.textures.contains_key(key) {
            return None;
        }
        self.order.retain(|entry| entry != key);
        self.order.push_back(key.to_owned());
        self.textures.get(key).map(|entry| &entry.texture)
    }
}

impl MediaProvider for MediaRuntime {
    fn resolve(&mut self, _ctx: &egui::Context, request: &MediaRequest) -> MediaState {
        let key = request.cache_key();
        if let Some(texture) = self.cached(&key) {
            return MediaState::Ready(texture.clone());
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
            })
            .is_err()
        {
            return MediaState::Error("The media worker stopped.".to_owned());
        }
        self.pending.insert(key);
        MediaState::Loading
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
        for _ in 0..200 {
            runtime.poll(&ctx);
            if runtime.cached_bytes() > 0 {
                break;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        assert!(runtime.cached_bytes() > 0);
        assert!(matches!(runtime.resolve(&ctx, &request), MediaState::Ready(_)));
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
