//! Fifty-megabyte / one-hundred-thousand-line scale check.

use std::path::PathBuf;

use mdview_editor::Editor;
use mdview_model::{Document, Encoding, Eol};

#[test]
fn fifty_megabyte_hundred_thousand_line_file_enters_large_mode() {
    let line = "0123456789abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789abcdefghijklmnopqrs\n";
    let mut text = String::with_capacity(51 * 1024 * 1024);
    while text.len() < 50 * 1024 * 1024 {
        text.push_str(line);
    }
    let document = Document::from_loaded(PathBuf::from("huge.txt"), text, Eol::Lf, Encoding::Utf8, false);
    assert!(document.line_count() > 100_000);
    let editor = Editor::new(document);
    assert!(editor.status().large_mode);
    assert_eq!(editor.first_visible_line(), 0);
}
