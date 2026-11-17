//! Unit tests for [`super::Document`].

use super::*;

#[test]
fn detects_line_endings_including_mixed() {
    assert_eq!(Eol::detect("a\nb\n"), Eol::Lf);
    assert_eq!(Eol::detect("a\r\nb\r\n"), Eol::Crlf);
    assert_eq!(Eol::detect("a\rb\r"), Eol::Cr);
    assert_eq!(Eol::detect("a\r\nb\n"), Eol::Mixed);
    assert_eq!(Eol::Crlf.apply("a\nb"), "a\r\nb");
    assert_eq!(Eol::Cr.apply("a\nb"), "a\rb");
}

#[test]
fn edits_increment_revision_and_dirty() {
    let mut doc = Document::untitled();
    doc.insert_text(0, "hello");
    assert_eq!(doc.snapshot(), "hello");
    assert!(doc.is_dirty());
    assert_eq!(doc.line_count(), 1);
    assert_eq!(doc.line_col(5), (0, 5));
    assert_eq!(doc.line_start_char(0), 0);
}

#[test]
fn undo_and_redo_restore_text() {
    let mut doc = Document::untitled();
    doc.insert_text(0, "hello");
    doc.insert_text(5, " world");
    doc.delete_range(5, 6);
    assert_eq!(doc.snapshot(), "helloworld");
    assert!(doc.undo());
    assert_eq!(doc.snapshot(), "hello world");
    assert!(doc.undo());
    assert_eq!(doc.snapshot(), "hello");
    assert!(doc.redo());
    assert_eq!(doc.snapshot(), "hello world");
}

#[test]
fn line_helpers_handle_trailing_newline() {
    let doc = Document::from_loaded(
        PathBuf::from("/tmp/x.md"),
        "one\ntwo\n".into(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    assert_eq!(doc.line_count(), 3);
    assert_eq!(doc.line(0), "one");
    assert_eq!(doc.line(1), "two");
    assert_eq!(doc.line(2), "");
    assert_eq!(doc.char_index(1, 2), 6);
    assert_eq!(doc.slice(4, 7), "two");
}

#[test]
fn char_index_never_crosses_into_the_next_line() {
    let doc = Document::from_loaded(
        PathBuf::from("/tmp/x.rs"),
        "a long line\nab\nlast".into(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    // Column 9 on the two-character line stays at the end of that line.
    assert_eq!(doc.char_index(1, 9), 14);
    assert_eq!(doc.line_col(doc.char_index(1, 9)), (1, 2));
    assert_eq!(doc.char_index(2, 99), doc.len_chars());
}

#[test]
fn changed_line_is_the_earliest_edit_since_the_last_take() {
    let mut doc = Document::from_loaded(
        PathBuf::from("/tmp/x.rs"),
        "zero\none\ntwo\nthree\n".into(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    assert_eq!(doc.take_changed_line(), None);
    // Enter in the middle of line 1 changes line 1, although the cursor ends on line 2.
    doc.insert_text(6, "\n");
    doc.insert_text(16, "x");
    assert_eq!(doc.take_changed_line(), Some(1));
    assert_eq!(doc.take_changed_line(), None);
    assert!(doc.undo());
    assert_eq!(doc.take_changed_line(), Some(4));
}

#[test]
fn undo_and_redo_report_where_the_text_changed() {
    let mut doc = Document::untitled();
    doc.insert_text(0, "hello world");
    doc.break_undo_group();
    doc.delete_range(5, 11);
    assert!(doc.undo());
    assert_eq!(doc.last_edit_end(), 11);
    assert!(doc.redo());
    assert_eq!(doc.last_edit_end(), 5);
}

#[test]
fn line_edits_describe_each_edit_until_a_full_change() {
    let mut doc = Document::from_loaded(
        PathBuf::from("/tmp/x.txt"),
        "a\nb\nc\n".into(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    let start = doc.revision();
    assert_eq!(doc.line_edits_since(start), Some(&[][..]));
    doc.insert_text(2, "x\ny\n");
    doc.delete_range(0, 2);
    let edits = doc.line_edits_since(start).expect("journal");
    assert_eq!(
        edits,
        &[
            crate::LineEdit {
                line: 1,
                removed_lines: 0,
                inserted_lines: 2
            },
            crate::LineEdit {
                line: 0,
                removed_lines: 1,
                inserted_lines: 0
            },
        ]
    );
    let middle = doc.revision();
    assert!(doc.undo());
    assert_eq!(doc.line_edits_since(middle), None, "undo invalidates line caches");
    assert_eq!(doc.line_edits_since(doc.revision()), Some(&[][..]));
}

#[test]
fn map_lines_visits_every_line_without_breaks() {
    let doc = Document::from_loaded(
        PathBuf::from("/tmp/x.txt"),
        "ab\r\nc\n".into(),
        Eol::Lf,
        Encoding::Utf8,
        false,
    );
    assert_eq!(doc.map_lines(|chars| chars.count()), vec![2, 1, 0]);
    assert_eq!(Document::untitled().map_lines(|chars| chars.count()), vec![0]);
}
