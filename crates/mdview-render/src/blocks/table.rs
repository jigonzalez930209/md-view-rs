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
    // GitHub: `padding: 6px 13px` at its 16 px body, 1.5 line height, 1 px borders.
    let pad_x = (size * 0.8125).round();
    let pad_y = (size * 0.375).round();
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

    // Tables are as wide as their content (`width: max-content`) up to the column;
    // wider content follows the browser's automatic table layout (`distribute`).
    let natural = layout_rows(&|_| f32::INFINITY);
    let mut max_widths = vec![pad_x * 2.0 + 8.0; columns];
    for row in &natural {
        for (column, cell) in row.iter().enumerate() {
            max_widths[column] = max_widths[column].max(cell.galley.rect.width().ceil() + pad_x * 2.0);
        }
    }
    let min_widths = min_content_widths(ui, &all_rows, columns, size, pad_x);
    let total: f32 = max_widths.iter().sum();
    let mut widths = max_widths.clone();
    let galleys = if total > available {
        widths = distribute(&min_widths, &max_widths, available);
        let wrap = widths.clone();
        layout_rows(&move |column| (wrap.get(column).copied().unwrap_or(64.0) - pad_x * 2.0).max(16.0))
    } else {
        natural
    };
    let heights: Vec<f32> = galleys
        .iter()
        // Each row owns its 1 px top border, as with GitHub's collapsed borders.
        .map(|row| row.iter().map(|cell| cell.galley.rect.height()).fold(line, f32::max) + pad_y * 2.0 + 1.0)
        .collect();
    let total_width: f32 = widths.iter().sum::<f32>().min(available);
    let total_height: f32 = heights.iter().sum::<f32>() + 1.0;
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
            // Header cells without an alignment are centered, like a browser's `th`;
            // every cell is vertically centered (`vertical-align: middle`).
            let alignment = match alignments.get(column).copied().unwrap_or_default() {
                Alignment::None if row == 0 => Alignment::Center,
                alignment => alignment,
            };
            let content = heights[row] - 1.0 - pad_y * 2.0;
            let origin = egui::pos2(
                match alignment {
                    Alignment::Right => left + inner - cell.galley.rect.width(),
                    Alignment::Center => left + (inner - cell.galley.rect.width()) / 2.0,
                    Alignment::Left | Alignment::None => left,
                }
                .max(left),
                row_rect.top() + 1.0 + pad_y + ((content - cell.galley.rect.height()) * 0.5).max(0.0).round(),
            );
            for &(start, end) in &cell.code {
                let from = cell.galley.pos_from_cursor(egui::text::CCursor::new(start));
                let to = cell.galley.pos_from_cursor(egui::text::CCursor::new(end));
                if (from.center().y - to.center().y).abs() < 1.0 {
                    let chip = egui::Rect::from_min_max(
                        origin + from.left_top().to_vec2(),
                        origin + to.right_bottom().to_vec2(),
                    )
                    .expand2(egui::vec2((size * 0.85 * 0.4).round(), (size * 0.2 * 0.8).round()));
                    painter.rect_filled(chip, 6.0, ctx.theme.table_header);
                }
            }
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

/// Widest unbreakable word per column plus padding (CSS min-content).
fn min_content_widths(ui: &Ui, rows: &[Vec<Vec<Inline>>], columns: usize, size: f32, pad_x: f32) -> Vec<f32> {
    let mut widths = vec![pad_x * 2.0 + 8.0; columns];
    for (row_index, row) in rows.iter().enumerate() {
        for (column, cell) in row.iter().enumerate().take(columns) {
            let face = if row_index == 0 {
                mdview_theme::ReadingFace::SemiBold
            } else {
                mdview_theme::ReadingFace::Regular
            };
            for word in text_of_cell(cell).split_whitespace() {
                let width = ui
                    .painter()
                    .layout_no_wrap(word.to_owned(), face.font(size), Color32::WHITE)
                    .size()
                    .x;
                widths[column] = widths[column].max(width.ceil() + pad_x * 2.0);
            }
        }
    }
    widths
}

fn text_of_cell(cell: &[Inline]) -> String {
    mdview_markdown::plain_text(cell)
}

/// Column widths for a table wider than `available`, like Chrome's automatic table
/// layout: every column gets its min-content width, and the remaining space is shared
/// in proportion to how much wider each column would like to be (max - min).
/// Measured against GitHub: a 156 px first column there, 155 px here.
pub(super) fn distribute(min: &[f32], max: &[f32], available: f32) -> Vec<f32> {
    let minimum: f32 = min.iter().sum();
    let flexible: f32 = max
        .iter()
        .zip(min)
        .map(|(max, min)| (max - min).max(0.0))
        .sum::<f32>()
        .max(1.0);
    let spare = (available - minimum).max(0.0);
    max.iter()
        .zip(min)
        .map(|(max, min)| min + (max - min).max(0.0) * spare / flexible)
        .collect()
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
    /// Character ranges of inline code, painted as GitHub chips behind the text.
    code: Vec<(usize, usize)>,
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
    let mut code = Vec::new();
    // GitHub pads inline code by 0.4em on each side; the pad is layout space
    // (`leading_space`) so neighbors keep their distance from the chip.
    let code_pad = (size * 0.85 * 0.4).round();
    let mut pending_pad = 0.0;
    for segment in crate::inline::segments(inlines) {
        let start = chars;
        chars += segment.text.chars().count();
        let font = if segment.monospace {
            // Inline code in cells is GitHub's 85% chip.
            egui::FontId::monospace((size * 0.85).round().max(9.0))
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
        let mut leading = pending_pad + if segment.monospace { code_pad } else { 0.0 };
        pending_pad = if segment.monospace { code_pad } else { 0.0 };
        for (run, matched) in runs {
            job.append(
                &run,
                std::mem::take(&mut leading),
                egui::TextFormat {
                    font_id: font.clone(),
                    color,
                    background: if matched {
                        theme.selection
                    } else {
                        egui::Color32::TRANSPARENT
                    },
                    // Code keeps its own glyph-height box so its chip does not fill the row.
                    line_height: (!segment.monospace).then_some(line),
                    strikethrough: if segment.strike {
                        egui::Stroke::new(1.0, theme.text)
                    } else {
                        egui::Stroke::NONE
                    },
                    ..Default::default()
                },
            );
        }
        if segment.monospace {
            code.push((start, chars));
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
        code,
    }
}

#[cfg(test)]
mod tests {
    use super::distribute;

    #[test]
    fn wide_tables_split_like_browser_auto_layout() {
        let widths = distribute(&[120.0, 100.0], &[260.0, 780.0], 520.0);
        assert!((widths[0] - 171.2).abs() < 0.1, "{widths:?}");
        assert!((widths[1] - 348.8).abs() < 0.1, "{widths:?}");
        assert!((widths.iter().sum::<f32>() - 520.0).abs() < 0.01);
    }
}
