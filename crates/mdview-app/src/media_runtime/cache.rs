//! Texture cache eviction and access.

use eframe::egui;

use super::MediaRuntime;
use super::types::TextureEntry;

impl MediaRuntime {
    /// Cached texture bytes.
    #[cfg(test)]
    pub(super) fn cached_bytes(&self) -> usize {
        self.bytes
    }

    pub(super) fn insert_texture(&mut self, key: String, texture: egui::TextureHandle, bytes: usize) {
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
                self.playbacks.remove(&oldest);
            }
        }
    }

    pub(super) fn cached(&mut self, key: &str) -> Option<&egui::TextureHandle> {
        if !self.textures.contains_key(key) {
            return None;
        }
        self.order.retain(|entry| entry != key);
        self.order.push_back(key.to_owned());
        self.textures.get(key).map(|entry| &entry.texture)
    }
}
