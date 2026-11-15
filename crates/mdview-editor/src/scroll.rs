//! Scroll offsets, line mapping, and user-scroll reporting.

use crate::Editor;
use crate::render::rows_for;

impl Editor {
    /// Give the editor a stable identity for its scroll area.
    pub fn set_identity(&mut self, identity: u64) {
        self.identity = identity;
    }

    /// The stable identity used for scroll and focus state.
    pub fn identity(&self) -> u64 {
        self.identity
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

    /// The first logical line in the visible viewport.
    pub fn first_visible_line(&self) -> usize {
        let row_height = self.last_row_height.max(1.0);
        let row = (self.scroll_y / row_height).floor().max(0.0) as usize;
        self.row_to_line(row, self.last_columns).0
    }

    /// Scroll to a zero-based logical line with an intra-line fraction.
    pub fn scroll_to_line(&mut self, line: usize, fraction: f32) {
        self.pending_line = Some((line, fraction.clamp(0.0, 1.0)));
        self.programmatic_scroll = true;
    }

    /// Scroll to the maximum offset on the next frame.
    pub fn scroll_to_end(&mut self) {
        self.pending_end = true;
        self.programmatic_scroll = true;
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
        (0..line)
            .map(|index| rows_for(self.display_widths.get(index).copied().unwrap_or(0), self.last_columns))
            .sum()
    }
}
