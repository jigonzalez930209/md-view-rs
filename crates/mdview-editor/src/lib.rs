//! A minimal text editor for milestone H1.
//!
//! The custom virtualized editor, gutter, syntax highlighting, and large-file mode arrive in H2.
//! This module intentionally wraps egui's multiline text editor so the application shell can be
//! completed and tested first.

use egui::{Response, ScrollArea, TextEdit};
use mdview_model::Document;

/// Show the document editor and mark the document dirty when the text changes.
pub fn show(ui: &mut egui::Ui, document: &mut Document) -> Response {
    let response = ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.add(
                TextEdit::multiline(document.text_mut())
                    .font(egui::TextStyle::Monospace)
                    .code_editor()
                    .desired_width(f32::INFINITY)
                    .desired_rows(30)
                    .lock_focus(true),
            )
        })
        .inner;

    if response.changed() {
        document.mark_dirty();
    }
    response
}

/// Show a read-only preview for a document that has no editor yet.
pub fn show_read_only(ui: &mut egui::Ui, document: &Document) {
    ScrollArea::vertical().auto_shrink([false, false]).show(ui, |ui| {
        ui.add(egui::Label::new(egui::RichText::new(document.text()).monospace()).wrap());
    });
}
