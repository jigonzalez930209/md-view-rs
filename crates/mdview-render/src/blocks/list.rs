//! Ordered, unordered, and task list layout.
//!
//! The geometry follows GitHub's Markdown body: lists indent their content by 2em,
//! items are 0.25em apart, bullets change shape by nesting depth (disc, circle,
//! square), and ordered markers switch between decimal, lower-roman, and
//! lower-alpha. Markers are placed on the *measured* center of the item's first
//! text row, so they stay on the line even when it contains inline code or math.

use super::*;

pub(super) fn show_list(
    ui: &mut Ui,
    ordered: bool,
    start: u64,
    items: &[ListItem],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let body = ctx.typography.body;
    let indent = (body * 2.0).round();
    let item_gap = (body * 0.25).round();
    let line = crate::inline::line_height(ctx.typography);
    let depth = ctx.list.depth;
    let content_width = (ui.available_width() - indent).max(80.0);
    for (index, item) in items.iter().enumerate() {
        if index > 0 {
            ui.add_space(item_gap);
        }
        let mut first_line = None;
        let row = ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let (marker_rect, _) = ui.allocate_exact_size(egui::vec2(indent, line), egui::Sense::hover());
            ui.allocate_ui_with_layout(egui::vec2(content_width, 0.0), Layout::top_down(Align::Min), |ui| {
                ui.set_width(content_width);
                ui.spacing_mut().item_spacing.y = 0.0;
                ctx.list.depth = depth + 1;
                first_line = show_item_blocks(ui, &item.blocks, ctx, media);
                ctx.list.depth = depth;
            });
            marker_rect
        });
        let marker_rect = row.inner;
        let center_y = first_line.unwrap_or(marker_rect.center().y);
        let content_left = marker_rect.right();
        match item.task {
            Some(checked) => task_checkbox(ui, content_left - body * 0.9, center_y, checked, ctx.theme, body),
            None if ordered => {
                let number = start.saturating_add(index as u64);
                ui.painter().text(
                    egui::pos2(content_left - body * 0.3, center_y),
                    Align2::RIGHT_CENTER,
                    format!("{}.", ordered_marker(number, depth)),
                    mdview_theme::ReadingFace::Regular.font(body),
                    ctx.theme.text,
                );
            }
            None => bullet(
                ui,
                egui::pos2(content_left - body * 0.72, center_y),
                depth,
                body,
                ctx.theme.text,
            ),
        }
    }
}

/// Paint an item's blocks and return the center of its first text row.
fn show_item_blocks(
    ui: &mut Ui,
    blocks: &[Block],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) -> Option<f32> {
    let (first, rest) = blocks.split_first()?;
    ctx.list.first_line = None;
    ctx.list.capture_first_line = true;
    show_block(ui, first, ctx, media);
    ctx.list.capture_first_line = false;
    let center = ctx.list.first_line.take();
    // A tight item continues straight into its nested list; a loose item (several
    // paragraphs) keeps the normal paragraph gap.
    let loose = rest.iter().any(|block| !matches!(block.kind, BlockKind::List { .. }));
    let mut previous_list = matches!(first.kind, BlockKind::List { .. });
    for block in rest {
        let is_list = matches!(block.kind, BlockKind::List { .. });
        let gap = if loose && !(is_list && !previous_list) {
            block_gap(ctx.typography)
        } else {
            (ctx.typography.body * 0.25).round()
        };
        ui.add_space(gap);
        show_block(ui, block, ctx, media);
        previous_list = is_list;
    }
    center
}

fn bullet(ui: &Ui, center: egui::Pos2, depth: usize, body: f32, color: egui::Color32) {
    let radius = (body * 0.18).max(2.2);
    match depth {
        0 => {
            ui.painter().circle_filled(center, radius, color);
        }
        1 => {
            ui.painter()
                .circle_stroke(center, radius - 0.5, egui::Stroke::new(1.0, color));
        }
        _ => {
            let side = radius * 1.7;
            ui.painter()
                .rect_filled(egui::Rect::from_center_size(center, egui::vec2(side, side)), 0.0, color);
        }
    }
}

/// The marker text for an ordered item at a nesting depth.
pub(crate) fn ordered_marker(number: u64, depth: usize) -> String {
    match depth % 3 {
        0 => number.to_string(),
        1 => roman(number),
        _ => alpha(number),
    }
}

fn roman(mut number: u64) -> String {
    if number == 0 || number >= 4_000 {
        return number.to_string();
    }
    let table = [
        (1000, "m"),
        (900, "cm"),
        (500, "d"),
        (400, "cd"),
        (100, "c"),
        (90, "xc"),
        (50, "l"),
        (40, "xl"),
        (10, "x"),
        (9, "ix"),
        (5, "v"),
        (4, "iv"),
        (1, "i"),
    ];
    let mut out = String::new();
    for (value, symbol) in table {
        while number >= value {
            out.push_str(symbol);
            number -= value;
        }
    }
    out
}

fn alpha(number: u64) -> String {
    if number == 0 {
        return "0".to_owned();
    }
    let mut number = number;
    let mut out = Vec::new();
    while number > 0 {
        number -= 1;
        out.push(b'a' + (number % 26) as u8);
        number /= 26;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

fn task_checkbox(ui: &Ui, center_x: f32, center_y: f32, checked: bool, theme: &mdview_theme::Theme, body: f32) {
    let side = (body * 0.93).clamp(12.0, 18.0).round();
    let box_rect = egui::Rect::from_center_size(egui::pos2(center_x, center_y), egui::vec2(side, side));
    let painter = ui.painter();
    let radius = egui::CornerRadius::same(3);
    if checked {
        painter.rect_filled(box_rect, radius, theme.accent);
        let check = egui::Stroke::new(1.6, theme.accent_text);
        let point = |x: f32, y: f32| box_rect.min + egui::vec2(side * x, side * y);
        painter.line_segment([point(0.24, 0.52), point(0.43, 0.70)], check);
        painter.line_segment([point(0.43, 0.70), point(0.77, 0.32)], check);
    } else {
        painter.rect_stroke(
            box_rect,
            radius,
            egui::Stroke::new(1.0, theme.text_muted),
            egui::StrokeKind::Inside,
        );
    }
}

#[cfg(test)]
mod tests {
    use super::ordered_marker;

    #[test]
    fn ordered_markers_follow_github_nesting_styles() {
        assert_eq!(ordered_marker(3, 0), "3");
        assert_eq!(ordered_marker(4, 1), "iv");
        assert_eq!(ordered_marker(14, 1), "xiv");
        assert_eq!(ordered_marker(1, 2), "a");
        assert_eq!(ordered_marker(27, 2), "aa");
        assert_eq!(ordered_marker(5, 3), "5");
    }
}
