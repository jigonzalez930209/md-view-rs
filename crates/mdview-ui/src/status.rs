//! The status bar.

use egui::{Layout, RichText, Ui};
use mdview_i18n::Translations;
use mdview_model::Document;

/// Draw the document status bar.
pub fn show_status(ui: &mut Ui, translations: &Translations, lang: craft_i18n::Lang, document: &Document) {
    let t = |source: &str| translations.t(lang, source);
    ui.horizontal(|ui| {
        let path = document
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| t("No file open").to_owned());
        ui.label(RichText::new(path).small());
        ui.separator();
        ui.label(RichText::new(if document.is_dirty() { t("Modified") } else { t("Saved") }).small());

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new("UTF-8").small());
            ui.separator();
            ui.label(RichText::new(document.eol().label()).small());
            ui.separator();
            ui.label(RichText::new(t("Ready")).small());
        });
    });
}
