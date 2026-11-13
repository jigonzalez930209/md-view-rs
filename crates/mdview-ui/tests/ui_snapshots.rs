//! Settings and palette visual references.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use egui_kittest::Harness;
use mdview_i18n::{LanguagePreference, Translations};
use mdview_theme::{ThemeId, apply_id};
use mdview_ui::{PaletteItem, SettingsView, show_palette, show_settings};

fn translations() -> (Translations, craft_i18n::Lang) {
    let translations = Translations::new();
    let lang = translations.lang(LanguagePreference::English);
    (translations, lang)
}

#[test]
fn settings_snapshot() {
    let (translations, lang) = translations();
    let mut open = true;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(560.0, 720.0))
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::GithubLight))
        .build_ui(move |ui| {
            show_settings(
                ui.ctx(),
                &translations,
                lang,
                &mut open,
                SettingsView {
                    theme_id: ThemeId::GithubLight,
                    follow_system: false,
                    language: LanguagePreference::English,
                    zoom: 1.0,
                    wrap: false,
                    line_numbers: true,
                    sync_scroll: true,
                    remote_images: true,
                    private_networks: false,
                    animate_svg: true,
                    reduce_motion: false,
                },
            );
        });
    harness.run();
    harness.snapshot("settings");
}

#[test]
fn palette_snapshot() {
    let mut open = true;
    let mut query = "view".to_owned();
    let items = vec![
        PaletteItem {
            label: "Reading view".to_owned(),
            detail: String::new(),
            command: mdview_ui::Command::SetMode(mdview_ui::ViewMode::View),
        },
        PaletteItem {
            label: "Split view".to_owned(),
            detail: String::new(),
            command: mdview_ui::Command::SetMode(mdview_ui::ViewMode::Split),
        },
    ];
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 420.0))
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::Dracula))
        .build_ui(move |ui| {
            show_palette(ui.ctx(), &mut open, &mut query, "Command palette", &items);
        });
    harness.run();
    harness.snapshot("palette");
}
