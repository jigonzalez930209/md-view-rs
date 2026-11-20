//! Watcher reconciliation and disk polling.

use std::path::PathBuf;

use eframe::egui;

use crate::app::MdViewApp;
use crate::conflict::{DiskState, classify_disk};
use crate::file_ops;

impl MdViewApp {
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

    /// Check every open file when the disk poller saw one change outside the application.
    ///
    /// The poller thread stats the files and wakes the UI, so an idle window draws no
    /// frames just to look for changes.
    pub(crate) fn poll_open_files(&mut self, ctx: &egui::Context) {
        let files = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                let path = tab.editor.document().path()?;
                Some((path.to_path_buf(), tab.disk_mtime))
            })
            .collect();
        self.disk_poller.watch(files);
        if !self.disk_poller.take_changed() {
            return;
        }
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
        if self.tabs.tabs().get(index).is_some_and(|tab| tab.is_viewer()) {
            if let Some(tab) = self.tabs.tab_mut(index) {
                tab.disk_mtime = mtime;
            }
            self.media.clear_cache();
            ctx.request_repaint();
            return;
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
}
