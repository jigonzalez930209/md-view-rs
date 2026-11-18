//! The PDF viewer: continuous pages, fit-width or manual zoom, rendered off the UI thread.
//!
//! A worker thread owns the document (hayro, through `mdview_media::PdfDocument`) and renders
//! the pages the view asks for at the current zoom. The view keeps textures only for pages near
//! the viewport and asks again when the zoom changes, so memory stays bounded on long files.

use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, Receiver, Sender};

use eframe::egui;
use mdview_media::{PdfDocument, Raster};

use super::{ViewerContext, ViewerInstance, ViewerOutput, ViewerPlugin};

/// Largest PDF read into memory.
const MAX_PDF_BYTES: u64 = 512 * 1024 * 1024;
/// Space between pages and around them, in points.
const GAP: f32 = 16.0;
/// Pages kept as textures on each side of the visible ones.
const KEEP_AROUND: usize = 2;
const MIN_ZOOM: f32 = 0.1;
const MAX_ZOOM: f32 = 8.0;

pub(super) struct PdfPlugin;

impl ViewerPlugin for PdfPlugin {
    fn id(&self) -> &'static str {
        "mdview.pdf"
    }

    fn name(&self) -> &'static str {
        "PDF"
    }

    fn extensions(&self) -> &'static [&'static str] {
        &["pdf"]
    }

    fn create(&self, path: &Path) -> Result<Box<dyn ViewerInstance>, String> {
        Ok(Box::new(PdfInstance::new(path)))
    }
}

enum Request {
    Render { page: usize, scale_key: u32 },
}

enum Reply {
    Opened(Result<Vec<(f32, f32)>, String>),
    Page {
        page: usize,
        scale_key: u32,
        result: Result<Raster, String>,
    },
}

/// Render scales are quantised to 1/16 pixel per point so zoom steps reuse renders.
fn scale_key(scale: f32) -> u32 {
    (scale * 16.0).round().max(1.0) as u32
}

fn key_scale(key: u32) -> f32 {
    key as f32 / 16.0
}

struct PdfInstance {
    path: PathBuf,
    requests: Option<Sender<Request>>,
    replies: Option<Receiver<Reply>>,
    pages: Option<Vec<(f32, f32)>>,
    error: Option<String>,
    /// Points to pixels; `None` fits the widest page to the view width.
    zoom: Option<f32>,
    textures: HashMap<usize, (u32, egui::TextureHandle)>,
    pending: HashSet<(usize, u32)>,
    current_page: usize,
    jump_to: Option<usize>,
}

impl PdfInstance {
    fn new(path: &Path) -> Self {
        Self {
            path: path.to_path_buf(),
            requests: None,
            replies: None,
            pages: None,
            error: None,
            zoom: None,
            textures: HashMap::new(),
            pending: HashSet::new(),
            current_page: 0,
            jump_to: None,
        }
    }

    /// Start the worker on first paint, when the context for repaints is known.
    fn start(&mut self, ctx: &egui::Context) {
        if self.replies.is_some() {
            return;
        }
        let (requests, request_receiver) = mpsc::channel::<Request>();
        let (reply_sender, replies) = mpsc::channel::<Reply>();
        let path = self.path.clone();
        let ctx = ctx.clone();
        let spawned = std::thread::Builder::new()
            .name("mdview-pdf".to_owned())
            .spawn(move || worker(&path, &request_receiver, &reply_sender, &ctx));
        if let Err(error) = spawned {
            self.error = Some(format!("Could not start the PDF renderer: {error}"));
            return;
        }
        self.requests = Some(requests);
        self.replies = Some(replies);
    }

    fn receive(&mut self, ctx: &egui::Context) {
        let Some(replies) = &self.replies else {
            return;
        };
        while let Ok(reply) = replies.try_recv() {
            match reply {
                Reply::Opened(Ok(pages)) => self.pages = Some(pages),
                Reply::Opened(Err(error)) => self.error = Some(error),
                Reply::Page {
                    page,
                    scale_key,
                    result,
                } => {
                    self.pending.remove(&(page, scale_key));
                    match result {
                        Ok(raster) => {
                            let image = egui::ColorImage::from_rgba_unmultiplied(
                                [raster.width as usize, raster.height as usize],
                                &raster.rgba,
                            );
                            let texture =
                                ctx.load_texture(format!("pdf-page-{page}"), image, egui::TextureOptions::LINEAR);
                            self.textures.insert(page, (scale_key, texture));
                        }
                        Err(error) => log::warn!("pdf page {} render failed: {error}", page + 1),
                    }
                }
            }
        }
    }

    fn request(&mut self, page: usize, key: u32) {
        let current = self.textures.get(&page).map(|(existing, _)| *existing);
        if current == Some(key) || self.pending.contains(&(page, key)) {
            return;
        }
        if let Some(requests) = &self.requests
            && requests.send(Request::Render { page, scale_key: key }).is_ok()
        {
            self.pending.insert((page, key));
        }
    }

    fn toolbar(&mut self, ui: &mut egui::Ui, count: usize, fit: f32) {
        ui.horizontal(|ui| {
            ui.add_space(8.0);
            if ui.button("◀").on_hover_text("Previous page").clicked() && self.current_page > 0 {
                self.jump_to = Some(self.current_page - 1);
            }
            ui.label(format!("{} / {count}", self.current_page + 1));
            if ui.button("▶").on_hover_text("Next page").clicked() && self.current_page + 1 < count {
                self.jump_to = Some(self.current_page + 1);
            }
            ui.separator();
            let zoom = self.zoom.unwrap_or(fit);
            if ui.button("−").on_hover_text("Zoom out").clicked() {
                self.zoom = Some((zoom / 1.25).max(MIN_ZOOM));
            }
            ui.label(format!("{:.0}%", zoom * 100.0));
            if ui.button("+").on_hover_text("Zoom in").clicked() {
                self.zoom = Some((zoom * 1.25).min(MAX_ZOOM));
            }
            if ui.selectable_label(self.zoom.is_none(), "Fit width").clicked() {
                self.zoom = None;
            }
            if ui.button("100%").clicked() {
                self.zoom = Some(1.0);
            }
        });
    }
}

fn worker(path: &Path, requests: &Receiver<Request>, replies: &Sender<Reply>, ctx: &egui::Context) {
    let opened = std::fs::metadata(path)
        .map_err(|error| error.to_string())
        .and_then(|metadata| {
            if metadata.len() > MAX_PDF_BYTES {
                Err(format!("The PDF is larger than {} MB.", MAX_PDF_BYTES / (1024 * 1024)))
            } else {
                std::fs::read(path).map_err(|error| error.to_string())
            }
        })
        .and_then(|bytes| PdfDocument::open(bytes).map_err(|error| error.to_string()));
    let document = match opened {
        Ok(document) => document,
        Err(error) => {
            let _ = replies.send(Reply::Opened(Err(error)));
            ctx.request_repaint();
            return;
        }
    };
    let sizes = (0..document.page_count())
        .map(|page| document.page_size(page).unwrap_or((612.0, 792.0)))
        .collect();
    if replies.send(Reply::Opened(Ok(sizes))).is_err() {
        return;
    }
    ctx.request_repaint();
    while let Ok(first) = requests.recv() {
        // Render only the newest request per page: older zoom levels are stale.
        let mut latest: Vec<Request> = vec![first];
        latest.extend(requests.try_iter());
        let mut seen = HashSet::new();
        for Request::Render { page, scale_key } in latest.into_iter().rev() {
            if !seen.insert(page) {
                continue;
            }
            let result = document
                .render_page(page, key_scale(scale_key))
                .map_err(|error| error.to_string());
            if replies
                .send(Reply::Page {
                    page,
                    scale_key,
                    result,
                })
                .is_err()
            {
                return;
            }
            ctx.request_repaint();
        }
    }
}

impl ViewerInstance for PdfInstance {
    fn name(&self) -> &'static str {
        "PDF"
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
        let Some(pages) = self.pages.clone() else {
            ui.centered_and_justified(|ui| ui.spinner());
            return ViewerOutput::default();
        };
        let widest = pages.iter().map(|(width, _)| *width).fold(1.0_f32, f32::max);
        let fit = ((ui.available_width() - GAP * 2.0 - 16.0) / widest).clamp(MIN_ZOOM, MAX_ZOOM);
        self.toolbar(ui, pages.len(), fit);
        let zoom = self.zoom.unwrap_or(fit);
        let pixels_per_point = ctx.pixels_per_point();
        let key = scale_key(zoom * pixels_per_point);
        // Page tops in layout points.
        let mut tops = Vec::with_capacity(pages.len());
        let mut y = GAP;
        for (_, height) in &pages {
            tops.push(y);
            y += height * zoom + GAP;
        }
        let content_height = y;
        let content_width = widest * zoom + GAP * 2.0;
        let mut area = egui::ScrollArea::both()
            .id_salt("pdf-pages")
            .auto_shrink([false, false]);
        if let Some(page) = self.jump_to.take() {
            area = area.vertical_scroll_offset(tops[page.min(tops.len() - 1)] - GAP);
        }
        area.show_viewport(ui, |ui, viewport| {
            let width = content_width.max(ui.available_width());
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, content_height), egui::Sense::hover());
            let painter = ui.painter_at(rect);
            let middle = viewport.center().y;
            let mut visible = Vec::new();
            for (index, (page_width, page_height)) in pages.iter().enumerate() {
                let top = tops[index];
                let size = egui::vec2(page_width * zoom, page_height * zoom);
                if top + size.y < viewport.top() || top > viewport.bottom() {
                    continue;
                }
                visible.push(index);
                if top <= middle {
                    self.current_page = index;
                }
                let left = rect.left() + (width - size.x) / 2.0;
                let page_rect = egui::Rect::from_min_size(egui::pos2(left, rect.top() + top), size);
                painter.rect_filled(
                    page_rect.translate(egui::vec2(0.0, 2.0)),
                    2.0,
                    egui::Color32::from_black_alpha(40),
                );
                painter.rect_filled(page_rect, 0.0, egui::Color32::WHITE);
                match self.textures.get(&index) {
                    Some((_, texture)) => {
                        let uv = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0));
                        painter.image(texture.id(), page_rect, uv, egui::Color32::WHITE);
                    }
                    None => {
                        painter.text(
                            page_rect.center(),
                            egui::Align2::CENTER_CENTER,
                            format!("{}", index + 1),
                            egui::FontId::proportional(28.0),
                            egui::Color32::from_gray(200),
                        );
                    }
                }
            }
            for index in &visible {
                self.request(*index, key);
            }
            let (first, last) = (
                visible.first().copied().unwrap_or(0),
                visible.last().copied().unwrap_or(0),
            );
            self.textures
                .retain(|page, _| *page + KEEP_AROUND >= first && *page <= last + KEEP_AROUND);
        });
        ViewerOutput::default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scale_keys_round_trip() {
        assert_eq!(scale_key(1.0), 16);
        assert!((key_scale(scale_key(1.5)) - 1.5).abs() < 0.01);
        assert_eq!(scale_key(0.0), 1, "never zero");
    }

    #[test]
    fn plugin_claims_pdf_files() {
        assert!(PdfPlugin.matches(Path::new("paper.PDF")));
        assert!(!PdfPlugin.matches(Path::new("paper.md")));
    }
}
