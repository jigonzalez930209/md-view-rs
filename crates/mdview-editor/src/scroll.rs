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

    /// Whether the user scrolled this frame, consuming the flag.
    pub fn take_user_scroll(&mut self) -> bool {
        std::mem::take(&mut self.user_scrolled)
    }

    /// The editor viewport rectangle from the last paint.
    pub fn viewport_rect(&self) -> egui::Rect {
        self.viewport_rect
    }

    /// The row index of a zero-based logical line in the current wrap layout.
    pub fn line_to_row(&self, line: usize) -> usize {
        let line = line.min(self.document.line_count().saturating_sub(1));
        (0..line)
            .map(|index| rows_for(self.display_widths.get(index).copied().unwrap_or(0), self.last_columns))
            .sum()
    }
}
