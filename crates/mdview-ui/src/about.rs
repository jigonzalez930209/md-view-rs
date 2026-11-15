//! The About window.

use egui::Context;
use mdview_i18n::Translations;

/// Draw the About window while `open` is true.
pub fn show_about(ctx: &Context, translations: &Translations, lang: craft_i18n::Lang, open: &mut bool) {
    let t = |source: &str| translations.t(lang, source);
    if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
        *open = false;
    }
    egui::Window::new(t("About MD View"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .default_width(360.0)
        .show(ctx, |ui| {
            ui.heading("MD View");
            ui.label(t("A native Markdown and text editor."));
            ui.add_space(8.0);
            ui.label(format!("{} {}", t("Version"), env!("CARGO_PKG_VERSION")));
            ui.label("Rust + egui");
        });
}
