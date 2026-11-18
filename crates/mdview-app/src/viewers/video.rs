//! The video viewer: frames from the system FFmpeg, audio from `ffplay`, both off the UI thread.
//!
//! A worker thread decodes raw RGBA frames through a pipe and hands each one over at its
//! presentation time; the pipe's backpressure keeps decoding at playback speed. Audio plays in
//! a windowless `ffplay` started and stopped with the video. Pausing stops both; seeking while
//! paused decodes a single frame. Without FFmpeg the viewer says how to install it.

use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use mdview_media::VideoInfo;

use super::video_worker::{Order, Reply, worker};
use super::{ViewerContext, ViewerInstance, ViewerOutput, ViewerPlugin};

const BAR_HEIGHT: f32 = 44.0;
const SKIP_SECONDS: f64 = 5.0;

pub(super) struct VideoPlugin;

impl ViewerPlugin for VideoPlugin {
    fn id(&self) -> &'static str {
        "mdview.video"
    }

    fn name(&self) -> &'static str {
        "Video"
    }

    fn extensions(&self) -> &'static [&'static str] {
        mdview_media::VIDEO_EXTENSIONS
    }

    fn create(&self, path: &Path) -> Result<Box<dyn ViewerInstance>, String> {
        Ok(Box::new(VideoInstance::new(path)))
    }
}

struct VideoInstance {
    path: PathBuf,
    orders: Option<Sender<Order>>,
    replies: Option<Receiver<Reply>>,
    info: Option<VideoInfo>,
    error: Option<String>,
    texture: Option<egui::TextureHandle>,
    position: f64,
    playing: bool,
    muted: bool,
    scrubbing: Option<f64>,
    decode_width: u32,
}

impl VideoInstance {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            orders: None,
            replies: None,
            info: None,
            error: None,
            texture: None,
            position: 0.0,
            playing: false,
            muted: false,
            scrubbing: None,
            decode_width: 1280,
        }
    }

    fn start(&mut self, ctx: &egui::Context) {
        if self.replies.is_some() {
            return;
        }
        let (orders, order_receiver) = mpsc::channel::<Order>();
        let (reply_sender, replies) = mpsc::channel::<Reply>();
        let path = self.path.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("mdview-video".to_owned())
            .spawn(move || worker(&path, &order_receiver, &reply_sender, &ctx));
        match spawned {
            Ok(_) => {
                self.orders = Some(orders);
                self.replies = Some(replies);
            }
            Err(error) => self.error = Some(format!("Could not start video playback: {error}")),
        }
    }

    fn send(&self, order: Order) {
        if let Some(orders) = &self.orders {
            let _ = orders.send(order);
        }
    }

    fn receive(&mut self, ctx: &egui::Context) {
        let Some(replies) = &self.replies else {
            return;
        };
        let mut first_info = false;
        while let Ok(reply) = replies.try_recv() {
            match reply {
                Reply::Info(Ok(info)) => {
                    self.info = Some(info);
                    first_info = true;
                }
                Reply::Info(Err(error)) => self.error = Some(error),
                Reply::Frame {
                    at,
                    width,
                    height,
                    rgba,
                } => {
                    let image = egui::ColorImage::from_rgba_unmultiplied([width as usize, height as usize], &rgba);
                    match &mut self.texture {
                        Some(texture) => texture.set(image, egui::TextureOptions::LINEAR),
                        None => {
                            self.texture = Some(ctx.load_texture("video-frame", image, egui::TextureOptions::LINEAR))
                        }
                    }
                    self.position = at;
                }
                Reply::Ended => self.playing = false,
            }
        }
        if first_info {
            self.send(Order::Still {
                at: 0.0,
                width: self.decode_width,
            });
        }
    }

    fn toggle(&mut self) {
        if self.playing {
            self.send(Order::Pause);
            self.playing = false;
            return;
        }
        let duration = self.info.as_ref().and_then(|info| info.duration).unwrap_or(f64::MAX);
        if self.position >= duration - 0.05 {
            self.position = 0.0;
        }
        self.send(Order::Play {
            from: self.position,
            width: self.decode_width,
            audio: !self.muted,
        });
        self.playing = true;
    }

    fn seek(&mut self, to: f64) {
        let duration = self.info.as_ref().and_then(|info| info.duration).unwrap_or(f64::MAX);
        let to = to.clamp(0.0, duration.max(0.0));
        self.position = to;
        if self.playing {
            self.send(Order::Play {
                from: to,
                width: self.decode_width,
                audio: !self.muted,
            });
        } else {
            self.send(Order::Still {
                at: to,
                width: self.decode_width,
            });
        }
    }

    fn controls(&mut self, ui: &mut egui::Ui, duration: Option<f64>) {
        let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), BAR_HEIGHT), egui::Sense::hover());
        let mut bar = ui.new_child(
            egui::UiBuilder::new()
                .max_rect(rect.shrink2(egui::vec2(10.0, 6.0)))
                .layout(egui::Layout::left_to_right(egui::Align::Center)),
        );
        let label = if self.playing { "Pause" } else { "Play" };
        if bar
            .add(play_button(self.playing))
            .on_hover_text(format!("{label} (Space)"))
            .clicked()
        {
            self.toggle();
        }
        let shown = self.scrubbing.unwrap_or(self.position);
        bar.label(
            egui::RichText::new(format!("{} / {}", clock(shown), duration.map_or("–".to_owned(), clock))).monospace(),
        );
        let mute = if self.muted { "Unmute" } else { "Mute" };
        let has_audio = self.info.as_ref().is_some_and(|info| info.has_audio);
        bar.with_layout(egui::Layout::right_to_left(egui::Align::Center), |bar| {
            if has_audio && bar.button(mute).clicked() {
                self.muted = !self.muted;
                if self.playing {
                    self.seek(self.position);
                }
            }
            if let Some(duration) = duration {
                let mut value = shown;
                let slider = egui::Slider::new(&mut value, 0.0..=duration).show_value(false);
                bar.spacing_mut().slider_width = (bar.available_width() - 16.0).max(80.0);
                let response = bar.add(slider);
                if response.dragged() || response.changed() {
                    self.scrubbing = Some(value);
                }
                if response.drag_stopped() || (response.changed() && !response.dragged()) {
                    self.scrubbing = None;
                    self.seek(value);
                }
            }
        });
    }
}

/// A round play or pause button drawn with shapes, so no icon font is needed.
fn play_button(playing: bool) -> impl egui::Widget {
    move |ui: &mut egui::Ui| {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
        let visuals = ui.style().interact(&response);
        ui.painter().circle_filled(rect.center(), 15.0, visuals.bg_fill);
        let color = visuals.fg_stroke.color;
        let center = rect.center();
        if playing {
            for dx in [-4.0, 4.0] {
                let bar = egui::Rect::from_center_size(center + egui::vec2(dx, 0.0), egui::vec2(4.0, 12.0));
                ui.painter().rect_filled(bar, 1.0, color);
            }
        } else {
            let points = vec![
                center + egui::vec2(-4.0, -7.0),
                center + egui::vec2(7.0, 0.0),
                center + egui::vec2(-4.0, 7.0),
            ];
            ui.painter()
                .add(egui::Shape::convex_polygon(points, color, egui::Stroke::NONE));
        }
        response.widget_info(|| {
            egui::WidgetInfo::labeled(egui::Role::Button, true, if playing { "Pause" } else { "Play" })
        });
        response
    }
}

fn clock(seconds: f64) -> String {
    let total = seconds.max(0.0).round() as u64;
    if total >= 3600 {
        format!("{}:{:02}:{:02}", total / 3600, total / 60 % 60, total % 60)
    } else {
        format!("{}:{:02}", total / 60, total % 60)
    }
}

impl ViewerInstance for VideoInstance {
    fn name(&self) -> &'static str {
        "Video"
    }

    fn rename_path(&mut self, path: &Path) {
        self.path = path.to_path_buf();
    }

    fn show(&mut self, ui: &mut egui::Ui, _context: &mut ViewerContext<'_>) -> ViewerOutput {
        let ctx = ui.ctx().clone();
        self.start(&ctx);
        self.receive(&ctx);
        if let Some(error) = &self.error {
            ui.centered_and_justified(|ui| {
                ui.label(egui::RichText::new(error).color(ui.visuals().error_fg_color));
            });
            return ViewerOutput::default();
        }
        let Some(info) = self.info.clone() else {
            ui.centered_and_justified(|ui| ui.spinner());
            return ViewerOutput::default();
        };
        let available = ui.available_rect_before_wrap();
        let area = egui::Rect::from_min_max(
            available.min,
            egui::pos2(available.right(), available.bottom() - BAR_HEIGHT),
        );
        self.decode_width = ((area.width() * ctx.pixels_per_point()) as u32).max(64);
        let (rect, response) = ui.allocate_exact_size(area.size(), egui::Sense::click());
        ui.painter().rect_filled(rect, 0.0, egui::Color32::BLACK);
        if let Some(texture) = &self.texture {
            let size = texture.size_vec2();
            let scale = (rect.width() / size.x).min(rect.height() / size.y);
            let shown = egui::Rect::from_center_size(rect.center(), size * scale);
            let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
            ui.painter().image(texture.id(), shown, uv, egui::Color32::WHITE);
        }
        if response.clicked() {
            self.toggle();
        }
        let no_text_focus = ctx.memory(|memory| memory.focused().is_none());
        if no_text_focus {
            let (space, left, right) = ui.input(|input| {
                (
                    input.key_pressed(egui::Key::Space),
                    input.key_pressed(egui::Key::ArrowLeft),
                    input.key_pressed(egui::Key::ArrowRight),
                )
            });
            if space {
                self.toggle();
            }
            if left {
                self.seek(self.position - SKIP_SECONDS);
            }
            if right {
                self.seek(self.position + SKIP_SECONDS);
            }
        }
        self.controls(ui, info.duration);
        ViewerOutput::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn clock_formats_minutes_and_hours() {
        assert_eq!(clock(0.0), "0:00");
        assert_eq!(clock(65.4), "1:05");
        assert_eq!(clock(3_725.0), "1:02:05");
    }

    #[test]
    fn plugin_claims_common_containers() {
        for name in ["clip.mp4", "movie.MKV", "talk.webm", "old.avi", "phone.mov"] {
            assert!(VideoPlugin.matches(Path::new(name)), "{name}");
        }
        assert!(!VideoPlugin.matches(Path::new("song.mp3")));
    }
}
