//! Lazy, reactive workspace tree.
//!
//! The tree loads directory children only when a folder is expanded. A debounced native watcher
//! reports changed paths; [`Workspace::poll`] reloads the affected loaded directories. When a
//! native watcher cannot be installed, the workspace falls back to periodic rescans of the
//! directories that are already loaded.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use ignore::gitignore::{Gitignore, GitignoreBuilder};
use notify::RecursiveMode;
use notify_debouncer_full::{Debouncer, RecommendedCache, new_debouncer};

type NativeWatcher = Debouncer<notify::RecommendedWatcher, RecommendedCache>;

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
    pub fn refresh_dir(&mut self, path: &Path) {
        if path == self.root.path {
            self.reload_root();
            return;
        }
        let loaded = self.root.find(path).is_some_and(|entry| entry.children.is_some());
        if loaded {
            let children = self.scan_dir(path);
            if let Some(entry) = self.root.find_mut(path) {
                entry.children = Some(children);
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
        self.root.children = Some(self.scan_dir(&self.root.path));
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
        let mut debouncer = match new_debouncer(
            Duration::from_millis(250),
            None,
            move |result: notify_debouncer_full::DebounceEventResult| {
                if let Ok(events) = result {
                    let paths = events
                        .into_iter()
                        .flat_map(|event| event.paths.clone())
                        .collect::<Vec<_>>();
                    let _ = sender.send(paths);
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
mod tests {
    use super::*;

    #[test]
    fn lazy_tree_filters_and_expands() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::create_dir(dir.path().join("src")).expect("src");
        fs::write(dir.path().join("src/main.rs"), "fn main() {}\n").expect("file");
        fs::write(dir.path().join(".hidden"), "hidden").expect("hidden");
        let mut workspace = Workspace::open(dir.path()).expect("open");
        assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(1));
        let src = workspace.root().join("src");
        workspace.expand(&src);
        assert_eq!(
            workspace
                .root_entry()
                .find(&src)
                .and_then(|entry| entry.children.as_ref())
                .map(Vec::len),
            Some(1)
        );
        workspace.set_show_hidden(true);
        assert!(workspace.root_entry().find(&workspace.root().join(".hidden")).is_some());
    }

    #[test]
    fn gitignore_and_hidden_files_are_filtered() {
        let dir = tempfile::tempdir().expect("temp dir");
        fs::write(dir.path().join(".gitignore"), "ignored.md\nbuild/\n").expect("gitignore");
        fs::write(dir.path().join("kept.md"), "kept").expect("kept");
        fs::write(dir.path().join("ignored.md"), "ignored").expect("ignored");
        fs::create_dir(dir.path().join("build")).expect("build");
        let mut workspace = Workspace::open(dir.path()).expect("open");
        let names: Vec<&str> = workspace
            .root_entry()
            .children
            .as_deref()
            .unwrap_or_default()
            .iter()
            .map(|entry| entry.name.as_str())
            .collect();
        assert!(names.contains(&"kept.md"));
        assert!(!names.contains(&"ignored.md"));
        assert!(!names.contains(&"build"));
        assert!(!names.contains(&".gitignore"));
        workspace.set_show_hidden(true);
        assert!(workspace.root_entry().find(&dir.path().join(".gitignore")).is_some());
        workspace.set_respect_gitignore(false);
        assert!(workspace.root_entry().find(&dir.path().join("ignored.md")).is_some());
    }

    #[test]
    fn watcher_reports_changed_paths_or_falls_back_to_polling() {
        let dir = tempfile::tempdir().expect("temp dir");
        let mut workspace = Workspace::open(dir.path()).expect("open");
        let path = dir.path().join("watched.md");
        fs::write(&path, "one").expect("write");
        if workspace.watcher_warning().is_some() {
            workspace.refresh_dir(dir.path());
            assert!(workspace.root_entry().find(&path).is_some());
            return;
        }
        let deadline = Instant::now() + Duration::from_secs(5);
        let mut reported = false;
        while Instant::now() < deadline {
            workspace.poll();
            if workspace.take_changed_paths().contains(&path) {
                reported = true;
                break;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        assert!(reported, "the watcher did not report the new file");
        assert!(workspace.root_entry().find(&path).is_some());
    }

    #[test]
    fn fifty_thousand_files_load_lazily() {
        let dir = tempfile::tempdir().expect("temp dir");
        for index in 0..50_000 {
            std::fs::File::create(dir.path().join(format!("file{index:05}.md"))).expect("file");
        }
        let nested = dir.path().join("nested");
        fs::create_dir(&nested).expect("nested");
        fs::write(nested.join("deep.md"), "deep").expect("deep");
        let start = Instant::now();
        let mut workspace = Workspace::open(dir.path()).expect("open");
        let elapsed = start.elapsed();
        assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(50_001));
        let nested_entry = workspace.root_entry().find(&nested).expect("nested entry");
        assert!(nested_entry.children.is_none(), "children load only on expand");
        workspace.expand(&nested);
        assert_eq!(
            workspace
                .root_entry()
                .find(&nested)
                .and_then(|entry| entry.children.as_ref())
                .map(Vec::len),
            Some(1)
        );
        eprintln!("50k workspace first level loaded in {elapsed:?}");
    }

    #[test]
    fn refreshes_created_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let workspace = Workspace::open(dir.path()).expect("open");
        assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(0));
        fs::write(dir.path().join("new.md"), "# new\n").expect("write");
        let mut workspace = workspace;
        let root = workspace.root().to_path_buf();
        workspace.refresh_dir(&root);
        assert_eq!(workspace.root_entry().children.as_ref().map(Vec::len), Some(1));
    }
}
