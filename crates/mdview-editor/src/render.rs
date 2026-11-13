//! Painting the editor viewport.

use egui::{Align, Align2, Color32, FontId, Pos2, Rect, Response, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::input::Geometry;
use crate::{Editor, display_col, display_width};

impl Editor {
    /// Show the editor, handle input, and draw the go-to dialog.
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let (response, geometry) = self.paint(ui);
        self.handle_input(ui, &response, &geometry);
        self.show_goto(ui.ctx());
        response
    }

    /// Paint the editor and return its response and geometry.
    fn paint(&mut self, ui: &mut Ui) -> (Response, Geometry) {
        self.ensure_layout();
        self.ensure_language();
        self.ensure_brackets();
        self.show_find_bar(ui);

        let font = FontId::monospace(13.0);
        let row_height = ui.ctx().fonts_mut(|fonts| fonts.row_height(&font));
        let char_width = ui.ctx().fonts_mut(|fonts| fonts.glyph_width(&font, ' '));
        let digits = self.document.line_count().to_string().len().max(3);
        let gutter = if self.line_numbers {
            digits as f32 * char_width + 18.0
        } else {
            0.0
        };
        let available = ui.available_width().max(240.0);
        let available_height = ui.available_height().max(160.0);
        let columns = if self.wrap && !self.large_mode {
            (((available - gutter - 24.0) / char_width).floor() as usize).max(8)
        } else {
            0
        };
        let total_rows = self.total_rows(columns);
        let max_width = self.display_widths.iter().copied().max().unwrap_or(0);
        let content_width = if columns > 0 {
            available
        } else {
            (gutter + max_width as f32 * char_width + 40.0).max(available)
        };
        let content_height = (total_rows as f32 * row_height + 10.0).max(available_height);

        let mut response = None;
        let mut geometry = None;
        ScrollArea::both()
            .auto_shrink([false, false])
            .show_viewport(ui, |ui, viewport| {
                let (painted, painter) =
                    ui.allocate_painter(Vec2::new(content_width, content_height), Sense::click_and_drag());
                let origin = painted.rect.min;
                let start_row = (viewport.min.y / row_height).floor().max(0.0) as usize;
                let end_row = ((viewport.max.y / row_height).ceil() as usize + 1).min(total_rows);

                painter.rect_filled(painted.rect, 0.0, ui.visuals().extreme_bg_color);
                if gutter > 0.0 {
                    let gutter_rect =
                        Rect::from_min_max(origin, Pos2::new(origin.x + gutter, origin.y + content_height));
                    painter.rect_filled(gutter_rect, 0.0, ui.visuals().faint_bg_color);
                }

                let mut row = 0usize;
                let mut line = 0usize;
                while line < self.document.line_count() {
                    let rows = rows_for(self.display_widths.get(line).copied().unwrap_or(0), columns);
                    if row + rows > start_row {
                        break;
                    }
                    row += rows;
                    line += 1;
                }

                let end_line = self.row_to_line(end_row.saturating_sub(1), columns).0;
                self.ensure_highlight_range(line, end_line);

                let (cursor_line, cursor_column) = self.document.line_col(self.cursor);
                let (selection_start, selection_end) = self.selection();
                let selection_start_line = self.document.line_col(selection_start).0;
                let selection_end_line = self.document.line_col(selection_end).0;

                while line < self.document.line_count() && row < end_row {
                    let rows = rows_for(self.display_widths.get(line).copied().unwrap_or(0), columns);
                    let line_start = self.document.line_start_char(line);
                    for segment in 0..rows {
                        if row >= end_row {
                            break;
                        }
                        let y = origin.y + row as f32 * row_height;
                        let row_rect = Rect::from_min_max(
                            Pos2::new(origin.x, y),
                            Pos2::new(origin.x + content_width, y + row_height),
                        );
                        if line == cursor_line {
                            painter.rect_filled(
                                Rect::from_min_max(
                                    Pos2::new(origin.x + gutter, y),
                                    Pos2::new(origin.x + content_width, y + row_height),
                                ),
                                0.0,
                                ui.visuals().faint_bg_color,
                            );
                        }
                        self.paint_brackets(
                            &painter, row_rect, line, line_start, segment, columns, gutter, char_width, origin,
                        );
                        self.paint_selection(
                            &painter,
                            row_rect,
                            line,
                            line_start,
                            segment,
                            columns,
                            selection_start,
                            selection_end,
                            selection_start_line,
                            selection_end_line,
                            gutter,
                            char_width,
                            origin,
                        );
                        if self.find.open {
                            self.paint_matches(
                                &painter, row_rect, line, line_start, segment, columns, gutter, char_width, origin,
                            );
                        }
                        if segment == 0 && gutter > 0.0 {
                            let number = (line + 1).to_string();
                            painter.text(
                                Pos2::new(origin.x + gutter - 8.0, y),
                                Align2::RIGHT_TOP,
                                number,
                                font.clone(),
                                ui.visuals().weak_text_color(),
                            );
                        }

                        let text = self.document.line(line);
                        let clip = Rect::from_min_max(
                            Pos2::new(origin.x + gutter, y),
                            Pos2::new(origin.x + content_width, y + row_height),
                        );
                        let row_painter = painter.with_clip_rect(clip);
                        let segment_shift = segment as f32 * columns as f32 * char_width;
                        let mut x = origin.x + gutter + 8.0 - segment_shift;
                        let spans = self
                            .highlight_cache
                            .line(line)
                            .map(ToOwned::to_owned)
                            .filter(|spans| !spans.is_empty())
                            .unwrap_or_else(|| {
                                vec![mdview_syntax::Span {
                                    text: text.clone(),
                                    color: None,
                                }]
                            });
                        for span in spans {
                            let shown = span.text.replace('\t', "    ");
                            if !shown.is_empty() {
                                let color = span
                                    .color
                                    .map_or_else(|| ui.visuals().text_color(), |[r, g, b]| Color32::from_rgb(r, g, b));
                                row_painter.text(Pos2::new(x, y), Align2::LEFT_TOP, &shown, font.clone(), color);
                                x += display_width(&span.text) as f32 * char_width;
                            }
                        }

                        let cursor_display = display_col(&text, cursor_column);
                        if line == cursor_line && segment == cursor_display / columns.max(1) {
                            let cursor_x = origin.x + gutter + 8.0 + cursor_display as f32 * char_width;
                            if cursor_x >= origin.x + gutter {
                                painter.line_segment(
                                    [Pos2::new(cursor_x, y + 2.0), Pos2::new(cursor_x, y + row_height - 2.0)],
                                    Stroke::new(2.0, ui.visuals().text_color()),
                                );
                            }
                        }
                        row += 1;
                    }
                    line += 1;
                }

                if self.scroll_to_cursor {
                    let cursor_row = self.row_for_cursor(columns);
                    let y = origin.y + cursor_row as f32 * row_height;
                    ui.scroll_to_rect(
                        Rect::from_min_size(Pos2::new(origin.x, y), Vec2::new(content_width, row_height)),
                        Some(Align::Center),
                    );
                    self.scroll_to_cursor = false;
                }

                let content_rect = painted.rect;
                painted.widget_info(|| egui::WidgetInfo::text_edit(ui.is_enabled(), "", "", "Code editor"));
                response = Some(painted);
                geometry = Some(Geometry {
                    content_rect,
                    row_height,
                    char_width,
                    gutter,
                    columns,
                });
            });

        let response = response.unwrap_or_else(|| ui.allocate_exact_size(Vec2::ZERO, Sense::hover()).1);
        let geometry = geometry.unwrap_or(Geometry {
            content_rect: response.rect,
            row_height,
            char_width,
            gutter,
            columns,
        });
        (response, geometry)
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_brackets(
        &self,
        painter: &egui::Painter,
        row_rect: Rect,
        line: usize,
        line_start: usize,
        segment: usize,
        columns: usize,
        gutter: f32,
        char_width: f32,
        origin: Pos2,
    ) {
        let Some((first, second)) = self.bracket_match else {
            return;
        };
        let text = self.document.line(line);
        let line_end = line_start + text.chars().count();
        for index in [first, second] {
            if index < line_start || index >= line_end {
                continue;
            }
            let column = index - line_start;
            let display = display_col(&text, column);
            let segment_start = segment * columns;
            let segment_end = if columns > 0 {
                segment_start + columns
            } else {
                usize::MAX
            };
            if display < segment_start || display >= segment_end {
                continue;
            }
            let x = origin.x + gutter + 8.0 + display as f32 * char_width;
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x, row_rect.min.y), Pos2::new(x + char_width, row_rect.max.y)),
                2.0,
                Color32::from_rgba_unmultiplied(110, 170, 255, 60),
            );
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_selection(
        &self,
        painter: &egui::Painter,
        row_rect: Rect,
        line: usize,
        line_start: usize,
        segment: usize,
        columns: usize,
        selection_start: usize,
        selection_end: usize,
        selection_start_line: usize,
        selection_end_line: usize,
        gutter: f32,
        char_width: f32,
        origin: Pos2,
    ) {
        if selection_start == selection_end || line < selection_start_line || line > selection_end_line {
            return;
        }
        let text = self.document.line(line);
        let start_col = selection_start.saturating_sub(line_start).min(text.chars().count());
        let end_col = selection_end.saturating_sub(line_start).min(text.chars().count());
        if start_col == end_col {
            return;
        }
        let segment_start = segment * columns;
        let segment_end = if columns > 0 {
            segment_start + columns
        } else {
            usize::MAX
        };
        let start = display_col(&text, start_col).max(segment_start);
        let end = display_col(&text, end_col).min(segment_end);
        if start >= end {
            return;
        }
        let x = origin.x + gutter + 8.0 + start as f32 * char_width;
        let width = (end - start) as f32 * char_width;
        painter.rect_filled(
            Rect::from_min_max(Pos2::new(x, row_rect.min.y), Pos2::new(x + width, row_rect.max.y)),
            0.0,
            self.selection_color(painter),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn paint_matches(
        &self,
        painter: &egui::Painter,
        row_rect: Rect,
        line: usize,
        line_start: usize,
        segment: usize,
        columns: usize,
        gutter: f32,
        char_width: f32,
        origin: Pos2,
    ) {
        let text = self.document.line(line);
        let line_end = line_start + text.chars().count();
        for (start, end) in &self.find.matches {
            if *end <= line_start || *start >= line_end {
                continue;
            }
            let start_col = start.saturating_sub(line_start).min(text.chars().count());
            let end_col = end.saturating_sub(line_start).min(text.chars().count());
            let segment_start = segment * columns;
            let segment_end = if columns > 0 {
                segment_start + columns
            } else {
                usize::MAX
            };
            let start = display_col(&text, start_col).max(segment_start);
            let end = display_col(&text, end_col).min(segment_end);
            if start >= end {
                continue;
            }
            let x = origin.x + gutter + 8.0 + start as f32 * char_width;
            let width = (end - start) as f32 * char_width;
            painter.rect_filled(
                Rect::from_min_max(Pos2::new(x, row_rect.min.y), Pos2::new(x + width, row_rect.max.y)),
                0.0,
                Color32::from_rgba_unmultiplied(245, 196, 70, 70),
            );
        }
    }

    fn selection_color(&self, painter: &egui::Painter) -> Color32 {
        painter.ctx().global_style().visuals.selection.bg_fill
    }

    fn total_rows(&self, columns: usize) -> usize {
        if columns == 0 {
            return self.document.line_count().max(1);
        }
        (0..self.document.line_count())
            .map(|line| rows_for(self.display_widths.get(line).copied().unwrap_or(0), columns))
            .sum::<usize>()
            .max(1)
    }

    fn row_for_cursor(&self, columns: usize) -> usize {
        let (line, column) = self.document.line_col(self.cursor);
        if columns == 0 {
            return line;
        }
        let mut row = 0;
        for index in 0..line {
            row += rows_for(self.display_widths.get(index).copied().unwrap_or(0), columns);
        }
        let text = self.document.line(line);
        row + display_col(&text, column) / columns.max(1)
    }
}

fn rows_for(width: usize, columns: usize) -> usize {
    if columns == 0 {
        1
    } else {
        width.max(1).div_ceil(columns)
    }
}
