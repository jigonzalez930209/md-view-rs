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
    /// Detect tabs versus spaces and the indentation step from a sample of lines.
    ///
    /// The step is the most common change in leading spaces between consecutive
    /// non-blank lines; the most common absolute width would report 8 for nested
    /// four-space code.
    pub fn detect_indentation(&self) -> Indentation {
        let mut tab_lines = 0usize;
        let mut space_lines = 0usize;
        let mut steps = [0usize; 9];
        let mut previous = 0usize;
        for line in 0..self.line_count().min(2_000) {
            let text = self.line(line);
            let content = text.trim_start_matches([' ', '\t']);
            if content.is_empty() {
                continue;
            }
            let leading = &text[..text.len() - content.len()];
            if leading.contains('\t') {
                if !leading.contains(' ') {
                    tab_lines += 1;
                }
                continue;
            }
            let width = leading.len();
            if width > 0 {
                space_lines += 1;
            }
            // Block-comment continuation lines (` * text`) are aligned, not indented.
            if content.starts_with('*') && !content.starts_with("**") {
                continue;
            }
            let step = width.abs_diff(previous);
            if (1..=8).contains(&step) {
                steps[step] += 1;
            }
            previous = width;
        }
        let use_tabs = tab_lines > 0 && tab_lines >= space_lines;
        let width = if use_tabs {
            4
        } else {
            // Ties prefer the smaller step: 2 and 4 both explain four-space nesting.
            (1..=8)
                .rev()
                .max_by_key(|width| steps[*width])
                .filter(|width| steps[*width] > 0)
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

    #[test]
    fn nested_code_reports_the_step_not_the_deepest_width() {
        let nested = "impl A {\n    fn a() {\n        if x {\n            y();\n        }\n        z();\n        w();\n    }\n}\n";
        assert_eq!(
            document(nested).detect_indentation(),
            Indentation {
                use_tabs: false,
                width: 4
            }
        );
        let doc_comment = "/**\n * one\n * two\n * three\n */\nfn a() {\n  b();\n}\n";
        assert_eq!(document(doc_comment).detect_indentation().width, 2);
    }
}
