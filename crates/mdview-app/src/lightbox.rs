//! Image lightbox window.

use eframe::egui;
use mdview_media::MediaRequest;
use mdview_render::{MediaProvider, MediaState};

use crate::app::MdViewApp;

/// The open lightbox state.
pub struct LightboxState {
    /// The image request.
    pub request: MediaRequest,
    /// Alternative text.
    pub alt: String,
    /// Display scale, 0.5, 1.0, or 2.0.
    pub scale: f32,
}

impl MdViewApp {
    /// Show the lightbox window when one is open.
    pub(crate) fn show_lightbox(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.lightbox.take() else {
            return;
        };
        let mut open = true;
        let title = if state.alt.is_empty() {
            "Image".to_owned()
        } else {
            state.alt.clone()
        };
        let media = self.media.resolve(ctx, &state.request);
        let mut scale = state.scale;
        egui::Window::new(title)
            .open(&mut open)
            .resizable(true)
            .default_width(720.0)
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    for (label, value) in [("50%", 0.5), ("100%", 1.0), ("200%", 2.0)] {
                        if ui.selectable_label((scale - value).abs() < 0.01, label).clicked() {
                            scale = value;
                        }
                    }
                });
                ui.separator();
                match media {
                    MediaState::Loading => {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Loading image…");
                        });
                    }
                    MediaState::Ready(texture) => {
                        let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0) * scale;
                        egui::ScrollArea::both().auto_shrink([false, false]).show(ui, |ui| {
                            ui.add(egui::Image::new(&texture).fit_to_exact_size(points));
                        });
                    }
                    MediaState::Error(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                }
            });
        state.scale = scale;
        if open {
            self.lightbox = Some(state);
        }
    }
}
