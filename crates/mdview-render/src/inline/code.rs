//! Inline code chips.

use egui::{Label, RichText, Ui};
use mdview_theme::Theme;

/// Inline code size: 85% of the body, as in GitHub, capped by the code setting.
pub(super) fn code_size(body: f32, code: f32) -> f32 {
    code.min((body * 0.85).round()).max(9.0)
}

/// Paint one inline code run as a GitHub chip and return the label rectangle.
pub(super) fn show_code_chip(
    ui: &mut Ui,
    text: RichText,
    run: &str,
    theme: &Theme,
    size: f32,
    typography: mdview_theme::ReadingTypography,
) -> egui::Rect {
    // GitHub pads inline code by 0.2em x 0.4em on a 6 px rounded chip. The chip
    // is painted behind the label so it never splits from the text on wrap.
    let pad = egui::vec2((size * 0.3).round(), (size * 0.2).round());
    let font = egui::FontId::monospace(code_size(size, typography.code));
    let (glyph_height, width) = ui.fonts_mut(|fonts| {
        (
            fonts.row_height(&font),
            fonts.layout_no_wrap(run.to_owned(), font.clone(), theme.text).size().x,
        )
    });
    // A chip that does not fit the rest of the row moves to the next row whole.
    // Code longer than a full row wraps with a per-row background instead.
    let needed = width + pad.x * 2.0;
    let remaining = ui.max_rect().right() - ui.cursor().min.x;
    let wraps = needed > ui.max_rect().width();
    if !wraps && needed > remaining {
        ui.end_row();
    }
    let text = if wraps {
        text.background_color(theme.table_header)
    } else {
        text
    };
    let chip = ui.painter().add(egui::Shape::Noop);
    ui.add_space(pad.x);
    let label = Label::new(text);
    let label = if wraps {
        label
    } else {
        label.wrap_mode(egui::TextWrapMode::Extend)
    };
    let rect = ui.add(label).rect;
    ui.add_space(pad.x);
    if !wraps {
        let glyphs = egui::Rect::from_center_size(rect.center(), egui::vec2(rect.width(), glyph_height));
        ui.painter().set(
            chip,
            egui::epaint::RectShape::filled(glyphs.expand2(egui::vec2(pad.x, pad.y * 0.5)), 6.0, theme.table_header),
        );
    }
    rect
}
