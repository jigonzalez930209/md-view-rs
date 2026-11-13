//! File loading and atomic saving.
//!
//! Milestone H1 supports UTF-8 text and detects the three common line-ending styles. Encoding
//! detection, mixed line endings, and streaming saves are H2 work.

use std::fs;
use std::io;
use std::path::Path;

use mdview_model::{Document, Eol};
use thiserror::Error;

/// A file operation failure.
#[derive(Debug, Error)]
pub enum FileError {
    /// The file could not be read or written.
    #[error("{0}")]
    Io(#[from] io::Error),
    /// The bytes are not valid UTF-8.
    #[error("the file is not valid UTF-8 text")]
    NotUtf8,
    /// The file contains a NUL byte and looks binary.
    #[error("the file looks binary")]
    Binary,
}

/// Load a UTF-8 text document.
pub fn load(path: &Path) -> Result<Document, FileError> {
    let bytes = fs::read(path)?;
    if bytes.contains(&0) {
        return Err(FileError::Binary);
    }
    let bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
    let body = if bom { &bytes[3..] } else { &bytes };
    let text = std::str::from_utf8(body).map_err(|_| FileError::NotUtf8)?;
    let eol = Eol::detect(text);
    let normalized = eol.normalize(text);
    Ok(Document::from_loaded(path.to_path_buf(), normalized, eol, bom))
}

/// Save a document atomically, preserving its detected BOM and line endings.
pub fn save(document: &Document, path: &Path) -> Result<(), FileError> {
    let text = document.eol().apply(document.text());
    let mut bytes = Vec::with_capacity(text.len() + 3);
    if document.has_bom() {
        bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
    }
    bytes.extend_from_slice(text.as_bytes());
    craft_atomic::atomic_write(path, &bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_crlf_and_bom() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("note.md");
        let original = b"\xef\xbb\xbfhello\r\nworld\r\n";
        fs::write(&path, original).expect("write fixture");

        let document = load(&path).expect("load");
        assert_eq!(document.text(), "hello\nworld\n");
        assert_eq!(document.eol(), Eol::Crlf);
        assert!(document.has_bom());

        save(&document, &path).expect("save");
        assert_eq!(fs::read(&path).expect("read"), original);
    }

    #[test]
    fn rejects_binary_and_invalid_utf8() {
        let dir = tempfile::tempdir().expect("temp dir");
        let binary = dir.path().join("binary.bin");
        fs::write(&binary, b"a\0b").expect("write binary");
        assert!(matches!(load(&binary), Err(FileError::Binary)));

        let invalid = dir.path().join("invalid.txt");
        fs::write(&invalid, [0xff, 0xfe, 0xfd]).expect("write invalid");
        assert!(matches!(load(&invalid), Err(FileError::NotUtf8)));
    }

    #[test]
    fn save_reports_missing_parent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("missing").join("note.md");
        let document = Document::untitled();
        assert!(matches!(save(&document, &path), Err(FileError::Io(_))));
    }
}
