//! Inline images at GitHub's size: natural size, or the HTML `width`/`height`,
//! capped at the column width (`max-width: 100%`).

use egui::{RichText, Ui};
use mdview_media::MediaKind;
use mdview_theme::Theme;

use super::Segment;
use crate::{AnimationStatus, LinkActivation, MediaContext, MediaLightbox, MediaState, PreviewOutput};

/// Display size for an image with a known natural size.
pub(crate) fn display_size(natural: egui::Vec2, size: (Option<u32>, Option<u32>), max_width: f32) -> egui::Vec2 {
    let natural = natural.max(egui::vec2(1.0, 1.0));
    let aspect = natural.y / natural.x;
    let wanted = match size {
        (Some(width), Some(height)) => egui::vec2(width as f32, height as f32),
        (Some(width), None) => egui::vec2(width as f32, width as f32 * aspect),
        (None, Some(height)) => egui::vec2(height as f32 / aspect, height as f32),
        (None, None) => natural,
    };
    if wanted.x > max_width {
        wanted * (max_width / wanted.x)
    } else {
        wanted
    }
}

pub(super) fn show_image(
    ui: &mut Ui,
    segment: &Segment,
    theme: &Theme,
    output: &mut PreviewOutput,
    media: Option<&mut MediaContext<'_>>,
    typography: mdview_theme::ReadingTypography,
    media_sizes: &std::collections::HashMap<String, (f32, f32)>,
) {
    let Some(target) = &segment.image_source else {
        return;
    };
    let max_width = ui.max_rect().width().max(32.0);
    let alt = segment.text.clone();
    let Some(media_context) = media else {
        ui.label(RichText::new(&alt).italics().color(theme.text_muted));
        return;
    };
    // A stable raster width lets the same texture survive a split resize; the displayed
    // size is still derived from the pane width.
    let request = media_context.request(
        ui.ctx(),
        MediaKind::Image,
        &target.destination,
        1_600.0,
        theme.is_dark(),
    );
    let key = request.cache_key();
    let status = media_context.provider.animation_status(&key);
    let known = media_sizes
        .get(&target.destination)
        .map(|(width, height)| egui::vec2(*width, *height));
    match media_context.provider.resolve(ui.ctx(), &request) {
        MediaState::Loading => {
            let size = match known {
                Some(natural) => display_size(natural, segment.image_size, max_width),
                None => egui::vec2(
                    max_width,
                    crate::layout::reserved_media_height(
                        MediaKind::Image,
                        &target.destination,
                        max_width,
                        typography,
                        media_sizes,
                    ),
                ),
            };
            let (rect, _) = ui.allocate_exact_size(size, egui::Sense::hover());
            ui.painter().text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                &alt,
                egui::FontId::proportional(12.0),
                theme.text_muted,
            );
        }
        MediaState::Ready(texture) => {
            let natural = known.unwrap_or_else(|| texture.size_vec2());
            let size = display_size(natural, segment.image_size, max_width);
            // The image sits on the text baseline; the text's descent stays below it.
            let descent = super::line_height(typography) - super::math::text_baseline(ui, typography);
            let (slot, response) = ui.allocate_exact_size(size + egui::vec2(0.0, descent), egui::Sense::click());
            let rect = egui::Rect::from_min_size(slot.min, size);
            ui.painter().image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            if let Some(label) = animation_label(status) {
                ui.painter().text(
                    egui::pos2(rect.left() + 4.0, rect.bottom() - 4.0),
                    egui::Align2::LEFT_BOTTOM,
                    label,
                    egui::FontId::proportional(10.0),
                    theme.text_muted,
                );
            }
            let hover = match (&segment.link, alt.is_empty()) {
                (Some(link), _) => link.destination.clone(),
                (None, true) => target.destination.clone(),
                (None, false) => alt.clone(),
            };
            let response = response.on_hover_text(hover);
            media_context
                .provider
                .mark_visible(&key, ui.is_rect_visible(response.rect));
            if response.clicked() {
                // A linked image (a badge) follows its link, as on GitHub.
                match &segment.link {
                    Some(link) => {
                        output.link = Some(LinkActivation {
                            destination: link.destination.clone(),
                            kind: link.kind,
                        });
                    }
                    None => {
                        output.lightbox = Some(MediaLightbox {
                            request: request.clone(),
                            alt,
                        });
                    }
                }
            }
        }
        MediaState::Error(error) => {
            egui::Frame::new()
                .fill(theme.sunken)
                .stroke(egui::Stroke::new(1.0, theme.danger))
                .inner_margin(6.0)
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        ui.label(RichText::new(&alt).italics().color(theme.text_muted));
                        ui.label(RichText::new(error).small().color(theme.danger));
                    });
                });
        }
    }
}

fn animation_label(status: AnimationStatus) -> Option<&'static str> {
    match status {
        AnimationStatus::None => None,
        AnimationStatus::Playing => Some("Animated"),
        AnimationStatus::Paused => Some("Animation paused"),
        AnimationStatus::Unsupported => Some("Animation not fully supported"),
    }
}

#[cfg(test)]
mod tests {
    use super::display_size;

    #[test]
    fn images_use_natural_or_attribute_size_capped_at_the_column() {
        let natural = egui::vec2(200.0, 100.0);
        assert_eq!(display_size(natural, (None, None), 500.0), egui::vec2(200.0, 100.0));
        assert_eq!(display_size(natural, (Some(100), None), 500.0), egui::vec2(100.0, 50.0));
        assert_eq!(display_size(natural, (None, Some(20)), 500.0), egui::vec2(40.0, 20.0));
        assert_eq!(display_size(natural, (None, None), 100.0), egui::vec2(100.0, 50.0));
    }
}
