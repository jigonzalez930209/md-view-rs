//! Image lightbox with zoom and pan.

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
    /// Display scale, relative to the image's logical size.
    pub scale: f32,
    /// Fit the image to the window.
    pub fit: bool,
    /// Pan offset from the center.
    pub pan: egui::Vec2,
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
        if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
            open = false;
        }
        egui::Window::new(title)
            .open(&mut open)
            .resizable(true)
            .default_size(egui::vec2(820.0, 620.0))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    if ui.button("Fit").clicked() {
                        state.fit = true;
                        state.pan = egui::Vec2::ZERO;
                    }
                    for (label, value) in [("50%", 0.5), ("100%", 1.0), ("200%", 2.0)] {
                        if ui
                            .selectable_label((state.scale - value).abs() < 0.01 && !state.fit, label)
                            .clicked()
                        {
                            state.fit = false;
                            state.scale = value;
                        }
                    }
                    if ui.small_button("−").on_hover_text("Zoom out").clicked() {
                        state.fit = false;
                        state.scale = (state.scale / 1.2).max(0.1);
                    }
                    if ui.small_button("+").on_hover_text("Zoom in").clicked() {
                        state.fit = false;
                        state.scale = (state.scale * 1.2).min(4.0);
                    }
                    ui.label(
                        egui::RichText::new(format!("{:.0}%", state.scale * 100.0))
                            .small()
                            .weak(),
                    );
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
                        let available = (ui.available_size() - egui::vec2(8.0, 8.0)).max(egui::vec2(200.0, 160.0));
                        let (viewport, _) = ui.allocate_exact_size(available, egui::Sense::hover());
                        let points = texture.size_vec2() / ui.ctx().pixels_per_point().max(1.0);
                        if state.fit {
                            state.scale = (available.x / points.x).min(available.y / points.y).min(1.0);
                        }
                        let size = points * state.scale.clamp(0.1, 4.0);
                        let center = viewport.center() + state.pan;
                        let image_rect = egui::Rect::from_center_size(center, size);
                        let painter = ui.painter().with_clip_rect(viewport);
                        painter.image(
                            texture.id(),
                            image_rect,
                            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                            egui::Color32::WHITE,
                        );
                        let response = ui.interact(
                            viewport,
                            ui.scope_id().with("lightbox-image"),
                            egui::Sense::click_and_drag(),
                        );
                        if response.dragged() {
                            state.pan += response.drag_delta();
                            state.fit = false;
                        }
                        if viewport.contains(ui.ctx().pointer_latest_pos().unwrap_or(egui::Pos2::ZERO)) {
                            let zoom = ui.input(|input| input.zoom_delta());
                            if (zoom - 1.0).abs() > 0.001 {
                                state.fit = false;
                                state.scale = (state.scale * zoom).clamp(0.1, 4.0);
                            }
                        }
                    }
                    MediaState::Error(error) => {
                        ui.colored_label(ui.visuals().error_fg_color, error);
                    }
                }
            });
        if open {
            self.lightbox = Some(state);
        }
    }
}
