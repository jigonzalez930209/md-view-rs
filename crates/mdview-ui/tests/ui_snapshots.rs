//! Settings and palette visual references.

#![allow(clippy::expect_used, clippy::unwrap_used)]

use egui_kittest::Harness;
use mdview_i18n::{LanguagePreference, Translations};
use mdview_theme::{ThemeId, apply_id};
use mdview_ui::{
    ActivityState, ConfirmPrompt, NamePrompt, PaletteItem, SettingsView, TitleBarState, show_activity_bar,
    show_confirm_prompt, show_name_prompt, show_palette, show_settings, show_title_bar,
};

fn translations() -> (Translations, craft_i18n::Lang) {
    let translations = Translations::new();
    let lang = translations.lang(LanguagePreference::English);
    (translations, lang)
}

#[test]
fn settings_snapshot() {
    let (translations, lang) = translations();
    let mut icons = mdview_ui::MaterialIcons::new();
    let mut open = true;
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 620.0))
        .with_accessibility_check(false)
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
                    show_hidden: false,
                    respect_gitignore: true,
                    log_bytes: 0,
                },
                &mut icons,
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

#[test]
fn title_bar_snapshot() {
    let (translations, lang) = translations();
    let mut icons = mdview_ui::MaterialIcons::new();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1000.0, 34.0))
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::GithubDark))
        .build_ui(move |ui| {
            show_title_bar(
                ui,
                TitleBarState {
                    translations: &translations,
                    lang,
                    document_name: "notes.md",
                    dirty: true,
                    mode: mdview_ui::ViewMode::Split,
                    is_markdown: true,
                    sidebar_open: true,
                    workspace_open: true,
                    workspace_name: Some("xtalforge"),
                    sync_scroll: true,
                    wrap: false,
                    line_numbers: true,
                    maximized: false,
                },
                &mut icons,
            );
        });
    harness.run();
    harness.snapshot("header");
}

#[test]
fn activity_bar_snapshot() {
    let (translations, lang) = translations();
    let mut icons = mdview_ui::MaterialIcons::new();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(48.0, 300.0))
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::GithubDark))
        .build_ui(move |ui| {
            show_activity_bar(
                ui,
                ActivityState {
                    translations: &translations,
                    lang,
                    explorer_open: true,
                    workspace_open: true,
                    active: Some(mdview_ui::Activity::Explorer),
                },
                &mut icons,
            );
        });
    harness.run();
    harness.snapshot("activity");
}

#[test]
fn new_file_prompt_snapshot() {
    let mut name = "untitled.md".to_owned();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 460.0))
        .with_accessibility_check(false)
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::GithubDark))
        .build_ui(move |ui| {
            let mut prompt = NamePrompt {
                title: "New file".to_owned(),
                description: "Creating in /home/lestad/github/md-view-rs".to_owned(),
                label: "File name".to_owned(),
                value: &mut name,
                confirm: "Create".to_owned(),
                error: None,
            };
            show_name_prompt(ui.ctx(), &mut prompt);
        });
    harness.run();
    harness.snapshot("new-file");
}

#[test]
fn delete_prompt_snapshot() {
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 420.0))
        .with_accessibility_check(false)
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::GithubDark))
        .build_ui(|ui| {
            let prompt = ConfirmPrompt {
                title: "Delete".to_owned(),
                message: "Delete this entry permanently? src/old-notes.md".to_owned(),
                confirm: "Delete".to_owned(),
                danger: true,
            };
            show_confirm_prompt(ui.ctx(), &prompt);
        });
    harness.run();
    harness.snapshot("delete");
}
