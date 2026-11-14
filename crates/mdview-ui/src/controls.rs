//! Shared controls that keep the workbench visually consistent.
//!
//! Every text field uses the same sunken surface, height, padding, and vertical text alignment.
//! Primary, secondary, and danger buttons share one geometry so dialogs never mix baselines.

use egui::{Button, Color32, Frame, Margin, Response, RichText, Stroke, TextEdit, Ui};

use crate::MaterialIcons;

/// Height used by text fields, buttons, combo boxes, and steppers.
pub const CONTROL_HEIGHT: f32 = 30.0;

/// A single-line text field with centered text and a consistent sunken frame.
pub fn text_field(ui: &mut Ui, value: &mut String, hint: &str, width: f32) -> Response {
    let border = ui.visuals().widgets.inactive.bg_stroke.color;
    let frame = Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(6)
        .inner_margin(Margin::symmetric(8, 0));
    let output = frame.show(ui, |ui| {
        ui.add_sized(
            egui::vec2((width - 16.0).max(40.0), CONTROL_HEIGHT),
            TextEdit::singleline(value)
                .hint_text(hint)
                .frame(Frame::NONE)
                .margin(Margin::symmetric(0, 7))
                .align(egui::Align2::LEFT_CENTER)
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

/// The primary action in a dialog.
pub fn primary_button(ui: &mut Ui, label: &str, danger: bool) -> Response {
    let (fill, text) = if danger {
        (ui.visuals().error_fg_color, Color32::WHITE)
    } else {
        (ui.visuals().selection.bg_fill, ui.visuals().selection.stroke.color)
    };
    ui.add(
        Button::new(RichText::new(label).size(13.0).color(text))
            .fill(fill)
            .stroke(Stroke::NONE)
            .corner_radius(6)
            .min_size(egui::vec2(92.0, CONTROL_HEIGHT)),
    )
}

/// A neutral action in a dialog.
pub fn secondary_button(ui: &mut Ui, label: &str) -> Response {
    ui.add(
        Button::new(RichText::new(label).size(13.0).color(ui.visuals().text_color()))
            .fill(ui.visuals().widgets.inactive.weak_bg_fill)
            .stroke(Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color))
            .corner_radius(6)
            .min_size(egui::vec2(88.0, CONTROL_HEIGHT)),
    )
}

/// A numeric text field with up/down arrows.
pub fn numeric_stepper(
    ui: &mut Ui,
    icons: &mut MaterialIcons,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    step: f32,
    width: f32,
    format: impl Fn(f32) -> String,
) -> bool {
    let mut changed = false;
    let border = ui.visuals().widgets.inactive.bg_stroke.color;
    let frame = Frame::new()
        .fill(ui.visuals().extreme_bg_color)
        .stroke(Stroke::new(1.0, border))
        .corner_radius(6);
    let rect = frame
        .show(ui, |ui| {
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, CONTROL_HEIGHT), egui::Sense::hover());
            ui.painter().text(
                egui::pos2(rect.left() + 9.0, rect.center().y),
                egui::Align2::LEFT_CENTER,
                format(*value),
                egui::FontId::proportional(13.0),
                ui.visuals().text_color(),
            );
            rect
        })
        .inner;
    let button_w = 22.0;
    let up = egui::Rect::from_min_size(
        egui::pos2(rect.right() - button_w, rect.top() + 1.0),
        egui::vec2(button_w, CONTROL_HEIGHT / 2.0 - 1.0),
    );
    let down = egui::Rect::from_min_size(
        egui::pos2(rect.right() - button_w, rect.center().y),
        egui::vec2(button_w, CONTROL_HEIGHT / 2.0 - 1.0),
    );
    let stepper_id = ("stepper", rect.left().to_bits(), rect.top().to_bits());
    let up_response = ui.interact(up, ui.make_persistent_id(stepper_id).with("up"), egui::Sense::click());
    let down_response = ui.interact(
        down,
        ui.make_persistent_id(stepper_id).with("down"),
        egui::Sense::click(),
    );
    for (response, icon, delta) in [
        (up_response, "chevron-up", step),
        (down_response, "chevron-down", -step),
    ] {
        if response.hovered() {
            ui.painter().rect_filled(
                response.rect,
                egui::CornerRadius::same(3),
                ui.visuals().widgets.hovered.weak_bg_fill,
            );
        }
        if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
            let icon_rect = egui::Rect::from_center_size(response.rect.center(), egui::vec2(11.0, 11.0));
            ui.painter().image(
                texture.id(),
                icon_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                ui.visuals().weak_text_color(),
            );
        }
        if response.clicked() {
            *value = (*value + delta).clamp(*range.start(), *range.end());
            changed = true;
        }
    }
    changed
}

/// Apply the compact typography used by context menus and popups.
pub fn compact_menu(ui: &mut Ui) {
    ui.spacing_mut().item_spacing.y = 1.0;
    ui.spacing_mut().button_padding = egui::vec2(8.0, 2.0);
    ui.style_mut()
        .text_styles
        .insert(egui::TextStyle::Button, egui::FontId::proportional(12.0));
    ui.style_mut().spacing.menu_margin = Margin::same(4);
}
