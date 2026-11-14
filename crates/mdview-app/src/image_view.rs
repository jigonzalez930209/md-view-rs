//! Standalone image documents.
//!
//! Opening a PNG, JPEG, GIF, WebP, BMP, TIFF, or SVG file shows the decoded image instead of the
//! text editor. The viewer fits the image to the pane, keeps transparent pixels, plays supported
//! SVG animation, and opens the existing lightbox on click.

use std::path::PathBuf;

use eframe::egui;
use mdview_media::{MediaKind, MediaRequest};
use mdview_render::{MediaProvider, MediaState};

use crate::app::MdViewApp;

impl MdViewApp {
    /// Paint the active image document.
    pub(crate) fn show_image_document(&mut self, ui: &mut egui::Ui) {
        let Some(path) = self.tabs.active().editor.document().path().map(PathBuf::from) else {
            return;
        };
        let kind = mdview_media::image_kind_for_path(&path);
        let available = (ui.available_size() - egui::vec2(32.0, 32.0)).max(egui::vec2(64.0, 64.0));
        let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
        let max_width = (available.x * pixels_per_point).clamp(64.0, 16_384.0) as u32;
        let max_height = (available.y * pixels_per_point).clamp(64.0, 16_384.0) as u32;
        let source = path.to_string_lossy().into_owned();
        let mut request = MediaRequest::new(kind, &source);
        request.max_width = max_width;
        request.max_height = max_height;
        request.dark = self.theme == mdview_theme::ThemeMode::Dark;
        let key = request.cache_key();

        match self.media.resolve(ui.ctx(), &request) {
            MediaState::Loading => {
                ui.centered_and_justified(|ui| {
                    ui.spinner();
                });
            }
            MediaState::Ready(texture) => {
                let points = texture.size_vec2() / pixels_per_point;
                let scale = (available.x / points.x).min(available.y / points.y).min(1.0);
                let response = ui
                    .centered_and_justified(|ui| {
                        ui.add(
                            egui::Image::new(&texture)
                                .fit_to_exact_size(points * scale)
                                .sense(egui::Sense::click()),
                        )
                    })
                    .inner;
                self.media.mark_visible(&key, ui.is_rect_visible(response.rect));
                if response.clicked() {
                    let alt = path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    self.lightbox = Some(crate::lightbox::LightboxState {
                        request,
                        alt,
                        scale: 1.0,
                    });
                }
            }
            MediaState::Error(error) => {
                ui.centered_and_justified(|ui| {
                    egui::Frame::new()
                        .fill(ui.visuals().extreme_bg_color)
                        .stroke(egui::Stroke::new(1.0, ui.visuals().error_fg_color))
                        .corner_radius(6.0)
                        .inner_margin(12.0)
                        .show(ui, |ui| {
                            ui.vertical(|ui| {
                                ui.label(egui::RichText::new(format!("Image: {}", path.display())).italics());
                                ui.label(egui::RichText::new(error).small().color(ui.visuals().error_fg_color));
                            });
                        });
                });
            }
        }

        let format = match kind {
            MediaKind::Svg => "SVG".to_owned(),
            _ => path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_uppercase)
                .unwrap_or_else(|| "Image".to_owned()),
        };
        let label = match self.media.animation_status(&key) {
            mdview_render::AnimationStatus::Playing => format!("{format} · animated"),
            mdview_render::AnimationStatus::Paused => format!("{format} · paused"),
            mdview_render::AnimationStatus::Unsupported => format!("{format} · static preview"),
            mdview_render::AnimationStatus::None => format,
        };
        let rect = ui.max_rect();
        ui.painter().text(
            egui::pos2(rect.left() + 10.0, rect.bottom() - 8.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
    }
}
