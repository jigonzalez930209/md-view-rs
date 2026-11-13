//! Local visual snapshots for the editor's light/dark and wrap/no-wrap states.
//!
//! Run `UPDATE_SNAPSHOTS=1 cargo test -p mdview-editor --test editor_snapshots` to regenerate
//! the PNGs under `tests/snapshots/`. The harness uses the same egui/eframe revision as the app.

use std::path::PathBuf;

use egui_kittest::Harness;
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};

const SOURCE: &str = r#"# Editor snapshot

fn main() {
    let answer = 42;
    println!("answer = {answer}");
    let long_line = "a deliberately long line used to exercise soft wrapping and the horizontal viewport";
}
"#;

fn editor() -> Editor {
    let document = Document::from_loaded(
        PathBuf::from("snapshot.rs"),
        SOURCE.to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    Editor::new(document)
}

fn snapshot(name: &str, light: bool, wrap: bool) {
    let mut editor = editor();
    editor.set_light(light);
    editor.set_wrap(wrap);
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 360.0))
        .with_context_setup(move |ctx| {
            ctx.set_visuals(if light {
                egui::Visuals::light()
            } else {
                egui::Visuals::dark()
            });
        })
        .build_ui(move |ui| {
            editor.show(ui);
        });
    harness.run();
    harness.snapshot(name);
}

#[test]
fn editor_light_nowrap() {
    snapshot("editor-light-nowrap", true, false);
}

#[test]
fn editor_dark_nowrap() {
    snapshot("editor-dark-nowrap", false, false);
}

#[test]
fn editor_light_wrap() {
    snapshot("editor-light-wrap", true, true);
}

#[test]
fn editor_dark_wrap() {
    snapshot("editor-dark-wrap", false, true);
}
