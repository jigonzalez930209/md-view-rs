//! Opening, saving, closing, and reconciling documents.
//!
//! The application keeps one tab per file. This module owns the file-level transitions: opening a
//! path into a tab, saving it back, closing a clean tab, and reacting to disk changes reported by
//! the workspace watcher or the periodic safety poll.

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use eframe::egui;
use mdview_model::{Document, Encoding, Eol};

use crate::app::MdViewApp;

use crate::file_ops::{self, FileError};

mod conflict;
pub(crate) mod disk_poll;
mod session;
mod watch;
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
        let started = std::time::Instant::now();
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        let current = self.tabs.active().editor.document().path().map(Path::to_path_buf);
        if current.as_deref() != Some(path.as_path()) {
            self.record_navigation();
        }
        if let Some(index) = self.tabs.find_path(&path) {
            self.tabs.set_active(index);
            log::info!("file focus existing path={}", path.display());
            return Ok(());
        }
        if crate::viewers::supports(&path) {
            let mut document = Document::from_loaded(path.clone(), String::new(), Eol::Lf, Encoding::Utf8, false);
            document.mark_saved();
            log::info!(
                "viewer open path={} plugin={} elapsed_ms={:.3}",
                path.display(),
                crate::viewers::name_for(&path).unwrap_or("viewer"),
                started.elapsed().as_secs_f64() * 1000.0
            );
            self.open_document(document, file_ops::modified(&path));
            return Ok(());
        }
        let length = std::fs::metadata(&path)?.len();
        if length > 10 * 1024 * 1024 {
            if let Some(job) = self.pending_load.take() {
                job.cancel();
            }
            log::info!("file load queued path={} bytes={length}", path.display());
            self.pending_load = Some(LoadJob::spawn(path)?);
            return Ok(());
        }
        let document = file_ops::load(&path)?;
        log::info!(
            "file open path={} bytes={} lines={} elapsed_ms={:.3}",
            path.display(),
            document.len_bytes(),
            document.line_count(),
            started.elapsed().as_secs_f64() * 1000.0
        );
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
            if let Some(ratio) = session.tab_split_ratio.get(&text).copied()
                && let Some(tab) = self.tabs.tab_mut(index)
            {
                tab.restore_split_ratio(ratio);
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
        self.media.clear_cache();
        self.sync_session();
        self.update_title(ctx);
    }

    /// Ask for a path or save directly, then optionally close the window.
    pub(crate) fn request_save(&mut self, close_after: bool) {
        if self.tabs.active().is_viewer() {
            log::debug!("image document save ignored");
            return;
        }
        let Some(path) = self.tabs.editor().document().path().map(PathBuf::from) else {
            self.ask_save_as(close_after);
            return;
        };
        self.save_to(&path, close_after, None);
    }

    /// Save the active tab to a concrete path.
    pub(crate) fn save_to(&mut self, path: &Path, close_after: bool, ctx: Option<&egui::Context>) {
        if self.tabs.active().is_viewer() {
            log::debug!("image document save ignored");
            return;
        }
        let started = std::time::Instant::now();
        let bytes = self.tabs.editor().document().len_bytes();
        match file_ops::save(self.tabs.editor().document(), path) {
            Ok(()) => {
                log::info!(
                    "file save path={} bytes={bytes} elapsed_ms={:.3}",
                    path.display(),
                    started.elapsed().as_secs_f64() * 1000.0
                );
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
                log::error!("file save failed path={} error={error}", path.display());
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
}
