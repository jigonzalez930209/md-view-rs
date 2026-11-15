//! Conflict decisions and the diff window.

use std::path::Path;

use eframe::egui;

use crate::app::MdViewApp;
use crate::conflict::ConflictAction;
use crate::file_ops;

use super::DiffState;

impl MdViewApp {
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
                    log::info!("reloaded externally changed clean file: {}", path.display());
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
