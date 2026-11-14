//! Create, rename, and delete dialogs for the explorer.

use super::*;

impl Sidebar {
    pub(super) fn dialogs(
        &mut self,
        ctx: &egui::Context,
        workspace: &mut Workspace,
        events: &mut Vec<SidebarEvent>,
        t: &dyn Fn(&str) -> String,
    ) {
        if let Some(mut dialog) = self.creating.take() {
            let title = match dialog.kind {
                CreateKind::File => t("New file"),
                CreateKind::Folder => t("New folder"),
            };
            let label = match dialog.kind {
                CreateKind::File => t("File name"),
                CreateKind::Folder => t("Folder name"),
            };
            let description = format!("{} {}", t("Creating in"), dialog.parent.display());
            let mut prompt = NamePrompt {
                title,
                description,
                label,
                value: &mut dialog.name,
                confirm: t("Create"),
                error: self.error.take(),
            };
            match mdview_ui::show_name_prompt(ctx, &mut prompt) {
                Some(PromptAction::Submit) => {
                    let result = match dialog.kind {
                        CreateKind::File => workspace_ops::create_file(&dialog.parent, &dialog.name),
                        CreateKind::Folder => workspace_ops::create_folder(&dialog.parent, &dialog.name),
                    };
                    match result {
                        Ok(path) => {
                            workspace.refresh_dir(&dialog.parent);
                            workspace.expand(&dialog.parent);
                            if path.is_file() {
                                events.push(SidebarEvent::Open(path));
                            }
                        }
                        Err(error) => {
                            self.error = Some(error.to_string());
                            self.creating = Some(dialog);
                        }
                    }
                }
                Some(PromptAction::Cancel) => {}
                None => self.creating = Some(dialog),
            }
        }

        if let Some((path, mut name)) = self.renaming.take() {
            let mut prompt = NamePrompt {
                title: t("Rename"),
                description: path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default(),
                label: t("New name"),
                value: &mut name,
                confirm: t("Rename"),
                error: self.error.take(),
            };
            match mdview_ui::show_name_prompt(ctx, &mut prompt) {
                Some(PromptAction::Submit) => match workspace_ops::rename(&path, &name) {
                    Ok(to) => {
                        if let Some(parent) = path.parent() {
                            workspace.refresh_dir(parent);
                        }
                        if let Some(parent) = to.parent() {
                            workspace.refresh_dir(parent);
                        }
                        events.push(SidebarEvent::Renamed { from: path, to });
                    }
                    Err(error) => {
                        self.error = Some(error.to_string());
                        self.renaming = Some((path, name));
                    }
                },
                Some(PromptAction::Cancel) => {}
                None => self.renaming = Some((path, name)),
            }
        }

        if let Some(path) = self.deleting.clone() {
            let prompt = ConfirmPrompt {
                title: t("Delete"),
                message: format!("{} {}", t("Delete this entry permanently?"), path.display()),
                confirm: t("Delete"),
                danger: true,
            };
            match mdview_ui::show_confirm_prompt(ctx, &prompt) {
                Some(PromptAction::Submit) => match workspace_ops::delete(&path) {
                    Ok(()) => {
                        if let Some(parent) = path.parent() {
                            workspace.refresh_dir(parent);
                        }
                        events.push(SidebarEvent::Deleted(path));
                        self.deleting = None;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                },
                Some(PromptAction::Cancel) => self.deleting = None,
                None => {}
            }
        }
    }
}
