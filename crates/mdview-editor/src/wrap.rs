//! Soft-wrap layout: where each long line breaks into visual rows.
//!
//! A row breaks after the last whitespace that fits, so words stay whole; a word longer
//! than the row breaks at the edge. Whitespace that reaches the edge stays on its row
//! instead of starting the next one. Rows are measured in display cells (tabs to their
//! stop, wide characters as two cells), the same unit as the cursor and hit testing.

use crate::{Editor, cell_width};

/// Display columns where the rows after the first start. Empty when the line fits.
pub(crate) fn wrap_breaks(text: &str, columns: usize) -> Vec<usize> {
    let mut breaks = Vec::new();
    if columns == 0 {
        return breaks;
    }
    let mut row_start = 0usize;
    // The display column just after the latest whitespace on this row.
    let mut opportunity = 0usize;
    let mut display = 0usize;
    for character in text.chars() {
        let width = cell_width(character, display);
        let end = display + width;
        if character.is_whitespace() {
            // Hanging whitespace: a space at the edge never begins the next row.
            opportunity = end;
            display = end;
            continue;
        }
        if end > row_start + columns && display > row_start {
            let at = if opportunity > row_start && opportunity <= display {
                opportunity
            } else {
                display
            };
            breaks.push(at);
            row_start = at;
        }
        display = end;
    }
    // A trailing whitespace run that hangs past the edge must not add an empty row.
    breaks.retain(|at| *at < display);
    breaks
}

impl Editor {
    /// Recompute the row breaks of every line for this frame's wrap width.
    pub(crate) fn ensure_wrap(&mut self, columns: usize) {
        let key = (self.document.revision(), columns);
        if self.wrap_key == Some(key) {
            return;
        }
        self.wrap_key = Some(key);
        self.wrap_breaks.clear();
        if columns == 0 {
            return;
        }
        self.wrap_breaks.reserve(self.document.line_count());
        for line in 0..self.document.line_count() {
            if self.line_width(line) <= columns {
                self.wrap_breaks.push(Vec::new());
            } else {
                self.wrap_breaks.push(wrap_breaks(&self.document.line(line), columns));
            }
        }
    }

    fn breaks(&self, line: usize, columns: usize) -> &[usize] {
        if columns == 0 {
            return &[];
        }
        self.wrap_breaks.get(line).map_or(&[], Vec::as_slice)
    }

    /// Visual rows of a line.
    pub(crate) fn rows_of(&self, line: usize, columns: usize) -> usize {
        self.breaks(line, columns).len() + 1
    }

    /// First display column of a row of a line.
    pub(crate) fn segment_start(&self, line: usize, segment: usize, columns: usize) -> usize {
        match segment {
            0 => 0,
            _ => self.breaks(line, columns).get(segment - 1).copied().unwrap_or(0),
        }
    }

    /// Display column where a row ends; `usize::MAX` for the last row of the line.
    pub(crate) fn segment_end(&self, line: usize, segment: usize, columns: usize) -> usize {
        self.breaks(line, columns).get(segment).copied().unwrap_or(usize::MAX)
    }

    /// The row of a line that shows a display column. A column at a break belongs to
    /// the row that starts there.
    pub(crate) fn segment_at(&self, line: usize, display: usize, columns: usize) -> usize {
        self.breaks(line, columns)
            .iter()
            .take_while(|at| **at <= display)
            .count()
    }
}

#[cfg(test)]
mod tests {
    use super::wrap_breaks;

    #[test]
    fn breaks_after_whole_words() {
        // "alpha beta gamma" at 12 columns: "alpha beta " | "gamma".
        assert_eq!(wrap_breaks("alpha beta gamma", 12), vec![11]);
        assert_eq!(wrap_breaks("alpha beta", 12), Vec::<usize>::new());
        // A space exactly at the edge hangs instead of starting the next row.
        assert_eq!(wrap_breaks("abcdefgh ijk", 8), vec![9]);
    }

    #[test]
    fn long_words_break_at_the_edge() {
        assert_eq!(wrap_breaks("abcdefghijkl", 5), vec![5, 10]);
        assert_eq!(wrap_breaks("ab abcdefghij", 5), vec![3, 8]);
    }

    #[test]
    fn trailing_spaces_do_not_add_rows() {
        assert_eq!(wrap_breaks("abcd      ", 5), Vec::<usize>::new());
        assert_eq!(wrap_breaks("", 5), Vec::<usize>::new());
    }

    #[test]
    fn wide_characters_take_two_cells() {
        // Four CJK characters are eight cells: two rows of three at six columns... and one.
        assert_eq!(wrap_breaks("中文中文", 6), vec![6]);
        assert_eq!(wrap_breaks("中文中文", 0), Vec::<usize>::new());
    }
}
