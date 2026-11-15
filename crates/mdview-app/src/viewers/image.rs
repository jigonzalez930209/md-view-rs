//! Built-in image viewer with zoom and pan.

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
            scale: 1.0,
            fit: true,
            pan: egui::Vec2::ZERO,
            texture: None,
            texture_key: None,
            raster_bucket: 0.0,
        }))
    }
}

struct ImageInstance {
    path: PathBuf,
    scale: f32,
    fit: bool,
    pan: egui::Vec2,
    texture: Option<egui::TextureHandle>,
    texture_key: Option<String>,
    raster_bucket: f32,
}

impl ViewerInstance for ImageInstance {
    fn name(&self) -> &'static str {
        "Image"
    }

    fn show(&mut self, ui: &mut egui::Ui, context: &mut ViewerContext<'_>) -> ViewerOutput {
        let kind = mdview_media::image_kind_for_path(&self.path);
        let viewport = ui.available_rect_before_wrap();
        let available = viewport.size().max(egui::vec2(64.0, 64.0));
        let pixels_per_point = ui.ctx().pixels_per_point().max(1.0);
        // Raster images decode once at up to 4× the viewport. SVG re-rasterizes in
        // half-step buckets as the user zooms, so vector edges stay sharp without a
        // request on every wheel notch.
        let bucket = if kind == mdview_media::MediaKind::Svg {
            ((self.scale.max(1.0) * 2.0).ceil() / 2.0).clamp(1.0, 4.0)
        } else {
            4.0
        };
        let max_width = (available.x * pixels_per_point * bucket).clamp(64.0, 16_384.0) as u32;
        let max_height = (available.y * pixels_per_point * bucket).clamp(64.0, 16_384.0) as u32;
        let source = self.path.to_string_lossy().into_owned();
        let mut request = MediaRequest::new(kind, &source);
        request.max_width = max_width;
        request.max_height = max_height;
        request.dark = context.dark;
        let key = request.cache_key();
        let mut output = ViewerOutput::default();

        let mut shown = None;
        match context.media.resolve(ui.ctx(), &request) {
            MediaState::Loading => {
                shown = self.texture.clone();
            }
            MediaState::Ready(texture) => {
                if self.texture_key.as_deref() != Some(key.as_str()) {
                    if self.texture_key.is_some() && (bucket - self.raster_bucket).abs() > 0.01 {
                        log::debug!("image reraster scale={bucket:.2}");
                    }
                    self.raster_bucket = bucket;
                    self.texture = Some(texture.clone());
                    self.texture_key = Some(key.clone());
                }
                shown = Some(texture);
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
        let status_label = match context.media.animation_status(&key) {
            AnimationStatus::Playing => " · animated",
            AnimationStatus::Paused => " · paused",
            AnimationStatus::Unsupported => " · static preview",
            AnimationStatus::None => "",
        };

        if let Some(texture) = shown {
            let points = texture.size_vec2() / pixels_per_point;
            let fit_scale = (available.x / points.x).min(available.y / points.y).min(1.0);
            if self.fit {
                self.scale = fit_scale;
                self.pan = egui::Vec2::ZERO;
            }
            self.scale = self.scale.clamp(0.1, 4.0);
            let size = points * self.scale;
            let center = viewport.center() + self.pan;
            let image_rect = egui::Rect::from_center_size(center, size);
            let clip = ui.painter().with_clip_rect(viewport);
            clip.image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );
            let response = ui.interact(
                viewport,
                ui.scope_id().with("image-viewer"),
                egui::Sense::click_and_drag(),
            );
            if response.dragged() {
                self.pan += response.drag_delta();
                log::debug!("image pan={:?}", self.pan);
            }
            if viewport.contains(ui.ctx().pointer_latest_pos().unwrap_or(egui::Pos2::ZERO)) {
                let wheel = ui.input(|input| input.smooth_scroll_delta.y);
                if wheel.abs() > 0.0 {
                    let factor: f32 = (1.0 + wheel * 0.0015).clamp(0.8, 1.25);
                    self.zoom_around(viewport, points, factor, ui.ctx());
                    log::debug!("image zoom scale={:.2} pan={:?}", self.scale, self.pan);
                }
            }
            context.media.mark_visible(&key, ui.is_rect_visible(response.rect));
            if response.clicked() {
                let alt = self
                    .path
                    .file_name()
                    .map(|name| name.to_string_lossy().into_owned())
                    .unwrap_or_default();
                output.lightbox = Some(mdview_render::MediaLightbox { request, alt });
            }
        } else {
            ui.centered_and_justified(|ui| {
                ui.spinner();
            });
        }
        if !context.lightbox_open {
            self.toolbar(ui, viewport, &format!("{format}{status_label}"), self.scale);
        }

        output
    }
}

impl ImageInstance {
    fn zoom_around(&mut self, viewport: egui::Rect, points: egui::Vec2, factor: f32, ctx: &egui::Context) {
        self.fit = false;
        let previous = self.scale;
        let next = (self.scale * factor).clamp(0.1, 4.0);
        if let Some(pointer) = ctx.pointer_latest_pos() {
            let image_point = (pointer - viewport.center() - self.pan) / previous;
            self.pan = pointer - viewport.center() - image_point * next;
            self.pan.x = self.pan.x.clamp(-points.x * 4.0, points.x * 4.0);
            self.pan.y = self.pan.y.clamp(-points.y * 4.0, points.y * 4.0);
        }
        self.scale = next;
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, viewport: egui::Rect, label: &str, scale: f32) {
        let position = egui::pos2(
            (viewport.right() - 330.0).max(viewport.left() + 8.0),
            viewport.top() + 8.0,
        );
        egui::Area::new(ui.scope_id().with("image-toolbar"))
            .order(egui::Order::Middle)
            .fixed_pos(position)
            .show(ui.ctx(), |ui| {
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .stroke(egui::Stroke::new(1.0, ui.visuals().window_stroke.color))
                    .corner_radius(6)
                    .inner_margin(egui::Margin::symmetric(6, 3))
                    .show(ui, |ui| {
                        ui.horizontal(|ui| {
                            if ui.small_button("−").on_hover_text("Zoom out").clicked() {
                                self.fit = false;
                                self.scale = (self.scale / 1.2).max(0.1);
                            }
                            ui.label(
                                egui::RichText::new(format!("{:.0}%", scale * 100.0))
                                    .size(11.0)
                                    .monospace(),
                            );
                            if ui.small_button("+").on_hover_text("Zoom in").clicked() {
                                self.fit = false;
                                self.scale = (self.scale * 1.2).min(4.0);
                            }
                            if ui
                                .selectable_label(self.fit, "Fit")
                                .on_hover_text("Fit to window")
                                .clicked()
                            {
                                self.fit = true;
                            }
                            if ui
                                .selectable_label((scale - 1.0).abs() < 0.01, "100%")
                                .on_hover_text("Actual size")
                                .clicked()
                            {
                                self.fit = false;
                                self.scale = 1.0;
                            }
                            ui.label(
                                egui::RichText::new(label)
                                    .size(10.5)
                                    .color(ui.visuals().weak_text_color()),
                            );
                        });
                    });
            });
    }
}
