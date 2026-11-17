//! The editable document model.

use std::fmt;
use std::path::{Path, PathBuf};

use ropey::Rope;

use crate::encoding::Encoding;
pub use crate::eol::Eol;
use crate::undo::{EditKind, UndoStack};

/// An open text document.
#[derive(Debug)]
pub struct Document {
    rope: Rope,
    path: Option<PathBuf>,
    dirty: bool,
    eol: Eol,
    mixed_endings: Vec<Eol>,
    encoding: Encoding,
    bom: bool,
    revision: u64,
    undo: UndoStack,
    /// The earliest character changed since the last [`Document::take_changed_line`].
    changed_from: Option<usize>,
    /// The character index just after the most recent edit, undo, or redo.
    last_edit_end: usize,
    /// Line-level edits since revision `journal_base`, one per revision.
    journal: Vec<LineEdit>,
    journal_base: u64,
}

/// How one edit changed the document's lines: from `line`, `removed_lines` line breaks were
/// replaced by `inserted_lines`. Lines before `line` and after the edit are unchanged, so
/// per-line caches can be updated without rescanning the document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LineEdit {
    /// The first line the edit touched.
    pub line: usize,
    /// Line breaks removed by the edit.
    pub removed_lines: usize,
    /// Line breaks inserted by the edit.
    pub inserted_lines: usize,
}

/// How many edits the journal keeps; older consumers rebuild from scratch.
const JOURNAL_LIMIT: usize = 256;

fn breaks(text: &str) -> usize {
    text.bytes().filter(|byte| *byte == b'\n').count()
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
            mixed_endings: Vec::new(),
            encoding: Encoding::Utf8,
            bom: false,
            revision: 0,
            undo: UndoStack::default(),
            changed_from: None,
            last_edit_end: 0,
            journal: Vec::new(),
            journal_base: 0,
        }
    }

    /// Build a document from decoded text.
    ///
    /// `text` must be normalized to LF; `file_ops` owns that conversion.
    pub fn from_loaded(path: PathBuf, text: String, eol: Eol, encoding: Encoding, bom: bool) -> Self {
        Self::from_loaded_with_endings(path, text, eol, encoding, bom, Vec::new())
    }

    /// Build a document with per-line endings for a mixed-line-ending file.
    pub fn from_loaded_with_endings(
        path: PathBuf,
        text: String,
        eol: Eol,
        encoding: Encoding,
        bom: bool,
        endings: Vec<Eol>,
    ) -> Self {
        Self {
            rope: Rope::from_str(&text),
            path: Some(path),
            dirty: false,
            eol,
            mixed_endings: if eol == Eol::Mixed { endings } else { Vec::new() },
            encoding,
            bom,
            revision: 0,
            undo: UndoStack::default(),
            changed_from: None,
            last_edit_end: 0,
            journal: Vec::new(),
            journal_base: 0,
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
        if eol != Eol::Mixed {
            self.mixed_endings.clear();
        }
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
        self.record(Some(LineEdit {
            line: 0,
            removed_lines: 0,
            inserted_lines: 0,
        }));
    }

    /// Per-line endings for a mixed file.
    pub fn mixed_endings(&self) -> &[Eol] {
        &self.mixed_endings
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
        // Clamp to the end of the line's text: a column past the end must not land
        // after the newline, which is the start of the next line.
        let line = line.min(self.line_count().saturating_sub(1));
        let start = self.rope.line_to_char(line);
        (start + column.min(self.line_len(line))).min(self.rope.len_chars())
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
        let line = self.rope.char_to_line(start);
        self.rope.insert(start, text);
        self.undo.inserted(start, text, EditKind::Typing);
        self.after_edit(start, start + text.chars().count());
        self.record(Some(LineEdit {
            line,
            removed_lines: 0,
            inserted_lines: breaks(text),
        }));
    }

    /// Delete a character range.
    pub fn delete_range(&mut self, start: usize, end: usize) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        if start == end {
            return;
        }
        let removed = self.rope.slice(start..end).to_string();
        let line = self.rope.char_to_line(start);
        self.rope.remove(start..end);
        self.undo.deleted(start, &removed);
        self.after_edit(start, start);
        self.record(Some(LineEdit {
            line,
            removed_lines: breaks(&removed),
            inserted_lines: 0,
        }));
    }

    /// Replace a character range.
    pub fn replace_range(&mut self, start: usize, end: usize, text: &str) {
        let start = start.min(self.rope.len_chars());
        let end = end.min(self.rope.len_chars()).max(start);
        if start == end && text.is_empty() {
            return;
        }
        let removed = self.rope.slice(start..end).to_string();
        let line = self.rope.char_to_line(start);
        self.rope.remove(start..end);
        self.rope.insert(start, text);
        self.undo.replaced(start, &removed, text);
        self.after_edit(start, start + text.chars().count());
        self.record(Some(LineEdit {
            line,
            removed_lines: breaks(&removed),
            inserted_lines: breaks(text),
        }));
    }

    /// Replace several character ranges, each with its own text, as one undo transaction.
    ///
    /// Ranges must not overlap; they are applied from the end so earlier offsets stay valid.
    pub fn replace_ranges(&mut self, edits: &[(usize, usize, String)]) {
        if edits.is_empty() {
            return;
        }
        let before = self.rope.to_string();
        let first = edits.iter().map(|edit| edit.0).min().unwrap_or(0);
        let mut sorted: Vec<&(usize, usize, String)> = edits.iter().collect();
        sorted.sort_by_key(|edit| edit.0);
        for (start, end, replacement) in sorted.into_iter().rev() {
            let start = (*start).min(self.rope.len_chars());
            let end = (*end).min(self.rope.len_chars()).max(start);
            self.rope.remove(start..end);
            self.rope.insert(start, replacement);
        }
        let after = self.rope.to_string();
        self.undo.replaced(0, &before, &after);
        self.after_edit(first.min(self.rope.len_chars()), first.min(self.rope.len_chars()));
        self.record(None);
    }

    /// Undo one edit.
    pub fn undo(&mut self) -> bool {
        let Some((start, end)) = self.undo.undo(&mut self.rope) else {
            return false;
        };
        self.after_edit(start, end);
        self.record(None);
        true
    }

    /// Redo one edit.
    pub fn redo(&mut self) -> bool {
        let Some((start, end)) = self.undo.redo(&mut self.rope) else {
            return false;
        };
        self.after_edit(start, end);
        self.record(None);
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
        self.mixed_endings.clear();
        self.encoding = encoding;
        self.bom = bom;
        self.dirty = false;
        self.revision = self.revision.wrapping_add(1);
        self.undo.clear();
        self.changed_from = Some(0);
        self.last_edit_end = 0;
        self.record(None);
    }

    /// The first line changed since the previous call, if any text changed.
    ///
    /// Caches keyed by line (syntax highlighting) invalidate from here. The line is
    /// resolved against the current text: everything before the earliest changed
    /// character is unchanged, so its line number is still valid.
    pub fn take_changed_line(&mut self) -> Option<usize> {
        let index = self.changed_from.take()?;
        Some(self.rope.char_to_line(index.min(self.rope.len_chars())))
    }

    /// The line edits made after `revision`, oldest first, or `None` when they are not
    /// known (too old, or an edit that replaced text in several places), so a per-line cache
    /// must be rebuilt.
    pub fn line_edits_since(&self, revision: u64) -> Option<&[LineEdit]> {
        let behind = self.revision.wrapping_sub(revision);
        let kept = self.revision.wrapping_sub(self.journal_base);
        if behind > kept || kept as usize != self.journal.len() {
            return None;
        }
        Some(&self.journal[self.journal.len() - behind as usize..])
    }

    /// Apply `measure` to the characters of every line (without its line break), in order.
    ///
    /// Walking the rope once is much faster than looking each line up by index.
    pub fn map_lines<T>(&self, mut measure: impl FnMut(&mut dyn Iterator<Item = char>) -> T) -> Vec<T> {
        let mut out = Vec::with_capacity(self.line_count());
        for line in self.rope.lines() {
            let mut chars = line.chars().filter(|character| !matches!(character, '\n' | '\r'));
            out.push(measure(&mut chars));
        }
        if out.is_empty() {
            out.push(measure(&mut std::iter::empty()));
        }
        out
    }

    fn record(&mut self, edit: Option<LineEdit>) {
        match edit {
            Some(edit) => {
                self.journal.push(edit);
                if self.journal.len() > JOURNAL_LIMIT {
                    self.journal.remove(0);
                    self.journal_base = self.journal_base.wrapping_add(1);
                }
            }
            None => {
                self.journal.clear();
                self.journal_base = self.revision;
            }
        }
    }

    /// The character index just after the most recent edit, undo, or redo.
    pub fn last_edit_end(&self) -> usize {
        self.last_edit_end.min(self.rope.len_chars())
    }

    fn after_edit(&mut self, start: usize, end: usize) {
        self.dirty = true;
        self.revision = self.revision.wrapping_add(1);
        self.changed_from = Some(self.changed_from.map_or(start, |changed| changed.min(start)));
        self.last_edit_end = end;
    }
}

impl fmt::Display for Document {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.rope)
    }
}

#[cfg(test)]
#[path = "document_tests.rs"]
mod tests;
