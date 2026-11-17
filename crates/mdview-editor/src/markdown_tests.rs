//! Markdown formatting and list editing, driven through egui like real input.

#![allow(clippy::expect_used)]

use std::{cell::RefCell, path::PathBuf, rc::Rc};

use egui::{Event, Key, Modifiers};
use egui_kittest::Harness;
use mdview_model::{Document, Encoding, Eol};

use crate::{Editor, MarkdownAction};

type Shared = Rc<RefCell<Editor>>;

fn setup(name: &str, text: &str) -> (Harness<'static>, Shared) {
    let mut editor = Editor::new(Document::from_loaded(
        PathBuf::from(name),
        text.to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    ));
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

fn md(text: &str) -> (Harness<'static>, Shared) {
    setup("notes.md", text)
}

fn text(editor: &Shared) -> String {
    editor.borrow().document.snapshot()
}

fn selected(editor: &Shared) -> String {
    let editor = editor.borrow();
    let (start, end) = editor.selection();
    editor.document.slice(start, end)
}

fn cursor(editor: &Shared) -> (usize, usize) {
    let editor = editor.borrow();
    editor.document.line_col(editor.cursor)
}

fn place(harness: &mut Harness<'static>, editor: &Shared, line: usize, column: usize) {
    let index = editor.borrow().document.char_index(line, column);
    editor.borrow_mut().move_cursor_to(index);
    harness.run();
}

fn select(harness: &mut Harness<'static>, editor: &Shared, start: usize, end: usize) {
    editor.borrow_mut().select(start, end);
    harness.run();
}

fn act(harness: &mut Harness<'static>, editor: &Shared, action: MarkdownAction) {
    editor.borrow_mut().apply_markdown(action);
    harness.run();
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

#[test]
fn bold_wraps_the_selection_and_toggles_off() {
    let (mut harness, editor) = md("make this bold\n");
    select(&mut harness, &editor, 5, 9);
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "make **this** bold\n");
    assert_eq!(selected(&editor), "this");
    assert!(editor.borrow().markdown_state().bold);
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "make this bold\n");
    assert_eq!(selected(&editor), "this");
    // Selecting the markers too also toggles off.
    select(&mut harness, &editor, 5, 9);
    act(&mut harness, &editor, MarkdownAction::Bold);
    select(&mut harness, &editor, 5, 13);
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "make this bold\n");
}

#[test]
fn spaces_at_the_selection_edges_stay_outside_the_markers() {
    let (mut harness, editor) = md("a word here\n");
    select(&mut harness, &editor, 1, 7);
    act(&mut harness, &editor, MarkdownAction::Italic);
    assert_eq!(text(&editor), "a _word_ here\n");
}

#[test]
fn formats_combine_and_unwrap_independently() {
    let (mut harness, editor) = md("word\n");
    select(&mut harness, &editor, 0, 4);
    act(&mut harness, &editor, MarkdownAction::Bold);
    act(&mut harness, &editor, MarkdownAction::Italic);
    act(&mut harness, &editor, MarkdownAction::Underline);
    assert_eq!(text(&editor), "**_<ins>word</ins>_**\n");
    let state = editor.borrow().markdown_state();
    assert!(state.bold && state.italic && state.underline);
    // Italic sits outside underline: it comes off without touching the inner format.
    act(&mut harness, &editor, MarkdownAction::Italic);
    assert_eq!(text(&editor), "**<ins>word</ins>**\n");
    assert_eq!(selected(&editor), "word");
    act(&mut harness, &editor, MarkdownAction::Underline);
    assert_eq!(text(&editor), "**word**\n");
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "word\n");
}

#[test]
fn italic_is_not_confused_with_bold_asterisks() {
    let (mut harness, editor) = md("**word**\n");
    select(&mut harness, &editor, 2, 6);
    assert!(!editor.borrow().markdown_state().italic);
    act(&mut harness, &editor, MarkdownAction::Italic);
    assert_eq!(text(&editor), "**_word_**\n");
}

#[test]
fn without_a_selection_the_word_or_an_empty_pair_is_formatted() {
    let (mut harness, editor) = md("one two\n");
    place(&mut harness, &editor, 0, 5);
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "one **two**\n");
    assert_eq!(cursor(&editor), (0, 7), "the cursor stays on the same letter");
    place(&mut harness, &editor, 0, 11);
    type_text(&mut harness, " ");
    act(&mut harness, &editor, MarkdownAction::Italic);
    type_text(&mut harness, "x");
    assert_eq!(text(&editor), "one **two** _x_\n");
    // Toggling an empty pair removes it.
    place(&mut harness, &editor, 0, 15);
    type_text(&mut harness, " ");
    act(&mut harness, &editor, MarkdownAction::Bold);
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "one **two** _x_ \n");
}

#[test]
fn several_lines_are_formatted_line_by_line() {
    let (mut harness, editor) = md("alpha\n\nbeta\n");
    select(&mut harness, &editor, 0, 11);
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "**alpha**\n\n**beta**\n");
    act(&mut harness, &editor, MarkdownAction::Bold);
    assert_eq!(text(&editor), "alpha\n\nbeta\n");
}

#[test]
fn headings_set_switch_and_clear() {
    let (mut harness, editor) = md("Title\n");
    place(&mut harness, &editor, 0, 3);
    act(&mut harness, &editor, MarkdownAction::Heading(2));
    assert_eq!(text(&editor), "## Title\n");
    assert_eq!(cursor(&editor), (0, 6), "the cursor stays on the same letter");
    act(&mut harness, &editor, MarkdownAction::Heading(1));
    assert_eq!(text(&editor), "# Title\n");
    assert_eq!(editor.borrow().markdown_state().heading, 1);
    act(&mut harness, &editor, MarkdownAction::Heading(1));
    assert_eq!(text(&editor), "Title\n");
}

#[test]
fn quotes_toggle_on_every_selected_line() {
    let (mut harness, editor) = md("one\n\ntwo\nafter\n");
    select(&mut harness, &editor, 0, 8);
    act(&mut harness, &editor, MarkdownAction::Quote);
    assert_eq!(text(&editor), "> one\n>\n> two\nafter\n");
    act(&mut harness, &editor, MarkdownAction::Quote);
    assert_eq!(text(&editor), "one\n\ntwo\nafter\n");
}

#[test]
fn list_buttons_convert_between_list_kinds() {
    let (mut harness, editor) = md("a\nb\n  c\n");
    select(&mut harness, &editor, 0, 7);
    act(&mut harness, &editor, MarkdownAction::BulletList);
    assert_eq!(text(&editor), "- a\n- b\n  - c\n");
    act(&mut harness, &editor, MarkdownAction::NumberedList);
    assert_eq!(text(&editor), "1. a\n2. b\n  1. c\n");
    act(&mut harness, &editor, MarkdownAction::TaskList);
    assert_eq!(text(&editor), "- [ ] a\n- [ ] b\n  - [ ] c\n");
    act(&mut harness, &editor, MarkdownAction::TaskList);
    assert_eq!(text(&editor), "a\nb\n  c\n");
}

#[test]
fn a_numbered_line_continues_the_list_above() {
    let (mut harness, editor) = md("1. one\n2. two\nthree\n");
    place(&mut harness, &editor, 2, 0);
    act(&mut harness, &editor, MarkdownAction::NumberedList);
    assert_eq!(text(&editor), "1. one\n2. two\n3. three\n");
}

#[test]
fn enter_continues_a_bullet_list_and_a_second_enter_ends_it() {
    let (mut harness, editor) = md("- first\n");
    place(&mut harness, &editor, 0, 7);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- first\n- \n");
    type_text(&mut harness, "second");
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- first\n- second\n- \n");
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- first\n- second\n\n");
    assert_eq!(cursor(&editor), (2, 0));
    press(&mut harness, Key::Enter);
    assert_eq!(
        text(&editor),
        "- first\n- second\n\n\n",
        "outside the list Enter is plain"
    );
}

#[test]
fn enter_numbers_the_next_item_and_renumbers_the_rest() {
    let (mut harness, editor) = md("1. a\n2. b\n3. c\n");
    place(&mut harness, &editor, 0, 4);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "1. a\n2. \n3. b\n4. c\n");
    assert_eq!(cursor(&editor), (1, 3));
    // One undo restores the list, numbers included.
    harness.key_press_modifiers(Modifiers::COMMAND, Key::Z);
    harness.run();
    assert_eq!(text(&editor), "1. a\n2. b\n3. c\n");
}

#[test]
fn enter_starts_unchecked_tasks_and_splits_item_text() {
    let (mut harness, editor) = md("- [x] done\n* hello world\n");
    place(&mut harness, &editor, 0, 10);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- [x] done\n- [ ] \n* hello world\n");
    place(&mut harness, &editor, 2, 8);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- [x] done\n- [ ] \n* hello\n* world\n");
    assert_eq!(cursor(&editor), (3, 2));
}

#[test]
fn empty_nested_items_move_up_one_level_before_leaving_the_list() {
    let (mut harness, editor) = md("1. parent\n   - child\n");
    place(&mut harness, &editor, 1, 10);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "1. parent\n   - child\n   - \n");
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "1. parent\n   - child\n2. \n", "back to the parent list");
    assert_eq!(cursor(&editor), (2, 3));
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "1. parent\n   - child\n\n");
}

#[test]
fn tab_nests_an_empty_item_and_shift_tab_brings_it_back() {
    let (mut harness, editor) = md("1. one\n");
    place(&mut harness, &editor, 0, 6);
    press(&mut harness, Key::Enter);
    press(&mut harness, Key::Tab);
    assert_eq!(text(&editor), "1. one\n   1. \n");
    assert_eq!(cursor(&editor), (1, 6));
    type_text(&mut harness, "sub");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(text(&editor), "1. one\n2. sub\n");
    assert_eq!(cursor(&editor), (1, 6));
}

#[test]
fn enter_in_the_marker_or_outside_markdown_is_a_plain_newline() {
    let (mut harness, editor) = md("- item\n");
    place(&mut harness, &editor, 0, 0);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "\n- item\n");
    let (mut harness, editor) = setup("notes.rs", "- item\n");
    place(&mut harness, &editor, 0, 6);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- item\n\n");
}

#[test]
fn tables_are_inserted_on_their_own_lines() {
    let (mut harness, editor) = md("Intro text\n");
    place(&mut harness, &editor, 0, 10);
    act(&mut harness, &editor, MarkdownAction::Table { rows: 3, columns: 2 });
    assert_eq!(
        text(&editor),
        "Intro text\n\n| Column 1 | Column 2 |\n| -------- | -------- |\n|          |          |\n|          |          |\n"
    );
    assert_eq!(selected(&editor), "Column 1");
}

#[test]
fn links_and_code() {
    let (mut harness, editor) = md("see site and https://a.dev\n");
    select(&mut harness, &editor, 4, 8);
    act(&mut harness, &editor, MarkdownAction::Link);
    assert_eq!(text(&editor), "see [site](url) and https://a.dev\n");
    assert_eq!(selected(&editor), "url");
    type_text(&mut harness, "https://site.dev");
    assert_eq!(text(&editor), "see [site](https://site.dev) and https://a.dev\n");
    let start = text(&editor).find("https://a.dev").expect("url");
    select(&mut harness, &editor, start, start + 13);
    act(&mut harness, &editor, MarkdownAction::Link);
    assert!(text(&editor).ends_with("and [](https://a.dev)\n"));

    let (mut harness, editor) = md("let x = 1;\nlet y = 2;\n");
    select(&mut harness, &editor, 0, 21);
    act(&mut harness, &editor, MarkdownAction::Code);
    assert_eq!(text(&editor), "```\nlet x = 1;\nlet y = 2;\n```\n");
    act(&mut harness, &editor, MarkdownAction::Code);
    assert_eq!(text(&editor), "let x = 1;\nlet y = 2;\n");
    select(&mut harness, &editor, 4, 5);
    act(&mut harness, &editor, MarkdownAction::Code);
    assert_eq!(text(&editor), "let `x` = 1;\nlet y = 2;\n");
}

#[test]
fn shortcuts_format_markdown_only() {
    let (mut harness, editor) = md("word\n");
    select(&mut harness, &editor, 0, 4);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::B);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::I);
    harness.run();
    harness.key_press_modifiers(Modifiers::COMMAND, Key::U);
    harness.run();
    assert_eq!(text(&editor), "**_<ins>word</ins>_**\n");
    let (mut harness, editor) = setup("main.rs", "word\n");
    select(&mut harness, &editor, 0, 4);
    harness.key_press_modifiers(Modifiers::COMMAND, Key::B);
    harness.run();
    assert_eq!(text(&editor), "word\n");
}

#[test]
fn tab_walks_table_cells_and_adds_a_row_at_the_end() {
    let (mut harness, editor) = md("");
    act(&mut harness, &editor, MarkdownAction::Table { rows: 2, columns: 2 });
    assert_eq!(selected(&editor), "Column 1");
    type_text(&mut harness, "Name");
    press(&mut harness, Key::Tab);
    assert_eq!(selected(&editor), "Column 2", "Tab selects the next cell");
    type_text(&mut harness, "Age");
    press(&mut harness, Key::Tab);
    // The delimiter row is skipped: the cursor lands in the first body cell.
    assert_eq!(cursor(&editor), (2, 2));
    type_text(&mut harness, "Ana");
    press(&mut harness, Key::Tab);
    type_text(&mut harness, "30");
    press(&mut harness, Key::Tab);
    assert_eq!(
        text(&editor),
        "| Name | Age |\n| ---- | --- |\n| Ana  | 30  |\n|      |     |",
        "Tab aligns the columns and, in the last cell, adds a row"
    );
    assert_eq!(cursor(&editor), (3, 2));
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(selected(&editor), "30");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(selected(&editor), "Ana");
    harness.key_press_modifiers(Modifiers::SHIFT, Key::Tab);
    harness.run();
    assert_eq!(selected(&editor), "Age", "Shift+Tab skips the delimiter row backwards");
}

#[test]
fn enter_over_a_selection_in_an_item_replaces_it_and_continues() {
    let (mut harness, editor) = md("- keep this drop\n");
    select(&mut harness, &editor, 11, 16);
    press(&mut harness, Key::Enter);
    assert_eq!(text(&editor), "- keep this\n- \n");
}
