//! File loading, encoding detection, and atomic saving.

use std::fs;
use std::io::{self, Read};
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use chardetng::{EncodingDetector, Iso2022JpDetection, Utf8Detection};
use encoding_rs::{UTF_16BE, UTF_16LE};
use mdview_model::{Document, Encoding, Eol};
use thiserror::Error;

/// The largest file the 0.2 editor accepts.
pub const MAX_FILE_BYTES: u64 = 100 * 1024 * 1024;

/// A file operation failure.
#[derive(Debug, Error)]
pub enum FileError {
    /// The file could not be read or written.
    #[error("{0}")]
    Io(#[from] io::Error),
    /// The file contains a NUL byte and looks binary.
    #[error("the file looks binary")]
    Binary,
    /// The file is larger than the editor's hard limit.
    #[error("the file is larger than the 100 MB limit")]
    TooLarge,
    /// The selected encoding cannot represent the text.
    #[error("the text cannot be represented in {0}")]
    Encoding(&'static str),
    /// The background load was cancelled.
    #[error("the load was cancelled")]
    Cancelled,
}

/// Load a text document and detect its encoding and line endings.
pub fn load(path: &Path) -> Result<Document, FileError> {
    decode_bytes(read_limited(path)?, path, None)
}

/// Load a file in chunks, reporting progress and checking a cancellation flag.
pub fn load_with_progress(
    path: &Path,
    cancel: &AtomicBool,
    mut progress: impl FnMut(u64, u64),
) -> Result<Document, FileError> {
    let total = fs::metadata(path)?.len();
    if total > MAX_FILE_BYTES {
        return Err(FileError::TooLarge);
    }
    let mut file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity(usize::try_from(total).unwrap_or(0));
    let mut buffer = [0u8; 1024 * 1024];
    loop {
        if cancel.load(Ordering::Relaxed) {
            return Err(FileError::Cancelled);
        }
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        bytes.extend_from_slice(&buffer[..read]);
        progress(bytes.len() as u64, total);
    }
    if cancel.load(Ordering::Relaxed) {
        return Err(FileError::Cancelled);
    }
    decode_bytes(bytes, path, None)
}

fn decode_bytes(bytes: Vec<u8>, path: &Path, forced: Option<Encoding>) -> Result<Document, FileError> {
    if bytes.contains(&0) && !has_utf16_bom(&bytes) {
        return Err(FileError::Binary);
    }
    decode(bytes, path, forced)
}

/// Reload a file using a user-selected encoding.
pub fn load_with_encoding(path: &Path, encoding: Encoding) -> Result<Document, FileError> {
    decode(read_limited(path)?, path, Some(encoding))
}

fn read_limited(path: &Path) -> Result<Vec<u8>, FileError> {
    let length = fs::metadata(path)?.len();
    if length > MAX_FILE_BYTES {
        return Err(FileError::TooLarge);
    }
    Ok(fs::read(path)?)
}

fn decode(bytes: Vec<u8>, path: &Path, forced: Option<Encoding>) -> Result<Document, FileError> {
    let (encoding, bom, body) = detect(&bytes, forced);
    let text = if encoding == Encoding::Utf8 {
        let slice = if bom { &body[3..] } else { &body };
        String::from_utf8(slice.to_vec()).map_err(|_| FileError::Encoding("UTF-8"))?
    } else {
        let slice = if bom { &body[bom_len(encoding)..] } else { &body };
        let (decoded, had_errors) = encoding.as_encoding_rs().decode_without_bom_handling(slice);
        if had_errors {
            return Err(FileError::Encoding(encoding.label()));
        }
        decoded.into_owned()
    };

    let eol = Eol::detect(&text);
    let endings = Eol::detect_endings(&text);
    let normalized = eol.normalize(&text);
    Ok(Document::from_loaded_with_endings(
        path.to_path_buf(),
        normalized,
        eol,
        encoding,
        bom,
        endings,
    ))
}

fn detect(bytes: &[u8], forced: Option<Encoding>) -> (Encoding, bool, Vec<u8>) {
    if let Some(encoding) = forced {
        let bom = has_bom(bytes, encoding);
        return (encoding, bom, bytes.to_vec());
    }
    if bytes.starts_with(&[0xef, 0xbb, 0xbf]) {
        return (Encoding::Utf8, true, bytes.to_vec());
    }
    if bytes.starts_with(&[0xff, 0xfe]) {
        return (Encoding::Utf16Le, true, bytes.to_vec());
    }
    if bytes.starts_with(&[0xfe, 0xff]) {
        return (Encoding::Utf16Be, true, bytes.to_vec());
    }
    if std::str::from_utf8(bytes).is_ok() {
        return (Encoding::Utf8, false, bytes.to_vec());
    }

    let mut detector = EncodingDetector::new(Iso2022JpDetection::Deny);
    detector.feed(bytes, true);
    let detected = detector.guess(None, Utf8Detection::Deny);
    let encoding = if detected == UTF_16LE {
        Encoding::Utf16Le
    } else if detected == UTF_16BE {
        Encoding::Utf16Be
    } else if detected == encoding_rs::SHIFT_JIS {
        Encoding::ShiftJis
    } else {
        Encoding::Windows1252
    };
    (encoding, false, bytes.to_vec())
}

fn has_utf16_bom(bytes: &[u8]) -> bool {
    bytes.starts_with(&[0xff, 0xfe]) || bytes.starts_with(&[0xfe, 0xff])
}

fn has_bom(bytes: &[u8], encoding: Encoding) -> bool {
    match encoding {
        Encoding::Utf8 => bytes.starts_with(&[0xef, 0xbb, 0xbf]),
        Encoding::Utf16Le => bytes.starts_with(&[0xff, 0xfe]),
        Encoding::Utf16Be => bytes.starts_with(&[0xfe, 0xff]),
        _ => false,
    }
}

fn bom_len(encoding: Encoding) -> usize {
    match encoding {
        Encoding::Utf8 => 3,
        Encoding::Utf16Le | Encoding::Utf16Be => 2,
        _ => 0,
    }
}

/// Save a document atomically, preserving its encoding, BOM, and line endings.
pub fn save(document: &Document, path: &Path) -> Result<(), FileError> {
    let text = if document.eol() == Eol::Mixed && !document.mixed_endings().is_empty() {
        apply_mixed_endings(&document.snapshot(), document.mixed_endings())
    } else {
        document.eol().apply(&document.snapshot())
    };
    let bytes = encode(&text, document.encoding(), document.has_bom())?;
    craft_atomic::atomic_write(path, &bytes)?;
    Ok(())
}

fn apply_mixed_endings(text: &str, endings: &[Eol]) -> String {
    let parts: Vec<&str> = text.split('\n').collect();
    let mut out = String::with_capacity(text.len() + parts.len());
    for (index, part) in parts.iter().enumerate() {
        out.push_str(part);
        if index + 1 < parts.len() {
            let ending = endings
                .get(index)
                .or_else(|| endings.last())
                .copied()
                .unwrap_or(Eol::Lf);
            match ending {
                Eol::Crlf => out.push_str("\r\n"),
                Eol::Cr => out.push('\r'),
                Eol::Lf | Eol::Mixed => out.push('\n'),
            }
        }
    }
    out
}

fn encode(text: &str, encoding: Encoding, bom: bool) -> Result<Vec<u8>, FileError> {
    match encoding {
        Encoding::Utf8 => {
            let mut bytes = Vec::with_capacity(text.len() + 3);
            if bom {
                bytes.extend_from_slice(&[0xef, 0xbb, 0xbf]);
            }
            bytes.extend_from_slice(text.as_bytes());
            Ok(bytes)
        }
        Encoding::Utf16Le | Encoding::Utf16Be => {
            let mut bytes = Vec::with_capacity(text.len() * 2 + 2);
            if bom {
                bytes.extend_from_slice(if encoding == Encoding::Utf16Le {
                    &[0xff, 0xfe]
                } else {
                    &[0xfe, 0xff]
                });
            }
            for unit in text.encode_utf16() {
                let pair = if encoding == Encoding::Utf16Le {
                    unit.to_le_bytes()
                } else {
                    unit.to_be_bytes()
                };
                bytes.extend_from_slice(&pair);
            }
            Ok(bytes)
        }
        _ => {
            let (encoded, _, had_errors) = encoding.as_encoding_rs().encode(text);
            if had_errors {
                return Err(FileError::Encoding(encoding.label()));
            }
            Ok(encoded.into_owned())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trips_crlf_bom() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("note.md");
        let original = b"\xef\xbb\xbfhello\r\nworld\r\n";
        fs::write(&path, original).expect("write fixture");

        let document = load(&path).expect("load");
        assert_eq!(document.snapshot(), "hello\nworld\n");
        assert_eq!(document.eol(), Eol::Crlf);
        assert!(document.has_bom());

        save(&document, &path).expect("save");
        assert_eq!(fs::read(&path).expect("read"), original);
    }

    #[test]
    fn detects_utf16_and_preserves_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("utf16.txt");
        let original = b"\xff\xfeh\0i\0\n\0";
        fs::write(&path, original).expect("write fixture");

        let document = load(&path).expect("load");
        assert_eq!(document.snapshot(), "hi\n");
        assert_eq!(document.encoding(), Encoding::Utf16Le);
        save(&document, &path).expect("save");
        assert_eq!(fs::read(&path).expect("read"), original);
    }

    #[test]
    fn detects_windows_1252() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("latin.txt");
        fs::write(&path, [0x63, 0x61, 0x66, 0xe9]).expect("write fixture");
        let document = load(&path).expect("load");
        assert_eq!(document.encoding(), Encoding::Windows1252);
        assert!(document.snapshot().contains('é'));
    }

    #[test]
    fn rejects_binary_and_oversized_files() {
        let dir = tempfile::tempdir().expect("temp dir");
        let binary = dir.path().join("binary.bin");
        fs::write(&binary, b"a\0b").expect("write binary");
        assert!(matches!(load(&binary), Err(FileError::Binary)));

        let invalid = dir.path().join("invalid.txt");
        fs::write(&invalid, [0xff, 0xfe, 0xfd]).expect("write invalid");
        // The UTF-16 BOM path finds odd-length data and reports an encoding failure.
        assert!(matches!(load(&invalid), Err(FileError::Encoding(_))));

        let huge = dir.path().join("huge.txt");
        let file = fs::File::create(&huge).expect("create huge");
        file.set_len(MAX_FILE_BYTES + 1).expect("set huge length");
        assert!(matches!(load(&huge), Err(FileError::TooLarge)));
    }

    #[test]
    fn round_trips_mixed_line_endings() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("mixed.md");
        let original = b"one\r\ntwo\nthree\rfour";
        fs::write(&path, original).expect("write fixture");
        let document = load(&path).expect("load");
        assert_eq!(document.eol(), Eol::Mixed);
        save(&document, &path).expect("save");
        assert_eq!(fs::read(&path).expect("read"), original);
    }

    #[test]
    fn background_load_reports_progress_and_cancels() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("large.txt");
        fs::write(&path, "a".repeat(2 * 1024 * 1024)).expect("write fixture");

        let cancel = std::sync::atomic::AtomicBool::new(false);
        let mut progress_calls = 0;
        let document = load_with_progress(&path, &cancel, |_, _| progress_calls += 1).expect("load");
        assert_eq!(document.snapshot().len(), 2 * 1024 * 1024);
        assert!(progress_calls >= 2);

        cancel.store(true, std::sync::atomic::Ordering::Relaxed);
        assert!(matches!(
            load_with_progress(&path, &cancel, |_, _| {}),
            Err(FileError::Cancelled)
        ));
    }

    #[test]
    fn save_reports_missing_parent() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join("missing").join("note.md");
        let document = Document::untitled();
        assert!(matches!(save(&document, &path), Err(FileError::Io(_))));
    }
}
