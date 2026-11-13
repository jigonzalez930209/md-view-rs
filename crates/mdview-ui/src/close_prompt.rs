//! The unsaved-changes prompt.

use egui::{Align2, Context};
use mdview_i18n::Translations;

/// What the user chose when closing a dirty document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CloseChoice {
    /// Save before closing.
    Save,
    /// Close and discard the buffer.
    Discard,
    /// Keep the document open.
    Cancel,
}

/// Draw the close prompt and return the user's choice.
pub fn show_close_prompt(
    ctx: &Context,
    translations: &Translations,
    lang: craft_i18n::Lang,
    open: &mut bool,
) -> Option<CloseChoice> {
    let t = |source: &str| translations.t(lang, source);
    let mut choice = None;
    egui::Window::new(t("Save changes?"))
        .open(open)
        .resizable(false)
        .collapsible(false)
        .anchor(Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.label(t("Do you want to save your changes before closing?"));
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(t("Save")).clicked() {
                    choice = Some(CloseChoice::Save);
                }
                if ui.button(t("Discard")).clicked() {
                    choice = Some(CloseChoice::Discard);
                }
                if ui.button(t("Cancel")).clicked() {
                    choice = Some(CloseChoice::Cancel);
                }
            });
        });
    choice
}
