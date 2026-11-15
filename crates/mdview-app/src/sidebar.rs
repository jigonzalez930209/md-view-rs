//! The workspace explorer.
//!
//! The panel mirrors the VS Code explorer: a section header with compact file actions, a lazy tree
//! with Material file and folder icons, and modal create/rename/delete flows. The application only
//! shows this panel when a folder is loaded.

use std::path::PathBuf;

use eframe::egui;
use mdview_i18n::Translations;
use mdview_ui::{ConfirmPrompt, MaterialIcons, NamePrompt, PromptAction, compact_menu, menu_item};
use mdview_workspace::{Entry, Workspace};

use crate::workspace_ops;

#[path = "sidebar_dialogs.rs"]
mod sidebar_dialogs;
#[path = "sidebar_keyboard.rs"]
mod sidebar_keyboard;

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

/// Persistent explorer state.
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
    /// Build the explorer with persisted display options.
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

    /// Whether the explorer should occupy screen space.
    pub fn should_show(&self, workspace_open: bool) -> bool {
        workspace_open && self.open
    }

    /// Force the explorer visible after a folder opens.
    pub fn reveal(&mut self) {
        self.open = true;
    }

    /// Start renaming a path from a command or shortcut.
    pub fn begin_rename(&mut self, path: PathBuf) {
        let name = path
            .file_name()
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_default();
        self.renaming = Some((path, name));
    }

    /// Start deleting a path from a command or shortcut.
    pub fn begin_delete(&mut self, path: PathBuf) {
        self.deleting = Some(path);
    }

    /// Draw the explorer and return the application-level events.
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

        let root_name = workspace.root_entry().name.to_uppercase();
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            ui.add_space(6.0);
            ui.label(
                egui::RichText::new(&root_name)
                    .size(11.5)
                    .strong()
                    .color(ui.visuals().weak_text_color()),
            );
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.add_space(4.0);
                if icon_button(ui, icons, "collapse", &t("Collapse folders")) {
                    actions.push(Action::CollapseAll);
                }
                if icon_button(ui, icons, "refresh", &t("Refresh")) {
                    actions.push(Action::RefreshRoot);
                }
                if icon_button(ui, icons, "folder-plus", &t("New folder")) {
                    self.creating = Some(CreateDialog {
                        kind: CreateKind::Folder,
                        parent: workspace.root().to_path_buf(),
                        name: "folder".to_owned(),
                    });
                }
                if icon_button(ui, icons, "file-plus", &t("New file")) {
                    self.creating = Some(CreateDialog {
                        kind: CreateKind::File,
                        parent: workspace.root().to_path_buf(),
                        name: "untitled.md".to_owned(),
                    });
                }
            });
        });
        ui.add_space(4.0);

        if let Some(error) = &self.error {
            ui.horizontal_wrapped(|ui| {
                ui.add_space(6.0);
                ui.colored_label(ui.visuals().error_fg_color, error);
            });
            ui.add_space(2.0);
        }
        let focus_id = ui.scope_id().with("tree_focus");
        egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(2.0);
            let root = workspace.root_entry().clone();
            if let Some(children) = &root.children {
                let mut ctx = TreeCtx {
                    actions: &mut actions,
                    icons,
                    light,
                    focus_id,
                };
                for child in children {
                    self.entry(ui, child, 0, &mut ctx, &t);
                }
            }
            ui.add_space(12.0);
        });
        if ui.memory(|memory| memory.has_focus(focus_id)) {
            self.keyboard_navigation(ui, workspace, focus_id, &mut actions);
        }

        if !actions.is_empty() {
            ui.ctx().request_repaint();
        }
        for action in actions {
            match action {
                Action::Toggle(path) => {
                    let before = workspace
                        .root_entry()
                        .find(&path)
                        .map(|entry| entry.expanded)
                        .unwrap_or(false);
                    workspace.toggle(&path);
                    let after = workspace
                        .root_entry()
                        .find(&path)
                        .map(|entry| entry.expanded)
                        .unwrap_or(false);
                    log::debug!("explorer toggle path={} before={before} after={after}", path.display());
                }
                Action::Open(path) => {
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
                Action::CollapseAll => {
                    log::debug!("explorer collapse all");
                    let children = workspace.root_entry().children.clone().unwrap_or_default();
                    for child in children.iter().filter(|entry| entry.is_dir) {
                        workspace.collapse(&child.path);
                    }
                }
            }
        }

        self.dialogs(ui.ctx(), workspace, &mut events, &t);
        events
    }

    fn entry(
        &mut self,
        ui: &mut egui::Ui,
        entry: &Entry,
        depth: usize,
        ctx: &mut TreeCtx<'_>,
        t: &dyn Fn(&str) -> String,
    ) {
        let selected = self.selected.as_deref() == Some(entry.path.as_path());
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 21.0), egui::Sense::hover());
        let hovered = ui.rect_contains_pointer(rect);
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else if hovered {
            ui.visuals().widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, egui::CornerRadius::same(4), fill);

        let inner = rect.shrink2(egui::vec2(5.0, 0.0));
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.spacing_mut().item_spacing.x = 4.0;
                ui.add_space((depth as f32) * 12.0 + 2.0);
                if entry.is_dir {
                    let arrow = if entry.expanded {
                        "chevron-down"
                    } else {
                        "chevron-right"
                    };
                    draw_glyph(ui, ctx.icons, arrow, 14.0);
                } else {
                    ui.add_space(12.0);
                }
                let icon = if entry.is_dir {
                    ctx.icons.folder_icon(&entry.name, entry.expanded, false, ctx.light)
                } else {
                    ctx.icons.file_icon(&entry.path, ctx.light)
                };
                ctx.icons.draw(ui, &icon, ctx.light, 16.0);
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(&entry.name)
                            .size(12.5)
                            .color(ui.visuals().text_color()),
                    )
                    .truncate(),
                );
            },
        );

        let response = ui
            .interact(
                rect,
                ui.scope_id().with(("tree_row", entry.path.as_os_str())),
                egui::Sense::click(),
            )
            .on_hover_cursor(egui::CursorIcon::Default);
        if response.clicked() {
            self.selected = Some(entry.path.clone());
            ui.memory_mut(|memory| memory.request_focus(ctx.focus_id));
        }
        if entry.is_dir {
            if response.clicked() {
                ctx.actions.push(Action::Toggle(entry.path.clone()));
            }
            if response.secondary_clicked() {
                log::debug!(
                    "explorer context menu path={} y={:.1}",
                    entry.path.display(),
                    response.rect.bottom() + 1.0
                );
            }
            egui::Popup::context_menu(&response)
                .at_position(egui::pos2(response.rect.left(), response.rect.bottom() + 1.0))
                .show(|ui| {
                    compact_menu(ui);
                    ui.set_min_width(196.0);
                    if menu_item(ui, ctx.icons, "file-plus", &t("New file"), None, false) {
                        ctx.actions.push(Action::NewFile(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "folder-plus", &t("New folder"), None, false) {
                        ctx.actions.push(Action::NewFolder(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "eye", &t("Reveal"), None, false) {
                        ctx.actions.push(Action::Reveal(entry.path.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if menu_item(ui, ctx.icons, "edit", &t("Rename…"), Some("F2"), false) {
                        ctx.actions.push(Action::Rename(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "close", &t("Delete…"), None, true) {
                        ctx.actions.push(Action::Delete(entry.path.clone()));
                        ui.close();
                    }
                });
            if entry.expanded
                && let Some(children) = &entry.children
            {
                for child in children {
                    self.entry(ui, child, depth + 1, ctx, t);
                }
            }
        } else if response.clicked() {
            ctx.actions.push(Action::Open(entry.path.clone()));
        }
        if !entry.is_dir {
            if response.secondary_clicked() {
                log::debug!(
                    "explorer context menu path={} y={:.1}",
                    entry.path.display(),
                    response.rect.bottom() + 1.0
                );
            }
            egui::Popup::context_menu(&response)
                .at_position(egui::pos2(response.rect.left(), response.rect.bottom() + 1.0))
                .show(|ui| {
                    compact_menu(ui);
                    ui.set_min_width(196.0);
                    if menu_item(ui, ctx.icons, "eye", &t("Open"), Some("Enter"), false) {
                        ctx.actions.push(Action::Open(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "history", &t("Reveal"), None, false) {
                        ctx.actions.push(Action::Reveal(entry.path.clone()));
                        ui.close();
                    }
                    ui.separator();
                    if menu_item(ui, ctx.icons, "edit", &t("Rename…"), Some("F2"), false) {
                        ctx.actions.push(Action::Rename(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "close", &t("Delete…"), None, true) {
                        ctx.actions.push(Action::Delete(entry.path.clone()));
                        ui.close();
                    }
                });
        }
    }
}

fn draw_glyph(ui: &mut egui::Ui, icons: &mut MaterialIcons, icon: &str, size: f32) {
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.add(
            egui::Image::new(&texture)
                .fit_to_exact_size(egui::vec2(size, size))
                .tint(ui.visuals().weak_text_color()),
        );
    } else {
        ui.add_space(size);
    }
}

fn icon_button(ui: &mut egui::Ui, icons: &mut MaterialIcons, icon: &str, tooltip: &str) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(21.0, 21.0), egui::Sense::click());
    if response.is_pointer_button_down_on() || response.hovered() {
        let fill = if response.is_pointer_button_down_on() {
            ui.visuals().widgets.active.weak_bg_fill
        } else {
            ui.visuals().widgets.hovered.weak_bg_fill
        };
        ui.painter().rect_filled(rect, egui::CornerRadius::same(4), fill);
    }
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.painter().image(
            texture.id(),
            rect.shrink(2.5),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            ui.visuals().text_color(),
        );
    }
    response.on_hover_text(tooltip).clicked()
}

struct TreeCtx<'a> {
    actions: &'a mut Vec<Action>,
    icons: &'a mut MaterialIcons,
    light: bool,
    focus_id: egui::Id,
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
    CollapseAll,
}

#[cfg(test)]
mod tests {
    use egui_kittest::Harness;
    use mdview_i18n::{LanguagePreference, Translations};

    use super::*;

    #[test]
    fn explorer_is_visible_only_with_a_folder() {
        let mut sidebar = Sidebar::new(260.0, false, true);
        assert!(sidebar.should_show(true));
        assert!(!sidebar.should_show(false));
        sidebar.open = false;
        assert!(!sidebar.should_show(true));
        sidebar.reveal();
        assert!(sidebar.should_show(true));
    }

    fn sidebar_snapshot_with(name: &str, pixels_per_point: f32) {
        let temp = tempfile::tempdir().expect("temp dir");
        let dir = temp.path().join("project");
        std::fs::create_dir(&dir).expect("project");
        std::fs::create_dir(dir.join("src")).expect("src");
        std::fs::create_dir(dir.join("docs")).expect("docs");
        std::fs::write(dir.join("README.md"), "# Read me\n").expect("readme");
        std::fs::write(dir.join("main.rs"), "fn main() {}\n").expect("main");
        std::fs::write(dir.join("src/lib.rs"), "").expect("lib");
        std::fs::write(dir.join("docs/guide.md"), "# Guide\n").expect("guide");
        let mut workspace = Workspace::open(&dir).expect("workspace");
        workspace.expand(&dir.join("src"));
        let mut sidebar = Sidebar::new(260.0, false, true);
        let mut icons = MaterialIcons::new();
        sidebar.open = true;
        let translations = Translations::new();
        let lang = translations.lang(LanguagePreference::English);
        let mut harness = Harness::builder()
            .with_size(egui::vec2(260.0, 380.0))
            .with_pixels_per_point(pixels_per_point)
            .build_ui(move |ui| {
                sidebar.show(ui, &mut workspace, &translations, lang, &mut icons, true);
            });
        harness.run();
        harness.snapshot(name);
    }

    #[test]
    fn sidebar_snapshot() {
        sidebar_snapshot_with("sidebar", 1.0);
    }

    #[test]
    fn sidebar_2x_snapshot() {
        sidebar_snapshot_with("sidebar-2x", 2.0);
    }
}
