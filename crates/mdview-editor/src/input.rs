//! Keyboard, pointer, and IME input for the editor.

use egui::{Event, Key, Modifiers, Response, Ui};

use crate::Editor;

/// Geometry of the painted editor content.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Geometry {
    pub(crate) content_rect: egui::Rect,
    pub(crate) row_height: f32,
    pub(crate) char_width: f32,
    pub(crate) gutter: f32,
    pub(crate) columns: usize,
}

impl Editor {
    /// Handle a frame's input after the content has been painted.
    pub(crate) fn handle_input(&mut self, ui: &mut Ui, response: &Response, geometry: &Geometry) {
        if response.clicked() || response.drag_started() {
            response.request_focus();
        }

        if response.clicked()
            && let Some(position) = ui.input(|input| input.pointer.interact_pos())
        {
            let now = std::time::Instant::now();
            let count = self
                .last_click
                .filter(|(last, last_position, _)| {
                    now.duration_since(*last) < std::time::Duration::from_millis(350)
                        && last_position.distance(position) < 4.0
                })
                .map_or(1, |(_, _, count)| count.saturating_add(1));
            self.last_click = Some((now, position, count));
            let (line, column) = self.position_at(position, geometry);
            if ui.input(|input| input.modifiers.shift) {
                self.cursor = self.document.char_index(line, column);
            } else if count >= 3 {
                self.anchor = self.document.line_start_char(line);
                self.cursor = self.document.char_index(line, self.document.line_len(line));
            } else if count == 2 {
                let (start, end) = self.word_at(line, column);
                self.anchor = start;
                self.cursor = end;
            } else {
                let index = self.document.char_index(line, column);
                self.anchor = index;
                self.cursor = index;
            }
            self.document.break_undo_group();
            self.cursor_blink_start = std::time::Instant::now();
        } else if response.dragged()
            && let Some(position) = ui.input(|input| input.pointer.interact_pos())
        {
            let (line, column) = self.position_at(position, geometry);
            self.cursor = self.document.char_index(line, column);
            self.scroll_to_cursor = true;
        }

        if !response.has_focus() {
            return;
        }

        let events = ui.input(|input| input.events.clone());
        for event in events {
            match event {
                Event::Text(text) => self.insert_text(&text),
                Event::Paste(text) => self.insert_text(&text),
                Event::Ime(egui::ImeEvent::Commit(text)) => self.apply_ime_commit(&text),
                Event::Ime(egui::ImeEvent::DeleteSurrounding {
                    before_chars,
                    after_chars,
                }) => self.delete_surrounding(before_chars, after_chars),
                Event::Copy => self.copy_selection(ui),
                Event::Cut => {
                    self.copy_selection(ui);
                    self.delete_selection();
                }
                Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } => {
                    self.handle_key(ui, key, modifiers);
                }
                _ => {}
            }
        }
    }

    fn handle_key(&mut self, ui: &mut Ui, key: Key, modifiers: Modifiers) {
        let command = modifiers.command;
        match key {
            Key::Z if command && !modifiers.shift => {
                self.document.undo();
                self.clamp_cursor();
            }
            Key::Z if command && modifiers.shift => {
                self.document.redo();
                self.clamp_cursor();
            }
            Key::Y if command => {
                self.document.redo();
                self.clamp_cursor();
            }
            Key::A if command => {
                self.anchor = 0;
                self.cursor = self.document.len_chars();
            }
            Key::C if command => self.copy_selection(ui),
            Key::X if command => {
                self.copy_selection(ui);
                self.delete_selection();
            }
            Key::F if command => self.open_find(false),
            Key::H if command => self.open_find(true),
            Key::G if command => self.open_goto(),
            Key::F3 => {
                if modifiers.shift {
                    self.find_previous();
                } else {
                    self.find_next();
                }
            }
            Key::Escape => {
                self.find.open = false;
                self.goto.reset();
            }
            Key::ArrowLeft => self.move_horizontal(-1, modifiers.shift, command),
            Key::ArrowRight => self.move_horizontal(1, modifiers.shift, command),
            Key::ArrowUp => self.move_vertical(-1, modifiers.shift),
            Key::ArrowDown => self.move_vertical(1, modifiers.shift),
            Key::Home => self.move_home(modifiers.shift, command),
            Key::End => self.move_end(modifiers.shift, command),
            Key::PageUp => self.move_vertical(-20, modifiers.shift),
            Key::PageDown => self.move_vertical(20, modifiers.shift),
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete_forward(),
            Key::Enter => self.newline(),
            Key::Tab => self.indent(modifiers.shift),
            _ => {}
        }
        self.document.break_undo_group();
    }

    fn move_horizontal(&mut self, delta: isize, select: bool, word: bool) {
        let mut next = self.cursor;
        if word {
            next = self.word_boundary(next, delta);
        } else if delta < 0 {
            next = next.saturating_sub(1);
        } else {
            next = (next + 1).min(self.document.len_chars());
        }
        self.set_cursor(next, select);
    }

    fn move_vertical(&mut self, delta: isize, select: bool) {
        let (line, column) = self.document.line_col(self.cursor);
        let target = if delta < 0 {
            line.saturating_sub(delta.unsigned_abs())
        } else {
            (line + delta as usize).min(self.document.line_count().saturating_sub(1))
        };
        let next = self.document.char_index(target, column);
        self.set_cursor(next, select);
    }

    fn move_home(&mut self, select: bool, document: bool) {
        let next = if document {
            0
        } else {
            let (line, _) = self.document.line_col(self.cursor);
            self.document.line_start_char(line)
        };
        self.set_cursor(next, select);
    }

    fn move_end(&mut self, select: bool, document: bool) {
        let next = if document {
            self.document.len_chars()
        } else {
            let (line, _) = self.document.line_col(self.cursor);
            self.document.char_index(line, self.document.line_len(line))
        };
        self.set_cursor(next, select);
    }

    fn set_cursor(&mut self, next: usize, select: bool) {
        self.cursor = next.min(self.document.len_chars());
        if !select {
            self.anchor = self.cursor;
        }
        self.scroll_to_cursor = true;
        self.cursor_blink_start = std::time::Instant::now();
    }

    fn insert_text(&mut self, text: &str) {
        if text.is_empty() {
            return;
        }
        if text.chars().count() == 1
            && let Some(paired) = self.auto_pair(text.chars().next().unwrap_or_default())
        {
            self.replace_selection(&paired);
            self.cursor = self.cursor.saturating_sub(1);
            self.anchor = self.cursor;
            return;
        }
        if text.chars().count() == 1
            && let Some(closing) = text.chars().next()
            && self.next_char() == Some(closing)
        {
            self.cursor += 1;
            self.anchor = self.cursor;
            return;
        }
        self.replace_selection(text);
    }

    fn auto_pair(&self, character: char) -> Option<String> {
        let closing = match character {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '"' => '"',
            '\'' => '\'',
            '`' => '`',
            _ => return None,
        };
        Some(format!("{character}{closing}"))
    }

    pub(crate) fn apply_ime_commit(&mut self, text: &str) {
        self.insert_text(text);
    }

    fn delete_surrounding(&mut self, before_chars: usize, after_chars: usize) {
        let start = self.cursor.saturating_sub(before_chars);
        let end = (self.cursor + after_chars).min(self.document.len_chars());
        if start < end {
            self.document.delete_range(start, end);
            self.cursor = start;
            self.anchor = start;
            self.scroll_to_cursor = true;
        }
    }

    fn replace_selection(&mut self, text: &str) {
        let (start, end) = self.selection();
        if start != end {
            self.document.delete_range(start, end);
        }
        self.document.insert_text(start, text);
        self.cursor = start + text.chars().count();
        self.anchor = self.cursor;
        self.scroll_to_cursor = true;
        self.cursor_blink_start = std::time::Instant::now();
    }

    fn backspace(&mut self) {
        let (start, end) = self.selection();
        if start != end {
            self.document.delete_range(start, end);
            self.cursor = start;
            self.anchor = start;
            return;
        }
        if start == 0 {
            return;
        }
        let previous = start - 1;
        let pair = self.document.slice(previous, start);
        let next = self.document.slice(start, (start + 1).min(self.document.len_chars()));
        let delete_pair = matches!(
            (pair.as_str(), next.as_str()),
            ("(", ")") | ("[", "]") | ("{", "}") | ("\"", "\"") | ("'", "'") | ("`", "`")
        );
        let end = if delete_pair { start + 1 } else { start };
        self.document.delete_range(previous, end);
        self.cursor = previous;
        self.anchor = previous;
        self.scroll_to_cursor = true;
    }

    fn delete_forward(&mut self) {
        let (start, end) = self.selection();
        if start != end {
            self.document.delete_range(start, end);
            self.cursor = start;
            self.anchor = start;
            return;
        }
        if start < self.document.len_chars() {
            self.document.delete_range(start, start + 1);
        }
    }

    fn delete_selection(&mut self) {
        let (start, end) = self.selection();
        if start != end {
            self.document.delete_range(start, end);
            self.cursor = start;
            self.anchor = start;
        }
    }

    fn newline(&mut self) {
        let (line, _) = self.document.line_col(self.cursor);
        let current = self.document.line(line);
        let indent: String = current
            .chars()
            .take_while(|character| character.is_whitespace())
            .collect();
        let mut extra = String::new();
        if current.trim_end().ends_with('{') {
            extra.push_str("    ");
        }
        let text = format!("\n{indent}{extra}");
        self.replace_selection(&text);
    }

    fn indent(&mut self, dedent: bool) {
        let (start, end) = self.selection();
        if start == end {
            if dedent {
                return;
            }
            let unit = self.indentation.unit();
            self.replace_selection(&unit);
            return;
        }
        let start_line = self.document.line_col(start).0;
        let end_line = self.document.line_col(end).0;
        for line in (start_line..=end_line).rev() {
            let line_start = self.document.line_start_char(line);
            if dedent {
                let text = self.document.line(line);
                let removed = if self.indentation.use_tabs {
                    text.chars().take(1).take_while(|character| *character == '\t').count()
                } else {
                    text.chars()
                        .take(self.indentation.width)
                        .take_while(|character| *character == ' ')
                        .count()
                };
                if removed > 0 {
                    self.document.delete_range(line_start, line_start + removed);
                }
            } else {
                let unit = self.indentation.unit();
                self.document.insert_text(line_start, &unit);
            }
        }
        self.anchor = self.document.line_start_char(start_line);
        self.cursor = self.document.char_index(end_line, self.document.line_len(end_line));
    }

    fn copy_selection(&self, ui: &mut Ui) {
        let (start, end) = self.selection();
        if start != end {
            ui.ctx().copy_text(self.document.slice(start, end));
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor = self.cursor.min(self.document.len_chars());
        self.anchor = self.anchor.min(self.document.len_chars());
    }

    fn next_char(&self) -> Option<char> {
        if self.cursor >= self.document.len_chars() {
            return None;
        }
        self.document.slice(self.cursor, self.cursor + 1).chars().next()
    }

    fn word_at(&self, line: usize, column: usize) -> (usize, usize) {
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

    fn word_boundary(&self, index: usize, delta: isize) -> usize {
        let text = self.document.snapshot();
        let chars: Vec<char> = text.chars().collect();
        let mut index = index.min(chars.len());
        if delta < 0 {
            while index > 0 && chars[index - 1].is_whitespace() {
                index -= 1;
            }
            while index > 0 && is_word(chars[index - 1]) {
                index -= 1;
            }
        } else {
            while index < chars.len() && is_word(chars[index]) {
                index += 1;
            }
            while index < chars.len() && chars[index].is_whitespace() {
                index += 1;
            }
        }
        index
    }

    fn position_at(&self, position: egui::Pos2, geometry: &Geometry) -> (usize, usize) {
        let local_y = (position.y - geometry.content_rect.min.y).max(0.0);
        let row = (local_y / geometry.row_height).floor() as usize;
        let (line, segment_start) = self.row_to_line(row, geometry.columns);
        let local_x = (position.x - geometry.content_rect.min.x - geometry.gutter - 8.0).max(0.0);
        let display_target = segment_start + (local_x / geometry.char_width).round() as usize;
        let text = self.document.line(line);
        let column = char_at_display_col(&text, display_target);
        (line, column)
    }

    pub(crate) fn row_to_line(&self, row: usize, columns: usize) -> (usize, usize) {
        if columns == 0 {
            return (row.min(self.document.line_count().saturating_sub(1)), 0);
        }
        let mut accumulated = 0;
        for line in 0..self.document.line_count() {
            let rows = rows_for_width(self.display_widths.get(line).copied().unwrap_or(0), columns);
            if row < accumulated + rows {
                let segment = row - accumulated;
                return (line, segment * columns);
            }
            accumulated += rows;
        }
        (self.document.line_count().saturating_sub(1), 0)
    }
}

fn rows_for_width(width: usize, columns: usize) -> usize {
    if columns == 0 {
        1
    } else {
        width.max(1).div_ceil(columns)
    }
}

fn char_at_display_col(text: &str, target: usize) -> usize {
    let mut width = 0;
    for (index, character) in text.chars().enumerate() {
        if width >= target {
            return index;
        }
        width += if character == '\t' { 4 - width % 4 } else { 1 };
    }
    text.chars().count()
}

fn is_word(character: char) -> bool {
    character.is_alphanumeric() || character == '_'
}
