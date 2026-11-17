//! Scroll offsets, line mapping, and user-scroll reporting.

use crate::Editor;

/// Offsets closer than this (in points) count as already synchronized. egui stores an
/// explicit offset exactly and the maximum offset matches egui's pixel-rounded sizes, so
/// targets land exactly; this only absorbs float noise.
const SYNC_TOLERANCE: f32 = 0.001;

/// Whether a pass painted past the scrollable end by accident.
///
/// Trackpad rubber-banding paints past the end on purpose and keeps the stored offset
/// unclamped; only a pass whose stored offset egui clamped back into range overshot.
pub(crate) fn overshot(painted: f32, state: &egui::scroll_area::State, max_offset: f32) -> bool {
    painted > max_offset + 0.5 && state.unclamped_offset().y <= max_offset + 0.5
}

/// The geometry needed to reveal the cursor before the scroll area is shown.
pub(crate) struct CursorView {
    pub(crate) columns: usize,
    pub(crate) row_height: f32,
    pub(crate) char_width: f32,
    pub(crate) gutter: f32,
    pub(crate) content_height: f32,
    /// The size available to the scroll area, used before the first paint.
    pub(crate) available: egui::Vec2,
}

impl Editor {
    /// The vertical and horizontal offsets that bring the cursor into view.
    ///
    /// Each is `None` when the cursor is already visible on that axis. The cursor
    /// keeps a row of context above and below and a few characters to the side.
    /// Navigation (`center_cursor`) puts an off-screen cursor in the middle instead.
    pub(crate) fn cursor_scroll_target(&self, view: &CursorView) -> (Option<f32>, Option<f32>) {
        let size = if self.viewport_rect.is_finite() && self.viewport_rect.height() > 0.0 {
            self.viewport_rect.size()
        } else {
            view.available
        };
        let row_top = self.row_for_cursor(view.columns) as f32 * view.row_height;
        let row_bottom = row_top + view.row_height;
        let max_y = (view.content_height - size.y).max(0.0);
        let margin = view.row_height.min((size.y - view.row_height).max(0.0) / 2.0);
        let top = self.scroll_y;
        let visible = row_top >= top && row_bottom <= top + size.y;
        let vertical = if self.center_cursor && !visible {
            Some(row_top + view.row_height / 2.0 - size.y / 2.0)
        } else if row_top - margin < top {
            Some(row_top - margin)
        } else if row_bottom + margin > top + size.y {
            Some(row_bottom + margin - size.y)
        } else {
            None
        }
        .map(|offset| offset.clamp(0.0, max_y))
        .filter(|offset| (offset - top).abs() > 0.01);

        let horizontal = if view.columns > 0 {
            // Soft wrap: there is no horizontal scrolling to do.
            (self.scroll_x > 0.0).then_some(0.0)
        } else {
            let (line, column) = self.document.line_col(self.cursor);
            let display = crate::display_col(&self.document.line(line), column);
            let x = view.gutter + 8.0 + display as f32 * view.char_width;
            let side = (4.0 * view.char_width).min(size.x / 4.0);
            if x + side <= size.x {
                // Keep the line numbers in view whenever the cursor fits without them gone.
                (self.scroll_x > 0.0).then_some(0.0)
            } else if x - side < self.scroll_x + view.gutter {
                Some((x - side - view.gutter).max(0.0))
            } else if x + side > self.scroll_x + size.x {
                Some(x + side - size.x)
            } else {
                None
            }
        };
        (vertical, horizontal)
    }

    /// Give the editor a stable identity for its scroll area.
    pub fn set_identity(&mut self, identity: u64) {
        self.identity = identity;
    }

    /// The stable identity used for scroll and focus state.
    pub fn identity(&self) -> u64 {
        self.identity
    }

    /// Clamp the editor's scroll range to the content end.
    ///
    /// Split view aligns the editor with a preview that cannot overscroll; allowing
    /// the editor to scroll the last row to the top would leave it blank while the
    /// preview still shows the end of the document. The setting stays in effect for
    /// the single-pane editor.
    pub fn set_split_mode(&mut self, split: bool) {
        self.split_mode = split;
    }

    /// Stop the editor from processing keyboard and pointer input for this frame.
    ///
    /// The app sets this while a keyboard overlay (the menu navigator or a dialog)
    /// owns the keys, so Enter and the arrow keys cannot also edit the document.
    pub fn set_ignore_input(&mut self, ignore: bool) {
        self.ignore_input = ignore;
    }

    /// The last vertical scroll offset.
    pub fn scroll_offset(&self) -> f32 {
        self.scroll_y
    }

    /// Restore a vertical scroll offset on the next frame.
    pub fn set_scroll_offset(&mut self, offset: f32) {
        self.restore_scroll = Some(offset.max(0.0));
        self.programmatic_scroll = true;
    }

    /// Scroll to a content-space offset for split synchronization.
    ///
    /// The target is clamped to the scrollable range from the last paint. An
    /// out-of-range request would be painted unclamped for one frame and then
    /// snap back, which shows as flicker at the end of the document. Returns
    /// `false` when the editor is already at the clamped target.
    pub fn sync_scroll_to(&mut self, offset: f32) -> bool {
        let target = offset.clamp(0.0, self.last_max_offset.max(0.0));
        // egui stores an explicit offset exactly, so a tight tolerance settles in one
        // frame and leaves no visible gap between the panes.
        if !target.is_finite() || (target - self.scroll_y).abs() <= SYNC_TOLERANCE {
            return false;
        }
        self.pending_line = None;
        self.pending_end = false;
        self.restore_scroll = Some(target);
        self.programmatic_scroll = true;
        true
    }

    /// How many frames were shown painted past the scrollable end.
    ///
    /// A pass that paints past the end (after a zoom, a resize, or an out-of-range
    /// request) is discarded and repainted in range before it is shown; only a pass
    /// that could not be discarded counts. Such a frame is visible as flicker.
    pub fn overshoot_paints(&self) -> u64 {
        self.overshoot_paints
    }

    /// The content-space top of every logical line in the last wrap layout.
    ///
    /// The returned vector has `line_count + 1` entries; the last one is the
    /// bottom of the final line.
    pub fn line_tops(&self) -> Vec<f32> {
        let row_height = self.last_row_height.max(1.0);
        let lines = self.document.line_count();
        let mut tops = Vec::with_capacity(lines + 1);
        let mut rows = 0usize;
        tops.push(0.0);
        for line in 0..lines {
            rows += self.rows_of(line, self.last_columns);
            tops.push(rows as f32 * row_height);
        }
        tops
    }

    /// The first logical line in the visible viewport.
    pub fn first_visible_line(&self) -> usize {
        self.first_visible_line_fraction().0
    }

    /// The first visible source line and the fraction inside that line.
    pub fn first_visible_line_fraction(&self) -> (usize, f32) {
        let row_height = self.last_row_height.max(1.0);
        let position = (self.scroll_y / row_height).max(0.0);
        let row = position.floor() as usize;
        let (line, _) = self.row_to_line(row, self.last_columns);
        let line_row = self.line_to_row(line);
        let rows = self.rows_of(line, self.last_columns);
        let within_row = position - position.floor();
        let fraction = ((row.saturating_sub(line_row)) as f32 + within_row) / rows as f32;
        (line, fraction.clamp(0.0, 1.0))
    }

    /// Scroll to a zero-based logical line with an intra-line fraction.
    ///
    /// Returns `false` when the editor is already at that offset, which prevents a
    /// synchronized pane from repainting the same target every frame.
    pub fn scroll_to_line(&mut self, line: usize, fraction: f32) -> bool {
        let fraction = fraction.clamp(0.0, 1.0);
        let target = (self.line_to_row(line) as f32 + fraction) * self.last_row_height.max(1.0);
        if (target - self.scroll_y).abs() <= 1.0 {
            return false;
        }
        self.pending_line = Some((line, fraction));
        self.programmatic_scroll = true;
        true
    }

    /// Scroll to the maximum offset on the next frame.
    ///
    /// Returns `false` when the editor is already at the end.
    pub fn scroll_to_end(&mut self) -> bool {
        if self.is_at_end() {
            return false;
        }
        self.pending_end = true;
        self.programmatic_scroll = true;
        true
    }

    /// Whether the editor is at the end of its scrollable range.
    pub fn is_at_end(&self) -> bool {
        self.last_max_offset > 0.0 && self.scroll_y >= self.last_max_offset - 2.0
    }

    /// The largest vertical offset from the last paint.
    pub fn max_scroll_offset(&self) -> f32 {
        self.last_max_offset
    }

    /// Space below the last text row for overlay scrollbars.
    pub fn bottom_clearance(&self) -> f32 {
        self.last_bottom_clearance
    }

    /// Whether a user scroll changed the offset recently.
    pub fn scrolling_recently(&self, window: std::time::Duration) -> bool {
        self.last_scroll_at.elapsed() <= window
    }

    /// The final row's top relative to the viewport top, in points.
    ///
    /// At the maximum offset with scroll-beyond-last-line enabled this is
    /// approximately zero, proving the final row can reach the viewport top.
    pub fn last_row_screen_top(&self) -> f32 {
        self.last_last_row_top
    }

    /// Whether the user scrolled this frame, consuming the flag.
    pub fn take_user_scroll(&mut self) -> bool {
        std::mem::take(&mut self.user_scrolled)
    }

    /// The editor viewport rectangle from the last paint.
    pub fn viewport_rect(&self) -> egui::Rect {
        self.viewport_rect
    }

    /// CPU time spent painting and handling input in the last frame.
    pub fn last_frame_time(&self) -> std::time::Duration {
        self.last_frame_time
    }

    /// The row index of a zero-based logical line in the current wrap layout.
    pub fn line_to_row(&self, line: usize) -> usize {
        let line = line.min(self.document.line_count().saturating_sub(1));
        (0..line).map(|index| self.rows_of(index, self.last_columns)).sum()
    }
}
