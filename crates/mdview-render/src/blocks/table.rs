//! Table layout and painting.

use super::*;
use mdview_markdown::{Alignment, Inline};

pub(super) fn show_table(
    ui: &mut Ui,
    alignments: &[Alignment],
    head: &[Vec<Inline>],
    rows: &[Vec<Vec<Inline>>],
    ctx: &mut BlockCtx<'_>,
    media: &mut Option<&mut MediaContext<'_>>,
) {
    let _ = media;
    let available = ui.available_width().max(120.0);
    let columns = alignments.len().max(1);
    let spacing = 12.0;
    let usable = (available - 12.0 - spacing * (columns.saturating_sub(1)) as f32).max(60.0);
    let mut weights = Vec::with_capacity(columns);
    for column in 0..columns {
        let mut chars = head.get(column).map(|cell| text_of(cell).chars().count()).unwrap_or(1);
        for row in rows {
            if let Some(cell) = row.get(column) {
                chars = chars.max(text_of(cell).chars().count().min(48));
            }
        }
        weights.push((chars as f32).clamp(6.0, 42.0));
    }
    let total: f32 = weights.iter().sum::<f32>().max(1.0);
    let widths: Vec<f32> = weights
        .iter()
        .map(|weight| (usable * weight / total).max(58.0))
        .collect();

    let mut all_rows = Vec::with_capacity(rows.len() + 1);
    all_rows.push(head.to_vec());
    all_rows.extend(rows.iter().cloned());
    let galleys: Vec<Vec<std::sync::Arc<egui::Galley>>> = all_rows
        .iter()
        .enumerate()
        .map(|(row, cells)| {
            cells
                .iter()
                .enumerate()
                .map(|(column, cell)| {
                    let width = widths.get(column).copied().unwrap_or(64.0) - 12.0;
                    layout_table_cell(ui, cell, width.max(24.0), ctx.theme, row == 0)
                })
                .collect()
        })
        .collect();
    let heights: Vec<f32> = galleys
        .iter()
        .map(|row| row.iter().map(|galley| galley.rect.height()).fold(16.0, f32::max) + 8.0)
        .collect();
    let total_height: f32 = heights.iter().sum();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(available, total_height), egui::Sense::hover());
    let painter = ui.painter();
    painter.rect_filled(rect, 0.0, ctx.theme.background);
    let mut y = rect.top();
    for (row, row_galleys) in galleys.iter().enumerate() {
        let row_rect = egui::Rect::from_min_size(egui::pos2(rect.left(), y), egui::vec2(rect.width(), heights[row]));
        if row == 0 {
            painter.rect_filled(row_rect, 0.0, ctx.theme.table_header);
        } else if row % 2 == 0 {
            painter.rect_filled(row_rect, 0.0, ctx.theme.table_zebra);
        }
        if row > 0 {
            painter.line_segment(
                [
                    egui::pos2(row_rect.left(), row_rect.top()),
                    egui::pos2(row_rect.right(), row_rect.top()),
                ],
                egui::Stroke::new(1.0, ctx.theme.border),
            );
        }
        let mut x = row_rect.left() + 6.0;
        for (column, galley) in row_galleys.iter().enumerate() {
            let width = widths.get(column).copied().unwrap_or(64.0);
            let text_x = match alignments.get(column).copied().unwrap_or_default() {
                Alignment::Right => x + width - 12.0 - galley.rect.width(),
                Alignment::Center => x + (width - 12.0 - galley.rect.width()) / 2.0,
                Alignment::Left | Alignment::None => x,
            };
            painter.galley(
                egui::pos2(text_x.max(x), row_rect.top() + 4.0),
                galley.clone(),
                ctx.theme.text,
            );
            x += width + spacing;
        }
        y += heights[row];
    }
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, ctx.theme.border),
        egui::StrokeKind::Inside,
    );
}

fn layout_table_cell(
    ui: &Ui,
    inlines: &[Inline],
    width: f32,
    theme: &mdview_theme::Theme,
    header: bool,
) -> std::sync::Arc<egui::Galley> {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    for segment in crate::inline::segments(inlines) {
        let font = if segment.monospace {
            egui::FontId::monospace(12.0)
        } else if header || segment.strong {
            egui::FontId::proportional(13.5)
        } else {
            egui::FontId::proportional(13.0)
        };
        let color = if segment.link.is_some() { theme.link } else { theme.text };
        job.append(
            &segment.text,
            0.0,
            egui::TextFormat {
                font_id: font,
                color,
                background: if segment.monospace {
                    theme.sunken
                } else {
                    egui::Color32::TRANSPARENT
                },
                italics: segment.emphasis,
                underline: if segment.link.is_some() {
                    egui::Stroke::new(1.0, theme.link)
                } else {
                    egui::Stroke::NONE
                },
                strikethrough: if segment.strike {
                    egui::Stroke::new(1.0, theme.text)
                } else {
                    egui::Stroke::NONE
                },
                ..Default::default()
            },
        );
    }
    ui.painter().layout_job(job)
}
