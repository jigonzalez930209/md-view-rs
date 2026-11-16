//! Explorer state and snapshot tests.

use std::path::PathBuf;

use egui_kittest::Harness;
use mdview_i18n::{LanguagePreference, Translations};
use mdview_workspace::Workspace;

use super::*;

#[test]
fn explorer_is_visible_only_with_a_folder() {
    let mut sidebar = Sidebar::new(260.0, false, true);
    assert!(sidebar.should_show(true));
    assert!(!sidebar.should_show(false));
    sidebar.open = false;
    assert!(!sidebar.should_show(true));
    sidebar.reveal();
    assert!(sidebar.should_show(true));
}

#[test]
fn active_workspace_file_expands_its_ancestors() {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = temp.path().join("project");
    std::fs::create_dir_all(dir.join("src").join("nested")).expect("src");
    std::fs::write(dir.join("README.md"), "# Read me\n").expect("readme");
    std::fs::write(dir.join("src/nested/file.md"), "# File\n").expect("file");
    let mut workspace = Workspace::open(&dir).expect("workspace");
    let mut sidebar = Sidebar::new(260.0, false, true);
    let file = workspace.root().join("src/nested/file.md");

    sidebar.sync_active(&mut workspace, Some(&file));

    assert!(
        workspace
            .root_entry()
            .find(&workspace.root().join("src"))
            .is_some_and(|entry| entry.expanded),
        "the parent folder was not expanded"
    );
    assert!(
        workspace
            .root_entry()
            .find(&workspace.root().join("src/nested"))
            .is_some_and(|entry| entry.expanded),
        "the nested folder was not expanded"
    );
    assert_eq!(sidebar.selected.as_deref(), Some(file.as_path()));
    assert!(sidebar.scroll_to_selected);

    // Staying on the same tab must not steal the tree selection back after the user clicks a
    // folder.
    let other = workspace.root().join("README.md");
    sidebar.selected = Some(other.clone());
    sidebar.sync_active(&mut workspace, Some(&file));
    assert_eq!(sidebar.selected, Some(other));
}

fn sidebar_snapshot_with(name: &str, pixels_per_point: f32, with_external: bool) {
    let temp = tempfile::tempdir().expect("temp dir");
    let dir = temp.path().join("project");
    std::fs::create_dir(&dir).expect("project");
    std::fs::create_dir(dir.join("src")).expect("src");
    std::fs::create_dir(dir.join("docs")).expect("docs");
    std::fs::write(dir.join("README.md"), "# Read me\n").expect("readme");
    std::fs::write(dir.join("main.rs"), "fn main() {}\n").expect("main");
    std::fs::write(dir.join("src/lib.rs"), "").expect("lib");
    std::fs::write(dir.join("docs/guide.md"), "# Guide\n").expect("guide");
    let mut workspace = Workspace::open(&dir).expect("workspace");
    workspace.expand(&dir.join("src"));
    let mut external = Vec::new();
    if with_external {
        let folder = temp.path().join("external");
        std::fs::create_dir(&folder).expect("external");
        std::fs::write(folder.join("notes.md"), "# Notes\n").expect("notes");
        std::fs::write(folder.join("photo.png"), "").expect("photo");
        external = vec![folder.join("notes.md"), folder.join("photo.png")];
    }
    let active: Option<PathBuf> = with_external.then(|| external[0].clone());
    let mut sidebar = Sidebar::new(260.0, false, true);
    let mut icons = MaterialIcons::new();
    sidebar.open = true;
    let translations = Translations::new();
    let lang = translations.lang(LanguagePreference::English);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(260.0, 380.0))
        .with_pixels_per_point(pixels_per_point)
        .build_ui(move |ui| {
            sidebar.show(
                ui,
                &mut workspace,
                SidebarView {
                    translations: &translations,
                    lang,
                    icons: &mut icons,
                    light: true,
                    active: active.as_deref(),
                    external: &external,
                },
            );
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn sidebar_snapshot() {
    sidebar_snapshot_with("sidebar", 1.0, false);
}

#[test]
fn sidebar_2x_snapshot() {
    sidebar_snapshot_with("sidebar-2x", 2.0, false);
}

#[test]
fn sidebar_external_snapshot() {
    sidebar_snapshot_with("sidebar-external", 1.0, true);
}
