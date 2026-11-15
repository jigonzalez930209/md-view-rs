//! The desktop workbench title bar.
//!
//! The title bar follows the compact desktop convention used by Ubuntu: a product mark, the
//! application menus, document history, centered document title, search, workbench toggles, and
//! window controls. Every action is also available from the menu bar or a keyboard shortcut.

use egui::{Align, Align2, Color32, FontId, Image, Layout, Rect, Sense, Ui, UiBuilder, Vec2};
use mdview_i18n::Translations;
use mdview_model::Encoding;

use crate::metrics::{ACTION_BUTTON, ICON_LARGE, ICON_MEDIUM, ICON_SMALL, WINDOW_BUTTON_HEIGHT, WINDOW_BUTTON_WIDTH};
use crate::{Command, MaterialIcons, ViewMode};

#[path = "header_menus.rs"]
mod header_menus;
use header_menus::{edit_menu, file_menu, go_menu, help_menu, view_menu};

/// Inputs for the title bar.
pub struct TitleBarState<'a> {
    /// Translator.
    pub translations: &'a Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Document title shown centered.
    pub document_name: &'a str,
    /// Dirty marker.
    pub dirty: bool,
    /// Presentation mode.
    pub mode: ViewMode,
    /// Markdown files can use reading and split modes.
    pub is_markdown: bool,
    /// Reading view is available.
    pub can_view: bool,
    /// Editor is available.
    pub can_edit: bool,
    /// Split view is available.
    pub can_split: bool,
    /// Sidebar visibility.
    pub sidebar_open: bool,
    /// Whether a workspace folder is loaded.
    pub workspace_open: bool,
    /// Workspace folder name, shown in the Open Folder tooltip.
    pub workspace_name: Option<&'a str>,
    /// Split scroll synchronization.
    pub sync_scroll: bool,
    /// Soft wrap state.
    pub wrap: bool,
    /// Line-number state.
    pub line_numbers: bool,
    /// Whether the window is maximized.
    pub maximized: bool,
    /// Whether document history can navigate back.
    pub can_back: bool,
    /// Whether document history can navigate forward.
    pub can_forward: bool,
    /// Whether the search popover is open.
    pub search_open: bool,
}

/// Draw the title bar and return the selected command.
pub fn show_title_bar(ui: &mut Ui, state: TitleBarState<'_>, icons: &mut MaterialIcons) -> Option<Command> {
    let translations = state.translations;
    let lang = state.lang;
    let t = |source: &str| translations.t(lang, source);
    let bar = ui.max_rect();
    let mut command = None;

    ui.painter().rect_filled(bar, 0.0, ui.visuals().panel_fill);

    let left = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(bar)
                .layout(Layout::left_to_right(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                ui.spacing_mut().button_padding = Vec2::new(7.0, 3.0);
                ui.style_mut()
                    .text_styles
                    .insert(egui::TextStyle::Button, FontId::proportional(13.0));
                let visuals = ui.visuals_mut();
                visuals.widgets.inactive.weak_bg_fill = Color32::TRANSPARENT;
                visuals.widgets.inactive.bg_fill = Color32::TRANSPARENT;
                visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                ui.add_space(8.0);
                if let Some(texture) = icons.ui_texture(ui.ctx(), "logo") {
                    ui.add(Image::new(&texture).fit_to_exact_size(Vec2::splat(ICON_LARGE)));
                }
                ui.add_space(6.0);
                egui::MenuBar::new().ui(ui, |ui| {
                    menu_button(ui, &t("File"), |ui| file_menu(ui, &t, &mut command));
                    menu_button(ui, &t("Edit"), |ui| edit_menu(ui, &t, &mut command));
                    menu_button(ui, &t("View"), |ui| view_menu(ui, &t, &state, &mut command));
                    menu_button(ui, &t("Go"), |ui| go_menu(ui, &t, &mut command));
                    menu_button(ui, &t("Help"), |ui| help_menu(ui, &t, &mut command));
                });
                ui.add_space(6.0);
                if icon_button(ui, icons, "back", &t("Back"), false, !state.can_back) {
                    command = Some(Command::Back);
                }
                if icon_button(ui, icons, "forward", &t("Forward"), false, !state.can_forward) {
                    command = Some(Command::Forward);
                }
            },
        )
        .response
        .rect
        .right();

    let right = ui
        .scope_builder(
            UiBuilder::new()
                .max_rect(bar)
                .layout(Layout::right_to_left(Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing = Vec2::new(2.0, 0.0);
                ui.add_space(1.0);
                if window_button(ui, icons, "close", WindowButton::Close, t("Close")).clicked() {
                    command = Some(Command::CloseWindow);
                }
                let (icon, tooltip) = if state.maximized {
                    ("restore", t("Restore"))
                } else {
                    ("maximize", t("Maximize"))
                };
                if window_button(ui, icons, icon, WindowButton::Normal, tooltip).clicked() {
                    command = Some(Command::ToggleMaximizeWindow);
                }
                if window_button(ui, icons, "minimize", WindowButton::Normal, t("Minimize")).clicked() {
                    command = Some(Command::MinimizeWindow);
                }
                ui.add_space(6.0);
                ui.separator();
                ui.add_space(4.0);
                if icon_button(ui, icons, "settings", &t("Settings"), false, true) {
                    command = Some(Command::Settings);
                }
                if icon_button(ui, icons, "command", &t("Command Palette"), false, true) {
                    command = Some(Command::CommandPalette);
                }
                if icon_button(ui, icons, "search", &t("Search"), state.search_open, true) {
                    command = Some(Command::Search);
                }
                if state.workspace_open && icon_button(ui, icons, "sidebar", &t("Explorer"), state.sidebar_open, true) {
                    command = Some(Command::ToggleSidebar);
                }
                ui.add_space(2.0);
            },
        )
        .response
        .rect
        .left();

    // Only the center of the title bar is a drag surface. Earlier versions covered the whole
    // bar, which stole clicks from the history and workbench buttons.
    let center = bar.center().x;
    let drag_left = (center - 70.0).max(bar.left() + 8.0);
    let drag_right = (center + 70.0).min(bar.right() - 8.0).max(drag_left);
    let drag = Rect::from_min_max(egui::pos2(drag_left, bar.top()), egui::pos2(drag_right, bar.bottom()));
    let response = ui.interact(drag, ui.scope_id().with("title_bar_drag"), Sense::click_and_drag());
    if response.drag_started() {
        ui.ctx().send_viewport_cmd(egui::ViewportCommand::StartDrag);
    }
    if response.double_clicked() {
        ui.ctx()
            .send_viewport_cmd(egui::ViewportCommand::Maximized(!state.maximized));
    }
    if response.hovered() {
        ui.ctx().set_cursor_icon(egui::CursorIcon::Default);
    }

    // A folder pill keeps the primary workspace action one click away, before the window controls.
    if !state.workspace_open
        && let Some(action) = folder_pill(
            ui,
            egui::pos2((right - 58.0).max(left + 12.0), bar.center().y),
            &t("Open Folder"),
            state.workspace_name.unwrap_or(""),
            icons,
        )
    {
        command = Some(action);
    }
    if state.workspace_open && drag.width() > 220.0 {
        let marker = if state.dirty { " •" } else { "" };
        let title = format!("{}{marker}", state.document_name);
        ui.painter().text(
            drag.center(),
            Align2::CENTER_CENTER,
            title,
            FontId::proportional(12.0),
            ui.visuals().weak_text_color(),
        );
    }

    command
}

fn menu_button(ui: &mut Ui, label: &str, add: impl FnOnce(&mut Ui)) {
    ui.menu_button(label, add);
}

fn folder_pill(
    ui: &mut Ui,
    center: egui::Pos2,
    label: &str,
    workspace_name: &str,
    icons: &mut MaterialIcons,
) -> Option<Command> {
    let width = 104.0;
    let rect = Rect::from_center_size(center, Vec2::new(width, 24.0));
    let response = ui.interact(rect, ui.scope_id().with("open_folder_pill"), Sense::click());
    let fill = if response.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        ui.visuals().widgets.inactive.weak_bg_fill
    };
    ui.painter().rect_filled(rect, 12.0, fill);
    ui.painter().rect_stroke(
        rect,
        12.0,
        ui.visuals().widgets.noninteractive.bg_stroke,
        egui::StrokeKind::Inside,
    );
    if let Some(texture) = icons.ui_texture(ui.ctx(), "folder-open") {
        let icon_rect = Rect::from_center_size(egui::pos2(rect.left() + 17.0, rect.center().y), Vec2::splat(14.0));
        ui.put(
            icon_rect,
            Image::new(&texture)
                .fit_to_exact_size(Vec2::splat(14.0))
                .tint(ui.visuals().text_color()),
        );
    }
    ui.painter().text(
        egui::pos2(rect.left() + 29.0, rect.center().y),
        Align2::LEFT_CENTER,
        label,
        FontId::proportional(12.0),
        ui.visuals().text_color(),
    );
    let response = if workspace_name.is_empty() {
        response
    } else {
        response.on_hover_text(workspace_name)
    };
    response.clicked().then_some(Command::OpenFolder)
}

#[derive(Clone, Copy)]
enum WindowButton {
    Normal,
    Close,
}

fn window_button(
    ui: &mut Ui,
    icons: &mut MaterialIcons,
    icon: &str,
    kind: WindowButton,
    tooltip: String,
) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::new(WINDOW_BUTTON_WIDTH, WINDOW_BUTTON_HEIGHT), Sense::click());
    let hovered = response.hovered();
    let fill = if hovered {
        match kind {
            WindowButton::Close => Color32::from_rgb(196, 43, 28),
            WindowButton::Normal => ui.visuals().widgets.hovered.weak_bg_fill,
        }
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, 0.0, fill);
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        let color = if hovered && matches!(kind, WindowButton::Close) {
            Color32::WHITE
        } else {
            ui.visuals().text_color()
        };
        let size = ICON_SMALL;
        let icon_rect = Rect::from_center_size(rect.center(), Vec2::splat(size));
        ui.put(
            icon_rect,
            Image::new(&texture).fit_to_exact_size(Vec2::splat(size)).tint(color),
        );
    }
    response.on_hover_text(tooltip)
}

fn icon_button(ui: &mut Ui, icons: &mut MaterialIcons, icon: &str, tooltip: &str, active: bool, enabled: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(ACTION_BUTTON), Sense::click());
    if !enabled {
        ui.painter()
            .rect_filled(rect, egui::CornerRadius::same(5), Color32::TRANSPARENT);
        if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
            ui.put(
                rect.shrink(6.0),
                Image::new(&texture)
                    .fit_to_exact_size(Vec2::splat(ICON_MEDIUM))
                    .tint(ui.visuals().weak_text_color().gamma_multiply(0.45)),
            );
        }
        return false;
    }
    let visuals = ui.visuals();
    let fill = if response.is_pointer_button_down_on() {
        visuals.widgets.active.weak_bg_fill
    } else if active || response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(5), fill);
    if active {
        ui.painter().rect_filled(
            egui::Rect::from_min_max(
                egui::pos2(rect.left() + 7.0, rect.bottom() - 2.0),
                egui::pos2(rect.right() - 7.0, rect.bottom()),
            ),
            1.0,
            visuals.selection.bg_fill,
        );
    }
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.put(
            rect.shrink(6.0),
            Image::new(&texture)
                .fit_to_exact_size(Vec2::splat(ICON_MEDIUM))
                .tint(visuals.text_color()),
        );
    }
    response.on_hover_text(tooltip).clicked()
}
