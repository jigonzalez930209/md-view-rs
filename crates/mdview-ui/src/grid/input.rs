//! Grid mouse, keyboard, clipboard, and in-place editing.

use egui::{CursorIcon, Event, Key, Modifiers, Pos2, Rect, Response, TextEdit, Ui};
use mdview_table::Table;

use super::{CellPos, Editing, GridAction, GridState, HEADER_HEIGHT, Layout, Motion, RESIZE_GRIP};

/// Handle one frame of input on the grid.
pub(super) fn handle(
    ui: &mut Ui,
    table: &Table,
    state: &mut GridState,
    layout: &Layout,
    screen: Rect,
    response: &Response,
    actions: &mut Vec<GridAction>,
) {
    if std::mem::take(&mut state.focus_requested) {
        response.request_focus();
    }
    state.focused = response.has_focus();
    pointer(ui, table, state, layout, screen, response, actions);
    if state.editing.is_some() {
        edit_cell(ui, table, state, layout, actions);
        return;
    }
    if response.has_focus() {
        keyboard(ui, table, state, actions);
    }
}

/// Which part of the grid a screen point is on.
enum Hit {
    ColumnHeader(usize),
    ColumnEdge(usize),
    RowHeader(usize),
    Cell(CellPos),
}

fn hit(layout: &Layout, screen: Rect, point: Pos2) -> Option<Hit> {
    let in_header = point.y < screen.top() + HEADER_HEIGHT;
    let in_gutter = point.x < screen.left() + layout.gutter;
    let content = Pos2::new(
        if in_gutter {
            layout.origin.x + layout.gutter
        } else {
            point.x
        },
        if in_header {
            layout.origin.y + HEADER_HEIGHT
        } else {
            point.y
        },
    );
    let cell = layout.cell_at(content)?;
    if in_header && in_gutter {
        return None;
    }
    if in_header {
        let right = layout.cell_rect(cell).right();
        if (point.x - right).abs() <= RESIZE_GRIP {
            return Some(Hit::ColumnEdge(cell.column));
        }
        return Some(Hit::ColumnHeader(cell.column));
    }
    if in_gutter {
        return Some(Hit::RowHeader(cell.row));
    }
    Some(Hit::Cell(cell))
}

fn pointer(
    ui: &Ui,
    table: &Table,
    state: &mut GridState,
    layout: &Layout,
    screen: Rect,
    response: &Response,
    actions: &mut Vec<GridAction>,
) {
    let shift = ui.input(|input| input.modifiers.shift);
    if let Some(position) = response.hover_pos()
        && matches!(hit(layout, screen, position), Some(Hit::ColumnEdge(_)))
    {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
    }
    if response.drag_started()
        && let Some(origin) = ui.input(|input| input.pointer.press_origin())
        && let Some(Hit::ColumnEdge(column)) = hit(layout, screen, origin)
    {
        state.resizing = Some(column);
    }
    if let Some(column) = state.resizing {
        ui.ctx().set_cursor_icon(CursorIcon::ResizeHorizontal);
        if response.dragged() {
            let width = state.width(column) + response.drag_delta().x;
            state.set_width(column, width);
        }
        if response.drag_stopped() {
            state.resizing = None;
        }
        return;
    }
    let pressed = response.clicked() || response.drag_started();
    if pressed {
        response.request_focus();
    }
    let Some(position) = ui.input(|input| input.pointer.interact_pos()) else {
        return;
    };
    let rows = layout.rows;
    let columns = layout.columns;
    match hit(layout, screen, position) {
        Some(Hit::Cell(cell)) if pressed => {
            actions.extend(commit_pending(table, state));
            if shift {
                state.selection.cursor = cell;
            } else {
                state.selection.select(cell);
            }
            if response.double_clicked() {
                begin_edit(state, None);
            }
        }
        Some(Hit::Cell(cell)) if response.dragged() => state.selection.cursor = cell,
        Some(Hit::ColumnHeader(column)) if pressed => {
            state.selection.anchor = CellPos::new(0, column);
            state.selection.cursor = CellPos::new(rows.saturating_sub(1), column);
        }
        Some(Hit::RowHeader(row)) if pressed => {
            state.selection.anchor = CellPos::new(row, 0);
            state.selection.cursor = CellPos::new(row, columns.saturating_sub(1));
        }
        _ => {}
    }
}

/// A click on another cell commits the cell being edited, as in a spreadsheet.
fn commit_pending(table: &Table, state: &mut GridState) -> Option<GridAction> {
    let editing = state.editing.take()?;
    let cell = editing.cell;
    (table.value(cell.row, cell.column).unwrap_or("") != editing.buffer).then(|| GridAction::SetBlock {
        row: cell.row,
        column: cell.column,
        values: vec![vec![editing.buffer]],
    })
}

fn begin_edit(state: &mut GridState, initial: Option<String>) {
    state.editing = Some(Editing {
        cell: state.selection.cursor,
        buffer: initial.unwrap_or_default(),
        focus: true,
    });
}

fn keyboard(ui: &mut Ui, table: &Table, state: &mut GridState, actions: &mut Vec<GridAction>) {
    let rows = table.rows.len();
    let columns = table.columns().max(1);
    let events = ui.input(|input| input.events.clone());
    for event in events {
        match event {
            Event::Key {
                key,
                pressed: true,
                modifiers,
                ..
            } => {
                let command = modifiers.command;
                let motion = match key {
                    Key::ArrowUp => Some(Motion::Up),
                    Key::ArrowDown => Some(Motion::Down),
                    Key::ArrowLeft => Some(Motion::Left),
                    Key::ArrowRight => Some(Motion::Right),
                    Key::Home if command => Some(Motion::Start),
                    Key::End if command => Some(Motion::End),
                    Key::Home => Some(Motion::RowStart),
                    Key::End => Some(Motion::RowEnd),
                    Key::PageUp => Some(Motion::PageUp(state.page_rows)),
                    Key::PageDown => Some(Motion::PageDown(state.page_rows)),
                    Key::Tab if modifiers.shift => Some(Motion::Left),
                    Key::Tab => Some(Motion::Right),
                    Key::Enter if modifiers.shift => Some(Motion::Up),
                    _ => None,
                };
                if let Some(motion) = motion {
                    let extend = modifiers.shift && !matches!(key, Key::Tab | Key::Enter);
                    state.selection.apply(motion, rows, columns, extend);
                    state.reveal_cursor = true;
                    continue;
                }
                match key {
                    Key::Enter | Key::F2 => {
                        let current = table
                            .value(state.selection.cursor.row, state.selection.cursor.column)
                            .unwrap_or("");
                        begin_edit(state, Some(current.to_owned()));
                    }
                    Key::Delete | Key::Backspace => actions.extend(clear_selection(table, state)),
                    Key::Z if command && modifiers.shift => actions.push(GridAction::Redo),
                    Key::Z if command => actions.push(GridAction::Undo),
                    Key::Y if command => actions.push(GridAction::Redo),
                    Key::A if command => {
                        state.selection.anchor = CellPos::new(0, 0);
                        state.selection.cursor = CellPos::new(rows.saturating_sub(1), columns.saturating_sub(1));
                    }
                    _ => {}
                }
            }
            // Typing starts an edit that replaces the cell, as in a spreadsheet.
            Event::Text(text) if !text.chars().all(char::is_control) => begin_edit(state, Some(text)),
            Event::Copy => ui.ctx().copy_text(selected_text(table, state)),
            Event::Cut => {
                ui.ctx().copy_text(selected_text(table, state));
                actions.extend(clear_selection(table, state));
            }
            Event::Paste(text) => {
                let values = super::from_clipboard(&text);
                if !values.is_empty() {
                    let cursor = state.selection.cursor;
                    let height = values.len();
                    let width = values.iter().map(Vec::len).max().unwrap_or(1);
                    actions.push(GridAction::SetBlock {
                        row: cursor.row,
                        column: cursor.column,
                        values,
                    });
                    state.selection.anchor = cursor;
                    state.selection.cursor = CellPos::new(cursor.row + height - 1, cursor.column + width - 1);
                }
            }
            _ => {}
        }
    }
}

/// Paint the in-place editor over the cell; Enter and Tab commit, Escape cancels.
fn edit_cell(ui: &mut Ui, table: &Table, state: &mut GridState, layout: &Layout, actions: &mut Vec<GridAction>) {
    let (commit, motion, cancel) = ui.input_mut(|input| {
        let newline = input.consume_key(Modifiers::SHIFT, Key::Enter);
        let enter = input.consume_key(Modifiers::NONE, Key::Enter);
        let back_tab = input.consume_key(Modifiers::SHIFT, Key::Tab);
        let tab = input.consume_key(Modifiers::NONE, Key::Tab);
        let escape = input.consume_key(Modifiers::NONE, Key::Escape);
        let motion = if enter {
            Some(Motion::Down)
        } else if tab {
            Some(Motion::Right)
        } else if back_tab {
            Some(Motion::Left)
        } else {
            None
        };
        ((enter || tab || back_tab, newline), motion, escape)
    });
    let Some(editing) = state.editing.as_mut() else {
        return;
    };
    if commit.1 {
        editing.buffer.push('\n');
    }
    let rect = layout.cell_rect(editing.cell);
    let lines = editing.buffer.lines().count().max(1) as f32;
    let area = Rect::from_min_size(
        rect.min,
        egui::vec2(rect.width().max(160.0), rect.height().max(lines * 18.0 + 8.0)),
    );
    let response = ui.put(
        area,
        TextEdit::multiline(&mut editing.buffer)
            .desired_rows(1)
            .margin(egui::Margin::symmetric(6, 4))
            .lock_focus(true),
    );
    if std::mem::take(&mut editing.focus) {
        response.request_focus();
    }
    if cancel {
        state.editing = None;
        state.focus_requested = true;
        return;
    }
    if commit.0 {
        let cell = editing.cell;
        let value = std::mem::take(&mut editing.buffer);
        state.editing = None;
        if table.value(cell.row, cell.column).unwrap_or("") != value {
            actions.push(GridAction::SetBlock {
                row: cell.row,
                column: cell.column,
                values: vec![vec![value]],
            });
        }
        if let Some(motion) = motion {
            state
                .selection
                .apply(motion, table.rows.len(), table.columns().max(1), false);
            state.reveal_cursor = true;
        }
        state.focus_requested = true;
    }
}

/// The selected values as spreadsheet clipboard text.
pub(super) fn selected_text(table: &Table, state: &GridState) -> String {
    let (top, bottom) = state.selection.rows();
    let (left, right) = state.selection.columns();
    let values: Vec<Vec<String>> = (top..=bottom)
        .map(|row| {
            (left..=right)
                .map(|column| table.value(row, column).unwrap_or("").to_owned())
                .collect()
        })
        .collect();
    super::to_clipboard(&values)
}

/// Empty every selected cell that has a value, as one edit.
pub(super) fn clear_selection(table: &Table, state: &GridState) -> Option<GridAction> {
    let (top, bottom) = state.selection.rows();
    let (left, right) = state.selection.columns();
    let bottom = bottom.min(table.rows.len().saturating_sub(1));
    if top > bottom || left >= table.columns() {
        return None;
    }
    let right = right.min(table.columns() - 1);
    Some(GridAction::SetBlock {
        row: top,
        column: left,
        values: vec![vec![String::new(); right - left + 1]; bottom - top + 1],
    })
}
