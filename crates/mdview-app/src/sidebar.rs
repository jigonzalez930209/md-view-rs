//! The workspace explorer.
//!
//! The panel mirrors the VS Code explorer: a section header with compact file actions, a lazy tree
//! with Material file and folder icons, and modal create/rename/delete flows. The application only
//! shows this panel when a folder is loaded.
//!
//! Open documents that live outside the workspace are listed in a synthetic folder at the end of
//! the tree. It is not a real directory: it cannot be renamed, deleted, or used as a parent for
//! new entries, and its files can only be renamed.

use std::path::{Path, PathBuf};

use eframe::egui;
use mdview_i18n::Translations;
use mdview_ui::{ConfirmPrompt, MaterialIcons, NamePrompt, PromptAction};
use mdview_workspace::Workspace;

use crate::workspace_ops;

#[path = "sidebar_dialogs.rs"]
mod sidebar_dialogs;
#[path = "sidebar_keyboard.rs"]
mod sidebar_keyboard;
#[path = "sidebar_tree.rs"]
mod sidebar_tree;
use sidebar_tree::icon_button;
use sidebar_tree::{Action, EntryRole, TreeCtx};

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

pub(super) enum CreateKind {
    File,
    Folder,
}

pub(super) struct CreateDialog {
    pub(super) kind: CreateKind,
    pub(super) parent: PathBuf,
    pub(super) name: String,
}

/// Everything the explorer needs for one frame.
pub struct SidebarView<'a> {
    /// Translator.
    pub translations: &'a Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Icon cache.
    pub icons: &'a mut MaterialIcons,
    /// Whether the light theme is active.
    pub light: bool,
    /// Path of the file in the active tab.
    pub active: Option<&'a Path>,
    /// Open files outside the workspace, already sorted for display.
    pub external: &'a [PathBuf],
}

/// Persistent explorer state.
pub struct Sidebar {
    /// Whether the panel is visible.
    pub open: bool,
    pub width: f32,
    pub show_hidden: bool,
    pub respect_gitignore: bool,
    pub(super) selected: Option<PathBuf>,
    pub(super) last_active: Option<PathBuf>,
    pub(super) scroll_to_selected: bool,
    pub(super) renaming: Option<(PathBuf, String)>,
    pub(super) creating: Option<CreateDialog>,
    pub(super) deleting: Option<PathBuf>,
    pub(super) error: Option<String>,
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
            last_active: None,
            scroll_to_selected: false,
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

    /// Force the next frame to re-sync the active row after the tree is replaced.
    pub(super) fn reset_active(&mut self) {
        self.last_active = None;
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

    /// Keep the tree selection on the active tab and reveal its ancestors.
    ///
    /// The selection only follows a *change* of active tab, so clicking a folder does not get
    /// overwritten on the next frame.
    fn sync_active(&mut self, workspace: &mut Workspace, active: Option<&Path>) {
        if self.last_active.as_deref() == active {
            return;
        }
        self.last_active = active.map(Path::to_path_buf);
        self.selected = active.map(Path::to_path_buf);
        self.scroll_to_selected = active.is_some();
        if let Some(path) = active {
            expand_to_path(workspace, path);
        }
    }

    /// Draw the explorer and return the application-level events.
    pub fn show(&mut self, ui: &mut egui::Ui, workspace: &mut Workspace, view: SidebarView<'_>) -> Vec<SidebarEvent> {
        let SidebarView {
            translations,
            lang,
            icons,
            light,
            active,
            external,
        } = view;
        let t = |source: &str| translations.t(lang, source);
        let mut events = Vec::new();
        let mut actions = Vec::new();
        self.sync_active(workspace, active);

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
        let mut scroll_to_selected = self.scroll_to_selected;
        let mut focus_requested = false;
        let tree = egui::ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
            ui.add_space(2.0);
            // Borrow the tree instead of cloning it every frame: actions are applied after
            // drawing, so nothing mutates the workspace while its rows are shown.
            let root = workspace.root_entry();
            let mut ctx = TreeCtx {
                actions: &mut actions,
                icons,
                light,
                focus_requested: &mut focus_requested,
                active,
                scroll_to_selected: &mut scroll_to_selected,
            };
            if let Some(children) = &root.children {
                for child in children {
                    self.entry(ui, child, 0, &mut ctx, &t, EntryRole::Workspace);
                }
            }
            if !external.is_empty() {
                self.external_folder(ui, external, &mut ctx, &t);
            }
            ui.add_space(12.0);
        });
        self.scroll_to_selected = false;
        // The tree takes keyboard focus as one widget. egui drops focus from an id that no
        // widget registers in a frame, so register it here; without that, arrow keys and
        // Enter did nothing after clicking a row. The lock keeps the arrows in the tree.
        ui.interact(tree.inner_rect, focus_id, egui::Sense::focusable_noninteractive());
        if focus_requested {
            // Requested after registering the tree: registering applies click-away focus
            // loss, and the click landed on a row, not on the tree widget itself.
            ui.memory_mut(|memory| memory.request_focus(focus_id));
        }
        if ui.memory(|memory| memory.has_focus(focus_id)) {
            ui.memory_mut(|memory| {
                memory.set_focus_lock_filter(
                    focus_id,
                    egui::EventFilter {
                        tab: false,
                        horizontal_arrows: true,
                        vertical_arrows: true,
                        escape: false,
                    },
                );
            });
            self.keyboard_navigation(ui, workspace, focus_id, &mut actions, external);
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
}

/// Expand every workspace ancestor of `path` so the active row can be revealed.
fn expand_to_path(workspace: &mut Workspace, path: &Path) {
    let root = workspace.root().to_path_buf();
    if !path.starts_with(&root) {
        return;
    }
    let mut directories = Vec::new();
    let mut current = path.parent();
    while let Some(directory) = current {
        if !directory.starts_with(&root) {
            break;
        }
        directories.push(directory.to_path_buf());
        if directory == root {
            break;
        }
        current = directory.parent();
    }
    for directory in directories.into_iter().rev() {
        if directory != root {
            workspace.expand(&directory);
        }
    }
}

#[cfg(test)]
#[path = "sidebar_tests.rs"]
mod tests;
