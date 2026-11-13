//! Document state for MD View.
//!
//! This crate is deliberately free of egui and filesystem access. It owns the text, the
//! document path, the dirty flag, and the line-ending metadata. Loading and saving live in the
//! application layer.

use std::path::{Path, PathBuf};

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
}

impl Eol {
    /// Detect the first line-ending style in `text`.
    ///
    /// A mixed document is reported as the style of its first line ending until milestone H2
    /// introduces full mixed-line-ending handling.
    pub fn detect(text: &str) -> Self {
        if text.contains("\r\n") {
            Self::Crlf
        } else if text.contains('\r') {
            Self::Cr
        } else {
            Self::Lf
        }
    }

    /// Normalize `text` to LF for the in-memory buffer.
    pub fn normalize(self, text: &str) -> String {
        match self {
            Self::Lf => text.to_owned(),
            Self::Crlf => text.replace("\r\n", "\n"),
            Self::Cr => text.replace('\r', "\n"),
        }
    }

    /// Apply this style to LF-normalized text.
    pub fn apply(self, text: &str) -> String {
        match self {
            Self::Lf => text.to_owned(),
            Self::Crlf => text.replace('\n', "\r\n"),
            Self::Cr => text.replace('\n', "\r"),
        }
    }

    /// The short label used in the status bar.
    pub fn label(self) -> &'static str {
        match self {
            Self::Lf => "LF",
            Self::Crlf => "CRLF",
            Self::Cr => "CR",
        }
    }
}

/// An open text document.
#[derive(Clone, Debug)]
pub struct Document {
    text: String,
    path: Option<PathBuf>,
    dirty: bool,
    eol: Eol,
    bom: bool,
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
            text: String::new(),
            path: None,
            dirty: false,
            eol: Eol::Lf,
            bom: false,
        }
    }

    /// Build a document from decoded text.
    ///
    /// `text` must be normalized to LF; `file_ops` owns that conversion.
    pub fn from_loaded(path: PathBuf, text: String, eol: Eol, bom: bool) -> Self {
        Self {
            text,
            path: Some(path),
            dirty: false,
            eol,
            bom,
        }
    }

    /// The document text, normalized to LF.
    pub fn text(&self) -> &str {
        &self.text
    }

    /// Mutable access for the editor.
    ///
    /// The caller must call [`Document::mark_dirty`] when the editor reports a change.
    pub fn text_mut(&mut self) -> &mut String {
        &mut self.text
    }

    /// Replace the text, for example after reopening a file.
    pub fn replace_text(&mut self, text: String, eol: Eol, bom: bool) {
        self.text = text;
        self.eol = eol;
        self.bom = bom;
        self.dirty = false;
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
            .and_then(|p| p.file_name())
            .map(|n| n.to_string_lossy().into_owned())
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

    /// The line-ending style detected when the file was loaded.
    pub fn eol(&self) -> Eol {
        self.eol
    }

    /// Whether the file had a UTF-8 byte-order mark when loaded.
    pub fn has_bom(&self) -> bool {
        self.bom
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_and_applies_line_endings() {
        assert_eq!(Eol::detect("a\nb\n"), Eol::Lf);
        assert_eq!(Eol::detect("a\r\nb\r\n"), Eol::Crlf);
        assert_eq!(Eol::detect("a\rb\r"), Eol::Cr);
        assert_eq!(Eol::Crlf.normalize("a\r\nb"), "a\nb");
        assert_eq!(Eol::Crlf.apply("a\nb"), "a\r\nb");
        assert_eq!(Eol::Cr.apply("a\nb"), "a\rb");
    }

    #[test]
    fn dirty_state_tracks_edits_and_saves() {
        let mut doc = Document::untitled();
        assert!(!doc.is_dirty());
        doc.text_mut().push_str("hello");
        doc.mark_dirty();
        assert!(doc.is_dirty());
        doc.mark_saved();
        assert!(!doc.is_dirty());
        assert_eq!(doc.display_name(), "Untitled");
    }

    #[test]
    fn loaded_document_keeps_path_and_metadata() {
        let doc = Document::from_loaded(PathBuf::from("/tmp/note.md"), "hello\n".into(), Eol::Crlf, true);
        assert_eq!(doc.display_name(), "note.md");
        assert_eq!(doc.eol(), Eol::Crlf);
        assert!(doc.has_bom());
        assert!(!doc.is_dirty());
    }
}
