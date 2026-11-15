//! Editor scroll-end regression: programmatic end scrolls stay clamped.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, rc::Rc};

use egui_kittest::Harness;
use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};

fn document() -> Document {
    let text = (0..2_000)
        .map(|index| format!("line {index} with some text\n"))
        .collect::<String>();
    Document::from_loaded(
        std::path::PathBuf::from("large.txt"),
        text,
        Eol::Lf,
        Encoding::Utf8,
        false,
    )
}

#[test]
fn scroll_end_is_stable() {
    let editor = Rc::new(RefCell::new(Editor::new(document())));
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 500.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    editor.borrow_mut().set_style(mdview_editor::EditorStyle {
        scroll_beyond_last_line: false,
        ..mdview_editor::EditorStyle::default()
    });
    editor.borrow_mut().scroll_to_end();
    for _ in 0..5 {
        harness.run();
    }
    let first = editor.borrow().scroll_offset();
    let max = editor.borrow().max_scroll_offset();
    assert!(max > 0.0);
    assert!(editor.borrow().is_at_end(), "{first} / {max}");
    for _ in 0..20 {
        harness.run();
    }
    assert!((editor.borrow().scroll_offset() - first).abs() < 0.5);
    assert!((first - max).abs() < 2.0);
    assert!(editor.borrow().bottom_clearance() >= 6.0);
}

#[test]
fn scroll_beyond_last_line_reaches_the_viewport_top() {
    let editor = Rc::new(RefCell::new(Editor::new(document())));
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 500.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    editor.borrow_mut().set_style(mdview_editor::EditorStyle {
        scroll_beyond_last_line: true,
        ..mdview_editor::EditorStyle::default()
    });
    editor.borrow_mut().scroll_to_end();
    for _ in 0..6 {
        harness.run();
    }
    assert!(editor.borrow().max_scroll_offset() > 0.0);
    assert!(editor.borrow().is_at_end());
    let top = editor.borrow().last_row_screen_top();
    assert!(top.abs() <= 1.0, "last row top = {top}");
    let settled = editor.borrow().scroll_offset();
    assert!(!editor.borrow_mut().scroll_to_end());
    for _ in 0..10 {
        harness.run();
    }
    assert!((editor.borrow().scroll_offset() - settled).abs() < 0.5);
}
