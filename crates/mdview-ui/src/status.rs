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
    viewer: Option<&str>,
) {
    let t = |source: &str| translations.t(lang, source);
    ui.horizontal_centered(|ui| {
        let full_path = document
            .path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|| t("No file open"));
        let path = if full_path.chars().count() > 48 {
            let tail: String = full_path.chars().skip(full_path.chars().count() - 47).collect();
            format!("…{tail}")
        } else {
            full_path.clone()
        };
        ui.label(RichText::new(path).small()).on_hover_text(full_path);
        ui.separator();
        ui.label(
            RichText::new(if let Some(viewer) = viewer {
                viewer.to_owned()
            } else if document.is_dirty() {
                t("Modified")
            } else {
                t("Saved")
            })
            .small(),
        );
        if viewer.is_none() {
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
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            let extension = document
                .path()
                .and_then(|path| path.extension())
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_uppercase)
                .unwrap_or_else(|| t("Image"));
            ui.label(
                RichText::new(if viewer.is_some() {
                    extension
                } else {
                    status.encoding.label().to_owned()
                })
                .small(),
            );
            if viewer.is_none() {
                ui.separator();
                ui.label(RichText::new(status.eol.label()).small());
                ui.separator();
                ui.label(RichText::new(status.indentation.label()).small());
                ui.separator();
            }
            if viewer.is_none() {
                ui.label(RichText::new(status.language.clone()).small());
            }
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
