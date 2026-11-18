//! A spreadsheet-style grid for delimited files (CSV, TSV, …).
//!
//! The grid paints a [`mdview_table::Table`] and reports [`GridAction`]s; it never edits text.
//! The application turns actions into document edits, so undo, save, and conflict handling stay
//! with the document. Only the visible rows and columns are painted, the column letters and row
//! numbers stay pinned while scrolling, and cells are edited in place.

mod input;
mod selection;

use egui::{Align2, FontId, Pos2, Rect, Response, Sense, Stroke, Ui, Vec2};
use mdview_table::Table;

pub use selection::{CellPos, Motion, Selection, column_name, from_clipboard, to_clipboard};

/// Every label the grid translates.
pub const GRID_LABELS: &[&str] = &[
    "Insert row above",
    "Insert row below",
    "Delete rows",
    "Insert column left",
    "Insert column right",
    "Delete columns",
    "Copy",
    "Clear",
];

const ROW_HEIGHT: f32 = 26.0;
const HEADER_HEIGHT: f32 = 26.0;
const DEFAULT_WIDTH: f32 = 128.0;
const MIN_WIDTH: f32 = 36.0;
const CELL_PADDING: f32 = 7.0;
const RESIZE_GRIP: f32 = 4.0;

/// A grid edit for the application to apply to the document.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GridAction {
    /// Write a block of values with its top-left corner at (`row`, `column`).
    SetBlock {
        /// Top row.
        row: usize,
        /// Left column.
        column: usize,
        /// Values, row by row.
        values: Vec<Vec<String>>,
    },
    /// Insert empty rows before `at`.
    InsertRows {
        /// Row index the new rows take.
        at: usize,
        /// How many rows.
        count: usize,
    },
    /// Delete rows `start..end`.
    DeleteRows {
        /// First row deleted.
        start: usize,
        /// One past the last row deleted.
        end: usize,
    },
    /// Insert empty columns before `at`.
    InsertColumns {
        /// Column index the new columns take.
        at: usize,
        /// How many columns.
        count: usize,
    },
    /// Delete columns `start..end`.
    DeleteColumns {
        /// First column deleted.
        start: usize,
        /// One past the last column deleted.
        end: usize,
    },
    /// Undo the last document edit.
    Undo,
    /// Redo the last undone edit.
    Redo,
}

/// An in-place edit of one cell.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Editing {
    cell: CellPos,
    buffer: String,
    focus: bool,
}

/// Per-tab grid state: selection, the cell being edited, and column widths.
#[derive(Clone, Debug, Default)]
pub struct GridState {
    /// The selected cells.
    pub selection: Selection,
    editing: Option<Editing>,
    widths: Vec<f32>,
    reveal_cursor: bool,
    page_rows: usize,
    focus_requested: bool,
    /// The column whose right edge is being dragged.
    resizing: Option<usize>,
    focused: bool,
}

impl GridState {
    /// Whether a cell is being edited (keys go to the cell editor).
    pub fn is_editing(&self) -> bool {
        self.editing.is_some()
    }

    /// Whether the grid had keyboard focus in the last frame (a cell editor counts).
    pub fn has_focus(&self) -> bool {
        self.focused || self.editing.is_some()
    }

    /// Ask the grid to take keyboard focus on its next paint.
    pub fn request_focus(&mut self) {
        self.focus_requested = true;
    }

    fn width(&self, column: usize) -> f32 {
        self.widths.get(column).copied().unwrap_or(DEFAULT_WIDTH)
    }

    fn set_width(&mut self, column: usize, width: f32) {
        if self.widths.len() <= column {
            self.widths.resize(column + 1, DEFAULT_WIDTH);
        }
        self.widths[column] = width.max(MIN_WIDTH);
    }

    /// Left edge of each column and the right edge of the last, from x = 0.
    fn edges(&self, columns: usize) -> Vec<f32> {
        let mut edges = Vec::with_capacity(columns + 1);
        let mut x = 0.0;
        edges.push(x);
        for column in 0..columns {
            x += self.width(column);
            edges.push(x);
        }
        edges
    }
}

/// Geometry of one paint, in content coordinates (origin at the top-left of the cells).
struct Layout {
    origin: Pos2,
    gutter: f32,
    edges: Vec<f32>,
    rows: usize,
    columns: usize,
}

impl Layout {
    fn cell_rect(&self, cell: CellPos) -> Rect {
        let left = self.origin.x + self.gutter + self.edges[cell.column.min(self.columns)];
        let right = self.origin.x + self.gutter + self.edges[(cell.column + 1).min(self.columns)];
        let top = self.origin.y + HEADER_HEIGHT + cell.row as f32 * ROW_HEIGHT;
        Rect::from_min_max(Pos2::new(left, top), Pos2::new(right, top + ROW_HEIGHT))
    }

    /// The cell under a point in content coordinates, if the point is on the cells.
    fn cell_at(&self, point: Pos2) -> Option<CellPos> {
        let x = point.x - self.origin.x - self.gutter;
        let y = point.y - self.origin.y - HEADER_HEIGHT;
        if x < 0.0 || y < 0.0 {
            return None;
        }
        let row = ((y / ROW_HEIGHT) as usize).min(self.rows.saturating_sub(1));
        let column = self.edges.windows(2).position(|edge| x < edge[1])?;
        Some(CellPos::new(row, column))
    }
}

/// Show the grid for `table` and return the edits the user made this frame.
pub fn show_grid(ui: &mut Ui, table: &Table, state: &mut GridState, t: &dyn Fn(&str) -> String) -> Vec<GridAction> {
    let mut actions = Vec::new();
    // One spare row and column past the data, so new values can be typed at the edges.
    let rows = table.rows.len() + 1;
    let columns = table.columns().max(1) + 1;
    let gutter = (rows.to_string().len() as f32 * 8.0 + 22.0).max(44.0);
    let edges = state.edges(columns);
    let content = Vec2::new(gutter + edges[columns], HEADER_HEIGHT + rows as f32 * ROW_HEIGHT);
    let visuals = ui.visuals().clone();
    egui::ScrollArea::both()
        .id_salt("mdview-grid-scroll")
        .auto_shrink([false, false])
        .show_viewport(ui, |ui, viewport| {
            let (rect, response) = ui.allocate_exact_size(content, Sense::click_and_drag());
            let layout = Layout {
                origin: rect.min,
                gutter,
                edges,
                rows,
                columns,
            };
            state.page_rows = ((viewport.height() - HEADER_HEIGHT) / ROW_HEIGHT).max(1.0) as usize;
            let screen = viewport.translate(rect.min.to_vec2());
            paint_cells(ui, table, state, &layout, screen, &visuals);
            input::handle(ui, table, state, &layout, screen, &response, &mut actions);
            paint_headers(ui, state, &layout, screen, &visuals);
            context_menu(&response, table, state, t, &mut actions);
            if std::mem::take(&mut state.reveal_cursor) {
                ui.scroll_to_rect(layout.cell_rect(state.selection.cursor).expand(4.0), None);
            }
        });
    actions
}

fn paint_cells(ui: &Ui, table: &Table, state: &GridState, layout: &Layout, screen: Rect, visuals: &egui::Visuals) {
    let painter = ui.painter_at(screen);
    painter.rect_filled(screen, 0.0, visuals.extreme_bg_color);
    let first_row = ((screen.top() - layout.origin.y - HEADER_HEIGHT) / ROW_HEIGHT)
        .floor()
        .max(0.0) as usize;
    let last_row =
        (((screen.bottom() - layout.origin.y - HEADER_HEIGHT) / ROW_HEIGHT).ceil() as usize).min(layout.rows);
    let grid_line = Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color.gamma_multiply(0.7));
    let font = FontId::proportional(13.0);
    let text_color = visuals.text_color();
    let selection_fill = visuals.selection.bg_fill.gamma_multiply(0.35);
    for row in first_row..last_row {
        for column in 0..layout.columns {
            let cell = CellPos::new(row, column);
            let rect = layout.cell_rect(cell);
            if rect.right() < screen.left() || rect.left() > screen.right() {
                continue;
            }
            if state.selection.contains(cell) {
                painter.rect_filled(rect, 0.0, selection_fill);
            }
            painter.line_segment([rect.right_top(), rect.right_bottom()], grid_line);
            painter.line_segment([rect.left_bottom(), rect.right_bottom()], grid_line);
            if let Some(value) = table.value(row, column)
                && !value.is_empty()
            {
                let first_line = value.lines().next().unwrap_or("");
                let shown = if value.contains('\n') {
                    format!("{first_line} …")
                } else {
                    first_line.to_owned()
                };
                let numeric = shown.trim().parse::<f64>().is_ok();
                let (anchor, x) = if numeric {
                    (Align2::RIGHT_CENTER, rect.right() - CELL_PADDING)
                } else {
                    (Align2::LEFT_CENTER, rect.left() + CELL_PADDING)
                };
                let clip = painter.with_clip_rect(rect.shrink2(Vec2::new(2.0, 0.0)).intersect(screen));
                clip.text(Pos2::new(x, rect.center().y), anchor, shown, font.clone(), text_color);
            }
        }
    }
    let cursor = layout.cell_rect(state.selection.cursor);
    painter.rect_stroke(
        cursor,
        0.0,
        Stroke::new(2.0, visuals.selection.stroke.color),
        egui::StrokeKind::Inside,
    );
}

/// Column letters and row numbers, pinned to the top and left of the viewport.
fn paint_headers(ui: &Ui, state: &GridState, layout: &Layout, screen: Rect, visuals: &egui::Visuals) {
    let painter = ui.painter_at(screen);
    let band = visuals.widgets.noninteractive.bg_fill;
    let line = Stroke::new(1.0, visuals.widgets.noninteractive.bg_stroke.color);
    let font = FontId::proportional(12.0);
    let (top, bottom) = state.selection.rows();
    let (left, right) = state.selection.columns();
    let strong = visuals.text_color();
    let weak = visuals.weak_text_color();
    let header = Rect::from_min_size(screen.min, Vec2::new(screen.width(), HEADER_HEIGHT));
    painter.rect_filled(header, 0.0, band);
    for column in 0..layout.columns {
        let cell = layout.cell_rect(CellPos::new(0, column));
        let rect = Rect::from_x_y_ranges(cell.x_range(), header.y_range());
        if rect.right() < screen.left() + layout.gutter || rect.left() > screen.right() {
            continue;
        }
        let selected = (left..=right).contains(&column);
        if selected {
            painter.rect_filled(rect, 0.0, visuals.selection.bg_fill.gamma_multiply(0.5));
        }
        painter.line_segment([rect.right_top(), rect.right_bottom()], line);
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            column_name(column),
            font.clone(),
            if selected { strong } else { weak },
        );
    }
    painter.line_segment([header.left_bottom(), header.right_bottom()], line);
    let gutter = Rect::from_min_size(screen.min, Vec2::new(layout.gutter, screen.height()));
    painter.rect_filled(gutter, 0.0, band);
    let first_row = ((screen.top() - layout.origin.y) / ROW_HEIGHT).floor().max(0.0) as usize;
    let last_row = (((screen.bottom() - layout.origin.y) / ROW_HEIGHT).ceil() as usize).min(layout.rows);
    for row in first_row..last_row {
        let cell = layout.cell_rect(CellPos::new(row, 0));
        let rect = Rect::from_x_y_ranges(gutter.x_range(), cell.y_range());
        if rect.bottom() < screen.top() + HEADER_HEIGHT {
            continue;
        }
        let selected = (top..=bottom).contains(&row);
        if selected {
            painter.rect_filled(rect, 0.0, visuals.selection.bg_fill.gamma_multiply(0.5));
        }
        painter.line_segment([rect.left_bottom(), rect.right_bottom()], line);
        let label = (row + 1).to_string();
        painter.text(
            rect.center(),
            Align2::CENTER_CENTER,
            label,
            font.clone(),
            if selected { strong } else { weak },
        );
    }
    painter.line_segment([gutter.right_top(), gutter.right_bottom()], line);
    let corner = Rect::from_min_size(screen.min, Vec2::new(layout.gutter, HEADER_HEIGHT));
    painter.rect_filled(corner, 0.0, band);
}

fn context_menu(
    response: &Response,
    table: &Table,
    state: &mut GridState,
    t: &dyn Fn(&str) -> String,
    actions: &mut Vec<GridAction>,
) {
    response.context_menu(|ui| {
        crate::compact_menu(ui);
        let (top, bottom) = state.selection.rows();
        let (left, right) = state.selection.columns();
        let mut item = |ui: &mut Ui, label: &str, action: Option<GridAction>| {
            if ui.button(t(label)).clicked() {
                actions.extend(action);
                ui.close();
            }
        };
        item(
            ui,
            "Insert row above",
            Some(GridAction::InsertRows { at: top, count: 1 }),
        );
        item(
            ui,
            "Insert row below",
            Some(GridAction::InsertRows {
                at: bottom + 1,
                count: 1,
            }),
        );
        let rows_exist = top < table.rows.len();
        if rows_exist {
            item(
                ui,
                "Delete rows",
                Some(GridAction::DeleteRows {
                    start: top,
                    end: bottom + 1,
                }),
            );
        }
        ui.separator();
        item(
            ui,
            "Insert column left",
            Some(GridAction::InsertColumns { at: left, count: 1 }),
        );
        item(
            ui,
            "Insert column right",
            Some(GridAction::InsertColumns {
                at: right + 1,
                count: 1,
            }),
        );
        if left < table.columns() {
            item(
                ui,
                "Delete columns",
                Some(GridAction::DeleteColumns {
                    start: left,
                    end: right + 1,
                }),
            );
        }
        ui.separator();
        if ui.button(t("Copy")).clicked() {
            ui.ctx().copy_text(input::selected_text(table, state));
            ui.close();
        }
        if ui.button(t("Clear")).clicked() {
            actions.extend(input::clear_selection(table, state));
            ui.close();
        }
    });
}
