//! Background polling of open files for changes made outside the application.
//!
//! The workspace watcher covers files inside the open folder, but a file opened from
//! elsewhere has no watcher. The UI used to repaint the whole window twice a second just
//! to stat those files. This thread stats them instead and wakes the UI only when a file's
//! modification time differs from the one the application last read.

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, SystemTime};

/// An open file and the modification time the application last read from disk.
pub(crate) type Watched = (PathBuf, Option<SystemTime>);

/// How often the poller sleeps between checks of its stop flag.
const STOP_CHECK: Duration = Duration::from_millis(50);

/// A thread that reports open files whose modification time changed.
pub(crate) struct DiskPoller {
    watched: Arc<Mutex<Vec<Watched>>>,
    changed: Arc<AtomicBool>,
    stop: Arc<AtomicBool>,
    last_sent: Vec<Watched>,
}

impl DiskPoller {
    /// Start polling every `interval`; `wake` runs on the poller thread when a file changed.
    pub(crate) fn spawn(interval: Duration, wake: impl Fn() + Send + 'static) -> Self {
        let watched = Arc::new(Mutex::new(Vec::new()));
        let changed = Arc::new(AtomicBool::new(false));
        let stop = Arc::new(AtomicBool::new(false));
        let poller = Self {
            watched: watched.clone(),
            changed: changed.clone(),
            stop: stop.clone(),
            last_sent: Vec::new(),
        };
        let spawned = std::thread::Builder::new()
            .name("mdview-disk-poll".to_owned())
            .spawn(move || {
                let mut reported: HashMap<PathBuf, Option<SystemTime>> = HashMap::new();
                while sleep_unless_stopped(&stop, interval) {
                    let snapshot = watched.lock().map(|list| list.clone()).unwrap_or_default();
                    if check(&snapshot, &mut reported, crate::file_ops::modified) {
                        changed.store(true, Ordering::Release);
                        wake();
                    }
                }
            });
        if let Err(error) = spawned {
            log::warn!("disk poller unavailable: {error}");
        }
        poller
    }

    /// Replace the watched files. The list is only sent to the thread when it changed.
    pub(crate) fn watch(&mut self, files: Vec<Watched>) {
        if files == self.last_sent {
            return;
        }
        if let Ok(mut watched) = self.watched.lock() {
            watched.clone_from(&files);
        }
        self.last_sent = files;
    }

    /// Whether a watched file changed since the previous call.
    pub(crate) fn take_changed(&self) -> bool {
        self.changed.swap(false, Ordering::AcqRel)
    }
}

impl Drop for DiskPoller {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
    }
}

/// Sleep for `interval` in short steps; returns `false` once the poller must stop.
fn sleep_unless_stopped(stop: &AtomicBool, interval: Duration) -> bool {
    let mut slept = Duration::ZERO;
    while slept < interval {
        if stop.load(Ordering::Acquire) {
            return false;
        }
        let step = STOP_CHECK.min(interval - slept);
        std::thread::sleep(step);
        slept += step;
    }
    !stop.load(Ordering::Acquire)
}

/// Compare each watched file with the time the application read; report each new
/// difference once, so a conflict the user has not resolved does not wake the UI again.
fn check(
    watched: &[Watched],
    reported: &mut HashMap<PathBuf, Option<SystemTime>>,
    modified: impl Fn(&std::path::Path) -> Option<SystemTime>,
) -> bool {
    reported.retain(|path, _| watched.iter().any(|(watched, _)| watched == path));
    let mut changed = false;
    for (path, read) in watched {
        let now = modified(path);
        if now == *read {
            reported.remove(path);
            continue;
        }
        if reported.get(path) != Some(&now) {
            reported.insert(path.clone(), now);
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(seconds: u64) -> Option<SystemTime> {
        Some(SystemTime::UNIX_EPOCH + Duration::from_secs(seconds))
    }

    #[test]
    fn a_new_modification_time_is_reported_once() {
        let path = PathBuf::from("/notes.md");
        let watched = vec![(path.clone(), at(1))];
        let mut reported = HashMap::new();
        assert!(!check(&watched, &mut reported, |_| at(1)), "unchanged");
        assert!(check(&watched, &mut reported, |_| at(2)), "changed on disk");
        assert!(!check(&watched, &mut reported, |_| at(2)), "already reported");
        assert!(check(&watched, &mut reported, |_| at(3)), "changed again");
        assert!(check(&watched, &mut reported, |_| None), "deleted");
    }

    #[test]
    fn reading_the_file_again_resets_the_report() {
        let path = PathBuf::from("/notes.md");
        let mut reported = HashMap::new();
        assert!(check(&[(path.clone(), at(1))], &mut reported, |_| at(2)));
        // The application reloaded the file: its read time is now the disk time.
        assert!(!check(&[(path.clone(), at(2))], &mut reported, |_| at(2)));
        assert!(reported.is_empty());
        assert!(check(&[(path, at(2))], &mut reported, |_| at(1)), "reverted on disk");
    }

    #[test]
    fn the_thread_wakes_the_ui_when_a_file_changes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("outside.md");
        std::fs::write(&path, "one").expect("write");
        let read = crate::file_ops::modified(&path);
        let (sender, receiver) = std::sync::mpsc::channel();
        let sender = Mutex::new(sender);
        let mut poller = DiskPoller::spawn(Duration::from_millis(20), move || {
            if let Ok(sender) = sender.lock() {
                let _ = sender.send(());
            }
        });
        poller.watch(vec![(path.clone(), read)]);
        assert!(
            receiver.recv_timeout(Duration::from_millis(200)).is_err(),
            "an unchanged file must not wake the UI"
        );
        assert!(!poller.take_changed());
        std::fs::remove_file(&path).expect("remove");
        receiver
            .recv_timeout(Duration::from_secs(5))
            .expect("a deleted file must wake the UI");
        assert!(poller.take_changed());
        assert!(!poller.take_changed(), "the flag is consumed");
    }
}
