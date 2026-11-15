//! Inline and display formula painting.

use egui::{RichText, Ui};
use mdview_media::MediaKind;
use mdview_theme::Theme;

use super::Segment;
use crate::{MediaContext, MediaState};

/// Paint one formula and return its rectangle.
///
/// Formulas scale by font size, not by a fixed box height: one em of the raster
/// maps to the reading math size, so tall formulas (fractions, integrals, matrices)
/// grow instead of being squeezed into a text line.
pub(super) fn show_math(
    ui: &mut Ui,
    segment: &Segment,
    theme: &Theme,
    media: Option<&mut MediaContext<'_>>,
    typography: mdview_theme::ReadingTypography,
    media_sizes: &std::collections::HashMap<String, (f32, f32)>,
) -> egui::Rect {
    let Some(media_context) = media else {
        return ui
            .label(RichText::new(&segment.text).italics().color(theme.accent))
            .rect;
    };
    let source = crate::layout::math_source(&segment.text, segment.display_math);
    let max_width = ui.available_width().clamp(32.0, 960.0);
    let full_width = if segment.display_math {
        ui.max_rect().width().max(max_width)
    } else {
        max_width
    };
    // Render the formula once at a generous resolution. Display scaling then follows the
    // reading typography, so split-width changes do not rasterize the same formula again.
    let request_width = 1024.0;
    match media_context.load(ui.ctx(), MediaKind::Math, &source, request_width, theme.is_dark()) {
        MediaState::Loading => {
            if segment.display_math {
                let height =
                    crate::layout::reserved_media_height(MediaKind::Math, &source, full_width, typography, media_sizes);
                ui.allocate_ui_with_layout(
                    egui::vec2(full_width, height),
                    egui::Layout::top_down(egui::Align::Center),
                    |ui| {
                        ui.add_space((height - 18.0).max(0.0) / 2.0);
                        ui.spinner();
                    },
                )
                .response
                .rect
            } else {
                ui.spinner().rect
            }
        }
        MediaState::Ready(texture) => {
            // The raster is one pixel per SVG unit, independent of the screen scale.
            let natural = texture.size_vec2();
            let fit = if segment.display_math { full_width } else { max_width };
            let em_scale = if segment.display_math {
                crate::layout::math_scale(typography)
            } else {
                crate::layout::inline_math_scale(typography)
            };
            let scale = em_scale.min(fit / natural.x.max(1.0));
            let size = natural * scale;
            if segment.display_math {
                let height =
                    crate::layout::display_media_height(MediaKind::Math, natural.x, natural.y, full_width, typography);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(full_width, height), egui::Sense::hover());
                paint(ui, &texture, egui::Rect::from_center_size(rect.center(), size));
                rect
            } else {
                // Let the raster padding and short superscripts overlap the line gap
                // instead of growing the row: egui does not re-center items already
                // placed on a row, so a taller row would lift the text before it.
                let row = crate::inline::line_height(typography);
                let height = (size.y - typography.body * 0.5).max(row);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(size.x.max(1.0), height), egui::Sense::hover());
                // Items are centered on the row, so the row's text baseline is the row
                // center shifted by the body font's baseline within one line box. The
                // formula's own baseline is placed on it, like MathJax inline math.
                let image = match baseline(ui, &source) {
                    Some(fraction) => {
                        let text_baseline = rect.center().y - row * 0.5 + text_baseline(ui, typography);
                        let top = text_baseline - fraction * size.y;
                        egui::Rect::from_min_size(egui::pos2(rect.left(), top), size)
                    }
                    None => egui::Rect::from_center_size(rect.center(), size),
                };
                paint(ui, &texture, image);
                rect
            }
        }
        MediaState::Error(error) => {
            ui.label(
                RichText::new(format!("{}\n{error}", segment.text))
                    .italics()
                    .small()
                    .color(theme.danger),
            )
            .rect
        }
    }
}

fn paint(ui: &Ui, texture: &egui::TextureHandle, rect: egui::Rect) {
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        egui::Color32::WHITE,
    );
}

/// Formula baseline as a fraction of the raster height, cached per source.
fn baseline(ui: &Ui, source: &str) -> Option<f32> {
    let id = egui::Id::unique(("mdview-math-baseline", source));
    if let Some(cached) = ui.ctx().data(|data| data.get_temp::<Option<f32>>(id)) {
        return cached;
    }
    let value = mdview_media::math_baseline(source);
    ui.ctx().data_mut(|data| data.insert_temp(id, value));
    value
}

/// Distance from the top of a body line box to the text baseline.
pub(super) fn text_baseline(ui: &Ui, typography: mdview_theme::ReadingTypography) -> f32 {
    let row = crate::inline::line_height(typography);
    let mut job = egui::text::LayoutJob::default();
    job.append(
        "x",
        0.0,
        egui::TextFormat {
            font_id: mdview_theme::ReadingFace::Regular.font(typography.body),
            line_height: Some(row),
            ..Default::default()
        },
    );
    let galley = ui.painter().layout_job(job);
    galley
        .rows
        .first()
        .and_then(|row| row.glyphs.first())
        .map_or(row * 0.75, |glyph| glyph.pos.y)
}
