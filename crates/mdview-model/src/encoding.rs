//! Text encodings supported by the 0.2 editor.

use encoding_rs::{Encoding as RsEncoding, UTF_8, UTF_16BE, UTF_16LE, WINDOWS_1252};

/// A text encoding handled by the editor.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Encoding {
    /// UTF-8 without a byte-order mark.
    #[default]
    Utf8,
    /// UTF-16 little endian.
    Utf16Le,
    /// UTF-16 big endian.
    Utf16Be,
    /// Windows-1252.
    Windows1252,
    /// ISO-8859-1, decoded through Windows-1252 by the WHATWG mapping.
    Iso8859_1,
    /// Shift_JIS.
    ShiftJis,
}

impl Encoding {
    /// A short label for the status bar and menus.
    pub fn label(self) -> &'static str {
        match self {
            Self::Utf8 => "UTF-8",
            Self::Utf16Le => "UTF-16 LE",
            Self::Utf16Be => "UTF-16 BE",
            Self::Windows1252 => "Windows-1252",
            Self::Iso8859_1 => "ISO-8859-1",
            Self::ShiftJis => "Shift_JIS",
        }
    }

    /// The encoding_rs encoding, or UTF-8 for the editor's default.
    pub fn as_encoding_rs(self) -> &'static RsEncoding {
        match self {
            Self::Utf8 => UTF_8,
            Self::Utf16Le => UTF_16LE,
            Self::Utf16Be => UTF_16BE,
            Self::Windows1252 | Self::Iso8859_1 => WINDOWS_1252,
            Self::ShiftJis => encoding_rs::SHIFT_JIS,
        }
    }

    /// Parse a menu/settings label.
    pub fn from_label(label: &str) -> Option<Self> {
        match label {
            "UTF-8" => Some(Self::Utf8),
            "UTF-16 LE" => Some(Self::Utf16Le),
            "UTF-16 BE" => Some(Self::Utf16Be),
            "Windows-1252" => Some(Self::Windows1252),
            "ISO-8859-1" => Some(Self::Iso8859_1),
            "Shift_JIS" => Some(Self::ShiftJis),
            _ => None,
        }
    }
}
