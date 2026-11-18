//! Row-level painting helpers shared by selection, matches, indentation, and whitespace.

use egui::{Color32, Painter, Pos2, Rect, Stroke, Visuals};

use crate::{Editor, WhitespaceMode, display_col};

/// Everything one painted text row needs.
pub(crate) struct RowPaint<'a> {
    pub painter: &'a Painter,
    pub row_rect: Rect,
    pub line: usize,
    pub line_start: usize,
    /// First display column of this row; 0 without soft wrap.
    pub segment_start: usize,
    /// Display column where this row ends; `usize::MAX` on a line's last row.
    pub segment_end: usize,
    pub gutter: f32,
    pub char_width: f32,
    pub origin: Pos2,
    pub y: f32,
    pub row_height: f32,
}

impl RowPaint<'_> {
    fn segment_bounds(&self) -> (usize, usize) {
        (self.segment_start, self.segment_end)
    }

    fn contains(&self, display: usize) -> bool {
        display >= self.segment_start && display < self.segment_end
    }

    /// The x position of a display column of the line, on this wrapped row.
    pub(crate) fn x(&self, display: usize) -> f32 {
        let local = display.saturating_sub(self.segment_start);
        self.origin.x + self.gutter + 8.0 + local as f32 * self.char_width
    }
}

impl Editor {
    pub(crate) fn paint_indent_guides(&self, row: &RowPaint<'_>) {
        let text = self.document.line(row.line);
        let indent_width = display_col(
            &text,
            text.chars()
                .take_while(|character| *character == ' ' || *character == '\t')
                .count(),
        );
        if indent_width < 4 {
            return;
        }
        let color = row
            .painter
            .ctx()
            .global_style()
            .visuals
            .widgets
            .noninteractive
            .bg_stroke
            .color;
        for column in (4..=indent_width).step_by(4) {
            let display = column - 1;
            if !row.contains(display) {
                continue;
            }
            let x = row.x(display);
            row.painter.line_segment(
                [Pos2::new(x, row.y + 2.0), Pos2::new(x, row.y + row.row_height - 2.0)],
                Stroke::new(1.0, color.gamma_multiply(0.7)),
            );
        }
    }

    pub(crate) fn paint_whitespace(
        &self,
        row: &RowPaint<'_>,
        text: &str,
        selection_start: usize,
        selection_end: usize,
        visuals: &Visuals,
    ) {
        let color = visuals.weak_text_color().gamma_multiply(0.7);
        let mut display = 0usize;
        for (column, character) in text.chars().enumerate() {
            let width = crate::cell_width(character, display);
            if character == ' ' || character == '\t' {
                let show = self.style.whitespace == WhitespaceMode::All
                    || (row.line_start + column >= selection_start && row.line_start + column < selection_end);
                let segment_ok = row.contains(display);
                if show && segment_ok {
                    let x = row.x(display);
                    row.painter.circle_filled(
                        Pos2::new(x + row.char_width * 0.5, row.y + row.row_height - 4.0),
                        1.0,
                        color,
                    );
                    if character == '\t' {
                        row.painter.circle_filled(
                            Pos2::new(x + row.char_width * 1.5, row.y + row.row_height - 4.0),
                            1.0,
                            color,
                        );
                    }
                }
            }
            display += width;
        }
    }

    pub(crate) fn paint_brackets(&self, row: &RowPaint<'_>) {
        let Some((first, second)) = self.bracket_match else {
            return;
        };
        let text = self.document.line(row.line);
        let line_end = row.line_start + text.chars().count();
        for index in [first, second] {
            if index < row.line_start || index >= line_end {
                continue;
            }
            let display = display_col(&text, index - row.line_start);
            let (start, end) = row.segment_bounds();
            if display < start || display >= end {
                continue;
            }
            let x = row.x(display);
            row.painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x, row.row_rect.min.y),
                    Pos2::new(x + row.char_width, row.row_rect.max.y),
                ),
                2.0,
                Color32::from_rgba_unmultiplied(110, 170, 255, 60),
            );
        }
    }

    pub(crate) fn paint_selection(
        &self,
        row: &RowPaint<'_>,
        selection_start: usize,
        selection_end: usize,
        selection_start_line: usize,
        selection_end_line: usize,
    ) {
        if selection_start == selection_end || row.line < selection_start_line || row.line > selection_end_line {
            return;
        }
        let text = self.document.line(row.line);
        let length = text.chars().count();
        let start_col = selection_start.saturating_sub(row.line_start).min(length);
        let end_col = selection_end.saturating_sub(row.line_start).min(length);
        // A selection that continues past this line covers its newline: show it as half a
        // cell on the line's last row, so selected empty lines are visible too.
        let newline = selection_end > row.line_start + length && row.line < selection_end_line;
        let newline_cells = usize::from(newline && row.segment_end == usize::MAX);
        if start_col == end_col && newline_cells == 0 {
            return;
        }
        let (segment_start, segment_end) = row.segment_bounds();
        let start = display_col(&text, start_col).max(segment_start);
        let end = display_col(&text, end_col).min(segment_end) + newline_cells;
        if start >= end {
            return;
        }
        let x = row.x(start);
        let cells = (end - start) as f32 - 0.5 * newline_cells as f32;
        let width = cells * row.char_width;
        row.painter.rect_filled(
            Rect::from_min_max(
                Pos2::new(x, row.row_rect.min.y),
                Pos2::new(x + width, row.row_rect.max.y),
            ),
            0.0,
            self.selection_color(row.painter),
        );
    }

    pub(crate) fn paint_matches(&self, row: &RowPaint<'_>) {
        let text = self.document.line(row.line);
        let line_end = row.line_start + text.chars().count();
        for (start, end) in &self.find.matches {
            if *end <= row.line_start || *start >= line_end {
                continue;
            }
            let start_col = start.saturating_sub(row.line_start).min(text.chars().count());
            let end_col = end.saturating_sub(row.line_start).min(text.chars().count());
            let (segment_start, segment_end) = row.segment_bounds();
            let start = display_col(&text, start_col).max(segment_start);
            let end = display_col(&text, end_col).min(segment_end);
            if start >= end {
                continue;
            }
            let x = row.x(start);
            let width = (end - start) as f32 * row.char_width;
            row.painter.rect_filled(
                Rect::from_min_max(
                    Pos2::new(x, row.row_rect.min.y),
                    Pos2::new(x + width, row.row_rect.max.y),
                ),
                0.0,
                Color32::from_rgba_unmultiplied(245, 196, 70, 70),
            );
        }
    }

    pub(crate) fn selection_color(&self, painter: &Painter) -> Color32 {
        painter.ctx().global_style().visuals.selection.bg_fill
    }
}
