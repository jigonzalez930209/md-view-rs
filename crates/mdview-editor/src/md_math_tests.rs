//! Formula editing: locating, inserting, replacing, and opening formulas by double-click.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use egui::{Event, Modifiers, PointerButton, Pos2};
use egui_kittest::Harness;
use mdview_model::{Document, Encoding, Eol};

use crate::Editor;

fn editor(name: &str, text: &str) -> Editor {
    Editor::new(Document::from_loaded(
        PathBuf::from(name),
        text.to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    ))
}

#[test]
fn finds_formulas_only_in_markdown() {
    let source = "Energy $E = mc^2$ here";
    let markdown = editor("notes.md", source);
    let target = markdown.math_at(9).expect("formula");
    assert_eq!((target.start, target.end), (7, 17));
    assert_eq!(target.tex, "E = mc^2");
    assert!(!target.display);
    assert!(markdown.math_at(2).is_none());
    assert!(editor("notes.txt", source).math_at(9).is_none());
}

#[test]
fn replacing_a_formula_is_one_undo_step() {
    let mut editor = editor("notes.md", "a $x$ b");
    let target = editor.math_at(3).expect("formula");
    editor.replace_math(&target, "\\frac{1}{2}", false);
    assert_eq!(editor.document.snapshot(), "a $\\frac{1}{2}$ b");
    editor.document.undo();
    assert_eq!(editor.document.snapshot(), "a $x$ b");
}

#[test]
fn replacing_keeps_a_math_fence() {
    let mut editor = editor("notes.md", "```math\nx\n```\n");
    let target = editor.math_at(9).expect("fence");
    assert!(target.fence && target.display);
    editor.replace_math(&target, "y^2", true);
    assert_eq!(editor.document.snapshot(), "```math\ny^2\n```\n");
}

#[test]
fn display_formulas_are_inserted_on_their_own_lines() {
    let mut editor = editor("notes.md", "beforeafter");
    editor.move_cursor_to(6);
    editor.insert_math("x^2", true);
    assert_eq!(editor.document.snapshot(), "before\n$$\nx^2\n$$\nafter");
    let mut inline = self::editor("notes.md", "ab");
    inline.move_cursor_to(1);
    inline.insert_math("x", false);
    assert_eq!(inline.document.snapshot(), "a$x$b");
}

#[test]
fn double_click_inside_a_formula_requests_the_editor() {
    let mut editor = editor("notes.md", "$x^2 + y^2 = z^2$ and words");
    editor.set_line_numbers(false);
    let editor = Rc::new(RefCell::new(editor));
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 200.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    let position = Pos2::new(48.0, 14.0);
    harness.event(Event::PointerMoved(position));
    for _ in 0..2 {
        for pressed in [true, false] {
            harness.event(Event::PointerButton {
                pos: position,
                button: PointerButton::Primary,
                pressed,
                modifiers: Modifiers::NONE,
            });
        }
        harness.run();
    }
    let request = editor.borrow_mut().take_math_request().expect("request");
    assert_eq!(request.tex, "x^2 + y^2 = z^2");
    assert!(editor.borrow().hovered_char().is_some());
}
