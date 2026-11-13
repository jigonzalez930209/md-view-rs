//! Workspace, session, and external-path flows.

use std::path::{Path, PathBuf};

use eframe::egui;
use mdview_theme::ThemeMode;

use crate::app::MdViewApp;
use crate::workspace_ops;

impl MdViewApp {
    /// Open a workspace folder and remember it.
    pub(crate) fn open_workspace(&mut self, path: &Path, ctx: &egui::Context) {
        let path = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());
        match mdview_workspace::Workspace::open(&path) {
            Ok(mut workspace) => {
                self.sidebar.show_hidden = self.prefs.get().session.show_hidden;
                self.sidebar.respect_gitignore = self.prefs.get().session.respect_gitignore;
                workspace.set_show_hidden(self.sidebar.show_hidden);
                workspace.set_respect_gitignore(self.sidebar.respect_gitignore);
                let text = path.to_string_lossy().into_owned();
                self.prefs
                    .update(|prefs| prefs.session.remember_recent_workspace(&text));
                self.workspace = Some(workspace);
                self.sidebar.open = true;
                self.sync_session();
                ctx.request_repaint();
            }
            Err(error) => {
                self.show_error(format!("{} {error}", self.t("Could not open folder.")));
            }
        }
    }

    /// Close the workspace folder.
    pub(crate) fn close_workspace(&mut self, ctx: &egui::Context) {
        self.workspace = None;
        self.sync_session();
        ctx.request_repaint();
    }

    /// Create a new file in a directory and open it.
    pub(crate) fn create_file_in(&mut self, parent: &Path, ctx: &egui::Context) {
        match workspace_ops::create_file(parent, "untitled.md") {
            Ok(path) => {
                if let Some(workspace) = &mut self.workspace {
                    workspace.refresh_dir(parent);
                }
                if let Err(error) = self.open_path(&path) {
                    self.show_error(format!("{} {error}", self.t("Could not open file.")));
                }
                self.update_title(ctx);
            }
            Err(error) => self.show_error(error.to_string()),
        }
    }

    /// Restore the previous session when no explicit paths were passed on the command line.
    pub(crate) fn restore_session(&mut self, cc: &eframe::CreationContext<'_>) {
        let session = self.prefs.get().session.clone();
        if let Some(workspace) = session.workspace.as_deref().map(PathBuf::from)
            && workspace.is_dir()
        {
            self.open_workspace(&workspace, &cc.egui_ctx);
        }
        for file in &session.open_files {
            let path = PathBuf::from(file);
            if path.is_file() {
                if let Err(error) = self.open_path(&path) {
                    log::warn!("session: could not reopen {}: {error}", path.display());
                }
            } else {
                log::warn!("session: {} is missing", path.display());
                let mut document = mdview_model::Document::untitled();
                document.set_path(path);
                self.open_document(document, None);
                let index = self.tabs.active_index();
                if let Some(tab) = self.tabs.tab_mut(index) {
                    tab.missing = true;
                }
            }
        }
        if let Some(active) = session.active_file.as_deref().map(PathBuf::from)
            && let Some(index) = self.tabs.find_path(&active)
        {
            self.tabs.set_active(index);
        }
        self.sidebar.open = session.sidebar_open;
        self.sidebar.width = session.sidebar_width;
        self.sync_session();
    }

    /// Apply a path forwarded by another instance or the command line.
    pub(crate) fn handle_external_path(&mut self, path: &Path, ctx: &egui::Context) {
        if path.as_os_str().is_empty() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Focus);
            return;
        }
        if path.is_dir() {
            self.open_workspace(path, ctx);
            return;
        }
        if let Err(error) = self.open_path(path) {
            self.show_error(format!("{} {error}", self.t("Could not open file.")));
        } else {
            self.update_title(ctx);
        }
    }

    /// Resolve the active theme against the system preference and apply it everywhere.
    pub(crate) fn resolve_theme(&mut self, ctx: &egui::Context) {
        let system_dark = ctx
            .input(|input| input.raw.system_theme)
            .is_some_and(|theme| theme == egui::Theme::Dark);
        self.last_system_dark = system_dark;
        self.effective_theme = if self.follow_system {
            self.theme_id.follow_system(system_dark)
        } else {
            self.theme_id
        };
        self.theme = self.effective_theme.mode();
        self.tabs.set_light(self.theme == ThemeMode::Light);
        mdview_theme::apply_id(ctx, self.effective_theme);
        ctx.request_repaint();
    }
}
