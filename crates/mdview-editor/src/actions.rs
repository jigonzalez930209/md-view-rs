//! Editor actions that edit text or move keyboard focus.

use crate::Editor;

impl Editor {
    /// Ask the editor to take keyboard focus on its next paint.
    pub fn request_focus(&mut self) {
        self.focus_requested = true;
    }

    /// Drop a pending focus request, for example when a find field must own focus.
    pub fn clear_focus_request(&mut self) {
        self.focus_requested = false;
    }

    /// Insert a display-math block on its own lines and place the cursor inside it.
    pub fn insert_latex_block(&mut self) {
        let (line, _) = self.document.line_col(self.cursor);
        let line_start = self.document.line_start_char(line);
        let line_end = line_start + self.document.line_len(line);
        let prefix = if self.cursor == line_start { "" } else { "\n" };
        let suffix = if self.cursor == line_end { "" } else { "\n" };
        let start = self.selection().0;
        let block = format!("{prefix}$$\n\n$${suffix}");
        self.replace_selection(&block);
        self.cursor = start + prefix.chars().count() + 3;
        self.anchor = self.cursor;
        self.scroll_to_cursor = true;
        self.cursor_blink_start = std::time::Instant::now();
        self.focus_requested = true;
    }
}
