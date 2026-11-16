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
}
