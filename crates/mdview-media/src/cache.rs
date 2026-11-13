//! Bounded LRU cache for decoded media.

use std::collections::{HashMap, VecDeque};

use crate::Raster;

/// Cache occupancy.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CacheStats {
    /// Number of cached rasters.
    pub entries: usize,
    /// Bytes currently held.
    pub bytes: usize,
    /// Maximum bytes allowed.
    pub budget: usize,
}

/// A byte-budgeted LRU cache of decoded rasters.
#[derive(Debug)]
pub struct MediaCache {
    budget: usize,
    bytes: usize,
    entries: HashMap<String, Raster>,
    order: VecDeque<String>,
}

impl MediaCache {
    /// Build a cache with a byte budget.
    pub fn new(budget: usize) -> Self {
        Self {
            budget: budget.max(1),
            bytes: 0,
            entries: HashMap::new(),
            order: VecDeque::new(),
        }
    }

    /// Insert or replace an entry and return the evicted keys.
    pub fn insert(&mut self, key: impl Into<String>, raster: Raster) -> Vec<String> {
        let key = key.into();
        if let Some(previous) = self.entries.remove(&key) {
            self.bytes = self.bytes.saturating_sub(previous.byte_len());
        }
        self.order.retain(|entry| entry != &key);
        self.bytes += raster.byte_len();
        self.entries.insert(key.clone(), raster);
        self.order.push_back(key);
        self.evict()
    }

    /// Look up an entry, marking it most recently used.
    pub fn get(&mut self, key: &str) -> Option<&Raster> {
        if !self.entries.contains_key(key) {
            return None;
        }
        self.order.retain(|entry| entry != key);
        self.order.push_back(key.to_owned());
        self.entries.get(key)
    }

    /// Look up an entry without changing its recency.
    pub fn peek(&self, key: &str) -> Option<&Raster> {
        self.entries.get(key)
    }

    /// Remove an entry.
    pub fn remove(&mut self, key: &str) -> Option<Raster> {
        let raster = self.entries.remove(key)?;
        self.order.retain(|entry| entry != key);
        self.bytes = self.bytes.saturating_sub(raster.byte_len());
        Some(raster)
    }

    /// Remove every entry.
    pub fn clear(&mut self) {
        self.entries.clear();
        self.order.clear();
        self.bytes = 0;
    }

    /// Occupancy for diagnostics and tests.
    pub fn stats(&self) -> CacheStats {
        CacheStats {
            entries: self.entries.len(),
            bytes: self.bytes,
            budget: self.budget,
        }
    }

    fn evict(&mut self) -> Vec<String> {
        let mut evicted = Vec::new();
        while self.bytes > self.budget {
            let Some(key) = self.order.pop_front() else {
                break;
            };
            if let Some(raster) = self.entries.remove(&key) {
                self.bytes = self.bytes.saturating_sub(raster.byte_len());
                evicted.push(key);
            }
        }
        evicted
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn raster(pixels: u32) -> Raster {
        Raster {
            width: pixels,
            height: 1,
            rgba: vec![0; pixels as usize * 4],
        }
    }

    #[test]
    fn the_budget_evicts_the_least_recently_used() {
        let mut cache = MediaCache::new(16);
        cache.insert("a", raster(2));
        cache.insert("b", raster(2));
        assert!(cache.get("a").is_some());
        let evicted = cache.insert("c", raster(2));
        assert_eq!(evicted, ["b"]);
        let evicted = cache.insert("d", raster(2));
        assert_eq!(evicted, ["a"]);
        assert!(cache.peek("c").is_some());
    }

    #[test]
    fn replacing_an_entry_does_not_leak_bytes() {
        let mut cache = MediaCache::new(64);
        cache.insert("a", raster(4));
        cache.insert("a", raster(2));
        assert_eq!(cache.stats().entries, 1);
        assert_eq!(cache.stats().bytes, 8);
    }

    #[test]
    fn oversized_entries_are_evicted_immediately() {
        let mut cache = MediaCache::new(8);
        let evicted = cache.insert("big", raster(10));
        assert_eq!(evicted, ["big"]);
        assert_eq!(cache.stats().entries, 0);
    }
}
