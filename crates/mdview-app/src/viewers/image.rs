//! Built-in image viewer.

use std::path::{Path, PathBuf};

use eframe::egui;
use mdview_media::MediaRequest;
use mdview_render::{AnimationStatus, MediaProvider, MediaState};

use super::{ViewerContext, ViewerInstance, ViewerOutput, ViewerPlugin};

pub(super) struct ImagePlugin;

impl ViewerPlugin for ImagePlugin {
    fn id(&self) -> &'static str {
        "image"
    }

    fn name(&self) -> &'static str {
        "Image"
    }

    fn extensions(&self) -> &'static [&'static str] {
        mdview_media::SUPPORTED_IMAGE_EXTENSIONS
    }

    fn create(&self, path: &Path) -> Result<Box<dyn ViewerInstance>, String> {
        Ok(Box::new(ImageInstance {
            path: path.to_path_buf(),
        }))
    }
}

struct ImageInstance {
    path: PathBuf,
}

impl ViewerInstance for ImageInstance {
    fn name(&self) -> &'static str {
        "Image"
    }

    fn show(&mut self, ui: &mut egui::Ui, context: &mut ViewerContext<'_>) -> ViewerOutput {
        let kind = mdview_media::image_kind_for_path(&self.path);
        let available = (ui.available_size() - egui::vec2(32.0, 32.0)).max(egui::vec2(64.0, 64.0));
        let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
        let max_width = (available.x * pixels_per_point).clamp(64.0, 16_384.0) as u32;
        let max_height = (available.y * pixels_per_point).clamp(64.0, 16_384.0) as u32;
        let source = self.path.to_string_lossy().into_owned();
        let mut request = MediaRequest::new(kind, &source);
        request.max_width = max_width;
        request.max_height = max_height;
        request.dark = context.dark;
        let key = request.cache_key();
        let mut output = ViewerOutput::default();

        match context.media.resolve(ui.ctx(), &request) {
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
                context.media.mark_visible(&key, ui.is_rect_visible(response.rect));
                if response.clicked() {
                    let alt = self
                        .path
                        .file_name()
                        .map(|name| name.to_string_lossy().into_owned())
                        .unwrap_or_default();
                    output.lightbox = Some(mdview_render::MediaLightbox { request, alt });
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
                                ui.label(
                                    egui::RichText::new(format!("{}: {}", self.name(), self.path.display())).italics(),
                                );
                                ui.label(egui::RichText::new(error).small().color(ui.visuals().error_fg_color));
                            });
                        });
                });
            }
        }

        let format = if kind == mdview_media::MediaKind::Svg {
            "SVG".to_owned()
        } else {
            self.path
                .extension()
                .and_then(|extension| extension.to_str())
                .map(str::to_ascii_uppercase)
                .unwrap_or_else(|| "Image".to_owned())
        };
        let label = match context.media.animation_status(&key) {
            AnimationStatus::Playing => format!("{format} · animated"),
            AnimationStatus::Paused => format!("{format} · paused"),
            AnimationStatus::Unsupported => format!("{format} · static preview"),
            AnimationStatus::None => format,
        };
        let rect = ui.max_rect();
        ui.painter().text(
            egui::pos2(rect.left() + 10.0, rect.bottom() - 8.0),
            egui::Align2::LEFT_BOTTOM,
            label,
            egui::FontId::proportional(11.0),
            ui.visuals().weak_text_color(),
        );
        output
    }
}
