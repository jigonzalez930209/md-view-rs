//! Menu commands, shortcuts, and native dialog continuations.

use std::path::PathBuf;

use eframe::egui;
use mdview_ui::Command;

mod dialogs;
mod shortcuts;

use crate::app::MdViewApp;

impl MdViewApp {
    pub(crate) fn handle_command(&mut self, ctx: &egui::Context, command: Command) {
        log::info!("command {command:?}");
        match command {
            Command::New => self.new_document(ctx),
            Command::Open => self.ask_open(),
            Command::OpenFolder => self.ask_open_folder(),
            Command::CloseFolder => self.close_workspace(ctx),
            Command::RenameFile => {
                if let Some(path) = self
                    .tabs
                    .active()
                    .editor
                    .document()
                    .path()
                    .map(std::path::Path::to_path_buf)
                {
                    self.sidebar.begin_rename(path);
                    self.sidebar.reveal();
                    ctx.request_repaint();
                } else {
                    self.show_error(self.t("This document has no file to rename."));
                }
            }
            Command::DeleteFile => {
                if let Some(path) = self
                    .tabs
                    .active()
                    .editor
                    .document()
                    .path()
                    .map(std::path::Path::to_path_buf)
                {
                    self.sidebar.begin_delete(path);
                    self.sidebar.reveal();
                    ctx.request_repaint();
                } else {
                    self.show_error(self.t("This document has no file to delete."));
                }
            }
            Command::NewFile => {
                if let Some(workspace) = &self.workspace {
                    let parent = workspace.root().to_path_buf();
                    self.create_file_in(&parent, ctx);
                }
            }
            Command::OpenRecentWorkspace(index) => {
                let recent = self.prefs.get().session.recent_workspaces.clone();
                if let Some(path) = recent.get(index) {
                    let path = PathBuf::from(path);
                    if path.is_dir() {
                        self.open_workspace(&path, ctx);
                    } else {
                        self.show_error(self.t("The recent workspace no longer exists."));
                    }
                }
            }
            Command::OpenPath(path) => {
                if let Err(error) = self.open_path(&path) {
                    self.show_error(format!("{} {error}", self.t("Could not open file.")));
                } else {
                    self.update_title(ctx);
                }
            }
            Command::OpenRecent(index) => {
                let recent = self.prefs.get().session.recent_files.clone();
                if let Some(path) = recent.get(index) {
                    let path = PathBuf::from(path);
                    if path.is_file() {
                        if let Err(error) = self.open_path(&path) {
                            self.show_error(format!("{} {error}", self.t("Could not open file.")));
                        } else {
                            self.update_title(ctx);
                        }
                    } else {
                        self.show_error(self.t("The recent file no longer exists."));
                    }
                }
            }
            Command::ClearRecent => {
                self.prefs.update(|prefs| prefs.session.recent_files.clear());
            }
            Command::Save => self.request_save(false),
            Command::SaveAs => self.ask_save_as(false),
            Command::CloseTab => self.close_tab(self.tabs.active_index(), ctx),
            Command::CloseOtherTabs => {
                let active = self.tabs.active_index();
                let close = self
                    .tabs
                    .tabs()
                    .iter()
                    .enumerate()
                    .filter(|(index, tab)| *index != active && !tab.editor.document().is_dirty())
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                for index in close.into_iter().rev() {
                    self.tabs.close(index);
                }
                self.sync_session();
                self.update_title(ctx);
            }
            Command::CloseSavedTabs => {
                let close = self
                    .tabs
                    .tabs()
                    .iter()
                    .enumerate()
                    .filter(|(_, tab)| !tab.editor.document().is_dirty())
                    .map(|(index, _)| index)
                    .collect::<Vec<_>>();
                for index in close.into_iter().rev() {
                    self.tabs.close(index);
                }
                self.sync_session();
                self.update_title(ctx);
            }
            Command::MoveTabLeft => {
                let index = self.tabs.active_index();
                if index > 0 {
                    self.tabs.move_tab(index, index - 1);
                    self.sync_session();
                }
            }
            Command::MoveTabRight => {
                let index = self.tabs.active_index();
                if index + 1 < self.tabs.len() {
                    self.tabs.move_tab(index, index + 1);
                    self.sync_session();
                }
            }
            Command::NextTab => {
                let len = self.tabs.len();
                if len > 0 {
                    let next = (self.tabs.active_index() + 1) % len;
                    self.tabs.set_active(next);
                    self.update_title(ctx);
                }
            }
            Command::PreviousTab => {
                let len = self.tabs.len();
                if len > 0 {
                    let next = (self.tabs.active_index() + len - 1) % len;
                    self.tabs.set_active(next);
                    self.update_title(ctx);
                }
            }
            Command::ToggleSidebar => {
                self.sidebar.open = !self.sidebar.open;
                self.sync_session();
            }
            Command::Quit | Command::CloseWindow => {
                log::info!("window close requested");
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
            Command::MinimizeWindow => {
                ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
            }
            Command::ToggleMaximizeWindow => {
                let maximized = ctx.input(|input| input.viewport().maximized.unwrap_or(false));
                ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(!maximized));
            }
            Command::About => self.about_open = true,
            Command::Back => self.navigate_history(true, ctx),
            Command::Forward => self.navigate_history(false, ctx),
            Command::Search => {
                if self.search.open && self.search.scope == mdview_ui::SearchScope::Document {
                    self.search.close();
                } else {
                    self.search.open(mdview_ui::SearchScope::Document);
                }
            }
            Command::SearchWorkspace => self.search.open(mdview_ui::SearchScope::Workspace),
            Command::Find => {
                self.ensure_edit_mode(ctx);
                self.tabs.editor_mut().open_find(false);
            }
            Command::Replace => {
                self.ensure_edit_mode(ctx);
                self.tabs.editor_mut().open_find(true);
            }
            Command::GotoLine => {
                self.ensure_edit_mode(ctx);
                self.tabs.editor_mut().open_goto();
            }
            Command::ToggleWrap => {
                let wrap = !self.tabs.editor().status().wrap;
                let line_numbers = self.tabs.editor().status().line_numbers;
                self.tabs.set_view(wrap, line_numbers);
                self.prefs.update(|prefs| prefs.editor.wrap = wrap);
                self.sync_session();
            }
            Command::ToggleLineNumbers => {
                let line_numbers = !self.tabs.editor().status().line_numbers;
                let wrap = self.tabs.editor().status().wrap;
                self.tabs.set_view(wrap, line_numbers);
                self.prefs.update(|prefs| prefs.editor.line_numbers = line_numbers);
                self.sync_session();
            }
            Command::ReloadEncoding(encoding) => self.reload_encoding(ctx, encoding),
            Command::SetThemeId(id) => {
                self.theme_id = id;
                self.prefs.update(|prefs| prefs.set_theme_id(id));
                self.resolve_theme(ctx);
            }
            Command::SetZoom(zoom) => {
                let zoom = zoom.clamp(0.8, 2.0);
                self.prefs.update(|prefs| prefs.appearance.zoom = zoom);
                ctx.set_zoom_factor(zoom);
            }
            Command::ActivateTab(index) => {
                self.tabs.set_active(index);
                self.update_title(ctx);
            }
            Command::Settings => {
                self.palette_open = false;
                self.settings_open = true;
            }
            Command::CommandPalette => {
                self.settings_open = false;
                self.palette_open = true;
                self.palette_files = false;
                self.palette_query.clear();
            }
            Command::CheckUpdates => self.check_for_updates(),
            Command::OpenLogs => {
                let directory = crate::logging::log_directory();
                if let Err(error) = open::that(&directory) {
                    self.show_error(format!("Could not open the log folder: {error}"));
                }
            }
            Command::QuickOpen => {
                if let Some(workspace) = &self.workspace {
                    self.quick_open_paths = workspace.file_index(5000);
                }
                self.settings_open = false;
                self.palette_open = true;
                self.palette_files = true;
                self.palette_query.clear();
            }
            Command::SetFollowSystem(follow) => {
                self.follow_system = follow;
                self.prefs.update(|prefs| prefs.appearance.follow_system = follow);
                self.resolve_theme(ctx);
            }
            Command::SetLanguage(preference) => {
                self.language = preference;
                self.translations = if preference == mdview_i18n::LanguagePreference::Pseudo {
                    mdview_i18n::Translations::with_pseudo()
                } else {
                    mdview_i18n::Translations::new()
                };
                self.lang = self.translations.lang(preference);
                self.prefs.update(|prefs| prefs.set_language_preference(preference));
            }
            Command::ToggleRemoteMedia => {
                let remote = !self.prefs.get().media.remote_images;
                self.prefs.update(|prefs| prefs.media.remote_images = remote);
            }
            Command::TogglePrivateNetworks => {
                let private = !self.prefs.get().media.private_networks;
                self.prefs.update(|prefs| prefs.media.private_networks = private);
            }
            Command::ToggleAnimations => {
                let animate = !self.prefs.get().media.animate_svg;
                self.prefs.update(|prefs| prefs.media.animate_svg = animate);
                self.media.set_playing(animate);
            }
            Command::ToggleReduceMotion => {
                let reduce = !self.prefs.get().media.reduce_motion;
                self.prefs.update(|prefs| prefs.media.reduce_motion = reduce);
                self.media.set_reduced_motion(reduce);
            }
            Command::SetReadingMargins(margins) => {
                self.prefs
                    .update(|prefs| prefs.reading.margins = margins.as_setting().to_owned());
            }
            Command::SetEditorStyle(style) => {
                let style = style.sanitized();
                self.prefs.update(|prefs| prefs.editor.set_style(style));
                self.tabs.set_style(style);
            }
            Command::SetReadingTypography(typography) => {
                let typography = typography.sanitized();
                self.prefs.update(|prefs| {
                    prefs.reading.body_size = typography.body;
                    prefs.reading.heading_size = typography.headings;
                    prefs.reading.code_size = typography.code;
                    prefs.reading.table_size = typography.tables;
                });
            }
            Command::ToggleShowHidden => {
                let show = !self.sidebar.show_hidden;
                self.sidebar.show_hidden = show;
                if let Some(workspace) = &mut self.workspace {
                    workspace.set_show_hidden(show);
                }
                self.prefs.update(|prefs| prefs.session.show_hidden = show);
                self.sync_session();
            }
            Command::ToggleRespectGitignore => {
                let respect = !self.sidebar.respect_gitignore;
                self.sidebar.respect_gitignore = respect;
                if let Some(workspace) = &mut self.workspace {
                    workspace.set_respect_gitignore(respect);
                }
                self.prefs.update(|prefs| prefs.session.respect_gitignore = respect);
                self.sync_session();
            }
            Command::ToggleSyncScroll => {
                let sync = !self.prefs.get().editor.sync_scroll;
                self.prefs.update(|prefs| prefs.editor.sync_scroll = sync);
                self.sync_session();
            }
            Command::SetMode(mode) => {
                let index = self.tabs.active_index();
                self.tabs.set_mode(index, mode);
                let effective = self.tabs.active().mode;
                self.prefs.update(|prefs| {
                    if let Some(path) = self.tabs.active().editor.document().path() {
                        prefs
                            .session
                            .tab_modes
                            .insert(path.to_string_lossy().into_owned(), effective.as_setting().to_owned());
                    }
                });
                self.sync_session();
            }
        }
    }
}
