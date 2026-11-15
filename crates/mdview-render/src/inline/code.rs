//! Inline code chips.

use egui::{Label, RichText, Sense, Ui};
use mdview_theme::Theme;

/// Inline code size: 85% of the body, as in GitHub, capped by the code setting.
pub(super) fn code_size(body: f32, code: f32) -> f32 {
    code.min((body * 0.85).round()).max(9.0)
}

/// Paint one inline code run as a GitHub chip and return the label rectangle.
pub(super) fn show_code_chip(
    ui: &mut Ui,
    make: &dyn Fn(&str) -> RichText,
    run: &str,
    theme: &Theme,
    (size, glue): (f32, f32),
    typography: mdview_theme::ReadingTypography,
) -> egui::Rect {
    let text = make(run);
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
    let chip_width = width + pad.x * 2.0;
    let needed = chip_width + glue;
    // The paragraph row is one space wider than the column for egui's trailing-space
    // measurement; chips have no trailing space, so they use the real column edge.
    let tolerance = ui
        .painter()
        .layout_no_wrap(
            " ".to_owned(),
            mdview_theme::ReadingFace::Regular.font(size),
            theme.text,
        )
        .size()
        .x;
    let remaining = ui.max_rect().right() - tolerance - ui.cursor().min.x;
    let wraps = chip_width > ui.max_rect().width() - tolerance;
    if needed > remaining {
        // Browsers may break code after a hyphen: keep the longest hyphen-ended
        // prefix that fits on this row, then continue on the next one. This also
        // applies to code longer than a whole row.
        let fits = |prefix: &str| {
            ui.fonts_mut(|fonts| {
                fonts
                    .layout_no_wrap(prefix.to_owned(), font.clone(), theme.text)
                    .size()
                    .x
            }) + pad.x * 2.0
                <= remaining
        };
        let split = run
            .char_indices()
            .filter(|(_, character)| *character == '-')
            .map(|(index, _)| index + 1)
            .rev()
            .find(|end| *end < run.len() && fits(&run[..*end]));
        if let Some(end) = split {
            let rect = show_code_chip(ui, make, &run[..end], theme, (size, 0.0), typography);
            ui.end_row();
            show_code_chip(ui, make, &run[end..], theme, (size, glue), typography);
            return rect;
        }
        let at_row_start = ui.cursor().min.x <= ui.max_rect().left() + 0.5;
        if !wraps {
            ui.end_row();
        } else if !at_row_start {
            // Too long for any row: start a fresh row and try the hyphen breaks there.
            ui.end_row();
            return show_code_chip(ui, make, run, theme, (size, glue), typography);
        }
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
            egui::epaint::RectShape::filled(
                glyphs.expand2(egui::vec2(pad.x, (pad.y * 0.8).round())),
                6.0,
                theme.table_header,
            ),
        );
    }
    rect
}

/// GitHub's `<kbd>`: 11 px monospace at a 16 px body, 3 x 5 px padding, a 1 px border
/// with a darker bottom edge, and 6 px corners.
pub(super) fn show_kbd(
    ui: &mut Ui,
    run: &str,
    theme: &Theme,
    size: f32,
    typography: mdview_theme::ReadingTypography,
) -> egui::Rect {
    let font = egui::FontId::monospace((size * 0.6875).round().max(8.0));
    let galley = ui.painter().layout_no_wrap(run.to_owned(), font, theme.text);
    let pad = egui::vec2((size * 0.3125).round(), (size * 0.1875).round());
    let row = super::line_height(typography);
    let chip_size = galley.size() + pad * 2.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(chip_size.x + 2.0, row), Sense::hover());
    let chip = egui::Rect::from_center_size(rect.center(), chip_size);
    let painter = ui.painter();
    painter.rect_filled(chip, 6.0, theme.code_background);
    painter.rect_stroke(
        chip,
        6.0,
        egui::Stroke::new(1.0, theme.border),
        egui::StrokeKind::Inside,
    );
    painter.hline(
        chip.x_range().shrink(4.0),
        chip.bottom() - 1.0,
        egui::Stroke::new(1.0, theme.border),
    );
    painter.galley(chip.min + pad, galley, theme.text);
    rect
}

/// `<sub>` and `<sup>` at 75% size, lowered or raised like the browser's
/// `vertical-align: sub` and `super`.
pub(super) fn show_script(
    ui: &mut Ui,
    segment: &super::Segment,
    theme: &Theme,
    size: f32,
    typography: mdview_theme::ReadingTypography,
) -> egui::Rect {
    let face = mdview_theme::ReadingFace::from_style(segment.strong, segment.emphasis);
    let galley = ui
        .painter()
        .layout_no_wrap(segment.text.clone(), face.font((size * 0.75).round()), theme.text);
    let row = super::line_height(typography);
    let (rect, _) = ui.allocate_exact_size(egui::vec2(galley.size().x, row), Sense::hover());
    let shift = if segment.superscript { -size * 0.33 } else { size * 0.2 };
    let center = rect.center() + egui::vec2(0.0, shift);
    let origin = egui::pos2(rect.left(), center.y - galley.size().y * 0.5);
    ui.painter().galley(origin, galley, theme.text);
    rect
}
