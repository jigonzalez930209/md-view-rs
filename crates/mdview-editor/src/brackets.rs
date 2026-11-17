//! Matching bracket lookup for the cursor.

use crate::Editor;

impl Editor {
    pub(crate) fn ensure_brackets(&mut self) {
        if self.bracket_cursor == Some(self.cursor) && self.bracket_revision == Some(self.document.revision()) {
            return;
        }
        self.bracket_cursor = Some(self.cursor);
        self.bracket_revision = Some(self.document.revision());
        self.bracket_match = self.matching_bracket();
    }

    fn matching_bracket(&self) -> Option<(usize, usize)> {
        let len = self.document.len_chars();
        if len == 0 {
            return None;
        }
        let position = if self.cursor < len && self.document.char_at(self.cursor).is_some_and(is_bracket) {
            self.cursor
        } else if self.cursor > 0 && self.document.char_at(self.cursor - 1).is_some_and(is_bracket) {
            self.cursor - 1
        } else {
            return None;
        };
        let (open, close, forward) = match self.document.char_at(position)? {
            '(' => ('(', ')', true),
            '[' => ('[', ']', true),
            '{' => ('{', '}', true),
            ')' => ('(', ')', false),
            ']' => ('[', ']', false),
            '}' => ('{', '}', false),
            _ => return None,
        };
        let mut depth = 0usize;
        if forward {
            for index in position..len {
                let character = self.document.char_at(index)?;
                if character == open {
                    depth += 1;
                } else if character == close {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((position, index));
                    }
                }
            }
        } else {
            for index in (0..=position).rev() {
                let character = self.document.char_at(index)?;
                if character == close {
                    depth += 1;
                } else if character == open {
                    depth = depth.saturating_sub(1);
                    if depth == 0 {
                        return Some((index, position));
                    }
                }
            }
        }
        None
    }
}

fn is_bracket(character: char) -> bool {
    matches!(character, '(' | ')' | '[' | ']' | '{' | '}')
}
