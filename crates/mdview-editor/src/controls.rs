//! Editor-local controls that match the workbench field geometry.

use egui::{Align2, Frame, Margin, Response, Stroke, TextEdit, Ui};

/// A single-line field with centered text and the shared 30-point height.
pub(crate) fn text_field(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    let border = ui.visuals().widgets.inactive.bg_stroke.color;
    let output = Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(8, 0))
        .show(ui, |ui| {
            ui.add_sized(
                egui::vec2((width - 16.0).max(40.0), 30.0),
                TextEdit::singleline(value)
                    .hint_text(hint)
                    .frame(Frame::NONE)
                    .margin(Margin::symmetric(0, 7))
                    .align(Align2::LEFT_CENTER)
                    .desired_width(f32::INFINITY),
            )
        });
    if output.inner.has_focus() {
        ui.painter().rect_stroke(
            output.response.rect,
            6,
            Stroke::new(1.5, ui.visuals().selection.bg_fill),
            egui::StrokeKind::Inside,
        );
    }
    output.inner
}

/// Whether Enter was pressed in a single-line field this frame.
///
/// A single-line `TextEdit` gives up focus when Enter is pressed, so in that frame it
/// reports `lost_focus` and no longer `has_focus`; checking only `has_focus` never fires.
pub(crate) fn submitted(ui: &Ui, field: &Response) -> bool {
    (field.has_focus() || field.lost_focus()) && ui.input(|input| input.key_pressed(egui::Key::Enter))
}

/// A compact up/down triangle used by numeric fields.
pub(crate) fn step_button(ui: &mut Ui, up: bool) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(18.0, 15.0), egui::Sense::click());
    if response.hovered() {
        ui.painter().rect_filled(
            rect,
            egui::CornerRadius::same(3),
            ui.visuals().widgets.hovered.weak_bg_fill,
        );
    }
    let center = rect.center();
    let points = if up {
        vec![
            egui::pos2(center.x, center.y - 3.5),
            egui::pos2(center.x - 4.5, center.y + 2.5),
            egui::pos2(center.x + 4.5, center.y + 2.5),
        ]
    } else {
        vec![
            egui::pos2(center.x - 4.5, center.y - 2.5),
            egui::pos2(center.x + 4.5, center.y - 2.5),
            egui::pos2(center.x, center.y + 3.5),
        ]
    };
    ui.painter().add(egui::Shape::convex_polygon(
        points,
        ui.visuals().text_color(),
        Stroke::NONE,
    ));
    response.clicked()
}
