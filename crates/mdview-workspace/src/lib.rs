//! Lazy, reactive workspace tree.
//!
//! The tree loads directory children only when a folder is expanded. A debounced native watcher
//! reports changed paths; [`Workspace::poll`] reloads the affected loaded directories. When a
//! native watcher cannot be installed, the workspace falls back to periodic rescans of the
//! directories that are already loaded.
//!
//! An application that sleeps while idle installs a waker with [`Workspace::set_waker`]: the
//! watcher thread calls it after each batch of events so the next [`Workspace::poll`] runs
//! without a periodic timer.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::RecursiveMode;
use notify_debouncer_full::{Debouncer, RecommendedCache, new_debouncer};

type NativeWatcher = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

/// A callback that wakes the application from the watcher thread.
pub type Waker = Arc<dyn Fn() + Send + Sync>;

/// One file or folder in the sidebar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// Absolute path.
    pub path: PathBuf,
    /// Display name.
    pub name: String,
    /// Whether this is a directory.
    pub is_dir: bool,
    /// Whether an expanded directory is open.
    pub expanded: bool,
    /// Children, if the directory has been loaded.
    pub children: Option<Vec<Entry>>,
}

impl Entry {
    /// Find an entry by path.
    pub fn find<'a>(&'a self, path: &Path) -> Option<&'a Entry> {
        if self.path == path {
            return Some(self);
        }
        self.children.as_ref()?.iter().find_map(|child| child.find(path))
    }

    fn find_mut(&mut self, path: &Path) -> Option<&mut Entry> {
        if self.path == path {
            return Some(self);
        }
        self.children
            .as_mut()?
            .iter_mut()
            .find_map(|child| child.find_mut(path))
    }
}

/// A workspace rooted at one folder.
pub struct Workspace {
    root: Entry,
    show_hidden: bool,
    respect_gitignore: bool,
    gitignore: Option<Gitignore>,
    watcher: Option<NativeWatcher>,
    events: Option<Receiver<Vec<PathBuf>>>,
    polling: bool,
    last_poll: Instant,
    watcher_warning: Option<String>,
    changed_paths: Vec<PathBuf>,
    waker: Arc<Mutex<Option<Waker>>>,
}

impl std::fmt::Debug for Workspace {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("Workspace")
            .field("root", &self.root.path)
            .field("entries", &self.root.children.as_ref().map_or(0, Vec::len))
            .field("polling", &self.polling)
            .finish_non_exhaustive()
    }
}

impl Workspace {
    /// Open a folder, load its first level, and start watching it.
    pub fn open(path: impl Into<PathBuf>) -> std::io::Result<Self> {
        let path = path.into();
        if !path.is_dir() {
            return Err(std::io::Error::new(
                std::io::ErrorKind::NotADirectory,
                "the workspace path is not a folder",
            ));
        }
        let root_path = fs::canonicalize(&path).unwrap_or(path);
        let name = root_path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| root_path.display().to_string());
        let mut workspace = Self {
            root: Entry {
                path: root_path.clone(),
                name,
                is_dir: true,
                expanded: true,
                children: None,
            },
            show_hidden: false,
            respect_gitignore: true,
            gitignore: None,
            watcher: None,
            events: None,
            polling: false,
            last_poll: Instant::now(),
            watcher_warning: None,
            changed_paths: Vec::new(),
            waker: Arc::new(Mutex::new(None)),
        };
        workspace.rebuild_gitignore();
        workspace.reload_root();
        workspace.start_watcher();
        Ok(workspace)
    }

    /// The workspace root.
    pub fn root(&self) -> &Path {
        &self.root.path
    }

    /// The root entry and its loaded children.
    pub fn root_entry(&self) -> &Entry {
        &self.root
    }

    /// Whether hidden files are shown.
    pub fn show_hidden(&self) -> bool {
        self.show_hidden
    }

    /// Show or hide dotfiles.
    pub fn set_show_hidden(&mut self, show: bool) {
        if self.show_hidden != show {
            self.show_hidden = show;
            self.reload_loaded();
        }
    }

    /// Whether `.gitignore` rules are respected.
    pub fn respect_gitignore(&self) -> bool {
        self.respect_gitignore
    }

    /// Enable or disable `.gitignore` rules.
    pub fn set_respect_gitignore(&mut self, respect: bool) {
        if self.respect_gitignore != respect {
            self.respect_gitignore = respect;
            self.rebuild_gitignore();
            self.reload_loaded();
        }
    }

    /// A bounded list of files under the workspace, honoring the visibility settings.
    pub fn file_index(&self, limit: usize) -> Vec<PathBuf> {
        let mut files = Vec::new();
        let mut directories = vec![self.root.path.clone()];
        while let Some(directory) = directories.pop() {
            for entry in self.scan_dir(&directory) {
                if entry.is_dir {
                    directories.push(entry.path);
                } else if files.len() < limit {
                    files.push(entry.path);
                }
            }
            if files.len() >= limit {
                break;
            }
        }
        files
    }

    /// Wake the application whenever the native watcher reports events.
    pub fn set_waker(&mut self, waker: Waker) {
        if let Ok(mut slot) = self.waker.lock() {
            *slot = Some(waker);
        }
    }

    /// Whether the workspace rescans loaded folders on a timer because no native watcher
    /// could be installed. The application must then call [`Workspace::poll`] about once a
    /// second; the watcher's waker never fires in this mode.
    pub fn is_polling(&self) -> bool {
        self.polling
    }

    /// A warning when the native watcher could not be installed.
    pub fn watcher_warning(&self) -> Option<&str> {
        self.watcher_warning.as_deref()
    }

    /// Expand a directory, loading its children on first use.
    pub fn expand(&mut self, path: &Path) {
        if self.root.find(path).is_some_and(|entry| entry.is_dir) {
            let needs_load = self.root.find(path).is_some_and(|entry| entry.children.is_none());
            if needs_load {
                let children = self.scan_dir(path);
                if let Some(entry) = self.root.find_mut(path) {
                    entry.children = Some(children);
                }
            }
            if let Some(entry) = self.root.find_mut(path) {
                entry.expanded = true;
            }
        }
    }

    /// Collapse a directory.
    pub fn collapse(&mut self, path: &Path) {
        if let Some(entry) = self.root.find_mut(path) {
            entry.expanded = false;
        }
    }

    /// Toggle a directory.
    pub fn toggle(&mut self, path: &Path) {
        let expanded = self.root.find(path).is_some_and(|entry| entry.expanded);
        if expanded {
            self.collapse(path);
        } else {
            self.expand(path);
        }
    }

    /// Reload one loaded directory from disk.
    ///
    /// Rescanning replaces the child entries, so the expansions below that directory are
    /// captured first and re-applied afterwards. Without this, any watcher event in a parent
    /// (including the workspace root) would silently collapse folders the user had opened.
    pub fn refresh_dir(&mut self, path: &Path) {
        if path == self.root.path {
            self.reload_root();
            return;
        }
        let loaded = self.root.find(path).is_some_and(|entry| entry.children.is_some());
        if !loaded {
            return;
        }
        let mut expanded = Vec::new();
        if let Some(entry) = self.root.find(path) {
            collect_expanded_dirs(entry, &mut expanded);
        }
        let children = self.scan_dir(path);
        if let Some(entry) = self.root.find_mut(path) {
            entry.children = Some(children);
        }
        for expanded_path in expanded {
            if expanded_path != path {
                self.expand(&expanded_path);
            }
        }
    }

    /// Poll watcher events and refresh affected loaded directories.
    ///
    /// Returns true when the visible tree changed.
    pub fn poll(&mut self) -> bool {
        let mut changed = false;
        loop {
            let received = self.events.as_ref().and_then(|events| events.try_recv().ok());
            let Some(paths) = received else {
                break;
            };
            self.changed_paths.extend(paths.iter().cloned());
            let mut parents: Vec<PathBuf> = paths
                .iter()
                .filter_map(|path| path.parent().map(Path::to_path_buf))
                .collect();
            parents.sort();
            parents.dedup();
            let before = self.root.clone();
            for parent in parents {
                self.refresh_dir(&parent);
            }
            if self.root != before {
                changed = true;
            }
        }
        if self.polling && self.last_poll.elapsed() >= Duration::from_secs(1) {
            self.last_poll = Instant::now();
            let before = self.root.clone();
            self.reload_loaded();
            changed |= self.root != before;
        }
        changed
    }

    /// Paths reported by the watcher since the previous call.
    ///
    /// The application uses these to reconcile open documents with disk changes.
    pub fn take_changed_paths(&mut self) -> Vec<PathBuf> {
        let mut paths = std::mem::take(&mut self.changed_paths);
        paths.sort();
        paths.dedup();
        paths
    }

    fn reload_root(&mut self) {
        let mut expanded = Vec::new();
        collect_expanded_dirs(&self.root, &mut expanded);
        self.root.children = Some(self.scan_dir(&self.root.path));
        // Reloading the root must not collapse the tree the user just opened.
        for path in expanded {
            self.expand(&path);
        }
    }

    fn reload_loaded(&mut self) {
        let mut directories = Vec::new();
        collect_loaded_dirs(&self.root, &mut directories);
        for path in directories {
            self.refresh_dir(&path);
        }
    }

    fn scan_dir(&self, directory: &Path) -> Vec<Entry> {
        let Ok(read) = fs::read_dir(directory) else {
            return Vec::new();
        };
        let mut entries: Vec<Entry> = read
            .filter_map(Result::ok)
            .filter_map(|entry| {
                let path = entry.path();
                let name = entry.file_name().to_string_lossy().into_owned();
                let is_dir = entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false);
                if !self.show_hidden && name.starts_with('.') {
                    return None;
                }
                if is_dir && matches!(name.as_str(), "target" | "node_modules") {
                    return None;
                }
                if self.respect_gitignore
                    && let Some(gitignore) = &self.gitignore
                    && gitignore.matched_path_or_any_parents(&path, is_dir).is_ignore()
                {
                    return None;
                }
                Some(Entry {
                    path,
                    name,
                    is_dir,
                    expanded: false,
                    children: None,
                })
            })
            .collect();
        entries.sort_by(|left, right| {
            right
                .is_dir
                .cmp(&left.is_dir)
                .then_with(|| left.name.to_lowercase().cmp(&right.name.to_lowercase()))
        });
        entries
    }

    fn rebuild_gitignore(&mut self) {
        if !self.respect_gitignore {
            self.gitignore = None;
            return;
        }
        let mut builder = GitignoreBuilder::new(&self.root.path);
        let gitignore = self.root.path.join(".gitignore");
        if gitignore.is_file() {
            let _ = builder.add(&gitignore);
        }
        self.gitignore = builder.build().ok();
    }

    fn start_watcher(&mut self) {
        let (sender, receiver) = mpsc::channel();
        let root = self.root.path.clone();
        let waker = self.waker.clone();
        let mut debouncer = match new_debouncer(
            Duration::from_millis(250),
            None,
            move |result: notify_debouncer_full::DebounceEventResult| {
                if let Ok(events) = result {
                    let paths = events
                        .into_iter()
                        .flat_map(|event| event.paths.clone())
                        .collect::<Vec<_>>();
                    if sender.send(paths).is_ok()
                        && let Ok(slot) = waker.lock()
                        && let Some(wake) = slot.as_ref()
                    {
                        wake();
                    }
                }
            },
        ) {
            Ok(debouncer) => debouncer,
            Err(error) => {
                self.watcher_warning = Some(error.to_string());
                self.polling = true;
                return;
            }
        };
        if let Err(error) = debouncer.watch(root, RecursiveMode::Recursive) {
            self.watcher_warning = Some(error.to_string());
            self.polling = true;
            return;
        }
        self.watcher = Some(debouncer);
        self.events = Some(receiver);
    }
}

fn collect_expanded_dirs(entry: &Entry, out: &mut Vec<PathBuf>) {
    if entry.is_dir && entry.expanded {
        out.push(entry.path.clone());
    }
    if entry.expanded
        && let Some(children) = &entry.children
    {
        for child in children {
            collect_expanded_dirs(child, out);
        }
    }
}

fn collect_loaded_dirs(entry: &Entry, out: &mut Vec<PathBuf>) {
    if entry.is_dir && entry.children.is_some() {
        out.push(entry.path.clone());
    }
    if entry.expanded
        && let Some(children) = &entry.children
    {
        for child in children {
            collect_loaded_dirs(child, out);
        }
    }
}

#[cfg(test)]
#[path = "workspace_tests.rs"]
mod tests;
