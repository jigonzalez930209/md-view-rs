//! Split layout snapshots with the editor and the rendered preview.

use std::path::PathBuf;

use egui_kittest::Harness;
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};
use mdview_render::Preview;
use mdview_theme::{Theme, ThemeMode, apply};

const SOURCE: &str = r#"# Split view

The editor and the preview share the same document. Scroll synchronization uses the
block map rather than a global percentage.

## A tall section

| Key | Value |
|-----|-------|
| one | 1 |
| two | 2 |

```rust
fn main() {
    println!("hello");
}
```

- item one
- item two
- item three

> A quotation that wraps across a couple of lines so the preview has several blocks.

The end.
"#;

fn editor() -> Editor {
    let document = Document::from_loaded(
        PathBuf::from("split.md"),
        SOURCE.to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    Editor::new(document)
}

#[test]
fn horizontal_split_snapshot() {
    let theme = Theme::for_mode(ThemeMode::Light);
    let mut preview = Preview::default();
    let mut editor = editor();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(1100.0, 700.0))
        .with_context_setup(|ctx| apply(ctx, ThemeMode::Light))
        .build_ui(move |ui| {
            egui::Panel::right("split_preview").default_size(520.0).show(ui, |ui| {
                preview.show(ui, SOURCE, &theme, true);
            });
            editor.show(ui);
        });
    harness.run();
    harness.snapshot("split-horizontal");
}

#[test]
fn vertical_split_snapshot() {
    let theme = Theme::for_mode(ThemeMode::Dark);
    let mut preview = Preview::default();
    let mut editor = editor();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 820.0))
        .with_context_setup(|ctx| apply(ctx, ThemeMode::Dark))
        .build_ui(move |ui| {
            egui::Panel::bottom("split_preview_vertical")
                .default_size(360.0)
                .show(ui, |ui| {
                    preview.show(ui, SOURCE, &theme, false);
                });
            editor.show(ui);
        });
    harness.run();
    harness.snapshot("split-vertical");
}
