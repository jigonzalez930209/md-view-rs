//! Table layout and painting.

use super::*;
use crate::LinkActivation;
use crate::inline::LinkTarget;
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
    let columns = alignments
        .len()
        .max(head.len())
        .max(rows.iter().map(Vec::len).max().unwrap_or(0))
        .max(1);
    let size = ctx.typography.tables;
    // GitHub: `padding: 6px 13px` (fixed pixels, 14 px text here), 1.5 line height, 1 px borders.
    let pad_x = (size * 0.93).round();
    let pad_y = (size * 0.43).round();
    let line = (size * 1.5).round();

    let mut all_rows = Vec::with_capacity(rows.len() + 1);
    all_rows.push(head.to_vec());
    all_rows.extend(rows.iter().cloned());
    let layout_rows = |wrap: &dyn Fn(usize) -> f32| -> Vec<Vec<LaidCell>> {
        all_rows
            .iter()
            .enumerate()
            .map(|(row, cells)| {
                cells
                    .iter()
                    .enumerate()
                    .map(|(column, cell)| {
                        layout_table_cell(ui, cell, wrap(column), ctx.theme, (size, line, row == 0), ctx.search)
                    })
                    .collect()
            })
            .collect()
    };

    // Tables are as wide as their content (`width: max-content`) up to the column.
    let natural = layout_rows(&|_| f32::INFINITY);
    let mut widths = vec![pad_x * 2.0 + 8.0; columns];
    for row in &natural {
        for (column, cell) in row.iter().enumerate() {
            widths[column] = widths[column].max(cell.galley.rect.width().ceil() + pad_x * 2.0);
        }
    }
    let total: f32 = widths.iter().sum();
    let galleys = if total > available {
        let minimum = pad_x * 2.0 + size * 3.0;
        let flexible: f32 = widths
            .iter()
            .map(|width| (width - minimum).max(0.0))
            .sum::<f32>()
            .max(1.0);
        let excess = total - available;
        for width in &mut widths {
            let share = (*width - minimum).max(0.0) / flexible;
            *width = (*width - excess * share).max(minimum);
        }
        let wrap = widths.clone();
        layout_rows(&move |column| (wrap.get(column).copied().unwrap_or(64.0) - pad_x * 2.0).max(16.0))
    } else {
        natural
    };
    let heights: Vec<f32> = galleys
        .iter()
        .map(|row| row.iter().map(|cell| cell.galley.rect.height()).fold(line, f32::max) + pad_y * 2.0)
        .collect();
    let total_width: f32 = widths.iter().sum::<f32>().min(available);
    let total_height: f32 = heights.iter().sum();
    let (rect, _) = ui.allocate_exact_size(egui::vec2(total_width, total_height), egui::Sense::hover());
    let painter = ui.painter().clone();
    let border = egui::Stroke::new(1.0, ctx.theme.border);
    let mut y = rect.top();
    for (row, row_galleys) in galleys.iter().enumerate() {
        let row_rect = egui::Rect::from_min_size(egui::pos2(rect.left(), y), egui::vec2(rect.width(), heights[row]));
        // Even body rows get the subtle surface, as in GitHub's `tr:nth-child(2n)`.
        if row > 0 && row % 2 == 0 {
            painter.rect_filled(row_rect, 0.0, ctx.theme.table_zebra);
        }
        if row > 0 {
            painter.hline(row_rect.x_range(), row_rect.top(), border);
        }
        let mut x = row_rect.left();
        for (column, cell) in row_galleys.iter().enumerate() {
            let width = widths.get(column).copied().unwrap_or(64.0);
            let inner = width - pad_x * 2.0;
            let left = x + pad_x;
            let origin = egui::pos2(
                match alignments.get(column).copied().unwrap_or_default() {
                    Alignment::Right => left + inner - cell.galley.rect.width(),
                    Alignment::Center => left + (inner - cell.galley.rect.width()) / 2.0,
                    Alignment::Left | Alignment::None => left,
                }
                .max(left),
                row_rect.top() + pad_y,
            );
            painter.galley(origin, cell.galley.clone(), ctx.theme.text);
            let cell_rect = egui::Rect::from_min_size(egui::pos2(x, row_rect.top()), egui::vec2(width, heights[row]));
            for (link_index, link) in cell.links.iter().enumerate() {
                let start = cell.galley.pos_from_cursor(egui::text::CCursor::new(link.start));
                let end = cell.galley.pos_from_cursor(egui::text::CCursor::new(link.end));
                let hit = egui::Rect::from_min_max(
                    origin + start.left_top().to_vec2(),
                    origin + end.right_bottom().to_vec2(),
                )
                .intersect(cell_rect);
                if hit.width() <= 0.0 || hit.height() <= 0.0 {
                    continue;
                }
                let response = ui
                    .interact(
                        hit,
                        ui.make_persistent_id(("table_link", row, column, link_index)),
                        egui::Sense::click(),
                    )
                    .on_hover_cursor(egui::CursorIcon::PointingHand)
                    .on_hover_text(&link.target.destination);
                if response.clicked() {
                    ctx.output.link = Some(LinkActivation {
                        destination: link.target.destination.clone(),
                        kind: link.target.kind,
                    });
                }
            }
            x += width;
        }
        y += heights[row];
    }
    let mut x = rect.left();
    for width in widths.iter().take(columns.saturating_sub(1)) {
        x += width;
        painter.vline(x, rect.y_range(), border);
    }
    painter.rect_stroke(rect, 0.0, border, egui::StrokeKind::Inside);
}

struct CellLink {
    /// Character offsets in the laid-out cell, not byte offsets.
    start: usize,
    end: usize,
    target: LinkTarget,
}

struct LaidCell {
    galley: std::sync::Arc<egui::Galley>,
    links: Vec<CellLink>,
}

fn layout_table_cell(
    ui: &Ui,
    inlines: &[Inline],
    width: f32,
    theme: &mdview_theme::Theme,
    (size, line, header): (f32, f32, bool),
    search: Option<&str>,
) -> LaidCell {
    let mut job = egui::text::LayoutJob::default();
    job.wrap.max_width = width;
    let mut chars = 0usize;
    let mut links = Vec::new();
    for segment in crate::inline::segments(inlines) {
        let start = chars;
        chars += segment.text.chars().count();
        let font = if segment.monospace {
            egui::FontId::monospace((size - 1.0).max(9.0))
        } else {
            // GitHub table headers are semibold (600); bold runs stay bold.
            let face = if header && !segment.strong && !segment.emphasis {
                mdview_theme::ReadingFace::SemiBold
            } else {
                mdview_theme::ReadingFace::from_style(header || segment.strong, segment.emphasis)
            };
            face.font(size)
        };
        let color = if segment.link.is_some() { theme.link } else { theme.text };
        let runs = search
            .map(|query| crate::highlight::highlight_runs(&segment.text, query))
            .unwrap_or_else(|| vec![(segment.text.clone(), false)]);
        for (run, matched) in runs {
            job.append(
                &run,
                0.0,
                egui::TextFormat {
                    font_id: font.clone(),
                    color,
                    background: if matched {
                        theme.selection
                    } else if segment.monospace {
                        theme.table_header
                    } else {
                        egui::Color32::TRANSPARENT
                    },
                    line_height: Some(line),
                    strikethrough: if segment.strike {
                        egui::Stroke::new(1.0, theme.text)
                    } else {
                        egui::Stroke::NONE
                    },
                    ..Default::default()
                },
            );
        }
        if let Some(target) = &segment.link {
            links.push(CellLink {
                start,
                end: chars,
                target: target.clone(),
            });
        }
    }
    LaidCell {
        galley: ui.painter().layout_job(job),
        links,
    }
}
