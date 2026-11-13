//! The application menu bar.

use egui::{RichText, Ui};
use mdview_i18n::{LanguagePreference, Translations};
use mdview_model::Encoding;
use mdview_theme::ThemeMode;

use crate::Command;

/// Inputs for the menu bar.
pub struct MenuState<'a> {
    /// Translator.
    pub translations: &'a Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Active theme.
    pub theme: ThemeMode,
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
        });

        ui.menu_button(t("View"), |ui| {
            ui.menu_button(t("Appearance"), |ui| {
                if ui.selectable_label(theme == ThemeMode::Light, t("Light")).clicked() {
                    command = Some(Command::SetTheme(ThemeMode::Light));
                    ui.close();
                }
                if ui.selectable_label(theme == ThemeMode::Dark, t("Dark")).clicked() {
                    command = Some(Command::SetTheme(ThemeMode::Dark));
                    ui.close();
                }
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
