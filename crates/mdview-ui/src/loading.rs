//! Background loading dialog.

use egui::Context;
use mdview_i18n::Translations;

/// Draw the loading dialog. Returns true when the user asks to cancel.
pub fn show_loading(
    ctx: &Context,
    translations: &Translations,
    lang: craft_i18n::Lang,
    path: &std::path::Path,
    progress: f32,
) -> bool {
    let t = |source: &str| translations.t(lang, source);
    let mut cancel = false;
    egui::Window::new(t("Opening file…"))
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(460.0)
        .show(ctx, |ui| {
            ui.label(path.display().to_string());
            ui.add(egui::ProgressBar::new(progress).show_percentage());
            ui.add_space(8.0);
            if ui.button(t("Cancel")).clicked() {
                cancel = true;
            }
        });
    cancel
}
