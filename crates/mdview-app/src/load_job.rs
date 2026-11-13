//! Background loading for large files.

use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver};
use std::thread;

use mdview_model::Document;

use crate::file_ops::{self, FileError};

/// An event emitted by a background load.
pub enum LoadEvent {
    /// Bytes read so far.
    Progress(u64),
    /// The decoded document or an error.
    Done(Result<Document, FileError>),
}

/// A running background load.
pub struct LoadJob {
    path: PathBuf,
    total: u64,
    received: u64,
    cancel: Arc<AtomicBool>,
    receiver: Receiver<LoadEvent>,
    thread: Option<thread::JoinHandle<()>>,
}

impl LoadJob {
    /// Start loading `path` on a worker thread.
    pub fn spawn(path: PathBuf) -> Result<Self, FileError> {
        let total = std::fs::metadata(&path)?.len();
        if total > file_ops::MAX_FILE_BYTES {
            return Err(FileError::TooLarge);
        }
        let cancel = Arc::new(AtomicBool::new(false));
        let thread_cancel = cancel.clone();
        let (sender, receiver) = mpsc::channel();
        let thread_path = path.clone();
        let thread = thread::spawn(move || {
            let result = file_ops::load_with_progress(&thread_path, &thread_cancel, |read, _| {
                let _ = sender.send(LoadEvent::Progress(read));
            });
            let _ = sender.send(LoadEvent::Done(result));
        });
        Ok(Self {
            path,
            total,
            received: 0,
            cancel,
            receiver,
            thread: Some(thread),
        })
    }

    /// The file being loaded.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Progress in the range 0.0–1.0.
    pub fn progress(&self) -> f32 {
        if self.total == 0 {
            return 0.0;
        }
        (self.received as f32 / self.total as f32).clamp(0.0, 1.0)
    }

    /// Ask the worker to stop at the next chunk boundary.
    pub fn cancel(&self) {
        self.cancel.store(true, Ordering::Relaxed);
    }

    /// Poll for the next event without blocking.
    pub fn poll(&mut self) -> Option<LoadEvent> {
        match self.receiver.try_recv() {
            Ok(LoadEvent::Progress(read)) => {
                self.received = read;
                log::debug!("load progress {read}/{}", self.total);
                Some(LoadEvent::Progress(read))
            }
            Ok(LoadEvent::Done(result)) => {
                log::debug!("load done ok={}", result.is_ok());
                if let Some(thread) = self.thread.take() {
                    let _ = thread.join();
                }
                Some(LoadEvent::Done(result))
            }
            Err(mpsc::TryRecvError::Empty) => None,
            Err(mpsc::TryRecvError::Disconnected) => Some(LoadEvent::Done(Err(FileError::Io(std::io::Error::other(
                "the loading worker stopped",
            ))))),
        }
    }
}

impl Drop for LoadJob {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::Relaxed);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loads_a_large_file_in_the_background() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("large.txt");
        std::fs::write(&path, "a".repeat(11 * 1024 * 1024)).expect("write");
        let mut job = LoadJob::spawn(path).expect("spawn");
        let mut done = None;
        for _ in 0..500 {
            while let Some(event) = job.poll() {
                if let LoadEvent::Done(result) = event {
                    done = Some(result);
                    break;
                }
            }
            if done.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let document = done.expect("completed").expect("loaded");
        assert_eq!(document.snapshot().len(), 11 * 1024 * 1024);
    }
}

#[cfg(test)]
mod multiline_tests {
    use super::*;

    #[test]
    fn loads_a_large_multiline_file() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("large.txt");
        let text = "fn generated_line() { let value = 1234567890; }\n".repeat(220_000);
        std::fs::write(&path, text).expect("write");
        let start = std::time::Instant::now();
        let mut job = LoadJob::spawn(path).expect("spawn");
        let mut done = None;
        for _ in 0..2000 {
            while let Some(event) = job.poll() {
                if let LoadEvent::Done(result) = event {
                    done = Some(result);
                    break;
                }
            }
            if done.is_some() {
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
        let document = done.expect("completed").expect("loaded");
        assert_eq!(document.line_count(), 220_001);
        eprintln!("multiline load took {:?}", start.elapsed());
    }
}
