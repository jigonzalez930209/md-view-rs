//! Editor unit tests.

use std::path::PathBuf;

use mdview_model::{Document, Encoding, Eol, Indentation};

use super::{Editor, display_col};

fn editor(text: &str) -> Editor {
    Editor::new(Document::from_loaded(
        PathBuf::from("fixture.txt"),
        text.to_owned(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    ))
}

#[test]
fn ime_commit_inserts_multibyte_text() {
    let mut editor = editor("");
    editor.apply_ime_commit("こんにちは");
    assert_eq!(editor.document().snapshot(), "こんにちは");
    assert_eq!(editor.document().line_col(editor.cursor).1, 5);
}

#[test]
fn large_markdown_disables_expensive_highlighting() {
    let text = (0..3_000)
        .map(|index| format!("paragraph {index}\n"))
        .collect::<String>();
    let large = editor(&text);
    assert!(!large.highlight_enabled);
    let small = editor("fn main() {}\n");
    assert!(small.highlight_enabled);
}

#[test]
fn detects_indentation_in_the_editor() {
    let editor = editor("  one\n  two\n");
    assert_eq!(
        editor.indentation,
        Indentation {
            use_tabs: false,
            width: 2
        }
    );
}

#[test]
fn display_columns_expand_tabs() {
    assert_eq!(display_col("\tab", 1), 4);
    assert_eq!(display_col("\tab", 2), 5);
    assert_eq!(display_col("abc", 2), 2);
}

#[test]
fn matching_brackets_are_found_in_both_directions() {
    let mut editor = editor("fn main() { let x = [1, 2]; }\n");
    let open = editor.document().snapshot().find('(').unwrap_or(0);
    editor.cursor = open;
    editor.ensure_brackets();
    let (start, end) = editor.bracket_match.expect("matching pair");
    assert_eq!(start, open);
    assert_eq!(editor.document().slice(start, end + 1), "()");

    editor.cursor = end;
    editor.ensure_brackets();
    assert_eq!(editor.bracket_match, Some((start, end)));
}

#[test]
fn latex_block_is_inserted_on_its_own_lines() {
    let mut editor = editor("alpha\nbeta\n");
    editor.move_cursor_to(5);
    editor.insert_latex_block();
    assert_eq!(editor.document().snapshot(), "alpha\n$$\n\n$$\nbeta\n");
    assert_eq!(editor.document().line_col(editor.cursor()), (2, 0));
}

#[test]
fn latex_block_pushes_existing_text_to_a_new_line() {
    let mut editor = editor("alpha\nbeta\n");
    editor.move_cursor_to(6);
    editor.insert_latex_block();
    assert_eq!(editor.document().snapshot(), "alpha\n$$\n\n$$\nbeta\n");
    assert_eq!(editor.document().line_col(editor.cursor()), (2, 0));
}
