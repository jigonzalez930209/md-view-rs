//! The `↩` footnote back link, drawn as a vector glyph.
//!
//! GitHub ends each footnote with `↩`. The bundled reading fonts have no arrow glyphs,
//! so the arrow is painted as a hook-shaped path in the link color.

use egui::{Sense, Ui};

use crate::PreviewOutput;

/// Footnote back-link target, if the destination is one.
pub(super) fn target(destination: &str) -> Option<&str> {
    destination.strip_prefix("#fnref-")
}

/// Paint a clickable `↩` sized to the body text and return its rectangle.
pub(super) fn show(
    ui: &mut Ui,
    label: &str,
    size: f32,
    row: f32,
    color: egui::Color32,
    output: &mut PreviewOutput,
) -> egui::Rect {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(size * 0.85, row), Sense::click());
    let em = size * 0.5;
    let center = rect.center() + egui::vec2(0.0, size * 0.05);
    let stroke = egui::Stroke::new((size * 0.08).max(1.0), color);
    // Hook: from the top right, down the right side, back left along the bottom.
    let right = center.x + em * 0.55;
    let left = center.x - em * 0.55;
    let top = center.y - em * 0.55;
    let bottom = center.y + em * 0.25;
    let mut points = vec![egui::pos2(left + em * 0.35, top)];
    for step in 0..=8 {
        let angle = -std::f32::consts::FRAC_PI_2 + std::f32::consts::PI * step as f32 / 8.0;
        let radius = (bottom - top) * 0.5;
        points.push(egui::pos2(
            right - radius + angle.cos() * radius,
            top + radius + angle.sin() * radius,
        ));
    }
    points.push(egui::pos2(left, bottom));
    ui.painter().add(egui::Shape::line(points, stroke));
    let head = em * 0.35;
    ui.painter().line_segment(
        [egui::pos2(left, bottom), egui::pos2(left + head, bottom - head)],
        stroke,
    );
    ui.painter().line_segment(
        [egui::pos2(left, bottom), egui::pos2(left + head, bottom + head)],
        stroke,
    );
    let response = response
        .on_hover_text("Back to reference")
        .on_hover_cursor(egui::CursorIcon::PointingHand);
    if response.clicked() {
        output.anchor = Some(format!("fnref-{label}"));
    }
    response.rect
}

#[cfg(test)]
mod tests {
    #[test]
    fn recognizes_footnote_back_links() {
        assert_eq!(super::target("#fnref-note"), Some("note"));
        assert_eq!(super::target("#section"), None);
    }
}
