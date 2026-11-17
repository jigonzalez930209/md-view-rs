//! Explorer row rendering for workspace and external files.

use std::path::{Path, PathBuf};

use eframe::egui;
use mdview_ui::{MaterialIcons, compact_menu, menu_item};
use mdview_workspace::Entry;

use super::Sidebar;

pub(super) enum Action {
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

/// How a row behaves when clicked and which context-menu entries it offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum EntryRole {
    Workspace,
    ExternalFile,
}

pub(super) struct TreeCtx<'a> {
    pub(super) actions: &'a mut Vec<Action>,
    pub(super) icons: &'a mut MaterialIcons,
    pub(super) light: bool,
    /// Set when a row is clicked: the tree takes keyboard focus after it is registered.
    pub(super) focus_requested: &'a mut bool,
    pub(super) active: Option<&'a Path>,
    pub(super) scroll_to_selected: &'a mut bool,
}

impl Sidebar {
    pub(super) fn entry(
        &mut self,
        ui: &mut egui::Ui,
        entry: &Entry,
        depth: usize,
        ctx: &mut TreeCtx<'_>,
        t: &dyn Fn(&str) -> String,
        role: EntryRole,
    ) {
        let selected = self.selected.as_deref() == Some(entry.path.as_path());
        let active = ctx.active == Some(entry.path.as_path());
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 21.0), egui::Sense::hover());
        let hovered = ui.rect_contains_pointer(rect);
        let fill = if selected {
            ui.visuals().selection.bg_fill
        } else if active {
            ui.visuals().selection.bg_fill.gamma_multiply(0.45)
        } else if hovered {
            ui.visuals().widgets.hovered.weak_bg_fill
        } else {
            egui::Color32::TRANSPARENT
        };
        ui.painter().rect_filled(rect, egui::CornerRadius::same(4), fill);
        if active {
            // The open document's marker sits inside the rounded row in the focus accent;
            // the text colour drawn on the row edge read as a stray text cursor.
            let bar = egui::Rect::from_min_size(
                egui::pos2(rect.left() + 4.0, rect.top() + 4.0),
                egui::vec2(2.0, rect.height() - 8.0),
            );
            ui.painter().rect_filled(
                bar,
                egui::CornerRadius::same(1),
                ui.visuals().widgets.active.bg_stroke.color,
            );
        }

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
                    draw_glyph(ui, ctx.icons, arrow, CHEVRON_SIZE);
                } else {
                    // The same slot as a chevron, so files and folders at one depth align.
                    ui.allocate_exact_size(egui::vec2(CHEVRON_SIZE, CHEVRON_SIZE), egui::Sense::hover());
                }
                let icon = if entry.is_dir {
                    ctx.icons.folder_icon(&entry.name, entry.expanded, false, ctx.light)
                } else {
                    ctx.icons.file_icon(&entry.path, ctx.light)
                };
                draw_icon(ui, ctx.icons, &icon, ctx.light);
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
        let response = if role == EntryRole::ExternalFile {
            response.on_hover_text(entry.path.display().to_string())
        } else {
            response
        };
        if response.clicked() {
            self.selected = Some(entry.path.clone());
            *ctx.focus_requested = true;
        }
        if *ctx.scroll_to_selected && selected {
            // Minimal scroll: a visible row stays put; arrow keys never recentre the tree.
            response.scroll_to_me(None);
            *ctx.scroll_to_selected = false;
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
                    // Rows fill the available width; without a cap the menu spread over the editor.
                    ui.set_max_width(220.0);
                    if menu_item(ui, ctx.icons, "file-plus", &t("New file"), None, false) {
                        ctx.actions.push(Action::NewFile(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "folder-plus", &t("New folder"), None, false) {
                        ctx.actions.push(Action::NewFolder(entry.path.clone()));
                        ui.close();
                    }
                    if menu_item(ui, ctx.icons, "folder-open", &t("Reveal"), None, false) {
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
                    self.entry(ui, child, depth + 1, ctx, t, EntryRole::Workspace);
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
                    // Rows fill the available width; without a cap the menu spread over the editor.
                    ui.set_max_width(220.0);
                    match role {
                        EntryRole::Workspace => {
                            if menu_item(ui, ctx.icons, "eye", &t("Open"), Some("Enter"), false) {
                                ctx.actions.push(Action::Open(entry.path.clone()));
                                ui.close();
                            }
                            if menu_item(ui, ctx.icons, "folder-open", &t("Reveal"), None, false) {
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
                        }
                        EntryRole::ExternalFile => {
                            // External tabs are not workspace files: they may be renamed, but not
                            // deleted or revealed from this synthetic folder.
                            if menu_item(ui, ctx.icons, "edit", &t("Rename…"), Some("F2"), false) {
                                ctx.actions.push(Action::Rename(entry.path.clone()));
                                ui.close();
                            }
                        }
                    }
                });
        }
    }

    /// Draw the fixed folder that groups open documents outside the workspace.
    pub(super) fn external_folder(
        &mut self,
        ui: &mut egui::Ui,
        external: &[PathBuf],
        ctx: &mut TreeCtx<'_>,
        t: &dyn Fn(&str) -> String,
    ) {
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);
        let name = t("External files");
        let width = ui.available_width();
        let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 21.0), egui::Sense::hover());
        let inner = rect.shrink2(egui::vec2(5.0, 0.0));
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(inner)
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
            |ui| {
                ui.style_mut().interaction.selectable_labels = false;
                ui.spacing_mut().item_spacing.x = 4.0;
                // Align the folder icon with the workspace folders, leaving the chevron slot
                // empty: this synthetic group is always expanded and cannot be collapsed.
                ui.add_space(2.0);
                ui.allocate_exact_size(egui::vec2(CHEVRON_SIZE, CHEVRON_SIZE), egui::Sense::hover());
                let icon = ctx.icons.folder_icon(&name, true, false, ctx.light);
                draw_icon(ui, ctx.icons, &icon, ctx.light);
                ui.add(
                    egui::Label::new(egui::RichText::new(&name).size(12.5).color(ui.visuals().text_color())).truncate(),
                );
            },
        );
        let entries = external_entries(external);
        for entry in &entries {
            self.entry(ui, entry, 1, ctx, t, EntryRole::ExternalFile);
        }
    }
}

fn external_entries(paths: &[PathBuf]) -> Vec<Entry> {
    let mut entries = paths
        .iter()
        .map(|path| Entry {
            path: path.clone(),
            name: path
                .file_name()
                .map(|name| name.to_string_lossy().into_owned())
                .unwrap_or_else(|| path.display().to_string()),
            is_dir: false,
            expanded: false,
            children: None,
        })
        .collect::<Vec<_>>();
    entries.sort_by(|left, right| {
        left.name
            .to_ascii_lowercase()
            .cmp(&right.name.to_ascii_lowercase())
            .then_with(|| left.path.cmp(&right.path))
    });
    entries
}

/// Draw a 16-point file or folder icon, keeping its slot when it cannot be loaded.
fn draw_icon(ui: &mut egui::Ui, icons: &mut MaterialIcons, icon: &str, light: bool) {
    if !icons.draw(ui, icon, light, 16.0) {
        ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
    }
}

/// Width of the expand/collapse slot in front of every explorer row icon.
const CHEVRON_SIZE: f32 = 14.0;

fn draw_glyph(ui: &mut egui::Ui, icons: &mut MaterialIcons, icon: &str, size: f32) {
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.add(
            egui::Image::new(&texture)
                .fit_to_exact_size(egui::vec2(size, size))
                .tint(ui.visuals().weak_text_color()),
        );
    } else {
        // Reserve the slot like an image would (including item spacing) so a glyph that
        // is still loading does not shift the row.
        ui.allocate_exact_size(egui::vec2(size, size), egui::Sense::hover());
    }
}

pub(super) fn icon_button(ui: &mut egui::Ui, icons: &mut MaterialIcons, icon: &str, tooltip: &str) -> bool {
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
