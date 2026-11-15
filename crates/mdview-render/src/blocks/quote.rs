//! Block quotes and GFM alerts.

use super::*;

/// Paint a quote or alert the way GitHub does: no surface, a 0.25em bar on the
/// left, and 1em of padding. Plain quotes use muted text; alerts keep the body
/// color and add a colored title.
pub(super) fn show_quote(
    ui: &mut Ui,
    kind: QuoteKind,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let body = ctx.typography.body;
    let bar = (body * 0.25).round().max(3.0);
    let accent = alert_color(kind, ctx.theme);
    let alert = kind.alert_title();
    let vertical = if alert.is_some() { (body * 0.5).round() } else { 0.0 };
    let mut quote_theme = *ctx.theme;
    if alert.is_none() {
        quote_theme.text = ctx.theme.text_muted;
    }
    let response = egui::Frame::new()
        .inner_margin(egui::Margin {
            left: (bar + body).round() as i8,
            right: body.round() as i8,
            top: vertical as i8,
            bottom: vertical as i8,
        })
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.spacing_mut().item_spacing.y = 0.0;
            if let Some(title) = alert {
                let line = crate::inline::line_height(ctx.typography);
                let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), line), egui::Sense::hover());
                let icon_center = egui::pos2(rect.left() + body * 0.5, rect.center().y);
                // GitHub draws the 16 px Octicon at a 16 px body: one em.
                let icon = egui::Rect::from_center_size(icon_center, egui::vec2(body, body));
                super::octicon::paint(ui, super::octicon::alert_path(kind), icon, accent);
                let text_pos = egui::pos2(rect.left() + body * 1.5, rect.center().y);
                ui.painter().text(
                    text_pos,
                    Align2::LEFT_CENTER,
                    title,
                    // GitHub alert titles are weight 500; the regular face is the closest match.
                    mdview_theme::ReadingFace::Regular.font(body),
                    accent,
                );
                ui.add_space(block_gap(ctx.typography));
            }
            let mut nested = ctx.reborrow();
            nested.theme = &quote_theme;
            show_blocks(ui, blocks, &mut nested, media);
        })
        .response;
    let rect = response.rect;
    let bar_rect = egui::Rect::from_min_size(rect.min, egui::vec2(bar, rect.height()));
    let bar_color = if alert.is_some() {
        accent
    } else {
        ctx.theme.quote_border
    };
    ui.painter().rect_filled(bar_rect, 0.0, bar_color);
}

fn alert_color(kind: QuoteKind, theme: &Theme) -> Color32 {
    match kind {
        QuoteKind::Caution => theme.danger,
        QuoteKind::Warning => theme.warning,
        QuoteKind::Tip => theme.success,
        QuoteKind::Important | QuoteKind::Note => theme.accent,
        QuoteKind::Quote => theme.quote_border,
    }
}
