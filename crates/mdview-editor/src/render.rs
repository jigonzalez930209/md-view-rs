//! Painting the editor viewport.

use egui::{Align, Align2, Color32, FontId, Pos2, Rect, Response, ScrollArea, Sense, Stroke, Ui, Vec2};

use crate::input::Geometry;
use crate::{Editor, display_col, display_width};

impl Editor {
    /// Show the editor, handle input, and draw the go-to dialog.
    pub fn show(&mut self, ui: &mut Ui) -> Response {
        let started = std::time::Instant::now();
        let (response, geometry) = self.paint(ui);
        self.handle_input(ui, &response, &geometry);
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
                let rows = rows_for(self.display_widths.get(line).copied().unwrap_or(0), columns);
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
                                Pos2::new(origin.x, y),
                                Pos2::new(origin.x + content_width, y + row_height),
                            ),
                            0.0,
                            ui.visuals().selection.bg_fill.gamma_multiply(0.18),
                        );
                    }
                    let paint_row = crate::paint::RowPaint {
                        painter: &painter,
                        row_rect,
                        line,
                        line_start,
                        segment,
                        columns,
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
            let revision = self.document.revision();
            if self.accessibility_revision != Some(revision) {
                let text = if self.document.len_chars() <= 100_000 {
                    self.document.snapshot()
                } else {
                    String::new()
                };
                painted.widget_info(|| egui::WidgetInfo::text_edit(ui.is_enabled(), "", &text, "Code editor"));
                self.accessibility_revision = Some(revision);
            }
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
        row + display_col(&text, column).saturating_sub(1) / columns
    }
}

pub(crate) fn rows_for(width: usize, columns: usize) -> usize {
    if columns == 0 {
        1
    } else {
        width.max(1).div_ceil(columns)
    }
}
