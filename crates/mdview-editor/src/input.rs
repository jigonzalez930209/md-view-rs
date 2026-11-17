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
        if std::mem::take(&mut self.focus_requested) {
            response.request_focus();
        }
        if self.ignore_input {
            return;
        }
        // egui reports Enter and Space on a focused widget as a primary click
        // (`Response::clicked`). Only a real pointer click may move the cursor; otherwise
        // every Enter and every typed space would jump the cursor to the mouse pointer.
        let pointer_click = response.clicked_by(egui::PointerButton::Primary);
        if pointer_click || response.drag_started() {
            response.request_focus();
        }

        if pointer_click && let Some(position) = ui.input(|input| input.pointer.interact_pos()) {
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
            if response.drag_started()
                && !ui.input(|input| input.modifiers.shift)
                && let Some(origin) = ui.input(|input| input.pointer.press_origin())
            {
                // A drag selects from where the button went down, not from the old cursor.
                let (line, column) = self.position_at(origin, geometry);
                self.anchor = self.document.char_index(line, column);
                self.document.break_undo_group();
            }
            let (line, column) = self.position_at(position, geometry);
            self.cursor = self.document.char_index(line, column);
            self.preferred_column = None;
            self.scroll_to_cursor = true;
        }

        self.focused = response.has_focus();
        if !self.focused {
            return;
        }
        // Keep Tab and the arrow keys for editing: without a lock filter egui uses them to
        // move keyboard focus to the next widget, so Tab left the editor instead of indenting.
        ui.memory_mut(|memory| {
            memory.set_focus_lock_filter(
                response.id,
                egui::EventFilter {
                    tab: true,
                    horizontal_arrows: true,
                    vertical_arrows: true,
                    escape: false,
                },
            );
        });

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
                if self.document.undo() {
                    self.after_history();
                }
            }
            Key::Z if command && modifiers.shift => {
                if self.document.redo() {
                    self.after_history();
                }
            }
            Key::Y if command => {
                if self.document.redo() {
                    self.after_history();
                }
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
            Key::PageUp => self.move_vertical(-self.page_rows(), modifiers.shift),
            Key::PageDown => self.move_vertical(self.page_rows(), modifiers.shift),
            Key::Backspace if command => self.delete_word(-1),
            Key::Delete if command => self.delete_word(1),
            Key::Backspace => self.backspace(),
            Key::Delete => self.delete_forward(),
            Key::Enter => {
                if !self.markdown_newline() {
                    self.newline();
                }
                self.scroll_to_cursor = true;
            }
            Key::Tab => {
                if !self.markdown_table_tab(modifiers.shift) && !self.markdown_tab(modifiers.shift) {
                    self.indent(modifiers.shift);
                }
                self.scroll_to_cursor = true;
            }
            Key::B if command && self.is_markdown() => self.apply_markdown(crate::MarkdownAction::Bold),
            Key::I if command && self.is_markdown() => self.apply_markdown(crate::MarkdownAction::Italic),
            Key::U if command && self.is_markdown() => self.apply_markdown(crate::MarkdownAction::Underline),
            Key::K if command && self.is_markdown() => self.apply_markdown(crate::MarkdownAction::Link),
            _ => {}
        }
        self.document.break_undo_group();
    }

    /// Rows moved by Page Up and Page Down: one viewport, keeping a row of context.
    fn page_rows(&self) -> isize {
        let height = self.viewport_rect.height();
        if !height.is_finite() || height <= 0.0 {
            return 20;
        }
        ((height / self.last_row_height.max(1.0)).floor() as isize - 1).max(1)
    }

    /// Put the cursor where undo or redo changed the text and bring it into view.
    fn after_history(&mut self) {
        self.cursor = self.document.last_edit_end();
        self.anchor = self.cursor;
        self.preferred_column = None;
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
        // Only close a pair where nothing follows but space or a closing character, so
        // typing before a word does not leave a stray closing character.
        let next = self.char_after_selection();
        if next.is_some_and(|next| !next.is_whitespace() && !matches!(next, ')' | ']' | '}' | ',' | ';' | ':' | '.')) {
            return None;
        }
        if matches!(character, '"' | '\'' | '`') {
            // An apostrophe in prose (`don't`) or a quote after a word is not an opening quote.
            let previous = self
                .selection()
                .0
                .checked_sub(1)
                .and_then(|index| self.document.char_at(index));
            if previous.is_some_and(|previous| previous.is_alphanumeric() || previous == '_' || previous == character) {
                return None;
            }
        }
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

    pub(crate) fn replace_selection(&mut self, text: &str) {
        let (start, end) = self.selection();
        if start != end {
            // One undo step for typing over a selection.
            self.document.replace_range(start, end, text);
        } else {
            self.document.insert_text(start, text);
        }
        self.cursor = start + text.chars().count();
        self.anchor = self.cursor;
        self.preferred_column = None;
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
        let (line, column) = self.document.line_col(start);
        let before: String = self.document.line(line).chars().take(column).collect();
        if !self.indentation.use_tabs && column > 1 && before.chars().all(|character| character == ' ') {
            // Inside leading spaces, Backspace removes one indentation level.
            let width = self.indentation.width.max(1);
            let target = (column - 1) / width * width;
            self.document.delete_range(start - (column - target), start);
            self.cursor = start - (column - target);
            self.anchor = self.cursor;
            self.preferred_column = None;
            self.scroll_to_cursor = true;
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

    /// Delete to the previous (`-1`) or next (`1`) word boundary, or the selection.
    fn delete_word(&mut self, direction: isize) {
        let (start, end) = self.selection();
        let (from, to) = if start != end {
            (start, end)
        } else if direction < 0 {
            (self.word_boundary(start, -1), start)
        } else {
            (start, self.word_boundary(start, 1))
        };
        if from < to {
            self.document.delete_range(from, to);
        }
        self.cursor = from;
        self.anchor = from;
        self.preferred_column = None;
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
        let (start, _) = self.selection();
        let (line, column) = self.document.line_col(start);
        let current = self.document.line(line);
        // Indentation comes from the text before the cursor only, so Enter inside the
        // leading whitespace does not duplicate it.
        let indent: String = current
            .chars()
            .take(column)
            .take_while(|character| *character == ' ' || *character == '\t')
            .collect();
        let before = current.chars().take(column).collect::<String>();
        let opens = before.trim_end().ends_with(['{', '[', '(']);
        let closes_next = self
            .char_after_selection()
            .is_some_and(|next| matches!(next, '}' | ']' | ')'));
        if opens {
            let inner = format!("\n{indent}{}", self.indentation.unit());
            if closes_next {
                // `{|}` becomes an indented empty block with the brace on its own line.
                self.replace_selection(&format!("{inner}\n{indent}"));
                self.cursor = start + inner.chars().count();
                self.anchor = self.cursor;
            } else {
                self.replace_selection(&inner);
            }
            return;
        }
        let (_, end) = self.selection();
        if start == end && !indent.is_empty() && column == current.chars().count() && before.trim().is_empty() {
            // Enter on a line holding only indentation leaves that line empty instead of
            // keeping trailing whitespace; the indentation moves to the new line.
            self.anchor = self.document.line_start_char(line);
        }
        self.replace_selection(&format!("\n{indent}"));
    }

    fn char_after_selection(&self) -> Option<char> {
        self.document.char_at(self.selection().1)
    }

    fn indent(&mut self, dedent: bool) {
        let (start, end) = self.selection();
        if start == end && !dedent {
            let unit = self.indentation.unit();
            self.replace_selection(&unit);
            return;
        }
        let (start_line, start_column) = self.document.line_col(start);
        let (mut end_line, end_column) = self.document.line_col(end);
        if end_line > start_line && end_column == 0 {
            // A selection ending at the start of a line does not include that line.
            end_line -= 1;
        }
        let block_start = self.document.line_start_char(start_line);
        let block_end = self.document.line_start_char(end_line) + self.document.line_len(end_line);
        let unit = self.indentation.unit();
        let mut shifted = Vec::new();
        let lines: Vec<String> = (start_line..=end_line)
            .map(|line| {
                let text = self.document.line(line);
                let (new, shift) = if dedent {
                    let removed = if text.starts_with('\t') {
                        1
                    } else {
                        text.chars()
                            .take(self.indentation.width)
                            .take_while(|character| *character == ' ')
                            .count()
                    };
                    (text.chars().skip(removed).collect(), -(removed as isize))
                } else if text.is_empty() && start != end {
                    // Indenting a block leaves its empty lines empty.
                    (text, 0)
                } else {
                    (format!("{unit}{text}"), unit.chars().count() as isize)
                };
                shifted.push(shift);
                new
            })
            .collect();
        let replacement = lines.join("\n");
        if replacement == self.document.slice(block_start, block_end) {
            return;
        }
        // One edit, so a single undo restores the whole block.
        self.document.replace_range(block_start, block_end, &replacement);
        if start == end {
            let column = start_column as isize + shifted[0];
            self.cursor = block_start + column.max(0) as usize;
            self.anchor = self.cursor;
        } else {
            self.anchor = block_start;
            self.cursor = block_start + replacement.chars().count();
        }
        self.preferred_column = None;
        self.scroll_to_cursor = true;
    }

    fn copy_selection(&self, ui: &mut Ui) {
        let (start, end) = self.selection();
        if start != end {
            ui.ctx().copy_text(self.document.slice(start, end));
        }
    }

    fn next_char(&self) -> Option<char> {
        if self.cursor >= self.document.len_chars() {
            return None;
        }
        self.document.slice(self.cursor, self.cursor + 1).chars().next()
    }
}
