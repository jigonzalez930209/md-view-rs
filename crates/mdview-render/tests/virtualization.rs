//! Preview virtualization regression tests.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, rc::Rc};

use egui_kittest::Harness;
use mdview_markdown::parse;
use mdview_render::Preview;
use mdview_theme::Theme;

fn document() -> String {
    let mut source = String::from("# Large preview\n\n");
    for index in 0..800 {
        source.push_str(&format!(
            "Paragraph {index} with **bold**, a [link](#large-preview), and `code`.\n\n| a | b |\n|---|---|\n| {index} | value |\n\n"
        ));
    }
    source
}

#[test]
fn only_visible_blocks_are_painted() {
    let source = document();
    let blocks = parse(&source).blocks.len();
    assert!(blocks > 800);
    let preview = Rc::new(RefCell::new(Preview::default()));
    let handle = preview.clone();
    let theme = Theme::light();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(700.0, 500.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui, &source, &theme, true);
        });
    harness.run();
    let first = preview.borrow().painted_blocks();
    assert!(first > 0);
    assert!(first < 60, "painted {first} of {blocks} blocks");

    preview.borrow_mut().set_scroll_offset(1_000_000.0);
    harness.run();
    harness.run();
    let bottom = preview.borrow().painted_blocks();
    assert!(bottom > 0);
    assert!(bottom < 60, "painted {bottom} of {blocks} blocks");
    let map = preview.borrow().source_map().blocks.len();
    assert_eq!(map, blocks, "map has {map} of {blocks} blocks");
}
