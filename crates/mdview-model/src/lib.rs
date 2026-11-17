//! Document state for MD View.
//!
//! This crate is deliberately free of egui and filesystem access. It owns the rope, the
//! document path, undo history, encoding metadata, and line-ending metadata. Loading and saving
//! live in the application layer.

mod document;
mod encoding;
mod eol;
mod indentation;
mod undo;

pub use document::{Document, Eol, LineEdit};
pub use encoding::Encoding;
pub use indentation::Indentation;
pub use undo::{EditKind, UndoStack};

#[cfg(test)]
mod history_tests {
    use super::Document;

    #[test]
    fn ten_thousand_edits_undo_to_the_original() {
        let mut document = Document::untitled();
        for _ in 0..10_000 {
            document.replace_range(0, 0, "x");
        }
        assert_eq!(document.len_chars(), 10_000);
        while document.undo() {}
        assert!(document.is_empty());
    }
}
