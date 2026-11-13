//! The status bar.

use egui::{Layout, RichText, Ui};
use mdview_editor::EditorStatus;
use mdview_i18n::Translations;
use mdview_model::Document;

/// Draw the document status bar.
pub fn show_status(
    ui: &mut Ui,
    translations: &Translations,
    lang: craft_i18n::Lang,
    document: &Document,
    status: &EditorStatus,
) {
    let t = |source: &str| translations.t(lang, source);
    ui.horizontal(|ui| {
        let path = document
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| t("No file open"));
        ui.label(RichText::new(path).small());
        ui.separator();
        ui.label(RichText::new(if document.is_dirty() { t("Modified") } else { t("Saved") }).small());
        ui.separator();
        ui.label(
            RichText::new(format!(
                "{} {}, {} {}",
                t("Ln"),
                status.line + 1,
                t("Col"),
                status.column + 1
            ))
            .small(),
        );
        if status.selection_chars > 0 {
            ui.separator();
            ui.label(RichText::new(format!("{} {}", t("Sel"), status.selection_chars)).small());
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(RichText::new(status.encoding.label()).small());
            ui.separator();
            ui.label(RichText::new(status.eol.label()).small());
            ui.separator();
            ui.label(RichText::new(&status.language).small());
            if status.large_mode {
                ui.separator();
                ui.label(RichText::new(t("Large file")).color(ui.visuals().warn_fg_color).small());
            }
            if status.wrap {
                ui.separator();
                ui.label(RichText::new(t("Wrap")).small());
            }
        });
    });
}
