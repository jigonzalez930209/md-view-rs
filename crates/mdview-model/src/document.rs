//! The editable document model.

use std::fmt;
use std::path::{Path, PathBuf};

use ropey::Rope;

use crate::encoding::Encoding;
use crate::undo::{EditKind, UndoStack};

/// The line-ending style of a document.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Eol {
    /// Unix line feeds.
    #[default]
    Lf,
    /// Windows CRLF pairs.
    Crlf,
    /// Classic Mac carriage returns.
    Cr,
    /// More than one style is present.
    Mixed,
}

impl Eol {
    /// Detect the line-ending style in `text`.
    pub fn detect(text: &str) -> Self {
        let crlf = text.contains("\r\n");
        let bare_cr = text.replace("\r\n", "").contains('\r');
        let bare_lf = text.replace("\r\n", "").contains('\n');
        match (crlf, bare_cr, bare_lf) {
            (true, false, false) => Self::Crlf,
            (false, true, false) => Self::Cr,
            (false, false, true) | (false, false, false) => Self::Lf,
            _ => Self::Mixed,
        }
    }

    /// Normalize `text` to LF for the in-memory buffer.
    pub fn normalize(self, text: &str) -> String {
        text.replace("\r\n", "\n").replace('\r', "\n")
    }

    /// Apply this style to LF-normalized text. Mixed is saved as LF.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Crlf => text.replace('\n', "\r\n"),
            Self::Cr => text.replace('\n', "\r"),
            Self::Lf | Self::Mixed => text.to_owned(),
        }
    }

    /// The short label used in the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
            Self::Cr => "CR",
            Self::Mixed => "Mixed",
        }
    }
}

/// An open text document.
#[derive(Debug)]
pub struct Document {
    rope: Rope,
    path: Option<PathBuf>,
    dirty: bool,
    eol: Eol,
    encoding: Encoding,
    bom: bool,
    revision: u64,
    undo: UndoStack,
}

impl Default for Document {
    fn default() -> Self {
        Self::untitled()
    }
}

impl Document {
    /// Create an empty untitled document.
    pub fn untitled() -> Self {
        Self {
            rope: Rope::new(),
            path: None,
            dirty: false,
            eol: Eol::Lf,
            encoding: Encoding::Utf8,
            bom: false,
            revision: 0,
            undo: UndoStack::default(),
        }
    }

    /// Build a document from decoded text.
    ///
    /// `text` must be normalized to LF; `file_ops` owns that conversion.
    pub fn from_loaded(path: PathBuf, text: String, eol: Eol, encoding: Encoding, bom: bool) -> Self {
        Self {
            rope: Rope::from_str(&text),
            path: Some(path),
            dirty: false,
            eol,
            encoding,
            bom,
            revision: 0,
            undo: UndoStack::default(),
        }
    }

    /// The file path, or `None` for an untitled document.
    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }

    /// Update the path after Save As.
    pub fn set_path(&mut self, path: PathBuf) {
        self.path = Some(path);
    }

    /// A short name for the tab or window title.
    pub fn display_name(&self) -> String {
        self.path
            .as_ref()
            .and_then(|path| path.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Untitled".to_owned())
    }

    /// Whether the buffer differs from the last saved state.
    pub fn is_dirty(&self) -> bool {
        self.dirty
    }

    /// Mark the buffer as changed.
    pub fn mark_dirty(&mut self) {
        self.dirty = true;
    }

    /// Mark the buffer as saved.
    pub fn mark_saved(&mut self) {
        self.dirty = false;
    }

    /// Mark the buffer as saved with a new effective line-ending style.
    pub fn mark_saved_with_eol(&mut self, eol: Eol) {
        self.eol = eol;
        self.dirty = false;
    }

    /// The line-ending style detected when the file was loaded.
    pub fn eol(&self) -> Eol {
        self.eol
    }

    /// Change the saved line-ending style.
    pub fn set_eol(&mut self, eol: Eol) {
        self.eol = eol;
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
    }

    /// The detected encoding.
    pub fn encoding(&self) -> Encoding {
        self.encoding
    }

    /// Whether the file had a byte-order mark when loaded.
    pub fn has_bom(&self) -> bool {
        self.bom
    }

    /// A monotonically increasing revision, bumped by every edit.
    pub fn revision(&self) -> u64 {
        self.revision
    }

    /// The document text as an owned string.
    pub fn snapshot(&self) -> String {
        self.rope.to_string()
    }

    /// Whether the document is empty.
    pub fn is_empty(&self) -> bool {
        self.rope.len_chars() == 0
    }

    /// Number of characters.
    pub fn len_chars(&self) -> usize {
        self.rope.len_chars()
    }

    /// Number of bytes in the normalized text.
    pub fn len_bytes(&self) -> usize {
        self.rope.len_bytes()
    }

    /// Number of lines; an empty document has one line.
    pub fn line_count(&self) -> usize {
        self.rope.len_lines().max(1)
    }

    /// The text of one line without its newline.
    pub fn line(&self, index: usize) -> String {
        if index >= self.line_count() {
            return String::new();
        }
        let line = self.rope.line(index);
        line.to_string().trim_end_matches(['\r', '\n']).to_owned()
    }

    /// The character length of one line, excluding its newline.
    pub fn line_len(&self, index: usize) -> usize {
        if index >= self.line_count() {
            return 0;
        }
        let line = self.rope.line(index);
        let len = line.len_chars();
        if len > 0 && line.char(len - 1) == '\n' {
            len - 1
        } else {
            len
        }
    }

    /// The first character index of a line.
    pub fn line_start_char(&self, line: usize) -> usize {
        if line >= self.line_count() {
            return self.rope.len_chars();
        }
        self.rope.line_to_char(line)
    }

    /// The line and column for a character index.
    pub fn line_col(&self, char_index: usize) -> (usize, usize) {
        let index = char_index.min(self.rope.len_chars());
        let line = self.rope.char_to_line(index);
        (line, index.saturating_sub(self.rope.line_to_char(line)))
    }

    /// The character index for a line and column.
    pub fn char_index(&self, line: usize, column: usize) -> usize {
        let line = line.min(self.line_count().saturating_sub(1));
        let start = self.rope.line_to_char(line);
        let end = if line + 1 < self.rope.len_lines() {
            self.rope.line_to_char(line + 1)
        } else {
            self.rope.len_chars()
        };
        (start + column).min(end)
    }

    /// A character range as an owned string.
    pub fn slice(&self, start: usize, end: usize) -> String {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        self.rope.slice(start..end).to_string()
    }

    /// One character at a character index.
    pub fn char_at(&self, index: usize) -> Option<char> {
        if index >= self.rope.len_chars() {
            None
        } else {
            Some(self.rope.char(index))
        }
    }

    /// Convert a byte offset in [`Document::snapshot`] to a character index.
    pub fn byte_to_char(&self, byte: usize) -> usize {
        self.rope.byte_to_char(byte.min(self.rope.len_bytes()))
    }

    /// Convert a character index to a byte offset.
    pub fn char_to_byte(&self, char_index: usize) -> usize {
        self.rope.char_to_byte(char_index.min(self.rope.len_chars()))
    }

    /// Insert text at a character index.
    pub fn insert_text(&mut self, start: usize, text: &str) {
        if text.is_empty() {
            return;
        }
        let start = start.min(self.rope.len_chars());
        self.rope.insert(start, text);
        self.undo.inserted(start, text, EditKind::Typing);
        self.after_edit();
    }

    /// Delete a character range.
    pub fn delete_range(&mut self, start: usize, end: usize) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        if start == end {
            return;
        }
        let removed = self.rope.slice(start..end).to_string();
        self.rope.remove(start..end);
        self.undo.deleted(start, &removed);
        self.after_edit();
    }

    /// Replace a character range.
    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        if start == end && text.is_empty() {
            return;
        }
        let removed = self.rope.slice(start..end).to_string();
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        self.undo.replaced(start, &removed, text);
        self.after_edit();
    }

    /// Replace several character ranges as one undo transaction.
    pub fn replace_ranges(&mut self, ranges: &[(usize, usize)], replacement: &str) {
        if ranges.is_empty() {
            return;
        }
        let before = self.rope.to_string();
        for (start, end) in ranges.iter().copied().rev() {
            let start = start.min(self.rope.len_chars());
            let end = end.min(self.rope.len_chars()).max(start);
            self.rope.remove(start..end);
            self.rope.insert(start, replacement);
        }
        let after = self.rope.to_string();
        self.undo.replaced(0, &before, &after);
        self.after_edit();
    }

    /// Undo one edit.
    pub fn undo(&mut self) -> bool {
        if !self.undo.undo(&mut self.rope) {
            return false;
        }
        self.after_edit();
        true
    }

    /// Redo one edit.
    pub fn redo(&mut self) -> bool {
        if !self.undo.redo(&mut self.rope) {
            return false;
        }
        self.after_edit();
        true
    }

    /// Break the current typing group, for example after an arrow key or a mouse click.
    pub fn break_undo_group(&mut self) {
        self.undo.clear_group();
    }

    /// Whether undo is available.
    pub fn can_undo(&self) -> bool {
        self.undo.can_undo()
    }

    /// Whether redo is available.
    pub fn can_redo(&self) -> bool {
        self.undo.can_redo()
    }

    /// Replace the entire editor state after reopening a file.
    pub fn replace_content(&mut self, text: String, eol: Eol, encoding: Encoding, bom: bool) {
        self.rope = Rope::from_str(&text);
        self.eol = eol;
        self.encoding = encoding;
        self.bom = bom;
        self.dirty = false;
        self.revision = self.revision.wrapping_add(1);
        self.undo.clear();
    }

    fn after_edit(&mut self) {
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
    }
}

impl fmt::Display for Document {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.rope)
    }
}

#[cfg(test)]
mod tests {
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
}
