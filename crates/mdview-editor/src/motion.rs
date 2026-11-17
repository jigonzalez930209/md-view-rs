//! Cursor motion, hit testing, and the mapping between lines and wrapped rows.

use crate::input::Geometry;
use crate::{Editor, display_col};

impl Editor {
    pub(crate) fn move_horizontal(&mut self, delta: isize, select: bool, word: bool) {
        let (start, end) = self.selection();
        let next = if !select && start != end && !word {
            // An arrow key collapses a selection to its edge instead of moving past it.
            if delta < 0 { start } else { end }
        } else if word {
            self.word_boundary(self.cursor, delta)
        } else if delta < 0 {
            self.cursor.saturating_sub(1)
        } else {
            (self.cursor + 1).min(self.document.len_chars())
        };
        self.set_cursor(next, select);
    }

    /// Move by visual rows, keeping the preferred column across short lines.
    pub(crate) fn move_vertical(&mut self, delta: isize, select: bool) {
        let columns = self.last_columns;
        let (line, column) = self.document.line_col(self.cursor);
        let text = self.document.line(line);
        let display = display_col(&text, column);
        let segment = self.segment_at(line, display, columns);
        let start = self.segment_start(line, segment, columns);
        let goal = *self.preferred_column.get_or_insert(display - start);

        let row = self.line_to_row(line) + segment;
        let last_line = self.document.line_count().saturating_sub(1);
        let last_row = self.line_to_row(last_line) + self.rows_of(last_line, columns) - 1;
        let target_row = if delta < 0 {
            row.saturating_sub(delta.unsigned_abs())
        } else {
            (row + delta as usize).min(last_row)
        };
        let next = if delta < 0 && row == 0 {
            0
        } else if delta > 0 && row == last_row {
            self.document.len_chars()
        } else {
            let (target_line, target_segment) = self.row_segment(target_row, columns);
            let target_text = self.document.line(target_line);
            let segment_start = self.segment_start(target_line, target_segment, columns);
            let segment_end = self.segment_end(target_line, target_segment, columns);
            // Not the last row of the line: stay on this row.
            let wanted = (segment_start + goal).min(segment_end.saturating_sub(1));
            let column = char_at_display_col(&target_text, wanted);
            self.document.char_index(target_line, column)
        };
        let preferred = self.preferred_column;
        self.set_cursor(next, select);
        self.preferred_column = preferred;
    }

    /// Home toggles between the first non-blank character and the line start.
    pub(crate) fn move_home(&mut self, select: bool, document: bool) {
        let next = if document {
            0
        } else {
            let (line, column) = self.document.line_col(self.cursor);
            let indent = self
                .document
                .line(line)
                .chars()
                .take_while(|character| *character == ' ' || *character == '\t')
                .count();
            let target = if column == indent { 0 } else { indent };
            self.document.char_index(line, target)
        };
        self.set_cursor(next, select);
    }

    pub(crate) fn move_end(&mut self, select: bool, document: bool) {
        let next = if document {
            self.document.len_chars()
        } else {
            let (line, _) = self.document.line_col(self.cursor);
            self.document.char_index(line, self.document.line_len(line))
        };
        self.set_cursor(next, select);
    }

    pub(crate) fn set_cursor(&mut self, next: usize, select: bool) {
        self.cursor = next.min(self.document.len_chars());
        if !select {
            self.anchor = self.cursor;
        }
        self.preferred_column = None;
        self.scroll_to_cursor = true;
        self.cursor_blink_start = std::time::Instant::now();
    }

    pub(crate) fn word_at(&self, line: usize, column: usize) -> (usize, usize) {
        let text = self.document.line(line);
        let chars: Vec<char> = text.chars().collect();
        let column = column.min(chars.len());
        let mut start = column;
        let mut end = column;
        while start > 0 && is_word(chars[start - 1]) {
            start -= 1;
        }
        while end < chars.len() && is_word(chars[end]) {
            end += 1;
        }
        (
            self.document.char_index(line, start),
            self.document.char_index(line, end),
        )
    }

    /// The next word boundary: skip a run of one character class (word characters or
    /// punctuation) and the whitespace next to it, so Ctrl+arrows never stick on `;`.
    pub(crate) fn word_boundary(&self, index: usize, delta: isize) -> usize {
        let len = self.document.len_chars();
        let mut index = index.min(len);
        let class_at = |index: usize| self.document.char_at(index).map_or(CharClass::Space, char_class);
        if delta < 0 {
            while index > 0 && class_at(index - 1) == CharClass::Space {
                index -= 1;
            }
            if index > 0 {
                let class = class_at(index - 1);
                while index > 0 && class_at(index - 1) == class {
                    index -= 1;
                }
            }
        } else {
            if index < len && class_at(index) != CharClass::Space {
                let class = class_at(index);
                while index < len && class_at(index) == class {
                    index += 1;
                }
            }
            while index < len && class_at(index) == CharClass::Space {
                index += 1;
            }
        }
        index
    }

    pub(crate) fn position_at(&self, position: egui::Pos2, geometry: &Geometry) -> (usize, usize) {
        let local_y = (position.y - geometry.content_rect.min.y).max(0.0);
        let row = (local_y / geometry.row_height).floor() as usize;
        let (line, segment) = self.row_segment(row, geometry.columns);
        let segment_start = self.segment_start(line, segment, geometry.columns);
        let segment_end = self.segment_end(line, segment, geometry.columns);
        let local_x = (position.x - geometry.content_rect.min.x - geometry.gutter - 8.0).max(0.0);
        // A click past the right edge of a wrapped row stays on that row.
        let display_target =
            (segment_start + (local_x / geometry.char_width).round() as usize).min(segment_end.saturating_sub(1));
        let text = self.document.line(line);
        let column = char_at_display_col(&text, display_target);
        (line, column)
    }

    /// The line and the first display column of a visual row.
    pub(crate) fn row_to_line(&self, row: usize, columns: usize) -> (usize, usize) {
        let (line, segment) = self.row_segment(row, columns);
        (line, self.segment_start(line, segment, columns))
    }

    /// The line and the row within that line of a visual row.
    pub(crate) fn row_segment(&self, row: usize, columns: usize) -> (usize, usize) {
        let last = self.document.line_count().saturating_sub(1);
        if columns == 0 {
            return (row.min(last), 0);
        }
        let mut accumulated = 0;
        for line in 0..self.document.line_count() {
            let rows = self.rows_of(line, columns);
            if row < accumulated + rows {
                return (line, row - accumulated);
            }
            accumulated += rows;
        }
        (last, self.rows_of(last, columns) - 1)
    }

    pub(crate) fn line_width(&self, line: usize) -> usize {
        self.display_widths.get(line).copied().unwrap_or(0)
    }
}

/// The character index whose start is closest to a display column.
pub(crate) fn char_at_display_col(text: &str, target: usize) -> usize {
    let mut width = 0;
    for (index, character) in text.chars().enumerate() {
        let next = width + crate::cell_width(character, width);
        if target < next {
            // Round to the nearer edge of a wide (tab) character.
            return if target - width > (next - width) / 2 {
                index + 1
            } else {
                index
            };
        }
        width = next;
    }
    text.chars().count()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum CharClass {
    Space,
    Word,
    Punctuation,
}

fn char_class(character: char) -> CharClass {
    if character.is_whitespace() {
        CharClass::Space
    } else if is_word(character) {
        CharClass::Word
    } else {
        CharClass::Punctuation
    }
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}

#[cfg(test)]
mod tests {
    use super::char_at_display_col;

    #[test]
    fn display_columns_round_inside_tabs() {
        assert_eq!(char_at_display_col("\tab", 0), 0);
        assert_eq!(char_at_display_col("\tab", 1), 0);
        assert_eq!(char_at_display_col("\tab", 3), 1);
        assert_eq!(char_at_display_col("\tab", 4), 1);
        assert_eq!(char_at_display_col("\tab", 5), 2);
        assert_eq!(char_at_display_col("\tab", 99), 3);
    }
}
