//! Native dialog answers.

use craft_dialogs::{Answer, MessageResult};
use eframe::egui;

use crate::app::{DialogThen, MdViewApp};

impl MdViewApp {
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
}
