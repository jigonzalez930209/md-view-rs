//! A compact, icon-first application header.
//!
//! The window uses one row of direct action icons plus a hamburger menu for the complete command
//! set. All labels have tooltips and every control is also available from the keyboard.

use egui::{Align, Image, Layout, RichText, Ui};
use mdview_i18n::Translations;
use mdview_model::Encoding;

use crate::{Command, MaterialIcons, ViewMode};

/// Inputs for the header.
pub struct HeaderState<'a> {
    /// Translator.
    pub translations: &'a Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Document title.
    pub document_name: &'a str,
    /// Dirty marker.
    pub dirty: bool,
    /// Presentation mode.
    pub mode: ViewMode,
    /// Markdown files can use reading and split modes.
    pub is_markdown: bool,
    /// Sidebar visibility.
    pub sidebar_open: bool,
    /// Workspace availability.
    pub workspace_open: bool,
    /// Split scroll synchronization.
    pub sync_scroll: bool,
}

/// Draw the icon header and return the selected command.
pub fn show_header(ui: &mut Ui, state: HeaderState<'_>, icons: &mut MaterialIcons) -> Option<Command> {
    let translations = state.translations;
    let lang = state.lang;
    let t = |source: &str| translations.t(lang, source);
    let mut command = None;
    ui.horizontal_centered(|ui| {
        ui.spacing_mut().item_spacing.x = 3.0;

        if let Some(texture) = icons.ui_texture(ui.ctx(), "menu") {
            let image = Image::new(&texture)
                .fit_to_exact_size(egui::vec2(18.0, 18.0))
                .tint(ui.visuals().text_color());
            let response = ui
                .menu_image_button(image, |ui| {
                    ui.set_min_width(250.0);
                    if ui.button(t("New")).clicked() {
                        command = Some(Command::New);
                        ui.close();
                    }
                    if ui.button(t("Open…")).clicked() {
                        command = Some(Command::Open);
                        ui.close();
                    }
                    if ui.button(t("Open Folder…")).clicked() {
                        command = Some(Command::OpenFolder);
                        ui.close();
                    }
                    if ui.button(t("Save")).clicked() {
                        command = Some(Command::Save);
                        ui.close();
                    }
                    if ui.button(t("Save As…")).clicked() {
                        command = Some(Command::SaveAs);
                        ui.close();
                    }
                    ui.menu_button(t("Reopen with Encoding"), |ui| {
                        for encoding in [
                            Encoding::Utf8,
                            Encoding::Utf16Le,
                            Encoding::Utf16Be,
                            Encoding::Windows1252,
                            Encoding::Iso8859_1,
                            Encoding::ShiftJis,
                        ] {
                            if ui.button(encoding.label()).clicked() {
                                command = Some(Command::ReloadEncoding(encoding));
                                ui.close();
                            }
                        }
                    });
                    ui.separator();
                    if ui.button(t("Close Tab")).clicked() {
                        command = Some(Command::CloseTab);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(t("Command Palette…")).clicked() {
                        command = Some(Command::CommandPalette);
                        ui.close();
                    }
                    if ui.button(t("Quick Open…")).clicked() {
                        command = Some(Command::QuickOpen);
                        ui.close();
                    }
                    if ui.button(t("Find")).clicked() {
                        command = Some(Command::Find);
                        ui.close();
                    }
                    if ui.button(t("Replace")).clicked() {
                        command = Some(Command::Replace);
                        ui.close();
                    }
                    if ui.button(t("Go to Line")).clicked() {
                        command = Some(Command::GotoLine);
                        ui.close();
                    }
                    ui.separator();
                    if ui
                        .selectable_label(state.sidebar_open, t("Workspace sidebar"))
                        .clicked()
                    {
                        command = Some(Command::ToggleSidebar);
                        ui.close();
                    }
                    for mode in [ViewMode::Edit, ViewMode::View, ViewMode::Split] {
                        if ui
                            .add_enabled(
                                state.is_markdown || mode == ViewMode::Edit,
                                egui::Button::new(t(mode.label())).selected(state.mode == mode),
                            )
                            .clicked()
                        {
                            command = Some(Command::SetMode(mode));
                            ui.close();
                        }
                    }
                    if ui
                        .add_enabled(
                            state.mode == ViewMode::Split,
                            egui::Button::new(t("Synchronize scrolling")).selected(state.sync_scroll),
                        )
                        .clicked()
                    {
                        command = Some(Command::ToggleSyncScroll);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(t("Settings…")).clicked() {
                        command = Some(Command::Settings);
                        ui.close();
                    }
                    if ui.button(t("Check for Updates…")).clicked() {
                        command = Some(Command::CheckUpdates);
                        ui.close();
                    }
                    if ui.button(t("About MD View")).clicked() {
                        command = Some(Command::About);
                        ui.close();
                    }
                    ui.separator();
                    if ui.button(t("Quit")).clicked() {
                        command = Some(Command::Quit);
                        ui.close();
                    }
                })
                .response;
            response.on_hover_text(t("Menu"));
        }

        ui.separator();
        if icon_button(ui, icons, "sidebar", &t("Workspace sidebar"), state.sidebar_open) {
            command = Some(Command::ToggleSidebar);
        }
        if icon_button(ui, icons, "folder-open", &t("Open Folder…"), false) {
            command = Some(Command::OpenFolder);
        }
        if icon_button(ui, icons, "save", &t("Save"), false) {
            command = Some(Command::Save);
        }
        ui.separator();
        for (icon, mode, tooltip) in [
            ("edit", ViewMode::Edit, "Edit mode"),
            ("eye", ViewMode::View, "Reading view"),
            ("split", ViewMode::Split, "Split view"),
        ] {
            let enabled = state.is_markdown || mode == ViewMode::Edit;
            if icon_button(ui, icons, icon, &t(tooltip), state.mode == mode) && enabled {
                command = Some(Command::SetMode(mode));
            }
        }
        if icon_button(ui, icons, "search", &t("Find"), false) {
            command = Some(Command::Find);
        }

        ui.with_layout(Layout::left_to_right(Align::Center), |ui| {
            ui.add_space(10.0);
            let marker = if state.dirty { " •" } else { "" };
            ui.label(RichText::new(format!("{}{marker}", state.document_name)).color(ui.visuals().weak_text_color()));
        });

        ui.with_layout(Layout::right_to_left(Align::Center), |ui| {
            if icon_button(ui, icons, "settings", &t("Settings"), false) {
                command = Some(Command::Settings);
            }
            if icon_button(ui, icons, "command", &t("Command Palette"), false) {
                command = Some(Command::CommandPalette);
            }
        });
    });
    command
}

fn icon_button(ui: &mut Ui, icons: &mut MaterialIcons, icon: &str, tooltip: &str, active: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    let visuals = ui.visuals();
    let fill = if active {
        visuals.selection.bg_fill
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(5), fill);
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        let color = if active {
            visuals.selection.stroke.color
        } else {
            visuals.text_color()
        };
        ui.put(
            rect.shrink(4.0),
            Image::new(&texture)
                .fit_to_exact_size(egui::vec2(18.0, 18.0))
                .tint(color),
        );
    }
    response.on_hover_text(tooltip).clicked()
}
