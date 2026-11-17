//! Line widths in display cells: tabs to their stop, wide characters as two cells.

use crate::Editor;

impl Editor {
    pub(crate) fn ensure_layout(&mut self) {
        let revision = self.document.revision();
        if self.layout_revision == Some(revision) {
            return;
        }
        // Large files skip measuring; elsewhere tabs and wide characters set the width.
        let measure = !self.large_mode;
        let measure_line = |chars: &mut dyn Iterator<Item = char>| {
            if measure {
                chars.fold(0, |width, character| width + cell_width(character, width))
            } else {
                chars.count()
            }
        };
        if let Some(previous) = self.layout_revision
            && let Some(edits) = self.document.line_edits_since(previous)
        {
            // Splice in placeholders for the lines each edit touched, then measure only those:
            // typing in a 200,000-line file must not re-measure every line.
            const DIRTY: usize = usize::MAX;
            for edit in edits {
                let old_end = (edit.line + edit.removed_lines + 1).min(self.display_widths.len());
                let start = edit.line.min(old_end);
                self.display_widths
                    .splice(start..old_end, std::iter::repeat_n(DIRTY, edit.inserted_lines + 1));
            }
            if self.display_widths.len() == self.document.line_count() {
                for line in 0..self.display_widths.len() {
                    if self.display_widths[line] == DIRTY {
                        let text = self.document.line(line);
                        self.display_widths[line] = measure_line(&mut text.chars());
                    }
                }
                self.layout_revision = Some(revision);
                return;
            }
        }
        self.display_widths = self.document.map_lines(measure_line);
        self.layout_rebuilds += 1;
        self.layout_revision = Some(revision);
    }
}

/// Cells a character takes at a display column: tabs reach the next stop of four, and
/// wide characters (CJK, most emoji) take two cells of the monospace grid.
pub(crate) fn cell_width(character: char, display: usize) -> usize {
    if character == '\t' {
        4 - display % 4
    } else if unicode_width::UnicodeWidthChar::width(character) == Some(2) {
        2
    } else {
        1
    }
}

pub(crate) fn display_width(text: &str) -> usize {
    text.chars()
        .fold(0, |width, character| width + cell_width(character, width))
}

pub(crate) fn display_col(text: &str, char_col: usize) -> usize {
    text.chars()
        .take(char_col)
        .fold(0, |width, character| width + cell_width(character, width))
}
