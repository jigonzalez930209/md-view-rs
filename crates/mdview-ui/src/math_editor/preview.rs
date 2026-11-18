//! Formula textures for the gallery, the palette, and the live preview.
//!
//! Every formula is rasterized at the exact pixel size it is drawn at, so glyph edges stay
//! sharp: minifying a large raster on the GPU blurs thin strokes such as the bars of `=`.

use std::collections::HashMap;

use egui::{Color32, Context, Id, Rect, TextureHandle, TextureOptions, Ui, Vec2};

/// A formula rasterized for one display size.
#[derive(Clone)]
pub(crate) struct Sharp {
    texture: TextureHandle,
    /// Display size in points: the raster size divided by the pixels per point.
    size: Vec2,
}

/// A rendered formula, or the parse error to show instead.
pub(crate) type Rendered = Result<Sharp, String>;

fn rasterize(ctx: &Context, tex: &str, dark: bool, pixels_per_em: f32) -> Rendered {
    let raster = mdview_media::render_math_at(tex, dark, pixels_per_em).map_err(|error| error.to_string())?;
    let size = [raster.width as usize, raster.height as usize];
    let image = egui::ColorImage::from_rgba_unmultiplied(size, &raster.rgba);
    let texture = ctx.load_texture(
        format!("math-{dark}-{pixels_per_em}-{tex}"),
        image,
        TextureOptions::LINEAR,
    );
    let points = egui::vec2(raster.width as f32, raster.height as f32) / ctx.pixels_per_point();
    Ok(Sharp { texture, size: points })
}

/// Render `tex` at `em` points per em, or smaller so it fits `bounds` (points).
///
/// The first raster is at the full size; a formula too large for `bounds` is rasterized
/// once more at the scale that fits, never scaled when painted.
fn fit(ctx: &Context, tex: &str, dark: bool, bounds: Vec2, em: f32) -> Rendered {
    let pixels_per_point = ctx.pixels_per_point();
    let full = rasterize(ctx, tex, dark, quantize(em * pixels_per_point))?;
    let shrink = (bounds.x / full.size.x.max(1.0)).min(bounds.y / full.size.y.max(1.0));
    if shrink >= 1.0 {
        return Ok(full);
    }
    rasterize(ctx, tex, dark, quantize(em * shrink * pixels_per_point))
}

/// Round a size down to half a pixel so nearby sizes share one texture.
fn quantize(pixels_per_em: f32) -> f32 {
    (pixels_per_em * 2.0).floor().max(8.0) / 2.0
}

fn key(tex: &str, dark: bool, bounds: Vec2, em: f32, pixels_per_point: f32) -> String {
    format!("{dark}|{:.0}x{:.0}|{em}|{pixels_per_point}|{tex}", bounds.x, bounds.y)
}

/// A cached formula for a fixed template (gallery and palette), kept in egui memory.
pub(crate) fn template(ctx: &Context, tex: &str, dark: bool, bounds: Vec2, em: f32) -> Rendered {
    let id = Id::unique((
        "mdview-math-template",
        key(tex, dark, bounds, em, ctx.pixels_per_point()),
    ));
    if let Some(rendered) = ctx.data(|data| data.get_temp::<Rendered>(id)) {
        return rendered;
    }
    let rendered = fit(ctx, tex, dark, bounds, em);
    ctx.data_mut(|data| data.insert_temp(id, rendered.clone()));
    rendered
}

/// Paint a formula at its pixel size in `rect`, centered vertically and placed
/// horizontally by `align`. The position snaps to whole pixels so edges stay crisp.
pub(crate) fn paint(ui: &Ui, sharp: &Sharp, rect: Rect, align: egui::Align, tint: Color32) {
    let x = match align {
        egui::Align::Min => rect.left(),
        egui::Align::Center => rect.center().x - sharp.size.x / 2.0,
        egui::Align::Max => rect.right() - sharp.size.x,
    };
    let pixels_per_point = ui.ctx().pixels_per_point();
    let snap = |value: f32| (value * pixels_per_point).round() / pixels_per_point;
    let min = egui::pos2(snap(x), snap(rect.center().y - sharp.size.y / 2.0));
    ui.painter().image(
        sharp.texture.id(),
        Rect::from_min_size(min, sharp.size),
        Rect::from_min_max(egui::Pos2::ZERO, egui::pos2(1.0, 1.0)),
        tint,
    );
}

/// Live-preview renders keyed by source and size, so moving the pointer back and forth
/// over the source reuses earlier highlight renders instead of rasterizing again.
#[derive(Default)]
pub(crate) struct PreviewCache {
    renders: HashMap<String, Rendered>,
    /// The last source that rendered, shown dimmed while the current source has an error.
    last_good: Option<Sharp>,
}

/// Renders kept before the cache starts over; each one is a single formula texture.
const PREVIEW_CAPACITY: usize = 96;

impl Sharp {
    /// Display size in points.
    pub(crate) fn size(&self) -> Vec2 {
        self.size
    }
}

impl PreviewCache {
    pub(crate) fn get(&mut self, ctx: &Context, tex: &str, dark: bool, bounds: Vec2, em: f32) -> Rendered {
        let key = key(tex, dark, bounds, em, ctx.pixels_per_point());
        if let Some(rendered) = self.renders.get(&key) {
            return rendered.clone();
        }
        if self.renders.len() >= PREVIEW_CAPACITY {
            self.renders.clear();
        }
        let rendered = fit(ctx, tex, dark, bounds, em);
        self.renders.insert(key, rendered.clone());
        rendered
    }

    pub(crate) fn remember_good(&mut self, sharp: &Sharp) {
        self.last_good = Some(sharp.clone());
    }

    pub(crate) fn last_good(&self) -> Option<&Sharp> {
        self.last_good.as_ref()
    }
}

/// A hex color for TeX's `\textcolor`.
pub(crate) fn tex_color(color: Color32) -> String {
    format!("#{:02x}{:02x}{:02x}", color.r(), color.g(), color.b())
}
