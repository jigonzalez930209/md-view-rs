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
            ui.separator();
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
