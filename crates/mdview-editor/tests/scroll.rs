//! Scroll API tests for split synchronization.

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use egui_kittest::Harness;
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};

fn editor() -> Editor {
    let text = (0..200).map(|index| format!("line {index}\n")).collect::<String>();
    let document = Document::from_loaded(PathBuf::from("scroll.txt"), text, Eol::Lf, Encoding::Utf8, false);
    Editor::new(document)
}

#[test]
fn programmatic_line_scroll_moves_the_viewport() {
    let editor = Rc::new(RefCell::new(editor()));
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(720.0, 360.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    editor.borrow_mut().scroll_to_line(120, 0.0);
    harness.run();
    harness.run();
    let first = editor.borrow().first_visible_line();
    assert!((115..=125).contains(&first), "first visible line was {first}");
}

#[test]
fn line_to_row_is_monotonic() {
    let editor = editor();
    let mut previous = 0;
    for line in [0, 1, 10, 50, 199] {
        let row = editor.line_to_row(line);
        assert!(row >= previous);
        previous = row;
    }
}
