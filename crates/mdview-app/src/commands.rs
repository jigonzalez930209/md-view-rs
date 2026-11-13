//! Menu commands, shortcuts, and native dialog continuations.

use std::path::PathBuf;

use craft_dialogs::{Answer, MessageResult};
use craft_keymap::Chord;
use eframe::egui;
use mdview_ui::Command;

use crate::app::{DialogThen, MdViewApp};

impl MdViewApp {
    pub(crate) fn handle_command(&mut self, ctx: &egui::Context, command: Command) {
        match command {
            Command::New => self.new_document(ctx),
            Command::Open => self.ask_open(),
            Command::OpenFolder => self.ask_open_folder(),
            Command::CloseFolder => self.close_workspace(ctx),
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
            Command::Quit => ctx.send_viewport_cmd(egui::ViewportCommand::Close),
            Command::About => self.about_open = true,
            Command::Find => self.tabs.editor_mut().open_find(false),
            Command::Replace => self.tabs.editor_mut().open_find(true),
            Command::GotoLine => self.tabs.editor_mut().open_goto(),
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
            Command::SetTheme(mode) => {
                self.theme = mode;
                self.apply_theme(mode);
                self.prefs.update(|prefs| prefs.set_theme_mode(mode));
                mdview_theme::apply(ctx, mode);
            }
            Command::SetLanguage(preference) => {
                self.language = preference;
                self.lang = self.translations.lang(preference);
                self.prefs.update(|prefs| prefs.set_language_preference(preference));
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

    pub(crate) fn handle_dialog_answers(&mut self, ctx: &egui::Context) {
        for (then, answer) in self.dialogs.take_answers() {
            match (then, answer) {
                (DialogThen::Open, Answer::Paths(paths)) => {
                    for path in paths {
                        if let Err(error) = self.open_path(&path) {
                            self.show_error(format!("{} {error}", self.t("Could not open file.")));
                        }
                    }
                    self.update_title(ctx);
                }
                (DialogThen::OpenFolder, Answer::Paths(paths)) => {
                    if let Some(path) = paths.first() {
                        self.open_workspace(path, ctx);
                    }
                }
                (DialogThen::SaveAs, Answer::Paths(paths)) => {
                    if let Some(path) = paths.first() {
                        self.save_to(path, false, Some(ctx));
                    }
                }
                (DialogThen::SaveAsClose, Answer::Paths(paths)) => {
                    if let Some(path) = paths.first() {
                        self.save_to(path, true, Some(ctx));
                    }
                }
                (DialogThen::ReloadConflict(index), Answer::Message(MessageResult::Yes)) => {
                    self.apply_conflict(index, crate::conflict::ConflictAction::Reload, ctx);
                }
                (DialogThen::KeepMineConflict(index), Answer::Message(MessageResult::Yes)) => {
                    self.apply_conflict(index, crate::conflict::ConflictAction::KeepMine, ctx);
                }
                (_, Answer::Unavailable(reason)) => {
                    self.show_error(reason);
                }
                _ => {}
            }
        }
    }

    fn consume_shortcut(&self, ctx: &egui::Context, text: &str) -> bool {
        let Ok(chord) = Chord::parse(text) else {
            log::warn!("invalid shortcut in application code: {text}");
            return false;
        };
        ctx.input_mut(|input| input.consume_shortcut(&chord.to_egui_shortcut()))
    }

    pub(crate) fn handle_shortcuts(&mut self, ctx: &egui::Context) {
        if self.dialogs.is_open() || self.close_prompt_open {
            return;
        }
        if self.consume_shortcut(ctx, "Cmd+N") {
            self.new_document(ctx);
        } else if self.consume_shortcut(ctx, "Cmd+O") {
            self.ask_open();
        } else if self.consume_shortcut(ctx, "Cmd+Shift+O") {
            self.ask_open_folder();
        } else if self.consume_shortcut(ctx, "Cmd+S") {
            self.request_save(false);
        } else if self.consume_shortcut(ctx, "Cmd+Shift+S") {
            self.ask_save_as(false);
        } else if self.consume_shortcut(ctx, "Cmd+W") {
            self.close_tab(self.tabs.active_index(), ctx);
        } else if self.consume_shortcut(ctx, "Cmd+E") {
            let mode = self.tabs.active().mode;
            let next = if mode == mdview_ui::ViewMode::Edit {
                mdview_ui::ViewMode::View
            } else {
                mdview_ui::ViewMode::Edit
            };
            self.handle_command(ctx, Command::SetMode(next));
        } else if self.consume_shortcut(ctx, "Cmd+Shift+E") {
            self.handle_command(ctx, Command::SetMode(mdview_ui::ViewMode::Split));
        } else if self.consume_shortcut(ctx, "Cmd+B") {
            self.sidebar.open = !self.sidebar.open;
            self.sync_session();
        } else if self.consume_shortcut(ctx, "Ctrl+Tab") {
            self.handle_command(ctx, Command::NextTab);
        } else if self.consume_shortcut(ctx, "Ctrl+Shift+Tab") {
            self.handle_command(ctx, Command::PreviousTab);
        } else if self.consume_shortcut(ctx, "Cmd+Q") {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }
    }
}
