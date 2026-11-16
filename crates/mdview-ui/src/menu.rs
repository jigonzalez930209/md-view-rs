//! The application menu bar.

use egui::{RichText, Ui};
use mdview_i18n::{LanguagePreference, Translations};
use mdview_model::Encoding;
use mdview_theme::{ThemeId, ThemeMode};

use crate::{Command, ViewMode};

/// Inputs for the menu bar.
pub struct MenuState<'a> {
    /// Translator.
    pub translations: &'a Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Active theme mode.
    pub theme: ThemeMode,
    /// Active theme identifier.
    pub theme_id: ThemeId,
    /// Whether the family follows the system mode.
    pub follow_system: bool,
    /// Language preference.
    pub language: LanguagePreference,
    /// Document name shown at the end of the bar.
    pub document_name: &'a str,
    /// Whether the document is dirty.
    pub dirty: bool,
    /// Whether soft wrap is enabled.
    pub wrap: bool,
    /// Whether line numbers are enabled.
    pub line_numbers: bool,
    /// Whether the workspace sidebar is visible.
    pub sidebar_open: bool,
    /// Whether a workspace folder is open.
    pub workspace_open: bool,
    /// Recent file paths, newest first.
    pub recent_files: &'a [String],
    /// Recent workspace paths, newest first.
    pub recent_workspaces: &'a [String],
    /// Active tab index.
    pub tab_index: usize,
    /// Number of open tabs.
    pub tab_count: usize,
    /// Active presentation mode.
    pub mode: ViewMode,
    /// Whether the active document is Markdown.
    pub is_markdown: bool,
    /// Whether split scrolling is synchronized.
    pub sync_scroll: bool,
    /// Whether remote images may load.
    pub remote_images: bool,
    /// Whether private-network images may load.
    pub private_networks: bool,
    /// Whether SVG animations play.
    pub animate_svg: bool,
    /// Whether reduced motion is on.
    pub reduce_motion: bool,
}

/// Draw the menu bar and return the selected command.
pub fn show_menu(ui: &mut Ui, state: MenuState<'_>) -> Option<Command> {
    let mut command = None;
    let translations = state.translations;
    let lang = state.lang;
    let theme = state.theme;
    let language = state.language;
    let t = |source: &str| translations.t(lang, source);

    ui.horizontal(|ui| {
        ui.menu_button(t("File"), |ui| {
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
            if ui
                .add_enabled(state.workspace_open, egui::Button::new(t("Close Folder")))
                .clicked()
            {
                command = Some(Command::CloseFolder);
                ui.close();
            }
            if !state.recent_workspaces.is_empty() {
                ui.menu_button(t("Recent Workspaces"), |ui| {
                    for (index, path) in state.recent_workspaces.iter().enumerate() {
                        let name = std::path::Path::new(path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.clone());
                        if ui.button(name).on_hover_text(path).clicked() {
                            command = Some(Command::OpenRecentWorkspace(index));
                            ui.close();
                        }
                    }
                });
            }
            ui.menu_button(t("Open Recent"), |ui| {
                if state.recent_files.is_empty() {
                    ui.label(t("No recent files"));
                } else {
                    for (index, path) in state.recent_files.iter().enumerate() {
                        let name = std::path::Path::new(path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.clone());
                        if ui.button(name).on_hover_text(path).clicked() {
                            command = Some(Command::OpenRecent(index));
                            ui.close();
                        }
                    }
                    ui.separator();
                    if ui.button(t("Clear recent files")).clicked() {
                        command = Some(Command::ClearRecent);
                        ui.close();
                    }
                }
            });
            ui.separator();
            if ui.button(t("New File in Workspace")).clicked() {
                command = Some(Command::NewFile);
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
            ui.separator();
            if ui
                .add_enabled(state.tab_count > 0, egui::Button::new(t("Close Tab")))
                .clicked()
            {
                command = Some(Command::CloseTab);
                ui.close();
            }
            if ui.button(t("Close Other Tabs")).clicked() {
                command = Some(Command::CloseOtherTabs);
                ui.close();
            }
            if ui.button(t("Close Saved Tabs")).clicked() {
                command = Some(Command::CloseSavedTabs);
                ui.close();
            }
            ui.separator();
            if ui
                .add_enabled(state.tab_index > 0, egui::Button::new(t("Move Tab Left")))
                .clicked()
            {
                command = Some(Command::MoveTabLeft);
                ui.close();
            }
            if ui
                .add_enabled(
                    state.tab_index + 1 < state.tab_count,
                    egui::Button::new(t("Move Tab Right")),
                )
                .clicked()
            {
                command = Some(Command::MoveTabRight);
                ui.close();
            }
            ui.separator();
            if ui.button(t("Next Tab")).clicked() {
                command = Some(Command::NextTab);
                ui.close();
            }
            if ui.button(t("Previous Tab")).clicked() {
                command = Some(Command::PreviousTab);
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
            if ui.button(t("Quit")).clicked() {
                command = Some(Command::Quit);
                ui.close();
            }
        });

        ui.menu_button(t("Edit"), |ui| {
            if ui.button(t("Command Palette…")).clicked() {
                command = Some(Command::CommandPalette);
                ui.close();
            }
            if ui.button(t("Quick Open…")).clicked() {
                command = Some(Command::QuickOpen);
                ui.close();
            }
            ui.separator();
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
            if ui.button(t("Insert LaTeX Block")).clicked() {
                command = Some(Command::InsertLatexBlock);
                ui.close();
            }
        });

        ui.menu_button(t("View"), |ui| {
            ui.menu_button(t("Mode"), |ui| {
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
            });
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
            ui.menu_button(t("Media"), |ui| {
                if ui
                    .selectable_label(state.remote_images, t("Load remote images"))
                    .clicked()
                {
                    command = Some(Command::ToggleRemoteMedia);
                    ui.close();
                }
                if ui
                    .add_enabled(
                        state.remote_images,
                        egui::Button::new(t("Allow private network images")).selected(state.private_networks),
                    )
                    .clicked()
                {
                    command = Some(Command::TogglePrivateNetworks);
                    ui.close();
                }
            });
            ui.separator();
            ui.menu_button(t("Appearance"), |ui| {
                if ui.selectable_label(state.follow_system, t("Follow system")).clicked() {
                    command = Some(Command::SetFollowSystem(!state.follow_system));
                    ui.close();
                }
                ui.separator();
                for family in ["GitHub", "Dracula", "One Dark Pro"] {
                    ui.menu_button(family, |ui| {
                        for id in ThemeId::ALL.into_iter().filter(|id| id.family() == family) {
                            if ui.selectable_label(state.theme_id == id, id.label()).clicked() {
                                command = Some(Command::SetThemeId(id));
                                ui.close();
                            }
                        }
                    });
                }
                ui.separator();
                let mode = if theme == ThemeMode::Dark {
                    "Dark base"
                } else {
                    "Light base"
                };
                ui.label(egui::RichText::new(mode).weak());
            });
            ui.menu_button(t("Language"), |ui| {
                if ui
                    .selectable_label(language == LanguagePreference::Auto, t("Follow system"))
                    .clicked()
                {
                    command = Some(Command::SetLanguage(LanguagePreference::Auto));
                    ui.close();
                }
                if ui
                    .selectable_label(language == LanguagePreference::English, t("English"))
                    .clicked()
                {
                    command = Some(Command::SetLanguage(LanguagePreference::English));
                    ui.close();
                }
                if ui
                    .selectable_label(language == LanguagePreference::Spanish, t("Spanish"))
                    .clicked()
                {
                    command = Some(Command::SetLanguage(LanguagePreference::Spanish));
                    ui.close();
                }
                if ui
                    .selectable_label(language == LanguagePreference::Pseudo, t("Pseudo-locale"))
                    .clicked()
                {
                    command = Some(Command::SetLanguage(LanguagePreference::Pseudo));
                    ui.close();
                }
            });
            ui.separator();
            if ui
                .selectable_label(state.sidebar_open, t("Workspace sidebar"))
                .clicked()
            {
                command = Some(Command::ToggleSidebar);
                ui.close();
            }
            if ui.selectable_label(state.wrap, t("Word wrap")).clicked() {
                command = Some(Command::ToggleWrap);
                ui.close();
            }
            if ui.selectable_label(state.line_numbers, t("Line numbers")).clicked() {
                command = Some(Command::ToggleLineNumbers);
                ui.close();
            }
        });

        ui.menu_button(t("Help"), |ui| {
            if ui.button(t("Check for Updates…")).clicked() {
                command = Some(Command::CheckUpdates);
                ui.close();
            }
            if ui.button(t("Settings…")).clicked() {
                command = Some(Command::Settings);
                ui.close();
            }
            if ui.button(t("About MD View")).clicked() {
                command = Some(Command::About);
                ui.close();
            }
        });

        ui.separator();
        let marker = if state.dirty { " •" } else { "" };
        ui.label(RichText::new(format!("{}{marker}", state.document_name)).color(ui.visuals().weak_text_color()));
    });

    command
}
