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

#[test]
fn sync_targets_past_the_end_are_clamped_and_not_repeated() {
    let editor = Rc::new(RefCell::new(Editor::new(document())));
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 500.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    let max = editor.borrow().max_scroll_offset();
    assert!(max > 0.0);
    // An unclamped target would be painted past the end for one frame (flicker).
    assert!(editor.borrow_mut().sync_scroll_to(max + 5_000.0));
    harness.run();
    assert!((editor.borrow().scroll_offset() - max).abs() < 0.5);
    assert!(!editor.borrow_mut().sync_scroll_to(max + 5_000.0));
    assert!(!editor.borrow_mut().sync_scroll_to(f32::NAN));
    assert!(editor.borrow_mut().sync_scroll_to(-50.0));
    harness.run();
    assert!(editor.borrow().scroll_offset().abs() < 0.5);
}

#[test]
fn line_tops_follow_soft_wrap() {
    let editor = Rc::new(RefCell::new(Editor::new(document())));
    editor.borrow_mut().set_wrap(true);
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(160.0, 500.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    let tops = editor.borrow().line_tops();
    assert_eq!(tops.len(), editor.borrow().document().line_count() + 1);
    assert!(tops.windows(2).all(|pair| pair[1] > pair[0]));
    // Narrow wrapping makes the text lines span more rows than the empty final line.
    let heights = tops.windows(2).map(|pair| pair[1] - pair[0]).collect::<Vec<_>>();
    let shortest = heights.iter().copied().fold(f32::MAX, f32::min);
    let tallest = heights.iter().copied().fold(0.0, f32::max);
    assert!(tallest >= shortest * 2.0, "{shortest} / {tallest}");
}

#[test]
fn split_mode_clamps_the_editor_to_its_content_end() {
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
    editor.borrow_mut().set_split_mode(true);
    editor.borrow_mut().scroll_to_end();
    for _ in 0..6 {
        harness.run();
    }
    let split_max = editor.borrow().max_scroll_offset();
    let split_last_top = editor.borrow().last_row_screen_top();
    assert!(split_max > 0.0);
    assert!(editor.borrow().is_at_end());
    assert!(
        split_last_top > 20.0,
        "split editor left blank space below the last row: last row top = {split_last_top}"
    );

    // The single-pane editor keeps the scroll-beyond-last-line behavior.
    editor.borrow_mut().set_split_mode(false);
    harness.run();
    editor.borrow_mut().scroll_to_end();
    for _ in 0..6 {
        harness.run();
    }
    let single_max = editor.borrow().max_scroll_offset();
    assert!(
        single_max > split_max + 20.0,
        "single max {single_max} should exceed split max {split_max}"
    );
    assert!(
        editor.borrow().last_row_screen_top().abs() <= 1.0,
        "single-pane last row did not reach the viewport top"
    );
}
