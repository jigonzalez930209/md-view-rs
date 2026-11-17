//! Line-ending styles.

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

    /// Detect every line ending in source text.
    pub fn detect_endings(text: &str) -> Vec<Self> {
        let mut endings = Vec::new();
        let mut chars = text.chars().peekable();
        while let Some(character) = chars.next() {
            match character {
                '\r' => {
                    if chars.peek() == Some(&'\n') {
                        chars.next();
                        endings.push(Self::Crlf);
                    } else {
                        endings.push(Self::Cr);
                    }
                }
                '\n' => endings.push(Self::Lf),
                _ => {}
            }
        }
        endings
    }

    /// Apply this style to LF-normalized text. Mixed is saved as LF when no per-line data exists.
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
