//! Painting the editor viewport.

use egui::{Align2, Color32, FontId, Pos2, Rect, Response, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::input::Geometry;
use crate::scroll::CursorView;
use crate::{Editor, display_col};

impl Editor {
    /// Show the editor, handle input, and draw the go-to dialog.
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let started = std::time::Instant::now();
        let (response, geometry) = self.paint(ui);
        self.handle_input(ui, &response, &geometry);
        self.track_math_pointer(ui, &response, &geometry);
        self.show_goto(ui.ctx());
        self.last_frame_time = started.elapsed();
        response
    }

    /// Paint the editor and return its response and geometry.
    fn paint(&mut self, ui: &mut Ui) -> (Response, Geometry) {
        self.ensure_layout();
        self.ensure_language();
        self.ensure_brackets();

        let font = FontId::monospace(self.style.font_size);
        let natural_row_height = ui.ctx().fonts_mut(|fonts| fonts.row_height(&font));
        let row_height = natural_row_height.max(self.style.font_size * self.style.line_height);
        let char_width = self.char_advance(ui, &font);
        let digits = self.document.line_count().to_string().len().max(3);
        let gutter = if self.line_numbers {
            digits as f32 * char_width + 18.0
        } else {
            0.0
        };
        let available = ui.available_width().max(240.0);
        // The vertical bar is always allocated, so wrapped rows must fit beside it.
        let text_area = available - ui.spacing().scroll.allocated_width();
        let available_height = ui.available_height().max(160.0);
        let columns = if self.wrap && !self.large_mode {
            (((text_area - gutter - 24.0) / char_width).floor() as usize).max(8)
        } else {
            0
        };
        self.ensure_wrap(columns);
        let total_rows = self.total_rows(columns);
        let max_width = self.display_widths.iter().copied().max().unwrap_or(0);
        let content_width = if columns > 0 {
            available
        } else {
            (gutter + max_width as f32 * char_width + 40.0).max(available)
        };
        // Keep the content height independent of transient scrollbar allocation. The old
        // `.max(available_height)` made the maximum offset oscillate when the horizontal
        // scrollbar appeared, which looked like flicker at the end of the document.
        // Split view pins the editor and preview ends together; an editor that can
        // scroll past its last row would go blank while the preview still shows the
        // end of the document. Keep scroll-beyond for the single-pane editor only.
        let overscroll = if self.style.scroll_beyond_last_line && !self.split_mode {
            (available_height - row_height - crate::BOTTOM_CLEARANCE).max(0.0)
        } else {
            0.0
        };
        let content_height = total_rows as f32 * row_height + crate::BOTTOM_CLEARANCE + overscroll;

        let mut response = None;
        let mut geometry = None;
        let mut area = ScrollArea::both()
            .auto_shrink([false, false])
            // A transient horizontal scrollbar changes the inner height by a few points,
            // which makes the maximum offset oscillate at the end of the document. Keep
            // both bars allocated so the maximum is stable.
            .scroll_bar_visibility(egui::scroll_area::ScrollBarVisibility::AlwaysVisible)
            // Programmatic split-sync targets must not animate toward the target: the
            // animation overshoots at the end of the document and looks like flicker.
            .animated(false)
            .id_salt(("editor_scroll", self.identity));
        let mut requested = None;
        if let Some(offset) = self.restore_scroll.take() {
            requested = Some(offset);
        }
        if let Some((line, fraction)) = self.pending_line.take() {
            let row = self.line_to_row(line);
            requested = Some((row as f32 + fraction) * row_height);
        }
        if std::mem::take(&mut self.pending_end) {
            requested = Some((content_height - available_height).max(0.0));
        }
        let programmatic = requested.is_some();
        if let Some(offset) = requested {
            area = area.vertical_scroll_offset(offset);
        }
        // Reveal the cursor in this pass: computing the offset here (instead of asking
        // egui to scroll after painting) avoids a frame drawn at the old offset, and a
        // minimal scroll keeps the text still while typing near the viewport edge.
        if std::mem::take(&mut self.scroll_to_cursor) {
            let view = CursorView {
                columns,
                row_height,
                char_width,
                gutter,
                content_height,
                available: Vec2::new(available, available_height),
            };
            let (vertical, horizontal) = self.cursor_scroll_target(&view);
            if !programmatic && let Some(offset) = vertical {
                area = area.vertical_scroll_offset(offset);
            }
            if let Some(offset) = horizontal {
                area = area.horizontal_scroll_offset(offset);
            }
            self.center_cursor = false;
        }
        let mut painted_offset = 0.0_f32;
        let output = area.show_viewport(ui, |ui, viewport| {
            painted_offset = viewport.min.y;
            let (painted, painter) =
                ui.allocate_painter(Vec2::new(content_width, content_height), Sense::click_and_drag());
            let origin = painted.rect.min;
            let start_row = (viewport.min.y / row_height).floor().max(0.0) as usize;
            let end_row = ((viewport.max.y / row_height).ceil() as usize)
                .saturating_add(1)
                .min(total_rows);

            painter.rect_filled(painted.rect, 0.0, ui.visuals().extreme_bg_color);

            let mut row = 0usize;
            let mut line = 0usize;
            while line < self.document.line_count() {
                let rows = self.rows_of(line, columns);
                if row + rows > start_row {
                    break;
                }
                row += rows;
                line += 1;
            }

            let end_line = self.row_to_line(end_row.saturating_sub(1), columns).0;
            let highlight_start = std::time::Instant::now();
            self.ensure_highlight_range(line, end_line);
            if highlight_start.elapsed().as_millis() > 10 {
                eprintln!(
                    "editor profile: highlight {:?} lines {}..{}",
                    highlight_start.elapsed(),
                    line,
                    end_line
                );
            }

            let (cursor_line, cursor_column) = self.document.line_col(self.cursor);
            let (selection_start, selection_end) = self.selection();
            let selection_start_line = self.document.line_col(selection_start).0;
            let selection_end_line = self.document.line_col(selection_end).0;

            while line < self.document.line_count() && row < end_row {
                let rows = self.rows_of(line, columns);
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
                                Pos2::new(origin.x, y),
                                Pos2::new(origin.x + content_width, y + row_height),
                            ),
                            0.0,
                            ui.visuals().selection.bg_fill.gamma_multiply(0.18),
                        );
                    }
                    // Paint only this row's columns. Shifting the whole line left and clipping
                    // at the content edge repeated the end of each wrapped row on the next.
                    let segment_start = self.segment_start(line, segment, columns);
                    let segment_end = self.segment_end(line, segment, columns);
                    let paint_row = crate::paint::RowPaint {
                        painter: &painter,
                        row_rect,
                        line,
                        line_start,
                        segment_start,
                        segment_end,
                        gutter,
                        char_width,
                        origin,
                        y,
                        row_height,
                    };
                    if self.style.indent_guides {
                        self.paint_indent_guides(&paint_row);
                    }
                    self.paint_brackets(&paint_row);
                    self.paint_marked(&paint_row);
                    self.paint_selection(
                        &paint_row,
                        selection_start,
                        selection_end,
                        selection_start_line,
                        selection_end_line,
                    );
                    if self.find.open {
                        self.paint_matches(&paint_row);
                    }
                    if segment == 0 && gutter > 0.0 {
                        let number = (line + 1).to_string();
                        let color = if line == cursor_line {
                            ui.visuals().text_color()
                        } else {
                            ui.visuals().weak_text_color()
                        };
                        painter.text(
                            Pos2::new(origin.x + gutter - 8.0, y),
                            Align2::RIGHT_TOP,
                            number,
                            font.clone(),
                            color,
                        );
                    }

                    let text = self.document.line(line);
                    let clip = Rect::from_min_max(
                        Pos2::new(origin.x + gutter, y),
                        Pos2::new(origin.x + content_width, y + row_height),
                    );
                    let row_painter = painter.with_clip_rect(clip);
                    if self.style.whitespace != crate::WhitespaceMode::None {
                        self.paint_whitespace(&paint_row, &text, selection_start, selection_end, ui.visuals());
                    }
                    let segment_shift = segment_start as f32 * char_width;
                    let text_x = origin.x + gutter + 8.0 - segment_shift;
                    let mut display = 0usize;
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
                        let color = span
                            .color
                            .map_or_else(|| ui.visuals().text_color(), |[r, g, b]| Color32::from_rgb(r, g, b));
                        let span_text = span.text.trim_end_matches(['\r', '\n']);
                        let mut run = TextRun::default();
                        for character in span_text.chars() {
                            let width = crate::cell_width(character, display);
                            if display >= segment_start && display < segment_end {
                                if character == '\t' {
                                    run.push(display, &" ".repeat(width));
                                } else if width == 2 {
                                    // A wide glyph is centred in its two cells, so the grid
                                    // after it stays aligned whatever its advance.
                                    run.flush(&row_painter, text_x, y, char_width, &font, color);
                                    let x = text_x + (display + 1) as f32 * char_width;
                                    let glyph = character.to_string();
                                    row_painter.text(Pos2::new(x, y), Align2::CENTER_TOP, glyph, font.clone(), color);
                                } else {
                                    run.push(display, character.encode_utf8(&mut [0; 4]));
                                }
                            } else {
                                run.flush(&row_painter, text_x, y, char_width, &font, color);
                            }
                            display += width;
                        }
                        run.flush(&row_painter, text_x, y, char_width, &font, color);
                        if display >= segment_end {
                            break;
                        }
                    }

                    let cursor_display = display_col(&text, cursor_column);
                    if line == cursor_line && segment == self.segment_at(line, cursor_display, columns) {
                        let cursor_x = text_x + cursor_display as f32 * char_width;
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

            let content_rect = painted.rect;
            // Describe the widget every frame: accesskit rebuilds its tree each frame, and a
            // node described only after an edit has no name on the other frames. The closure
            // runs only when accessibility or an output event needs the text.
            let document = &self.document;
            painted.widget_info(|| {
                let text = if document.len_chars() <= 100_000 {
                    document.snapshot()
                } else {
                    String::new()
                };
                let mut info = egui::WidgetInfo::text_edit(ui.is_enabled(), "", &text, "Code editor");
                info.label = Some("Code editor".to_owned());
                info
            });
            let selection = self.selection();
            if self.announced_selection != Some(selection) {
                self.announced_selection = Some(selection);
                painted.output_event(egui::output::OutputEvent::TextSelectionChanged(
                    egui::WidgetInfo::text_selection_changed(
                        ui.is_enabled(),
                        egui::text::CharIndex(selection.0)..egui::text::CharIndex(selection.1),
                        "",
                    ),
                ));
            }
            response = Some(painted);
            geometry = Some(Geometry {
                content_rect,
                row_height,
                char_width,
                gutter,
                columns,
            });
        });
        let new_scroll = output.state.clamped_offset().y;
        if !programmatic && (new_scroll - self.scroll_y).abs() > 0.5 {
            self.user_scrolled = true;
            self.last_scroll_at = std::time::Instant::now();
        }
        self.scroll_y = new_scroll;
        self.scroll_x = output.state.clamped_offset().x;
        // Use the sizes egui clamps with (rounded to the pixel grid), so a target at this
        // maximum is stored exactly and split sync never chases a sub-pixel residue.
        self.last_max_offset = (output.content_size.y - output.inner_rect.height()).max(0.0);
        // egui lays out with the offset it had before this pass and clamps it only at the
        // end, so a zoom, resize, or out-of-range request can paint one pass past the end.
        // Discard that pass so it is never shown; egui repaints it in range immediately.
        if crate::scroll::overshot(painted_offset, &output.state, self.last_max_offset) {
            ui.ctx().request_discard("editor painted past its scroll end");
            if !ui.ctx().will_discard() {
                // Out of passes: this frame is shown past the end.
                self.overshoot_paints += 1;
            }
        }
        self.last_bottom_clearance = crate::BOTTOM_CLEARANCE;
        self.last_last_row_top = total_rows.saturating_sub(1) as f32 * row_height - new_scroll;
        self.last_row_height = row_height;
        self.last_columns = columns;
        self.viewport_rect = output.inner_rect;
        self.programmatic_scroll = false;

        let response = response.unwrap_or_else(|| ui.allocate_exact_size(Vec2::ZERO, Sense::hover()).1);
        self.show_find_overlay(ui, self.viewport_rect);
        let geometry = geometry.unwrap_or(Geometry {
            content_rect: response.rect,
            row_height,
            char_width,
            gutter,
            columns,
        });
        (response, geometry)
    }

    fn total_rows(&self, columns: usize) -> usize {
        if columns == 0 {
            return self.document.line_count().max(1);
        }
        (0..self.document.line_count())
            .map(|line| self.rows_of(line, columns))
            .sum::<usize>()
            .max(1)
    }

    pub(crate) fn row_for_cursor(&self, columns: usize) -> usize {
        let (line, column) = self.document.line_col(self.cursor);
        if columns == 0 {
            return line;
        }
        let mut row = 0;
        for index in 0..line {
            row += self.rows_of(index, columns);
        }
        let text = self.document.line(line);
        row + self.segment_at(line, display_col(&text, column), columns)
    }
}

impl Editor {
    /// The advance of one monospace character as the text layout places it.
    ///
    /// `glyph_width` reports the font's unrounded advance, while laid-out text can
    /// advance by a slightly different amount; over a long line the difference moved
    /// the cursor and selection away from the text. Measure a run of glyphs instead.
    fn char_advance(&mut self, ui: &Ui, font: &FontId) -> f32 {
        let key = (font.size, ui.ctx().pixels_per_point());
        if let Some((cached, width)) = self.char_advance_cache
            && cached == key
        {
            return width;
        }
        const RUN: usize = 64;
        let galley = ui
            .ctx()
            .fonts_mut(|fonts| fonts.layout_no_wrap("0".repeat(RUN), font.clone(), Color32::WHITE));
        let measured = galley.size().x / RUN as f32;
        let width = if measured.is_finite() && measured > 0.0 {
            measured
        } else {
            ui.ctx().fonts_mut(|fonts| fonts.glyph_width(font, ' '))
        };
        self.char_advance_cache = Some((key, width));
        width
    }
}

/// Consecutive single-cell characters painted as one string.
#[derive(Default)]
struct TextRun {
    start: usize,
    text: String,
}

impl TextRun {
    fn push(&mut self, display: usize, text: &str) {
        if self.text.is_empty() {
            self.start = display;
        }
        self.text.push_str(text);
    }

    fn flush(&mut self, painter: &egui::Painter, text_x: f32, y: f32, char_width: f32, font: &FontId, color: Color32) {
        if self.text.is_empty() {
            return;
        }
        let x = text_x + self.start as f32 * char_width;
        painter.text(
            Pos2::new(x, y),
            Align2::LEFT_TOP,
            std::mem::take(&mut self.text),
            font.clone(),
            color,
        );
    }
}
