//! The settings window.

use egui::{Context, RichText, Window};
use mdview_i18n::{LanguagePreference, Translations};
use mdview_theme::ThemeId;

use crate::Command;

/// Current values shown by the settings window.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsView {
    /// Theme identifier.
    pub theme_id: ThemeId,
    /// Follow the system light/dark mode.
    pub follow_system: bool,
    /// UI language.
    pub language: LanguagePreference,
    /// Text scale.
    pub zoom: f32,
    /// Soft wrap.
    pub wrap: bool,
    /// Line numbers.
    pub line_numbers: bool,
    /// Split scroll synchronization.
    pub sync_scroll: bool,
    /// Remote images.
    pub remote_images: bool,
    /// Private-network images.
    pub private_networks: bool,
    /// SVG animations.
    pub animate_svg: bool,
    /// Reduced motion.
    pub reduce_motion: bool,
}

/// Show the settings window and return the selected change.
pub fn show_settings(
    ctx: &Context,
    translations: &Translations,
    lang: craft_i18n::Lang,
    open: &mut bool,
    view: SettingsView,
) -> Option<Command> {
    let t = |source: &str| translations.t(lang, source);
    let mut command = None;
    let mut window_open = *open;
    let mut keep_open = *open;
    Window::new(t("Settings"))
        .open(&mut window_open)
        .resizable(false)
        .default_width(420.0)
        .show(ctx, |ui| {
            ui.heading(t("Appearance"));
            let mut theme_id = view.theme_id;
            egui::ComboBox::from_label(t("Theme"))
                .selected_text(theme_id.label())
                .show_ui(ui, |ui| {
                    for id in ThemeId::ALL {
                        ui.selectable_value(&mut theme_id, id, id.label());
                    }
                });
            if theme_id != view.theme_id {
                command = Some(Command::SetThemeId(theme_id));
            }
            let mut follow = view.follow_system;
            if ui.checkbox(&mut follow, t("Follow system")).changed() {
                command = Some(Command::SetFollowSystem(follow));
            }
            let mut zoom = view.zoom;
            if ui
                .add(egui::Slider::new(&mut zoom, 0.8..=2.0).text(t("Text scale")))
                .changed()
            {
                command = Some(Command::SetZoom(zoom));
            }
            ui.add_space(8.0);
            ui.heading(t("Language"));
            let mut language = view.language;
            egui::ComboBox::from_label(t("Language"))
                .selected_text(match language {
                    LanguagePreference::Auto => t("Follow system"),
                    LanguagePreference::English => t("English"),
                    LanguagePreference::Spanish => t("Spanish"),
                    LanguagePreference::Pseudo => t("Pseudo-locale"),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut language, LanguagePreference::Auto, t("Follow system"));
                    ui.selectable_value(&mut language, LanguagePreference::English, t("English"));
                    ui.selectable_value(&mut language, LanguagePreference::Spanish, t("Spanish"));
                    ui.selectable_value(&mut language, LanguagePreference::Pseudo, t("Pseudo-locale"));
                });
            if language != view.language {
                command = Some(Command::SetLanguage(language));
            }
            ui.add_space(8.0);
            ui.heading(t("Editor"));
            let mut wrap = view.wrap;
            if ui.checkbox(&mut wrap, t("Word wrap")).changed() {
                command = Some(Command::ToggleWrap);
            }
            let mut line_numbers = view.line_numbers;
            if ui.checkbox(&mut line_numbers, t("Line numbers")).changed() {
                command = Some(Command::ToggleLineNumbers);
            }
            let mut sync = view.sync_scroll;
            if ui.checkbox(&mut sync, t("Synchronize scrolling")).changed() {
                command = Some(Command::ToggleSyncScroll);
            }
            ui.add_space(8.0);
            ui.heading(t("Media"));
            let mut remote = view.remote_images;
            if ui.checkbox(&mut remote, t("Load remote images")).changed() {
                command = Some(Command::ToggleRemoteMedia);
            }
            let mut private = view.private_networks;
            if ui
                .add_enabled(
                    remote,
                    egui::Checkbox::new(&mut private, t("Allow private network images")),
                )
                .changed()
            {
                command = Some(Command::TogglePrivateNetworks);
            }
            let mut animate = view.animate_svg;
            if ui.checkbox(&mut animate, t("Play SVG animations")).changed() {
                command = Some(Command::ToggleAnimations);
            }
            let mut reduce = view.reduce_motion;
            if ui.checkbox(&mut reduce, t("Reduce motion")).changed() {
                command = Some(Command::ToggleReduceMotion);
            }
            ui.add_space(12.0);
            ui.horizontal(|ui| {
                if ui.button(t("Reset text scale")).clicked() {
                    command = Some(Command::SetZoom(1.0));
                }
                if ui.button(t("Close")).clicked() {
                    keep_open = false;
                }
            });
            ui.add_space(4.0);
            ui.label(
                RichText::new(format!("MD View {}", env!("CARGO_PKG_VERSION")))
                    .small()
                    .weak(),
            );
        });
    *open = window_open && keep_open;
    command
}
