//! GitHub Octicons used by GFM alerts.
//!
//! The path data is copied from Primer Octicons (MIT License, Copyright GitHub Inc.),
//! the exact 16 px glyphs GitHub renders in alert titles. Each glyph is rasterized
//! once per color and pixel size with the project's SVG rasterizer.

use egui::{Color32, Ui};
use mdview_markdown::QuoteKind;
use mdview_media::{MediaKind, MediaRequest};

const INFO: &str = "M0 8a8 8 0 1 1 16 0A8 8 0 0 1 0 8Zm8-6.5a6.5 6.5 0 1 0 0 13 6.5 6.5 0 0 0 0-13ZM6.5 7.75A.75.75 0 0 1 7.25 7h1a.75.75 0 0 1 .75.75v2.75h.25a.75.75 0 0 1 0 1.5h-2a.75.75 0 0 1 0-1.5h.25v-2h-.25a.75.75 0 0 1-.75-.75ZM8 6a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z";
const LIGHT_BULB: &str = "M8 1.5c-2.363 0-4 1.69-4 3.75 0 .984.424 1.625.984 2.304l.214.253c.223.264.47.556.673.848.284.411.537.896.621 1.49a.75.75 0 0 1-1.484.211c-.04-.282-.163-.547-.37-.847a8.456 8.456 0 0 0-.542-.68c-.084-.1-.173-.205-.268-.32C3.201 7.75 2.5 6.766 2.5 5.25 2.5 2.31 4.863 0 8 0s5.5 2.31 5.5 5.25c0 1.516-.701 2.5-1.328 3.259-.095.115-.184.22-.268.319-.207.245-.383.453-.541.681-.208.3-.33.565-.37.847a.751.751 0 0 1-1.485-.212c.084-.593.337-1.078.621-1.489.203-.292.45-.584.673-.848.075-.088.147-.173.213-.253.561-.679.985-1.32.985-2.304 0-2.06-1.637-3.75-4-3.75ZM5.75 12h4.5a.75.75 0 0 1 0 1.5h-4.5a.75.75 0 0 1 0-1.5ZM6 15.25a.75.75 0 0 1 .75-.75h2.5a.75.75 0 0 1 0 1.5h-2.5a.75.75 0 0 1-.75-.75Z";
const REPORT: &str = "M0 1.75C0 .784.784 0 1.75 0h12.5C15.216 0 16 .784 16 1.75v9.5A1.75 1.75 0 0 1 14.25 13H8.06l-2.573 2.573A1.458 1.458 0 0 1 3 14.543V13H1.75A1.75 1.75 0 0 1 0 11.25Zm1.75-.25a.25.25 0 0 0-.25.25v9.5c0 .138.112.25.25.25h2a.75.75 0 0 1 .75.75v2.19l2.72-2.72a.749.749 0 0 1 .53-.22h6.5a.25.25 0 0 0 .25-.25v-9.5a.25.25 0 0 0-.25-.25Zm7 2.25v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 9a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z";
const ALERT: &str = "M6.457 1.047c.659-1.234 2.427-1.234 3.086 0l6.082 11.378A1.75 1.75 0 0 1 14.082 15H1.918a1.75 1.75 0 0 1-1.543-2.575Zm1.763.707a.25.25 0 0 0-.44 0L1.698 13.132a.25.25 0 0 0 .22.368h12.164a.25.25 0 0 0 .22-.368Zm.53 3.996v2.5a.75.75 0 0 1-1.5 0v-2.5a.75.75 0 0 1 1.5 0ZM9 11a1 1 0 1 1-2 0 1 1 0 0 1 2 0Z";
const STOP: &str = "M4.47.22A.749.749 0 0 1 5 0h6c.199 0 .389.079.53.22l4.25 4.25c.141.14.22.331.22.53v6a.749.749 0 0 1-.22.53l-4.25 4.25A.749.749 0 0 1 11 16H5a.749.749 0 0 1-.53-.22L.22 11.53A.749.749 0 0 1 0 11V5c0-.199.079-.389.22-.53Zm.84 1.28L1.5 5.31v5.38l3.81 3.81h5.38l3.81-3.81V5.31L10.69 1.5ZM8 4a.75.75 0 0 1 .75.75v3.5a.75.75 0 0 1-1.5 0v-3.5A.75.75 0 0 1 8 4Zm0 8a1 1 0 1 1 0-2 1 1 0 0 1 0 2Z";

/// The Octicon path GitHub shows for an alert kind.
pub(super) fn alert_path(kind: QuoteKind) -> &'static str {
    match kind {
        QuoteKind::Note | QuoteKind::Quote => INFO,
        QuoteKind::Tip => LIGHT_BULB,
        QuoteKind::Important => REPORT,
        QuoteKind::Warning => ALERT,
        QuoteKind::Caution => STOP,
    }
}

/// Standalone SVG for one Octicon at a pixel size and color.
pub(super) fn svg(path: &str, pixels: u32, color: Color32) -> String {
    format!(
        "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{pixels}\" height=\"{pixels}\" viewBox=\"0 0 16 16\">\
         <path fill=\"#{:02x}{:02x}{:02x}\" d=\"{path}\"/></svg>",
        color.r(),
        color.g(),
        color.b()
    )
}

/// Paint an Octicon centered in `rect`. Returns `false` if it could not be rasterized.
pub(super) fn paint(ui: &Ui, path: &'static str, rect: egui::Rect, color: Color32) -> bool {
    let ppp = ui.ctx().pixels_per_point();
    let pixels = (rect.width() * ppp).round().clamp(8.0, 128.0) as u32;
    let id = egui::Id::unique(("mdview-octicon", path, pixels, color));
    let texture = ui.ctx().data(|data| data.get_temp::<egui::TextureHandle>(id));
    let texture = match texture {
        Some(texture) => texture,
        None => {
            let source = svg(path, pixels, color);
            let mut request = MediaRequest::new(MediaKind::Svg, source.clone());
            request.max_width = pixels;
            request.max_height = pixels;
            let Ok(raster) = mdview_media::rasterize_svg(source.as_bytes(), &request) else {
                return false;
            };
            let image =
                egui::ColorImage::from_rgba_unmultiplied([raster.width as usize, raster.height as usize], &raster.rgba);
            let texture = ui.ctx().load_texture(
                format!("octicon-{pixels}-{color:?}"),
                image,
                egui::TextureOptions::LINEAR,
            );
            ui.ctx().data_mut(|data| data.insert_temp(id, texture.clone()));
            texture
        }
    };
    ui.painter().image(
        texture.id(),
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        Color32::WHITE,
    );
    true
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_alert_octicon_rasterizes() {
        for kind in [
            QuoteKind::Note,
            QuoteKind::Tip,
            QuoteKind::Important,
            QuoteKind::Warning,
            QuoteKind::Caution,
        ] {
            let source = svg(alert_path(kind), 32, Color32::from_rgb(9, 105, 218));
            let mut request = MediaRequest::new(MediaKind::Svg, source.clone());
            request.max_width = 32;
            request.max_height = 32;
            let raster = mdview_media::rasterize_svg(source.as_bytes(), &request).expect("octicon");
            assert_eq!((raster.width, raster.height), (32, 32));
            assert!(raster.rgba.chunks_exact(4).any(|pixel| pixel[3] > 200), "{kind:?}");
        }
    }
}
