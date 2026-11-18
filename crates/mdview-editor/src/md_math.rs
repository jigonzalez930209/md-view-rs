//! Formula editing support: open a formula on double-click, insert or replace formulas,
//! and expose the hovered character and a marked range for the formula editor dialog.

use egui::{Color32, Pos2, Rect, Response, Ui};
use mdview_markdown::{MathDelimiter, format_math, math_source_at};

use crate::input::Geometry;
use crate::paint::RowPaint;
use crate::{Editor, display_col};

/// A formula in the editor's document, in character indices.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MathTarget {
    /// First character of the formula, delimiters included.
    pub start: usize,
    /// One past the last character of the formula.
    pub end: usize,
    /// The TeX between the delimiters.
    pub tex: String,
    /// Whether the formula renders as a block.
    pub display: bool,
    /// Whether the formula is a ```` ```math ```` fence.
    pub fence: bool,
}

/// A source range painted with a tint and an underline.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct MarkedRange {
    pub start: usize,
    pub end: usize,
    pub color: Color32,
}

impl Editor {
    /// The formula containing the character `index`, when the document is Markdown.
    pub fn math_at(&self, index: usize) -> Option<MathTarget> {
        if !self.is_markdown() {
            return None;
        }
        let text = self.document.snapshot();
        let formula = math_source_at(&text, self.document.char_to_byte(index))?;
        Some(MathTarget {
            start: self.document.byte_to_char(formula.range.start),
            end: self.document.byte_to_char(formula.range.end),
            display: formula.is_display(),
            fence: formula.delimiter == MathDelimiter::Fence,
            tex: formula.tex,
        })
    }

    /// The formula the user double-clicked since the last call, if any.
    pub fn take_math_request(&mut self) -> Option<MathTarget> {
        self.math_request.take()
    }

    /// Insert a formula at the cursor, replacing the selection. A display formula goes on
    /// its own lines.
    pub fn insert_math(&mut self, tex: &str, display: bool) {
        let mut text = format_math(tex, display, false);
        if display {
            let (start, end) = self.selection();
            let at_line_start = start == 0 || self.document.char_at(start - 1) == Some('\n');
            let at_line_end = matches!(self.document.char_at(end), None | Some('\n'));
            if !at_line_start {
                text.insert(0, '\n');
            }
            if !at_line_end {
                text.push('\n');
            }
        }
        self.replace_selection(&text);
        self.finish_math_edit();
    }

    /// Replace a formula found by [`Editor::math_at`] with new TeX, in one undo step.
    pub fn replace_math(&mut self, target: &MathTarget, tex: &str, display: bool) {
        let end = target.end.min(self.document.len_chars());
        let start = target.start.min(end);
        let text = format_math(tex, display, display && target.fence);
        self.document.replace_range(start, end, &text);
        self.cursor = start + text.chars().count();
        self.anchor = self.cursor;
        self.finish_math_edit();
    }

    /// Replace the selection with `text` as one undo step and return where it starts.
    pub fn insert_snippet(&mut self, text: &str) -> usize {
        let start = self.selection().0;
        self.replace_selection(text);
        self.finish_math_edit();
        start
    }

    /// The selection as a character range, start first.
    pub fn selection_range(&self) -> (usize, usize) {
        self.selection()
    }

    fn finish_math_edit(&mut self) {
        self.preferred_column = None;
        self.scroll_to_cursor = true;
        self.focus_requested = true;
        self.cursor_blink_start = std::time::Instant::now();
        self.document.break_undo_group();
    }

    /// The character under the pointer in the last frame, if the pointer was over text.
    pub fn hovered_char(&self) -> Option<usize> {
        self.hovered_char
    }

    /// Tint and underline a character range, or clear it with `None`.
    pub fn set_marked_range(&mut self, range: Option<(usize, usize)>, color: Color32) {
        self.marked = range
            .filter(|(start, end)| start < end)
            .map(|(start, end)| MarkedRange { start, end, color });
    }

    /// Track the hovered character and turn a double-click inside a formula into a request.
    pub(crate) fn track_math_pointer(&mut self, ui: &Ui, response: &Response, geometry: &Geometry) {
        self.hovered_char = response
            .hover_pos()
            .and_then(|position| self.char_under(position, geometry));
        // The editor counts clicks itself (see `handle_input`); reuse its count so a
        // double-click here is exactly the one that would otherwise select a word.
        let double_click = response.clicked_by(egui::PointerButton::Primary)
            && self.last_click.is_some_and(|(_, _, count)| count == 2);
        if self.ignore_input || !double_click || !self.is_markdown() {
            return;
        }
        let Some(index) = ui
            .input(|input| input.pointer.interact_pos())
            .and_then(|position| self.char_under(position, geometry))
        else {
            return;
        };
        if let Some(target) = self.math_at(index) {
            self.anchor = self.cursor;
            self.math_request = Some(target);
        }
    }

    /// The character whose cell contains `position`; `None` past the end of its line.
    fn char_under(&self, position: Pos2, geometry: &Geometry) -> Option<usize> {
        let local_y = position.y - geometry.content_rect.min.y;
        let local_x = position.x - geometry.content_rect.min.x - geometry.gutter - 8.0;
        if local_y < 0.0 || local_x < 0.0 {
            return None;
        }
        let row = (local_y / geometry.row_height).floor() as usize;
        let (line, segment) = self.row_segment(row, geometry.columns);
        let segment_start = self.segment_start(line, segment, geometry.columns);
        let segment_end = self.segment_end(line, segment, geometry.columns);
        let display = segment_start + (local_x / geometry.char_width).floor() as usize;
        let text = self.document.line(line);
        let text = text.trim_end_matches(['\r', '\n']);
        if display >= segment_end || display >= display_col(text, text.chars().count()) {
            return None;
        }
        let column = crate::motion::char_at_display_col(text, display);
        Some(self.document.char_index(line, column))
    }

    pub(crate) fn paint_marked(&self, row: &RowPaint<'_>) {
        let Some(marked) = self.marked else {
            return;
        };
        let text = self.document.line(row.line);
        let length = text.chars().count();
        let line_end = row.line_start + length;
        if marked.end <= row.line_start || marked.start >= line_end {
            return;
        }
        let start_col = marked.start.saturating_sub(row.line_start).min(length);
        let end_col = marked.end.saturating_sub(row.line_start).min(length);
        let start = display_col(&text, start_col).max(row.segment_start);
        let end = display_col(&text, end_col).min(row.segment_end);
        if start >= end {
            return;
        }
        let rect = Rect::from_min_max(
            Pos2::new(row.x(start), row.row_rect.min.y),
            Pos2::new(row.x(end), row.row_rect.max.y),
        );
        row.painter.rect_filled(rect, 2.0, marked.color.gamma_multiply(0.22));
        row.painter.line_segment(
            [
                rect.left_bottom() - egui::vec2(0.0, 1.0),
                rect.right_bottom() - egui::vec2(0.0, 1.0),
            ],
            egui::Stroke::new(1.5, marked.color),
        );
    }
}
