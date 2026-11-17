//! Frame tests for mouse selection, the clipboard, find options, large files, and brackets.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use egui::{Event, Key, Modifiers, PointerButton, Pos2};
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

/// Screen position of a display column on a visual row, as the editor paints it.
fn point(editor: &Shared, row: usize, display: usize) -> Pos2 {
    let editor = editor.borrow();
    let rect = editor.viewport_rect();
    let char_width = editor.char_advance_cache.map_or(8.0, |(_, width)| width);
    let digits = editor.document.line_count().to_string().len().max(3);
    let gutter = digits as f32 * char_width + 18.0;
    let row_height = editor.last_row_height;
    Pos2::new(
        rect.left() + gutter + 8.0 + display as f32 * char_width + 1.0,
        rect.top() - editor.scroll_offset() + (row as f32 + 0.5) * row_height,
    )
}

fn button(harness: &mut Harness<'static>, at: Pos2, pressed: bool) {
    harness.event(Event::PointerButton {
        pos: at,
        button: PointerButton::Primary,
        pressed,
        modifiers: Modifiers::NONE,
    });
}

fn click(harness: &mut Harness<'static>, at: Pos2) {
    harness.event(Event::PointerMoved(at));
    button(harness, at, true);
    button(harness, at, false);
    harness.run();
}

fn selected(editor: &Shared) -> String {
    let editor = editor.borrow();
    let (start, end) = editor.selection();
    editor.document.slice(start, end)
}

fn text(editor: &Shared) -> String {
    editor.borrow().document.snapshot()
}

#[test]
fn double_click_selects_a_word_and_triple_click_a_line() {
    let (mut harness, editor) = setup("main.rs", "let alpha_beta = gamma(1);\nnext line\n", false);
    let at = point(&editor, 0, 7);
    click(&mut harness, at);
    click(&mut harness, at);
    assert_eq!(selected(&editor), "alpha_beta");
    click(&mut harness, at);
    assert_eq!(selected(&editor), "let alpha_beta = gamma(1);");
}

#[test]
fn dragging_selects_across_wrapped_rows() {
    let long = "word ".repeat(40);
    let (mut harness, editor) = setup("notes.md", &format!("{long}\n"), true);
    harness.run();
    let columns = editor.borrow().last_columns;
    let second_row = editor.borrow().segment_start(0, 1, columns);
    let from = point(&editor, 0, 5);
    let to = point(&editor, 1, 5);
    harness.event(Event::PointerMoved(from));
    button(&mut harness, from, true);
    harness.run();
    for step in 1..=6 {
        let t = step as f32 / 6.0;
        harness.event(Event::PointerMoved(from + (to - from) * t));
        harness.run();
    }
    button(&mut harness, to, false);
    harness.run();
    let (start, end) = editor.borrow().selection();
    assert_eq!(
        (start, end),
        (5, second_row + 5),
        "selection ends on the second visual row"
    );
    // Shift+click extends the selection.
    let shift = point(&editor, 1, 10);
    harness.event(Event::ModifiersChanged(Modifiers::SHIFT));
    click(&mut harness, shift);
    harness.event(Event::ModifiersChanged(Modifiers::NONE));
    harness.run();
    assert_eq!(editor.borrow().selection(), (5, second_row + 10));
}

#[test]
fn copy_cut_and_paste() {
    let (mut harness, editor) = setup("notes.txt", "one two three\n", false);
    editor.borrow_mut().select(4, 7);
    harness.run();
    harness.event(Event::Copy);
    harness.step();
    let copied = harness
        .output()
        .platform_output
        .commands
        .iter()
        .any(|command| matches!(command, egui::OutputCommand::CopyText(text) if text == "two"));
    assert!(copied, "Copy puts the selection on the clipboard");
    harness.event(Event::Cut);
    harness.run();
    assert_eq!(text(&editor), "one  three\n");
    harness.event(Event::Paste("2\n2".to_owned()));
    harness.run();
    assert_eq!(text(&editor), "one 2\n2 three\n");
    assert_eq!(editor.borrow().document.line_col(editor.borrow().cursor()), (1, 1));
    // Pasting over a selection replaces it, and one undo restores it.
    editor.borrow_mut().select(0, 3);
    harness.run();
    harness.event(Event::Paste("ONE".to_owned()));
    harness.run();
    assert_eq!(text(&editor), "ONE 2\n2 three\n");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(text(&editor), "one 2\n2 three\n");
}

fn find(editor: &Shared, query: &str, regex: bool, whole_word: bool, case: bool) -> Vec<String> {
    let mut editor = editor.borrow_mut();
    editor.find.query = query.to_owned();
    editor.find.options.regex = regex;
    editor.find.options.whole_word = whole_word;
    editor.find.options.case_sensitive = case;
    editor.recompute_matches();
    editor
        .find
        .matches
        .iter()
        .map(|(start, end)| editor.document.slice(*start, *end))
        .collect()
}

#[test]
fn find_options_regex_whole_word_and_case() {
    let (_harness, editor) = setup("notes.txt", "Cat cat category\nconcat cat2\n", false);
    assert_eq!(find(&editor, "cat", false, false, false).len(), 5);
    assert_eq!(find(&editor, "cat", false, true, false), vec!["Cat", "cat"]);
    assert_eq!(find(&editor, "cat", false, true, true), vec!["cat"]);
    assert_eq!(find(&editor, r"cat\d", true, false, false), vec!["cat2"]);
    // `^` matches every line start; empty matches are dropped instead of trapping Next.
    assert_eq!(find(&editor, r"^\w+", true, false, false), vec!["Cat", "concat"]);
    assert!(find(&editor, "x*", true, false, false).is_empty());
    // An invalid pattern finds nothing instead of failing.
    assert!(find(&editor, "(", true, false, false).is_empty());
    // Escaped plain text: `.` is literal outside regex mode.
    assert!(find(&editor, "c.t", false, false, false).is_empty());
}

#[test]
fn regex_replace_all_expands_groups_in_one_undo() {
    let (mut harness, editor) = setup("notes.txt", "a=1\nb=22\n", false);
    {
        let mut editor = editor.borrow_mut();
        editor.find.query = r"(\w)=(\d+)".to_owned();
        editor.find.replacement = "$2:$1".to_owned();
        editor.find.options.regex = true;
        editor.replace_all();
    }
    harness.run();
    assert_eq!(text(&editor), "1:a\n22:b\n");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(text(&editor), "a=1\nb=22\n");
}

#[test]
fn large_file_mode_edits_scrolls_and_skips_highlighting() {
    let text_in: String = (0..3_000)
        .map(|index| format!("let value_{index} = {index};\n"))
        .collect();
    let (mut harness, editor) = setup("big.rs", &text_in, true);
    editor.borrow_mut().set_large_mode(true);
    harness.run();
    assert!(!editor.borrow().status().wrap, "large mode turns soft wrap off");
    harness.key_press_modifiers(Modifiers::COMMAND, Key::End);
    harness.run();
    harness.event(Event::Text("x".to_owned()));
    harness.run();
    let editor = editor.borrow();
    assert!(editor.document.snapshot().ends_with("x"));
    assert!(editor.scroll_offset() > 1000.0, "the end of the file is shown");
    assert!(
        editor.highlight_cache.line(2_999).is_none(),
        "no highlighting in large mode"
    );
}

#[test]
fn bracket_matching_follows_the_cursor_and_edits() {
    let (mut harness, editor) = setup("main.rs", "f(a[b]{c})\n", false);
    editor.borrow_mut().move_cursor_to(1);
    harness.run();
    assert_eq!(editor.borrow().bracket_match, Some((1, 9)));
    editor.borrow_mut().move_cursor_to(5);
    harness.run();
    assert_eq!(
        editor.borrow().bracket_match,
        Some((3, 5)),
        "the bracket before the cursor"
    );
    // After an edit inside, the pair is recomputed for the new text.
    harness.event(Event::Text("z".to_owned()));
    harness.run();
    assert_eq!(text(&editor), "f(a[bz]{c})\n");
    editor.borrow_mut().move_cursor_to(1);
    harness.run();
    assert_eq!(editor.borrow().bracket_match, Some((1, 10)));
}
