//! Tab presentation and session tests.

use std::path::PathBuf;

use egui_kittest::Harness;
use mdview_i18n::Translations;
use mdview_model::{Document, Encoding, Eol};
use mdview_theme::{ThemeId, apply_id};

use super::*;
use crate::tab_bar::{TabBarView, show_tab_bar};

fn document(path: &str) -> Document {
    Document::from_loaded(
        PathBuf::from(path),
        "hello\n".to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    )
}

fn tab_bar_snapshot_with(name: &str, pixels_per_point: f32) {
    let translations = Translations::new();
    let lang = translations.lang(mdview_i18n::LanguagePreference::English);
    let infos = vec![
        TabInfo {
            label: "README.md".to_owned(),
            icon: "markdown".to_owned(),
            dirty: false,
            conflict: false,
            missing: false,
        },
        TabInfo {
            label: "main.rs".to_owned(),
            icon: "rust".to_owned(),
            dirty: true,
            conflict: false,
            missing: false,
        },
        TabInfo {
            label: "deleted.md".to_owned(),
            icon: "markdown".to_owned(),
            dirty: false,
            conflict: false,
            missing: true,
        },
    ];
    let mut icons = mdview_ui::MaterialIcons::new();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(900.0, 34.0))
        .with_pixels_per_point(pixels_per_point)
        .with_context_setup(|ctx| apply_id(ctx, ThemeId::GithubDark))
        .build_ui(move |ui| {
            show_tab_bar(
                ui,
                &infos,
                1,
                TabBarView {
                    translations: &translations,
                    lang,
                    icons: &mut icons,
                    light: false,
                    mode: mdview_ui::ViewMode::Split,
                    can_view: true,
                    can_edit: true,
                    can_split: true,
                },
            );
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn tab_bar_snapshot() {
    tab_bar_snapshot_with("tab-bar", 1.0);
}

#[test]
fn tab_bar_2x_snapshot() {
    tab_bar_snapshot_with("tab-bar-2x", 2.0);
}

#[test]
fn image_documents_open_in_the_viewer_and_never_edit() {
    let mut tab = DocumentTab::new(document("/tmp/photo.png"), None);
    assert!(tab.is_viewer());
    assert_eq!(tab.mode, ViewMode::View);
    tab.set_mode(ViewMode::Edit);
    assert_eq!(tab.mode, ViewMode::View);
    tab.set_mode(ViewMode::Split);
    assert_eq!(tab.mode, ViewMode::View);
}

#[test]
fn opening_the_same_path_reuses_the_tab() {
    let mut tabs = TabSet::new(true, false, true);
    let first = tabs.open(document("/tmp/a.md"), None);
    tabs.new_untitled();
    let again = tabs.open(document("/tmp/a.md"), None);
    assert_eq!(first, again);
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs.active_index(), first);
}

#[test]
fn opening_a_document_replaces_a_pristine_untitled_tab() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.open(document("/tmp/a.md"), None);
    assert_eq!(tabs.len(), 1);
    assert_eq!(
        tabs.active().editor.document().path(),
        Some(std::path::Path::new("/tmp/a.md"))
    );
}

#[test]
fn moving_tabs_reorders_and_tracks_the_active_tab() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.open(document("/tmp/a.md"), None);
    tabs.open(document("/tmp/b.md"), None);
    tabs.move_tab(1, 0);
    assert_eq!(tabs.active_index(), 0);
    assert_eq!(tabs.tabs()[0].editor.document().display_name(), "b.md");
    assert_eq!(tabs.tabs()[1].editor.document().display_name(), "a.md");
}

#[test]
fn rename_updates_the_matching_tab() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.open(document("/tmp/a.md"), None);
    assert!(tabs.rename_path(std::path::Path::new("/tmp/a.md"), std::path::Path::new("/tmp/b.md")));
    assert_eq!(
        tabs.active().editor.document().path(),
        Some(std::path::Path::new("/tmp/b.md"))
    );
    assert_eq!(tabs.find_path(std::path::Path::new("/tmp/a.md")), None);
}

#[test]
fn missing_files_are_marked_in_the_label() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.open(document("/tmp/missing.md"), None);
    tabs.active_mut().missing = true;
    let mut icons = mdview_ui::MaterialIcons::new();
    let infos = tabs.infos(&mut icons, true);
    assert!(infos[0].label.contains("missing.md"));
    assert!(infos[0].missing);
}

#[test]
fn closing_the_last_tab_keeps_one_empty_tab() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.close(0);
    assert_eq!(tabs.len(), 1);
    assert!(tabs.active().editor.document().path().is_none());
}

#[test]
fn markdown_tabs_open_in_reading_view_and_code_tabs_in_edit_mode() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.open(document("/tmp/notes.md"), None);
    assert_eq!(tabs.active().mode, ViewMode::View);
    assert!(tabs.active().is_markdown());
    tabs.open(document("/tmp/main.rs"), None);
    assert_eq!(tabs.active().mode, ViewMode::Edit);
    assert!(!tabs.active().is_markdown());
    tabs.set_mode(tabs.active_index(), ViewMode::View);
    assert_eq!(tabs.active().mode, ViewMode::Edit);
}

#[test]
fn preview_source_refreshes_with_the_document_revision() {
    let mut tabs = TabSet::new(true, false, true);
    tabs.open(document("/tmp/notes.md"), None);
    let tab = tabs.active_mut();
    assert_eq!(tab.ensure_preview(), "hello\n");
    tab.editor.document_mut().replace_range(0, 0, "# ");
    assert_eq!(tab.ensure_preview(), "# hello\n");
}

#[test]
fn large_markdown_files_force_edit_mode() {
    let text = "x".repeat(10 * 1024 * 1024 + 1);
    let document = Document::from_loaded(PathBuf::from("/tmp/huge.md"), text, Eol::Lf, Encoding::Utf8, false);
    let mut tab = DocumentTab::new(document, None);
    assert!(tab.is_large());
    assert_eq!(tab.mode, ViewMode::Edit);
    tab.set_mode(ViewMode::View);
    assert_eq!(tab.mode, ViewMode::Edit);
    tab.set_mode(ViewMode::Split);
    assert_eq!(tab.mode, ViewMode::Edit);
}

#[test]
fn two_megabyte_markdown_files_disable_the_preview() {
    let text = format!("# Large\n\n{}", "x".repeat(2 * 1024 * 1024 + 1));
    let document = Document::from_loaded(
        PathBuf::from("/tmp/large-preview.md"),
        text,
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    let mut tab = DocumentTab::new(document, None);
    assert!(tab.is_large());
    assert_eq!(tab.mode, ViewMode::Edit);
    tab.set_mode(ViewMode::Split);
    assert_eq!(tab.mode, ViewMode::Edit);
}

#[test]
fn one_hundred_open_close_cycles_stay_stable() {
    let mut tabs = TabSet::new(true, false, true);
    for index in 0..100 {
        let path = format!("/tmp/cycle-{index}.md");
        tabs.open(document(&path), None);
        let active = tabs.active_index();
        tabs.close(active);
    }
    assert_eq!(tabs.len(), 1);
    assert!(tabs.active().editor.document().path().is_none());
}

#[test]
fn the_active_tab_is_scrolled_into_view() {
    use egui_kittest::kittest::Queryable;
    use std::{cell::Cell, rc::Rc};

    let translations = Translations::new();
    let lang = translations.lang(mdview_i18n::LanguagePreference::English);
    let infos: Vec<TabInfo> = (0..12)
        .map(|index| TabInfo {
            label: format!("document-number-{index}.md"),
            icon: "markdown".to_owned(),
            dirty: false,
            conflict: false,
            missing: false,
        })
        .collect();
    let active = Rc::new(Cell::new(0usize));
    let shown = active.clone();
    let mut icons = mdview_ui::MaterialIcons::new();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 34.0))
        .build_ui(move |ui| {
            show_tab_bar(
                ui,
                &infos,
                shown.get(),
                TabBarView {
                    translations: &translations,
                    lang,
                    icons: &mut icons,
                    light: true,
                    mode: mdview_ui::ViewMode::Edit,
                    can_view: true,
                    can_edit: true,
                    can_split: true,
                },
            );
        });
    harness.run();
    let visible = |harness: &Harness<'_>, label: &str| {
        let rect = harness.get_by_label_contains(label).rect();
        rect.left() >= 0.0 && rect.right() <= 700.0
    };
    assert!(visible(&harness, "document-number-0.md"));
    // Activating the last tab (Ctrl+Tab, Quick Open, or the explorer) must reveal it.
    active.set(11);
    harness.run();
    assert!(
        visible(&harness, "document-number-11.md"),
        "the active tab stays hidden"
    );
    active.set(0);
    harness.run();
    assert!(visible(&harness, "document-number-0.md"));
}
