//! The workspace sidebar.
//!
//! The sidebar renders the lazy tree owned by `mdview-workspace` and collects user actions into
//! events for the application. File operations run here because they are scoped to the tree; the
//! application only needs to know about renames and deletes that affect open tabs.

use std::path::PathBuf;

use eframe::egui;
use mdview_i18n::Translations;
use mdview_ui::MaterialIcons;
use mdview_workspace::{Entry, Workspace};

use crate::workspace_ops;

/// Something the application must react to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarEvent {
    /// Open a file in a tab.
    Open(PathBuf),
    /// A path opened from the tree should get focus.
    Renamed { from: PathBuf, to: PathBuf },
    /// A path was deleted.
    Deleted(PathBuf),
}

enum CreateKind {
    File,
    Folder,
}

struct CreateDialog {
    kind: CreateKind,
    parent: PathBuf,
    name: String,
}

/// Persistent sidebar state.
pub struct Sidebar {
    /// Whether the panel is visible.
    pub open: bool,
    pub width: f32,
    pub show_hidden: bool,
    pub respect_gitignore: bool,
    selected: Option<PathBuf>,
    renaming: Option<(PathBuf, String)>,
    creating: Option<CreateDialog>,
    deleting: Option<PathBuf>,
    error: Option<String>,
}

impl Sidebar {
    /// Build the sidebar with persisted display options.
    pub fn new(width: f32, show_hidden: bool, respect_gitignore: bool) -> Self {
        Self {
            open: true,
            width,
            show_hidden,
            respect_gitignore,
            selected: None,
            renaming: None,
            creating: None,
            deleting: None,
            error: None,
        }
    }

    /// Draw the sidebar and return the application-level events.
    pub fn show(
        &mut self,
        ui: &mut egui::Ui,
        workspace: &mut Workspace,
        translations: &Translations,
        lang: craft_i18n::Lang,
        icons: &mut MaterialIcons,
        light: bool,
    ) -> Vec<SidebarEvent> {
        let t = |source: &str| translations.t(lang, source);
        let mut events = Vec::new();
        let mut actions = Vec::new();

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 2.0;
            if icon_button(ui, icons, "file-plus", &t("New file")) {
                self.creating = Some(CreateDialog {
                    kind: CreateKind::File,
                    parent: workspace.root().to_path_buf(),
                    name: "untitled.md".to_owned(),
                });
            }
            if icon_button(ui, icons, "folder-plus", &t("New folder")) {
                self.creating = Some(CreateDialog {
                    kind: CreateKind::Folder,
                    parent: workspace.root().to_path_buf(),
                    name: "folder".to_owned(),
                });
            }
            if icon_button(ui, icons, "refresh", &t("Refresh")) {
                actions.push(Action::RefreshRoot);
            }
            ui.menu_button("...", |ui| {
                let mut hidden = self.show_hidden;
                if ui.checkbox(&mut hidden, t("Show hidden files")).changed() {
                    self.show_hidden = hidden;
                    actions.push(Action::SetHidden(hidden));
                }
                let mut gitignore = self.respect_gitignore;
                if ui.checkbox(&mut gitignore, t("Respect .gitignore")).changed() {
                    self.respect_gitignore = gitignore;
                    actions.push(Action::SetGitignore(gitignore));
                }
            });
        });

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let icon = icons.folder_icon(&workspace.root_entry().name, false, true, light);
            icons.draw(ui, &icon, light, 16.0);
            ui.label(egui::RichText::new(&workspace.root_entry().name).strong());
        });

        if let Some(error) = &self.error {
            ui.colored_label(ui.visuals().error_fg_color, error);
        }

        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            let root = workspace.root_entry().clone();
            if let Some(children) = &root.children {
                for child in children {
                    self.entry(ui, child, &mut actions, icons, &t, light);
                }
            }
        });

        for action in actions {
            match action {
                Action::Toggle(path) => {
                    log::info!("sidebar toggle: {}", path.display());
                    workspace.toggle(&path);
                }
                Action::Open(path) => {
                    log::info!("sidebar open: {}", path.display());
                    self.selected = Some(path.clone());
                    events.push(SidebarEvent::Open(path));
                }
                Action::NewFile(parent) => {
                    self.creating = Some(CreateDialog {
                        kind: CreateKind::File,
                        parent,
                        name: "untitled.md".to_owned(),
                    });
                }
                Action::NewFolder(parent) => {
                    self.creating = Some(CreateDialog {
                        kind: CreateKind::Folder,
                        parent,
                        name: "folder".to_owned(),
                    });
                }
                Action::Rename(path) => {
                    let name = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.renaming = Some((path, name));
                }
                Action::Delete(path) => self.deleting = Some(path),
                Action::Reveal(path) => {
                    if let Err(error) = workspace_ops::reveal(&path) {
                        self.error = Some(error.to_string());
                    }
                }
                Action::RefreshRoot => {
                    let root = workspace.root().to_path_buf();
                    workspace.refresh_dir(&root);
                }
                Action::SetHidden(show) => workspace.set_show_hidden(show),
                Action::SetGitignore(respect) => workspace.set_respect_gitignore(respect),
            }
        }

        self.dialogs(ui.ctx(), workspace, &mut events, &t);
        events
    }

    fn entry(
        &mut self,
        ui: &mut egui::Ui,
        entry: &Entry,
        actions: &mut Vec<Action>,
        icons: &mut MaterialIcons,
        t: &dyn Fn(&str) -> String,
        light: bool,
    ) {
        let selected = self.selected.as_deref() == Some(entry.path.as_path());
        if entry.is_dir {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 3.0;
                let arrow = if entry.expanded {
                    "chevron-down"
                } else {
                    "chevron-right"
                };
                if icon_button(ui, icons, arrow, "") {
                    actions.push(Action::Toggle(entry.path.clone()));
                }
                let icon = icons.folder_icon(&entry.name, entry.expanded, false, light);
                icons.draw(ui, &icon, light, 16.0);
                let response = ui.selectable_label(selected, &entry.name);
                if response.clicked() {
                    self.selected = Some(entry.path.clone());
                    actions.push(Action::Toggle(entry.path.clone()));
                }
                response.context_menu(|ui| {
                    if ui.button(t("New file")).clicked() {
                        actions.push(Action::NewFile(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("New folder")).clicked() {
                        actions.push(Action::NewFolder(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("Reveal")).clicked() {
                        actions.push(Action::Reveal(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("Rename…")).clicked() {
                        actions.push(Action::Rename(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("Delete…")).clicked() {
                        actions.push(Action::Delete(entry.path.clone()));
                        ui.close();
                    }
                });
            });
            if entry.expanded
                && let Some(children) = &entry.children
            {
                ui.indent(&entry.path, |ui| {
                    for child in children {
                        self.entry(ui, child, actions, icons, t, light);
                    }
                });
            }
        } else {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 5.0;
                let icon = icons.file_icon(&entry.path, light);
                icons.draw(ui, &icon, light, 16.0);
                let response = ui.selectable_label(selected, &entry.name);
                if response.clicked() {
                    actions.push(Action::Open(entry.path.clone()));
                }
                response.context_menu(|ui| {
                    if ui.button(t("Open")).clicked() {
                        actions.push(Action::Open(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("Reveal")).clicked() {
                        actions.push(Action::Reveal(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("Rename…")).clicked() {
                        actions.push(Action::Rename(entry.path.clone()));
                        ui.close();
                    }
                    if ui.button(t("Delete…")).clicked() {
                        actions.push(Action::Delete(entry.path.clone()));
                        ui.close();
                    }
                });
            });
        }
    }

    fn dialogs(
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
            let mut open = true;
            let mut submit = false;
            let mut cancel = false;
            egui::Window::new(title)
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    let response = ui.add(egui::TextEdit::singleline(&mut dialog.name).desired_width(220.0));
                    if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        submit = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.button(t("Create")).clicked() {
                            submit = true;
                        }
                        if ui.button(t("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });
            if open && submit {
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
            } else if open && !cancel {
                self.creating = Some(dialog);
            }
        }

        if let Some((path, mut name)) = self.renaming.take() {
            let mut open = true;
            let mut submit = false;
            let mut cancel = false;
            egui::Window::new(t("Rename"))
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    let response = ui.add(egui::TextEdit::singleline(&mut name).desired_width(220.0));
                    if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                        submit = true;
                    }
                    ui.horizontal(|ui| {
                        if ui.button(t("Rename")).clicked() {
                            submit = true;
                        }
                        if ui.button(t("Cancel")).clicked() {
                            cancel = true;
                        }
                    });
                });
            if open && submit {
                let from = path.clone();
                match workspace_ops::rename(&from, &name) {
                    Ok(to) => {
                        if let Some(parent) = from.parent() {
                            workspace.refresh_dir(parent);
                        }
                        if let Some(parent) = to.parent() {
                            workspace.refresh_dir(parent);
                        }
                        events.push(SidebarEvent::Renamed { from, to });
                    }
                    Err(error) => {
                        self.error = Some(error.to_string());
                        self.renaming = Some((path, name));
                    }
                }
            } else if open && !cancel {
                self.renaming = Some((path, name));
            }
        }

        if let Some(path) = self.deleting.clone() {
            let mut open = true;
            let mut submit = false;
            egui::Window::new(t("Delete"))
                .open(&mut open)
                .resizable(false)
                .collapsible(false)
                .show(ctx, |ui| {
                    ui.label(format!("{} {}", t("Delete this entry permanently?"), path.display()));
                    ui.horizontal(|ui| {
                        if ui.button(t("Delete")).clicked() {
                            submit = true;
                        }
                        if ui.button(t("Cancel")).clicked() {
                            self.deleting = None;
                        }
                    });
                });
            if open && submit {
                match workspace_ops::delete(&path) {
                    Ok(()) => {
                        if let Some(parent) = path.parent() {
                            workspace.refresh_dir(parent);
                        }
                        events.push(SidebarEvent::Deleted(path));
                        self.deleting = None;
                    }
                    Err(error) => self.error = Some(error.to_string()),
                }
            } else if !open {
                self.deleting = None;
            }
        }
    }
}

fn icon_button(ui: &mut egui::Ui, icons: &mut MaterialIcons, icon: &str, tooltip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(22.0, 22.0), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(4),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.put(
            rect.shrink(3.0),
            egui::Image::new(&texture)
                .fit_to_exact_size(egui::vec2(16.0, 16.0))
                .tint(ui.visuals().text_color()),
        );
    }
    let response = if tooltip.is_empty() {
        response
    } else {
        response.on_hover_text(tooltip)
    };
    response.clicked()
}

enum Action {
    Toggle(PathBuf),
    Open(PathBuf),
    NewFile(PathBuf),
    NewFolder(PathBuf),
    Rename(PathBuf),
    Delete(PathBuf),
    Reveal(PathBuf),
    RefreshRoot,
    SetHidden(bool),
    SetGitignore(bool),
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use mdview_i18n::{LanguagePreference, Translations};

    use super::*;

    #[test]
    fn sidebar_snapshot() {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path().join("project");
        std::fs::create_dir(&dir).expect("project");
        std::fs::create_dir(dir.join("src")).expect("src");
        std::fs::write(dir.join("README.md"), "# Read me\n").expect("readme");
        std::fs::write(dir.join("main.rs"), "fn main() {}\n").expect("main");
        std::fs::write(dir.join("src/lib.rs"), "").expect("lib");
        let mut workspace = Workspace::open(&dir).expect("workspace");
        workspace.expand(&dir.join("src"));
        let mut sidebar = Sidebar::new(260.0, false, true);
        let mut icons = MaterialIcons::new();
        sidebar.open = true;
        let translations = Translations::new();
        let lang = translations.lang(LanguagePreference::English);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(260.0, 360.0))
            .build_ui(move |ui| {
                sidebar.show(ui, &mut workspace, &translations, lang, &mut icons, true);
            });
        harness.run();
        harness.snapshot("sidebar");
    }
}
