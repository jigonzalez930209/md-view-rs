//! Ordered, unordered, and task list layout.

use super::*;

fn task_checkbox(ui: &mut Ui, rect: egui::Rect, checked: bool, theme: &mdview_theme::Theme, size: f32, center_y: f32) {
    let side = (size * 1.0).clamp(13.0, 18.0);
    let box_rect = egui::Rect::from_center_size(egui::pos2(rect.center().x, center_y), egui::vec2(side, side));
    ui.painter().rect_filled(
        box_rect,
        egui::CornerRadius::same(2),
        if checked {
            theme.accent
        } else {
            egui::Color32::TRANSPARENT
        },
    );
    ui.painter().rect_stroke(
        box_rect,
        egui::CornerRadius::same(2),
        egui::Stroke::new(1.2, if checked { theme.accent } else { theme.text_muted }),
        egui::StrokeKind::Inside,
    );
    if checked {
        let check = egui::Stroke::new(1.6, theme.accent_text);
        ui.painter().line_segment(
            [
                egui::pos2(box_rect.left() + 2.6, box_rect.center().y),
                egui::pos2(box_rect.left() + 5.6, box_rect.bottom() - 2.8),
            ],
            check,
        );
        ui.painter().line_segment(
            [
                egui::pos2(box_rect.left() + 5.6, box_rect.bottom() - 2.8),
                egui::pos2(box_rect.right() - 2.2, box_rect.top() + 3.0),
            ],
            check,
        );
    }
}

fn list_first_line_height(
    ui: &egui::Ui,
    inlines: &[mdview_markdown::Inline],
    body: f32,
    typography: mdview_theme::ReadingTypography,
) -> f32 {
    let mut height = ui
        .ctx()
        .fonts_mut(|fonts| fonts.row_height(&FontId::proportional(body)))
        .max(body * 1.1);
    for segment in crate::inline::segments(inlines) {
        let candidate = if segment.monospace || segment.keyboard {
            ui.ctx()
                .fonts_mut(|fonts| fonts.row_height(&FontId::monospace(typography.code)))
        } else if segment.math {
            (typography.math * 1.35).max(body * 1.1)
        } else {
            height
        };
        height = height.max(candidate);
    }
    height
}

pub(super) fn show_list(
    ui: &mut Ui,
    ordered: bool,
    start: u64,
    items: &[ListItem],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let list_width = ui.available_width().max(120.0);
    let body = ctx.typography.body;
    let marker_width = if ordered { 26.0 } else { 16.0 };
    for (index, item) in items.iter().enumerate() {
        ui.horizontal_top(|ui| {
            ui.add_space(4.0);
            // The first line can be taller than the body row when it contains inline
            // code or math. Center the marker on that real line height with a small
            // optical nudge toward the text's visual center.
            let line_height = item
                .blocks
                .first()
                .and_then(|block| match &block.kind {
                    BlockKind::Paragraph(inlines) => Some(list_first_line_height(ui, inlines, body, ctx.typography)),
                    _ => None,
                })
                .unwrap_or_else(|| {
                    ui.ctx()
                        .fonts_mut(|fonts| fonts.row_height(&FontId::proportional(body)))
                        .max(body * 1.1)
                });
            let (marker_rect, _) = ui.allocate_exact_size(egui::vec2(marker_width, line_height), egui::Sense::hover());
            let marker_center = egui::pos2(marker_rect.center().x, marker_rect.top() + line_height * 0.82);
            match item.task {
                Some(checked) => task_checkbox(ui, marker_rect, checked, ctx.theme, body, marker_center.y),
                None => {
                    let marker = if ordered {
                        format!("{}.", start + index as u64)
                    } else {
                        "•".to_owned()
                    };
                    ui.painter().text(
                        marker_center,
                        Align2::CENTER_CENTER,
                        marker,
                        FontId::proportional(body),
                        ctx.theme.text_muted,
                    );
                }
            }
            ui.add_space(4.0);
            let content_width = (list_width - marker_width - 14.0).max(80.0);
            ui.allocate_ui_with_layout(egui::vec2(content_width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(content_width);
                if item.task.is_some() {
                    show_blocks_spacing(ui, &item.blocks, ctx, media, 2.0);
                } else {
                    show_blocks(ui, &item.blocks, ctx, media);
                }
            });
        });
    }
}
