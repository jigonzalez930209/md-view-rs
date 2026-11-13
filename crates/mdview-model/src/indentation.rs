//! Indentation detection.

use crate::Document;

/// The indentation convention detected from a document.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Indentation {
    /// Use tab characters for one indentation level.
    pub use_tabs: bool,
    /// Spaces per indentation level when `use_tabs` is false.
    pub width: usize,
}

impl Default for Indentation {
    fn default() -> Self {
        Self {
            use_tabs: false,
            width: 4,
        }
    }
}

impl Indentation {
    /// The text for one indentation level.
    pub fn unit(self) -> String {
        if self.use_tabs {
            "\t".to_owned()
        } else {
            " ".repeat(self.width)
        }
    }

    /// A short status label.
    pub fn label(self) -> String {
        if self.use_tabs {
            "Tabs".to_owned()
        } else {
            format!("Spaces: {}", self.width)
        }
    }
}

impl Document {
    /// Detect tabs versus spaces and the most common indentation width from a sample of lines.
    pub fn detect_indentation(&self) -> Indentation {
        let mut tab_lines = 0usize;
        let mut space_lines = 0usize;
        let mut widths = [0usize; 9];
        for line in 0..self.line_count().min(2_000) {
            let text = self.line(line);
            if text.trim().is_empty() {
                continue;
            }
            let leading: Vec<char> = text
                .chars()
                .take_while(|character| *character == '\t' || *character == ' ')
                .collect();
            if leading.is_empty() {
                continue;
            }
            if leading.iter().all(|character| *character == '\t') {
                tab_lines += 1;
            } else if leading.iter().all(|character| *character == ' ') {
                space_lines += 1;
                let count = leading.len().min(8);
                if count > 0 {
                    widths[count] += 1;
                }
            }
        }
        let use_tabs = tab_lines > 0 && tab_lines >= space_lines;
        let width = if use_tabs {
            4
        } else {
            (1..=8)
                .max_by_key(|width| widths[*width])
                .filter(|width| widths[*width] > 0)
                .unwrap_or(4)
        };
        Indentation { use_tabs, width }
    }
}

#[cfg(test)]
mod indentation_tests {
    use std::path::PathBuf;

    use crate::{Document, Encoding, Eol, Indentation};

    fn document(text: &str) -> Document {
        Document::from_loaded(
            PathBuf::from("fixture"),
            text.to_owned(),
            Eol::Lf,
            Encoding::Utf8,
            false,
        )
    }

    #[test]
    fn detects_tabs_and_spaces() {
        assert_eq!(
            document("fn main() {\n\tlet x = 1;\n}\n").detect_indentation(),
            Indentation {
                use_tabs: true,
                width: 4
            }
        );
        assert_eq!(
            document("fn main() {\n  let x = 1;\n  let y = 2;\n}\n").detect_indentation(),
            Indentation {
                use_tabs: false,
                width: 2
            }
        );
        assert_eq!(
            document("fn main() {\n    let x = 1;\n}\n").detect_indentation(),
            Indentation {
                use_tabs: false,
                width: 4
            }
        );
        assert_eq!(Document::untitled().detect_indentation(), Indentation::default());
    }
}
