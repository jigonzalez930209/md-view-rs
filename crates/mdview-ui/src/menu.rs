//! The application menu bar.

use egui::{RichText, Ui};
use mdview_i18n::{LanguagePreference, Translations};
use mdview_theme::ThemeMode;

use crate::Command;

/// Draw the menu bar and return the selected command.
pub fn show_menu(
    ui: &mut Ui,
    translations: &Translations,
    lang: craft_i18n::Lang,
    theme: ThemeMode,
    language: LanguagePreference,
    document_name: &str,
    dirty: bool,
) -> Option<Command> {
    let mut command = None;
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
            ui.separator();
            if ui.button(t("Quit")).clicked() {
                command = Some(Command::Quit);
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
        });

        ui.menu_button(t("Help"), |ui| {
            if ui.button(t("About MD View")).clicked() {
                command = Some(Command::About);
                ui.close();
            }
        });

        ui.separator();
        let marker = if dirty { " •" } else { "" };
        ui.label(RichText::new(format!("{document_name}{marker}")).color(ui.visuals().weak_text_color()));
    });

    command
}
