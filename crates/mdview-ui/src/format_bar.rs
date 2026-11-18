//! The Markdown formatting bar shown above the editor for Markdown documents.
//!
//! It follows GitHub's comment toolbar (heading, bold, italic, quote, code, link, and the
//! three list kinds) and adds underline, a Word-style table picker (hover or arrow over a
//! grid to choose rows and columns, then click or press Enter), and equations: a gallery of
//! common formulas and the formula editor.

use egui::{Color32, CornerRadius, Id, Key, Pos2, Rect, Sense, Stroke, Ui, Vec2};
use mdview_editor::{MarkdownAction, MarkdownState};

use crate::MaterialIcons;
use crate::math_editor::GalleryPick;

/// What the user picked in the formatting bar this frame.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FormatBarAction {
    /// Apply a Markdown formatting action.
    Format(MarkdownAction),
    /// Insert a gallery equation as a display formula.
    InsertEquation(&'static str),
    /// Open the formula editor: on this template, or on the formula at the cursor (a new
    /// one when the cursor is not in a formula) with `None`.
    EditEquation(Option<&'static str>),
}

/// Every label the bar translates, so callers can resolve them ahead of time.
pub const FORMAT_BAR_LABELS: &[&str] = &[
    "Heading",
    "Normal text",
    "Bold",
    "Italic",
    "Underline",
    "Quote",
    "Code",
    "Link",
    "Table",
    "table",
    "Bulleted list",
    "Numbered list",
    "Task list",
    "Equation",
    "Equation editor",
    "New equation",
    "Edit before inserting",
];

/// Height of the bar, in points.
pub const FORMAT_BAR_HEIGHT: f32 = 30.0;
/// Rows and columns the table picker offers.
const GRID_ROWS: usize = 8;
const GRID_COLUMNS: usize = 10;
const CELL: f32 = 16.0;
const GAP: f32 = 3.0;

/// Draw the bar and return the action the user picked this frame.
///
/// `t` translates labels; `shortcut` formats a `Cmd+B` style chord for the platform.
pub fn show_format_bar(
    ui: &mut Ui,
    icons: &mut MaterialIcons,
    state: MarkdownState,
    t: &dyn Fn(&str) -> String,
    shortcut: &dyn Fn(&str) -> String,
) -> Option<FormatBarAction> {
    let mut action = None;
    let width = ui.available_width();
    let (rect, _) = ui.allocate_exact_size(Vec2::new(width, FORMAT_BAR_HEIGHT), Sense::hover());
    ui.painter().rect_filled(rect, 0.0, ui.visuals().panel_fill);
    ui.painter().line_segment(
        [
            Pos2::new(rect.left(), rect.bottom() - 0.5),
            Pos2::new(rect.right(), rect.bottom() - 0.5),
        ],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
    let mut bar = ui.new_child(
        egui::UiBuilder::new()
            .max_rect(rect.shrink2(Vec2::new(6.0, 3.0)))
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
    );
    let ui = &mut bar;
    ui.spacing_mut().item_spacing.x = 2.0;

    let heading = button(ui, icons, "fmt-heading", state.heading > 0, &t("Heading"));
    heading_menu(&heading, state.heading, t, &mut action);
    let tip = |label: &str, chord: &str| format!("{} ({})", t(label), shortcut(chord));
    if button(ui, icons, "fmt-bold", state.bold, &tip("Bold", "Cmd+B")).clicked() {
        action = Some(MarkdownAction::Bold);
    }
    if button(ui, icons, "fmt-italic", state.italic, &tip("Italic", "Cmd+I")).clicked() {
        action = Some(MarkdownAction::Italic);
    }
    if button(ui, icons, "fmt-underline", state.underline, &tip("Underline", "Cmd+U")).clicked() {
        action = Some(MarkdownAction::Underline);
    }
    separator(ui);
    if button(ui, icons, "fmt-quote", state.quote, &t("Quote")).clicked() {
        action = Some(MarkdownAction::Quote);
    }
    if button(ui, icons, "fmt-code", false, &t("Code")).clicked() {
        action = Some(MarkdownAction::Code);
    }
    if button(ui, icons, "fmt-link", false, &tip("Link", "Cmd+K")).clicked() {
        action = Some(MarkdownAction::Link);
    }
    let table = button(ui, icons, "fmt-table", false, &t("Table"));
    table_picker(ui, &table, t, &mut action);
    separator(ui);
    if button(ui, icons, "fmt-list-bullet", state.bullet, &t("Bulleted list")).clicked() {
        action = Some(MarkdownAction::BulletList);
    }
    if button(ui, icons, "fmt-list-numbered", state.numbered, &t("Numbered list")).clicked() {
        action = Some(MarkdownAction::NumberedList);
    }
    if button(ui, icons, "fmt-list-task", state.task, &t("Task list")).clicked() {
        action = Some(MarkdownAction::TaskList);
    }
    separator(ui);
    let gallery = button(ui, icons, "fmt-equation", false, &t("Equation"));
    let equation = match crate::math_editor::show_gallery(&gallery, icons, t) {
        Some(GalleryPick::Insert(tex)) => Some(FormatBarAction::InsertEquation(tex)),
        Some(GalleryPick::Edit(tex)) => Some(FormatBarAction::EditEquation(Some(tex))),
        Some(GalleryPick::New) => Some(FormatBarAction::EditEquation(None)),
        None => None,
    };
    if button(ui, icons, "fmt-equation-edit", false, &t("Equation editor")).clicked() {
        return Some(FormatBarAction::EditEquation(None));
    }
    equation.or(action.map(FormatBarAction::Format))
}

/// A square icon button; `active` draws it pressed, like a set format in Word.
fn button(ui: &mut Ui, icons: &mut MaterialIcons, icon: &str, active: bool, tooltip: &str) -> egui::Response {
    let (rect, response) = ui.allocate_exact_size(Vec2::splat(24.0), Sense::click());
    response.widget_info(|| egui::WidgetInfo::selected(egui::Role::Button, true, active, tooltip));
    let visuals = ui.visuals();
    let fill = if response.is_pointer_button_down_on() {
        visuals.widgets.active.weak_bg_fill
    } else if active {
        visuals.selection.bg_fill
    } else if response.hovered() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, CornerRadius::same(4), fill);
    let tint = if active || response.hovered() {
        visuals.text_color()
    } else {
        visuals.weak_text_color()
    };
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.painter().image(
            texture.id(),
            rect.shrink(4.0),
            Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)),
            tint,
        );
    }
    response.on_hover_text(tooltip)
}

fn separator(ui: &mut Ui) {
    let (rect, _) = ui.allocate_exact_size(Vec2::new(9.0, 18.0), Sense::hover());
    ui.painter().line_segment(
        [
            Pos2::new(rect.center().x, rect.top()),
            Pos2::new(rect.center().x, rect.bottom()),
        ],
        Stroke::new(1.0, ui.visuals().widgets.noninteractive.bg_stroke.color),
    );
}

fn heading_menu(button: &egui::Response, current: u8, t: &dyn Fn(&str) -> String, action: &mut Option<MarkdownAction>) {
    egui::Popup::menu(button).show(|ui| {
        crate::compact_menu(ui);
        ui.set_min_width(170.0);
        for level in 0..=6u8 {
            let label = if level == 0 {
                t("Normal text")
            } else {
                format!("{} {level}", t("Heading"))
            };
            let size = if level == 0 {
                13.0
            } else {
                20.0 - f32::from(level) * 1.5
            };
            let text = egui::RichText::new(label).size(size);
            let text = if level > 0 { text.strong() } else { text };
            if ui.selectable_label(level == current, text).clicked() {
                *action = Some(MarkdownAction::Heading(level));
                ui.close();
            }
        }
    });
}

fn table_picker(ui: &Ui, button: &egui::Response, t: &dyn Fn(&str) -> String, action: &mut Option<MarkdownAction>) {
    let id = Id::unique("mdview-table-picker");
    if button.clicked() {
        // Start each opening at a 1 × 1 table so the keyboard has a starting cell.
        ui.ctx().data_mut(|data| data.insert_temp(id, (1usize, 1usize)));
    }
    egui::Popup::menu(button).show(|ui| {
        let (mut rows, mut columns) = ui
            .ctx()
            .data(|data| data.get_temp::<(usize, usize)>(id))
            .unwrap_or((1, 1));
        let size = Vec2::new(
            GRID_COLUMNS as f32 * (CELL + GAP) - GAP,
            GRID_ROWS as f32 * (CELL + GAP) - GAP,
        );
        let (grid, response) = ui.allocate_exact_size(size, Sense::click());
        if let Some(pointer) = response.hover_pos() {
            let local = pointer - grid.min;
            columns = ((local.x / (CELL + GAP)).floor() as usize + 1).clamp(1, GRID_COLUMNS);
            rows = ((local.y / (CELL + GAP)).floor() as usize + 1).clamp(1, GRID_ROWS);
        }
        ui.input(|input| {
            if input.key_pressed(Key::ArrowRight) {
                columns = (columns + 1).min(GRID_COLUMNS);
            }
            if input.key_pressed(Key::ArrowLeft) {
                columns = columns.saturating_sub(1).max(1);
            }
            if input.key_pressed(Key::ArrowDown) {
                rows = (rows + 1).min(GRID_ROWS);
            }
            if input.key_pressed(Key::ArrowUp) {
                rows = rows.saturating_sub(1).max(1);
            }
        });
        let visuals = ui.visuals().clone();
        for row in 0..GRID_ROWS {
            for column in 0..GRID_COLUMNS {
                let min = grid.min + Vec2::new(column as f32 * (CELL + GAP), row as f32 * (CELL + GAP));
                let cell = Rect::from_min_size(min, Vec2::splat(CELL));
                let picked = row < rows && column < columns;
                let (fill, stroke) = if picked {
                    (visuals.selection.bg_fill, visuals.widgets.active.bg_stroke.color)
                } else {
                    (visuals.extreme_bg_color, visuals.widgets.noninteractive.bg_stroke.color)
                };
                ui.painter().rect_filled(cell, CornerRadius::same(2), fill);
                ui.painter().rect_stroke(
                    cell,
                    CornerRadius::same(2),
                    Stroke::new(1.0, stroke),
                    egui::StrokeKind::Inside,
                );
            }
        }
        ui.add_space(4.0);
        ui.label(
            egui::RichText::new(format!("{columns} × {rows} {}", t("table")))
                .small()
                .color(visuals.weak_text_color()),
        );
        ui.ctx().data_mut(|data| data.insert_temp(id, (rows, columns)));
        let enter = ui.input(|input| input.key_pressed(Key::Enter));
        if response.clicked() || enter {
            *action = Some(MarkdownAction::Table { rows, columns });
            ui.close();
        }
    });
}

#[cfg(test)]
mod tests {
    use std::{cell::RefCell, rc::Rc};

    use egui_kittest::Harness;
    use egui_kittest::kittest::Queryable;
    use mdview_editor::{MarkdownAction, MarkdownState};

    use super::{FormatBarAction, show_format_bar};
    use crate::MaterialIcons;

    fn harness(picked: Rc<RefCell<Vec<FormatBarAction>>>) -> Harness<'static> {
        let mut icons = MaterialIcons::new();
        Harness::builder()
            .with_size(egui::vec2(560.0, 640.0))
            .build_ui(move |ui| {
                let state = MarkdownState {
                    bold: true,
                    ..MarkdownState::default()
                };
                let translate = |text: &str| text.to_owned();
                let chord = |text: &str| text.replace("Cmd", "Ctrl");
                if let Some(action) = show_format_bar(ui, &mut icons, state, &translate, &chord) {
                    picked.borrow_mut().push(action);
                }
            })
    }

    #[test]
    fn table_picker_inserts_the_hovered_size_and_supports_keys() {
        let picked = Rc::new(RefCell::new(Vec::new()));
        let mut harness = harness(picked.clone());
        harness.run();
        harness.get_by_label("Table").click();
        harness.run();
        // Keyboard: two right, one down from 1 × 1, then Enter.
        harness.key_press(egui::Key::ArrowRight);
        harness.key_press(egui::Key::ArrowRight);
        harness.key_press(egui::Key::ArrowDown);
        harness.run();
        harness.key_press(egui::Key::Enter);
        harness.run();
        assert_eq!(
            picked.borrow().as_slice(),
            &[FormatBarAction::Format(MarkdownAction::Table { rows: 2, columns: 3 })]
        );
    }

    #[test]
    fn buttons_report_their_actions() {
        let picked = Rc::new(RefCell::new(Vec::new()));
        let mut harness = harness(picked.clone());
        harness.run();
        for label in [
            "Bold (Ctrl+B)",
            "Italic (Ctrl+I)",
            "Underline (Ctrl+U)",
            "Bulleted list",
            "Task list",
        ] {
            harness.get_by_label(label).click();
            harness.run();
        }
        harness.get_by_label("Heading").click();
        harness.run();
        harness.get_by_label("Heading 2").click();
        harness.run();
        assert_eq!(
            picked.borrow().as_slice(),
            &[
                MarkdownAction::Bold,
                MarkdownAction::Italic,
                MarkdownAction::Underline,
                MarkdownAction::BulletList,
                MarkdownAction::TaskList,
                MarkdownAction::Heading(2),
            ]
            .map(FormatBarAction::Format)
        );
    }

    #[test]
    fn equation_buttons_open_the_gallery_and_the_editor() {
        let picked = Rc::new(RefCell::new(Vec::new()));
        let mut harness = harness(picked.clone());
        harness.run();
        harness.get_by_label("Equation").click();
        harness.run();
        harness.get_by_label("Pythagorean theorem").click();
        harness.run();
        harness.get_by_label("Equation").click();
        harness.run();
        harness
            .get_all_by_label("Edit before inserting")
            .next()
            .expect("edit button")
            .click();
        harness.run();
        harness.get_by_label("Equation editor").click();
        harness.run();
        assert_eq!(
            picked.borrow().as_slice(),
            &[
                FormatBarAction::InsertEquation("a^2 + b^2 = c^2"),
                FormatBarAction::EditEquation(Some(crate::GALLERY[0].tex)),
                FormatBarAction::EditEquation(None),
            ]
        );
    }
}
