//! Opening, saving, closing, and reconciling documents.
//!
//! The application keeps one tab per file. This module owns the file-level transitions: opening a
//! path into a tab, saving it back, closing a clean tab, and reacting to disk changes reported by
//! the workspace watcher or the periodic safety poll.

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use eframe::egui;
use mdview_model::{Document, Encoding};

use crate::app::MdViewApp;
use crate::conflict::{ConflictAction, DiskState, classify_disk};
use crate::file_ops::{self, FileError};
use crate::load_job::{LoadEvent, LoadJob};

/// The comparison window state.
pub struct DiffState {
    /// Index of the conflicting tab.
    pub index: usize,
    /// The local buffer.
    pub mine: String,
    /// The disk contents.
    pub disk: String,
    /// Whether the window is open.
    pub open: bool,
}

impl MdViewApp {
    /// Open a file path in a tab, reusing an existing tab when it is already open.
    pub(crate) fn open_path(&mut self, path: &Path) -> Result<(), FileError> {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        if let Some(index) = self.tabs.find_path(&path) {
            self.tabs.set_active(index);
            return Ok(());
        }
        let length = std::fs::metadata(&path)?.len();
        if length > 10 * 1024 * 1024 {
            if let Some(job) = self.pending_load.take() {
                job.cancel();
            }
            self.pending_load = Some(LoadJob::spawn(path)?);
            return Ok(());
        }
        let document = file_ops::load(&path)?;
        self.open_document(document, file_ops::modified(&path));
        Ok(())
    }

    /// Put a loaded document into a new tab and remember it for the session.
    pub(crate) fn open_document(&mut self, document: Document, mtime: Option<SystemTime>) {
        let path = document.path().map(Path::to_path_buf);
        self.tabs.open(document, mtime);
        let index = self.tabs.active_index();
        if let Some(path) = path {
            let text = path.to_string_lossy().into_owned();
            self.prefs.update(|prefs| prefs.session.remember_recent(&text));
            let session = self.prefs.get().session.clone();
            if let Some(offset) = session.tab_scroll.get(&text).copied() {
                self.tabs.set_scroll_offset(index, offset);
            }
            if let Some(offset) = session.tab_view_scroll.get(&text).copied()
                && let Some(tab) = self.tabs.tab_mut(index)
            {
                tab.restore_view_scroll(offset);
            }
            if let Some(mode) = session.tab_modes.get(&text).map(String::as_str)
                && let Some(tab) = self.tabs.tab_mut(index)
            {
                tab.set_mode(crate::tabs::ViewMode::from_setting(mode));
            }
        }
        self.sync_session();
    }

    /// Add an empty tab.
    pub(crate) fn new_document(&mut self, ctx: &egui::Context) {
        self.tabs.new_untitled();
        self.sync_session();
        self.update_title(ctx);
    }

    /// Close a tab when it has no unsaved changes.
    pub(crate) fn close_tab(&mut self, index: usize, ctx: &egui::Context) {
        let Some(tab) = self.tabs.tabs().get(index) else {
            return;
        };
        if tab.editor.document().is_dirty() {
            self.show_error(self.t("Save or discard your changes first."));
            return;
        }
        self.tabs.close(index);
        self.sync_session();
        self.update_title(ctx);
    }

    /// Ask for a path or save directly, then optionally close the window.
    pub(crate) fn request_save(&mut self, close_after: bool) {
        let Some(path) = self.tabs.editor().document().path().map(PathBuf::from) else {
            self.ask_save_as(close_after);
            return;
        };
        self.save_to(&path, close_after, None);
    }

    /// Save the active tab to a concrete path.
    pub(crate) fn save_to(&mut self, path: &Path, close_after: bool, ctx: Option<&egui::Context>) {
        match file_ops::save(self.tabs.editor().document(), path) {
            Ok(()) => {
                let mtime = file_ops::modified(path);
                let tab = self.tabs.active_mut();
                tab.editor.document_mut().set_path(path.to_path_buf());
                tab.editor.document_mut().mark_saved();
                tab.disk_mtime = mtime;
                tab.conflict = false;
                tab.missing = false;
                let text = path.to_string_lossy().into_owned();
                self.prefs.update(|prefs| prefs.session.remember_recent(&text));
                self.sync_session();
                if let Some(ctx) = ctx {
                    self.update_title(ctx);
                }
                if close_after {
                    self.close_prompt_open = false;
                    if let Some(ctx) = ctx {
                        self.continue_close(ctx);
                    }
                }
            }
            Err(error) => {
                self.show_error(format!("{} {error}", self.t("Could not save file.")));
            }
        }
    }

    /// Save every dirty tab and then close. Untitled tabs are saved first through Save As.
    pub(crate) fn continue_close(&mut self, ctx: &egui::Context) {
        let Some(index) = self.tabs.first_dirty() else {
            self.allow_close = true;
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            return;
        };
        self.tabs.set_active(index);
        if self.tabs.active().editor.document().path().is_some() {
            let path = self
                .tabs
                .active()
                .editor
                .document()
                .path()
                .map(Path::to_path_buf)
                .unwrap_or_default();
            self.save_to(&path, true, Some(ctx));
        } else {
            self.ask_save_as(true);
        }
    }

    /// Reload the active file using a user-selected encoding.
    pub(crate) fn reload_encoding(&mut self, ctx: &egui::Context, encoding: Encoding) {
        if self.tabs.editor().document().is_dirty() {
            self.show_error(self.t("Save or discard your changes first."));
            return;
        }
        let Some(path) = self.tabs.editor().document().path().map(PathBuf::from) else {
            return;
        };
        match file_ops::load_with_encoding(&path, encoding) {
            Ok(document) => {
                self.tabs.editor_mut().replace_document_preserving_position(document);
                let index = self.tabs.active_index();
                self.tabs.configure(index);
                self.tabs.active_mut().disk_mtime = file_ops::modified(&path);
                self.tabs.active_mut().conflict = false;
                self.sync_session();
                self.update_title(ctx);
            }
            Err(error) => self.show_error(format!("{} {error}", self.t("Could not open file."))),
        }
    }

    /// Reload open tabs after a workspace rename or delete.
    pub(crate) fn handle_sidebar_event(&mut self, event: &crate::sidebar::SidebarEvent, ctx: &egui::Context) {
        match event {
            crate::sidebar::SidebarEvent::Open(path) => {
                if let Err(error) = self.open_path(path) {
                    self.show_error(format!("{} {error}", self.t("Could not open file.")));
                } else {
                    self.update_title(ctx);
                }
            }
            crate::sidebar::SidebarEvent::Renamed { from, to } => {
                if self.tabs.rename_path(from, to)
                    && let Some(index) = self.tabs.find_path(to)
                    && let Some(tab) = self.tabs.tab_mut(index)
                {
                    tab.disk_mtime = file_ops::modified(to);
                }
                self.sync_session();
            }
            crate::sidebar::SidebarEvent::Deleted(path) => {
                if let Some(index) = self.tabs.find_path(path)
                    && let Some(tab) = self.tabs.tab_mut(index)
                {
                    tab.missing = true;
                    tab.conflict = false;
                    self.tabs.set_active(index);
                }
                self.sync_session();
            }
        }
    }

    /// Persist the open files, active path, workspace, and sidebar state.
    pub(crate) fn sync_session(&mut self) {
        let open_files = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| tab.editor.document().path())
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let active_file = self
            .tabs
            .active()
            .editor
            .document()
            .path()
            .map(|path| path.to_string_lossy().into_owned());
        let workspace = self
            .workspace
            .as_ref()
            .map(|workspace| workspace.root().to_string_lossy().into_owned());
        let tab_scroll = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.editor.scroll_offset()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let tab_view_scroll = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.view_scroll()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let tab_modes = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.mode.as_setting().to_owned()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let sidebar_open = self.sidebar.open;
        let sidebar_width = self.sidebar.width;
        let show_hidden = self.sidebar.show_hidden;
        let respect_gitignore = self.sidebar.respect_gitignore;
        self.prefs.update(|prefs| {
            prefs.session.open_files = open_files;
            prefs.session.active_file = active_file;
            prefs.session.workspace = workspace;
            prefs.session.tab_scroll = tab_scroll;
            prefs.session.tab_view_scroll = tab_view_scroll;
            prefs.session.tab_modes = tab_modes;
            prefs.session.sidebar_open = sidebar_open;
            prefs.session.sidebar_width = sidebar_width;
            prefs.session.show_hidden = show_hidden;
            prefs.session.respect_gitignore = respect_gitignore;
        });
    }

    /// Poll the background loader and open its document when it finishes.
    pub(crate) fn poll_load(&mut self, ctx: &egui::Context) {
        let mut finished = None;
        if let Some(job) = self.pending_load.as_mut() {
            while let Some(event) = job.poll() {
                if let LoadEvent::Done(result) = event {
                    finished = Some(result);
                    break;
                }
            }
        }
        let Some(result) = finished else {
            return;
        };
        self.pending_load = None;
        match result {
            Ok(document) => {
                let mtime = document
                    .path()
                    .and_then(|path| std::fs::metadata(path).ok())
                    .and_then(|metadata| metadata.modified().ok());
                self.open_document(document, mtime);
                self.update_title(ctx);
            }
            Err(FileError::Cancelled) => {}
            Err(error) => self.show_error(format!("{} {error}", self.t("Could not open file."))),
        }
    }

    /// Reconcile tabs with paths reported by the workspace watcher.
    pub(crate) fn reconcile_paths(&mut self, paths: &[PathBuf], ctx: &egui::Context) {
        if paths.is_empty() {
            return;
        }
        let indexes = self
            .tabs
            .tabs()
            .iter()
            .enumerate()
            .filter(|(_, tab)| {
                tab.editor.document().path().is_some_and(|path| {
                    paths
                        .iter()
                        .any(|changed| changed == path || path.parent() == Some(changed.as_path()))
                })
            })
            .map(|(index, _)| index)
            .collect::<Vec<_>>();
        for index in indexes {
            self.check_tab_disk(index, ctx);
        }
    }

    /// Periodically check every open file for changes outside the watched workspace.
    pub(crate) fn poll_open_files(&mut self, ctx: &egui::Context) {
        if self.last_file_poll.elapsed() < Duration::from_millis(1_000) {
            return;
        }
        self.last_file_poll = std::time::Instant::now();
        for index in 0..self.tabs.len() {
            self.check_tab_disk(index, ctx);
        }
    }

    fn check_tab_disk(&mut self, index: usize, ctx: &egui::Context) {
        let Some((path, stored_mtime, dirty)) = self.tabs.tabs().get(index).and_then(|tab| {
            tab.editor
                .document()
                .path()
                .map(|path| (path.to_path_buf(), tab.disk_mtime, tab.editor.document().is_dirty()))
        }) else {
            return;
        };
        let mtime = file_ops::modified(&path);
        let changed = mtime != stored_mtime && mtime.is_some();
        match classify_disk(path.exists(), changed, dirty) {
            DiskState::Unchanged => return,
            DiskState::Missing => {
                if let Some(tab) = self.tabs.tab_mut(index)
                    && !tab.missing
                {
                    log::info!("file is missing on disk: {}", path.display());
                    tab.missing = true;
                    tab.conflict = false;
                    ctx.request_repaint();
                }
                return;
            }
            DiskState::Conflict => {
                if let Some(tab) = self.tabs.tab_mut(index) {
                    if !tab.conflict {
                        log::info!("disk conflict for dirty file: {}", path.display());
                    }
                    tab.conflict = true;
                    tab.missing = false;
                }
                ctx.request_repaint();
                return;
            }
            DiskState::Reload => {
                if let Some(tab) = self.tabs.tab_mut(index) {
                    tab.missing = false;
                }
            }
        }
        if let Ok(document) = file_ops::load(&path) {
            log::info!("reloaded externally changed clean file: {}", path.display());
            if let Some(tab) = self.tabs.tab_mut(index) {
                tab.editor.replace_document_preserving_position(document);
                tab.disk_mtime = mtime;
                tab.conflict = false;
            }
            self.tabs.configure(index);
            ctx.request_repaint();
        }
    }

    /// Ask for confirmation before a destructive conflict choice.
    pub(crate) fn confirm_conflict(&mut self, index: usize, action: ConflictAction, ctx: &egui::Context) {
        match action {
            ConflictAction::Compare => self.apply_conflict(index, action, ctx),
            ConflictAction::Reload => self.dialogs.ask_message(
                craft_dialogs::Message {
                    level: craft_dialogs::MessageLevel::Warning,
                    title: self.t("Reload from disk").to_owned(),
                    description: self
                        .t("Discard your unsaved changes and reload the disk version?")
                        .to_owned(),
                    buttons: craft_dialogs::MessageButtons::YesNo,
                },
                crate::app::DialogThen::ReloadConflict(index),
            ),
            ConflictAction::KeepMine => self.dialogs.ask_message(
                craft_dialogs::Message {
                    level: craft_dialogs::MessageLevel::Warning,
                    title: self.t("Keep my changes").to_owned(),
                    description: self
                        .t("Keep your buffer and overwrite the disk version when you save?")
                        .to_owned(),
                    buttons: craft_dialogs::MessageButtons::YesNo,
                },
                crate::app::DialogThen::KeepMineConflict(index),
            ),
        }
    }

    /// Apply a choice from the conflict banner after confirmation.
    pub(crate) fn apply_conflict(&mut self, index: usize, action: ConflictAction, ctx: &egui::Context) {
        let Some(tab) = self.tabs.tabs().get(index) else {
            return;
        };
        let Some(path) = tab.editor.document().path().map(Path::to_path_buf) else {
            return;
        };
        match action {
            ConflictAction::Reload => match file_ops::load(&path) {
                Ok(document) => {
                    if let Some(tab) = self.tabs.tab_mut(index) {
                        tab.editor.replace_document_preserving_position(document);
                        tab.disk_mtime = file_ops::modified(&path);
                        tab.conflict = false;
                        tab.missing = false;
                    }
                    self.tabs.configure(index);
                    self.tabs.set_active(index);
                    self.sync_session();
                    self.update_title(ctx);
                }
                Err(error) => {
                    self.show_error(format!("{} {error}", self.t("Could not open file.")));
                }
            },
            ConflictAction::KeepMine => {
                if let Some(tab) = self.tabs.tab_mut(index) {
                    tab.conflict = false;
                    tab.disk_mtime = file_ops::modified(&path);
                }
                self.tabs.set_active(index);
            }
            ConflictAction::Compare => {
                let mine = tab.editor.document().snapshot();
                let disk = std::fs::read(&path)
                    .ok()
                    .and_then(|bytes| String::from_utf8(bytes).ok())
                    .unwrap_or_else(|| mine.clone());
                self.diff = Some(DiffState {
                    index,
                    mine,
                    disk,
                    open: true,
                });
            }
        }
        ctx.request_repaint();
    }
}
