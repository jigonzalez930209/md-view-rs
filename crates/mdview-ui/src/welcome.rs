//! The empty-state screen.

use egui::Ui;
use mdview_i18n::Translations;

/// Draw the welcome screen. Returns true when the user asks to open a file.
pub fn show_welcome(ui: &mut Ui, translations: &Translations, lang: craft_i18n::Lang) -> bool {
    let t = |source: &str| translations.t(lang, source);
    let mut open = false;
    ui.vertical_centered(|ui| {
        ui.add_space((ui.available_height() * 0.24).max(24.0));
        ui.heading("MD View");
        ui.label(t("A native Markdown and text editor."));
        ui.add_space(16.0);
        if ui.button(t("Open file")).clicked() {
            open = true;
        }
        ui.add_space(8.0);
        ui.label(t("Open a file to begin."));
    });
    open
}
