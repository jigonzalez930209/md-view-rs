//! Frame-level typing and navigation tests: keys go through egui like real input.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use mdview_model::{Document, Encoding, Eol};

use crate::Editor;

type Shared = Rc<RefCell<Editor>>;

fn setup(name: &str, text: &str, wrap: bool) -> (Harness<'static>, Shared) {
    let mut editor = Editor::new(Document::from_loaded(
        PathBuf::from(name),
        text.to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    ));
    editor.set_wrap(wrap);
    editor.request_focus();
    let editor = Rc::new(RefCell::new(editor));
    let handle = editor.clone();
    let mut harness = Harness::builder()
        .with_size(egui::vec2(640.0, 400.0))
        .build_ui(move |ui| {
            handle.borrow_mut().show(ui);
        });
    harness.run();
    (harness, editor)
}

fn press(harness: &mut Harness<'static>, key: Key) {
    harness.key_press(key);
    harness.run();
}

fn type_text(harness: &mut Harness<'static>, text: &str) {
    for character in text.chars() {
        harness.event(Event::Text(character.to_string()));
        harness.run();
    }
}

fn numbered(lines: usize) -> String {
    (0..lines)
        .map(|index| format!("let value_{index} = {index};\n"))
        .collect()
}

fn cursor_line(editor: &Shared) -> (usize, usize) {
    let editor = editor.borrow();
    editor.document.line_col(editor.cursor)
}

/// The cursor row must lie fully inside the viewport.
fn assert_cursor_visible(editor: &Shared) {
    let editor = editor.borrow();
    let row = editor.row_for_cursor(editor.last_columns) as f32 * editor.last_row_height;
    let top = editor.scroll_offset();
    let height = editor.viewport_rect().height();
    assert!(
        row >= top - 0.5 && row + editor.last_row_height <= top + height + 0.5,
        "cursor row at {row} outside viewport {top}..{}",
        top + height
    );
}

#[test]
fn enter_mid_line_repaints_the_line_it_split() {
    let (mut harness, editor) = setup("main.rs", "fn main() { let value = 1; }\n", false);
    editor.borrow_mut().move_cursor_to(11);
    harness.run();
    press(&mut harness, Key::Enter);
    let editor = editor.borrow();
    assert_eq!(editor.document.line(0), "fn main() {");
    // The highlight cache must describe the new first line, not the line before Enter.
    let spans = editor.highlight_cache.line(0).expect("line 0 highlighted");
    let shown: String = spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(shown.trim_end_matches('\n'), "fn main() {");
}

#[test]
fn an_edit_above_the_cursor_repaints_the_visible_lines() {
    // Replace all, undo, and redo change text far from the cursor. The highlight cache
    // must refresh from the edited line, not from the cursor line below the viewport.
    let (mut harness, editor) = setup("notes.rs", &numbered(200), false);
    harness.run();
    {
        let mut editor = editor.borrow_mut();
        editor.cursor = editor.document.line_start_char(190);
        editor.anchor = editor.cursor;
        let start = editor.document.line_start_char(2);
        let end = start + editor.document.line_len(2);
        editor.document.replace_range(start, end, "let changed = 2;");
    }
    harness.run();
    let editor = editor.borrow();
    assert!(editor.scroll_offset() < 1.0, "the edit must not scroll");
    let spans = editor.highlight_cache.line(2).expect("line 2 highlighted");
    let shown: String = spans.iter().map(|span| span.text.as_str()).collect();
    assert_eq!(shown.trim_end_matches('\n'), "let changed = 2;");
}

#[test]
fn typing_inside_the_viewport_never_scrolls() {
    let (mut harness, editor) = setup("notes.rs", &numbered(300), false);
    let rows = {
        let editor = editor.borrow();
        (editor.viewport_rect().height() / editor.last_row_height).floor() as usize
    };
    // A line near the bottom edge: the old reveal centred it on every keystroke.
    let index = editor.borrow().document.line_start_char(rows - 3);
    editor.borrow_mut().cursor = index;
    editor.borrow_mut().anchor = index;
    harness.run();
    let before = editor.borrow().scroll_offset();
    assert_eq!(before, 0.0);
    type_text(&mut harness, "abc ");
    press(&mut harness, Key::Backspace);
    assert_eq!(editor.borrow().scroll_offset(), before, "typing moved the view");
}

#[test]
fn enter_at_the_bottom_scrolls_one_row_at_a_time() {
    let (mut harness, editor) = setup("notes.rs", &numbered(300), false);
    let (row_height, height) = {
        let editor = editor.borrow();
        (editor.last_row_height, editor.viewport_rect().height())
    };
    // Put the cursor on the last fully visible row.
    let last_visible = (height / row_height).floor() as usize - 2;
    let index = editor.borrow().document.line_start_char(last_visible);
    editor.borrow_mut().move_cursor_to(index);
    harness.run();
    let mut previous = editor.borrow().scroll_offset();
    for _ in 0..10 {
        press(&mut harness, Key::Enter);
        let offset = editor.borrow().scroll_offset();
        assert!(offset >= previous, "Enter scrolled backwards: {previous} -> {offset}");
        assert!(
            offset - previous <= row_height + 0.5,
            "Enter jumped {} points instead of at most one row",
            offset - previous
        );
        assert_cursor_visible(&editor);
        previous = offset;
    }
}

#[test]
fn down_from_a_long_line_stays_on_the_short_line_and_keeps_the_column() {
    let (mut harness, editor) = setup("notes.rs", "a long first line\nab\nanother long line\n", false);
    editor.borrow_mut().move_cursor_to(12);
    harness.run();
    press(&mut harness, Key::ArrowDown);
    assert_eq!(cursor_line(&editor), (1, 2));
    press(&mut harness, Key::ArrowDown);
    assert_eq!(cursor_line(&editor), (2, 12));
    press(&mut harness, Key::ArrowUp);
    press(&mut harness, Key::ArrowUp);
    assert_eq!(cursor_line(&editor), (0, 12));
}

#[test]
fn up_and_down_move_by_wrapped_rows() {
    let long = "word ".repeat(60);
    let (mut harness, editor) = setup("notes.md", &format!("{long}\nshort\n"), true);
    harness.run();
    let columns = editor.borrow().last_columns;
    assert!(columns > 8, "soft wrap must be active");
    editor.borrow_mut().move_cursor_to(3);
    harness.run();
    // Rows break after whole words, so the second row starts at a word boundary.
    let second_row = editor.borrow().segment_start(0, 1, columns);
    assert_eq!(second_row % 5, 0, "row break after `word `");
    assert!(second_row > 0 && second_row <= columns);
    press(&mut harness, Key::ArrowDown);
    assert_eq!(
        cursor_line(&editor),
        (0, second_row + 3),
        "Down moves to the next wrapped row"
    );
    press(&mut harness, Key::ArrowUp);
    assert_eq!(cursor_line(&editor), (0, 3));
}

#[test]
fn apostrophes_in_prose_are_not_paired() {
    let (mut harness, editor) = setup("notes.md", "", false);
    type_text(&mut harness, "don't (x) \"q\"");
    assert_eq!(editor.borrow().document.snapshot(), "don't (x) \"q\"");
}

#[test]
fn enter_between_braces_opens_an_indented_block() {
    let (mut harness, editor) = setup("main.rs", "fn main() \n", false);
    editor.borrow_mut().move_cursor_to(10);
    harness.run();
    type_text(&mut harness, "{");
    press(&mut harness, Key::Enter);
    assert_eq!(editor.borrow().document.snapshot(), "fn main() {\n    \n}\n");
    assert_eq!(cursor_line(&editor), (1, 4));
}

#[test]
fn enter_keeps_indentation_without_adding_lines() {
    let (mut harness, editor) = setup("notes.md", "    item\n\nend\n", false);
    editor.borrow_mut().move_cursor_to(8);
    harness.run();
    press(&mut harness, Key::Enter);
    assert_eq!(editor.borrow().document.snapshot(), "    item\n    \n\nend\n");
    assert_eq!(cursor_line(&editor), (1, 4));
    // Enter again leaves the indentation-only line empty and keeps the indentation.
    press(&mut harness, Key::Enter);
    assert_eq!(editor.borrow().document.snapshot(), "    item\n\n    \n\nend\n");
    assert_eq!(cursor_line(&editor), (2, 4));
    // One Backspace removes the indentation level, the next joins the lines.
    press(&mut harness, Key::Backspace);
    assert_eq!(editor.borrow().document.snapshot(), "    item\n\n\n\nend\n");
    press(&mut harness, Key::Backspace);
    assert_eq!(editor.borrow().document.snapshot(), "    item\n\n\nend\n");
    // Enter on an empty line inserts exactly one line.
    let index = editor.borrow().document.line_start_char(2);
    editor.borrow_mut().move_cursor_to(index);
    harness.run();
    press(&mut harness, Key::Enter);
    assert_eq!(editor.borrow().document.snapshot(), "    item\n\n\n\nend\n");
    assert_eq!(cursor_line(&editor), (3, 0));
}

#[test]
fn undo_puts_the_cursor_at_the_change() {
    let (mut harness, editor) = setup("notes.rs", &numbered(200), false);
    editor.borrow_mut().goto_line(180);
    harness.run();
    type_text(&mut harness, "xyz");
    editor.borrow_mut().goto_line(1);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(cursor_line(&editor), (179, 0));
    assert_cursor_visible(&editor);
}

#[test]
fn the_cursor_follows_a_long_line_horizontally() {
    let long = format!("{}\nshort\n", "x".repeat(400));
    let (mut harness, editor) = setup("notes.txt", &long, false);
    press(&mut harness, Key::End);
    harness.run();
    let scrolled = editor.borrow().scroll_x;
    assert!(
        scrolled > 1000.0,
        "End on a long line must scroll right, got {scrolled}"
    );
    press(&mut harness, Key::Home);
    harness.run();
    assert_eq!(editor.borrow().scroll_x, 0.0, "Home must bring the line start back");
}

#[test]
fn page_down_moves_one_viewport() {
    let (mut harness, editor) = setup("notes.rs", &numbered(300), false);
    let rows = {
        let editor = editor.borrow();
        (editor.viewport_rect().height() / editor.last_row_height).floor() as usize - 1
    };
    press(&mut harness, Key::PageDown);
    assert_eq!(cursor_line(&editor).0, rows);
    assert_cursor_visible(&editor);
}

#[test]
fn enter_after_click_and_end_splits_at_the_cursor() {
    let (mut harness, editor) = setup("main.rs", "    let next = if a && b && !word {\n    x();\n}\n", false);
    let rect = editor.borrow().viewport_rect();
    let row_height = editor.borrow().last_row_height;
    let position = egui::pos2(rect.left() + 260.0, rect.top() + row_height * 0.5);
    harness.event(Event::PointerMoved(position));
    harness.event(Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed: true,
        modifiers: Modifiers::NONE,
    });
    harness.event(Event::PointerButton {
        pos: position,
        button: egui::PointerButton::Primary,
        pressed: false,
        modifiers: Modifiers::NONE,
    });
    harness.run();
    let clicked = cursor_line(&editor);
    assert!(clicked.1 > 0 && clicked.1 < 35, "click column {clicked:?}");
    press(&mut harness, Key::End);
    assert_eq!(cursor_line(&editor), (0, 35));
    // egui reports Enter and Space as a click on the focused editor; neither may move
    // the cursor to the mouse pointer, which is still over the first line.
    press(&mut harness, Key::Enter);
    type_text(&mut harness, "a b c");
    assert_eq!(
        editor.borrow().document.snapshot(),
        "    let next = if a && b && !word {\n        a b c\n    x();\n}\n"
    );
}

#[test]
fn tab_and_shift_tab_shift_whole_lines_as_one_undo_step() {
    let (mut harness, editor) = setup("main.rs", "a\n    b\n\nc\nd\n", false);
    // Select from line 0 to the start of line 4: line 4 is not part of the block.
    let end = editor.borrow().document.line_start_char(4);
    editor.borrow_mut().select(0, end);
    harness.run();
    press(&mut harness, Key::Tab);
    assert_eq!(editor.borrow().document.snapshot(), "    a\n        b\n\n    c\nd\n");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(editor.borrow().document.snapshot(), "a\n    b\n\nc\nd\n");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(editor.borrow().document.snapshot(), "    a\n        b\n\n    c\nd\n");
    // Shift+Tab without a selection dedents the cursor line and keeps the cursor on its text.
    let index = editor.borrow().document.line_start_char(1) + 9;
    editor.borrow_mut().move_cursor_to(index);
    harness.run();
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(editor.borrow().document.line(1), "    b");
    assert_eq!(cursor_line(&editor), (1, 5));
}

#[test]
fn backspace_in_indentation_and_word_deletion() {
    let (mut harness, editor) = setup(
        "main.rs",
        "fn a() {\n    if b {\n        let value = 1;\n    }\n}\n",
        false,
    );
    let index = editor.borrow().document.line_start_char(2) + 8;
    editor.borrow_mut().move_cursor_to(index);
    harness.run();
    press(&mut harness, Key::Backspace);
    assert_eq!(editor.borrow().document.line(2), "    let value = 1;");
    press(&mut harness, Key::End);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Backspace);
    harness.run();
    assert_eq!(editor.borrow().document.line(2), "    let value = 1");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Backspace);
    harness.run();
    assert_eq!(editor.borrow().document.line(2), "    let value = ");
    press(&mut harness, Key::Home);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Delete);
    harness.run();
    assert_eq!(editor.borrow().document.line(2), "    value = ");
}

#[test]
fn ctrl_arrows_cross_punctuation() {
    let (mut harness, editor) = setup("main.rs", "call(a.b);\n", false);
    let mut stops = Vec::new();
    for _ in 0..7 {
        harness.key_press_modifiers(Modifiers::COMMAND, Key::ArrowRight);
        harness.run();
        stops.push(cursor_line(&editor));
    }
    // `call`, `(`, `a`, `.`, `b`, then `);` with the newline after it.
    assert_eq!(stops, vec![(0, 4), (0, 5), (0, 6), (0, 7), (0, 8), (1, 0), (1, 0)]);
}

#[test]
fn enter_in_find_walks_matches_and_keeps_the_document_unchanged() {
    let (mut harness, editor) = setup("notes.rs", &numbered(40), false);
    editor.borrow_mut().open_find(false);
    harness.run();
    type_text(&mut harness, "value_3");
    press(&mut harness, Key::Enter);
    let first = editor.borrow().selection();
    press(&mut harness, Key::Enter);
    let second = editor.borrow().selection();
    assert_ne!(first, second, "Enter must move to the next match");
    assert_eq!(editor.borrow().document.slice(second.0, second.1), "value_3");
    assert_eq!(
        editor.borrow().document.snapshot(),
        numbered(40),
        "Enter must not edit the text"
    );
}

#[test]
fn enter_in_replace_replaces_and_moves_on() {
    let (mut harness, editor) = setup("notes.rs", "a x\nb x\nc x\n", false);
    editor.borrow_mut().open_find(true);
    harness.run();
    type_text(&mut harness, "x");
    editor.borrow_mut().find.focus_replace = true;
    harness.run();
    type_text(&mut harness, "y");
    for _ in 0..4 {
        press(&mut harness, Key::Enter);
    }
    assert_eq!(editor.borrow().document.snapshot(), "a y\nb y\nc y\n");
}

#[test]
fn enter_in_go_to_line_jumps() {
    let (mut harness, editor) = setup("notes.rs", &numbered(300), false);
    editor.borrow_mut().open_goto();
    harness.run();
    type_text(&mut harness, "150");
    press(&mut harness, Key::Enter);
    assert!(!editor.borrow().goto.open, "the dialog closes");
    assert_eq!(cursor_line(&editor), (149, 0));
    assert_cursor_visible(&editor);
}
