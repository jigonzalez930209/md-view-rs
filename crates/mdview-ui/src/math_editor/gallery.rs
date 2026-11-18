//! The Word-style equation gallery under the formatting bar's equation button.

use egui::{Color32, Response, Sense, Ui};

use super::GALLERY;
use super::preview;
use crate::MaterialIcons;

/// What the user picked in the gallery.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GalleryPick {
    /// Insert this TeX as a display formula.
    Insert(&'static str),
    /// Open the formula editor on this TeX.
    Edit(&'static str),
    /// Open the formula editor empty.
    New,
}

const GALLERY_WIDTH: f32 = 420.0;
const ITEM_HEIGHT: f32 = 86.0;
/// Type size of the gallery equations, in points per TeX em.
const GALLERY_EM: f32 = 16.0;

/// Show the gallery popup attached to `button`.
pub(crate) fn show_gallery(
    button: &Response,
    icons: &mut MaterialIcons,
    t: &dyn Fn(&str) -> String,
) -> Option<GalleryPick> {
    let mut pick = None;
    egui::Popup::menu(button).show(|ui| {
        crate::compact_menu(ui);
        ui.set_width(GALLERY_WIDTH);
        let dark = ui.visuals().dark_mode;
        egui::ScrollArea::vertical().max_height(460.0).show(ui, |ui| {
            for equation in GALLERY {
                let name = t(equation.name);
                let (item, edit) = item(ui, icons, equation.tex, &name, dark, t);
                if edit.clicked() {
                    pick = Some(GalleryPick::Edit(equation.tex));
                } else if item.clicked() {
                    pick = Some(GalleryPick::Insert(equation.tex));
                }
                ui.add_space(4.0);
            }
        });
        ui.separator();
        if crate::menu_item(ui, icons, "fmt-equation-edit", &t("New equation"), None, false) {
            pick = Some(GalleryPick::New);
        }
        if pick.is_some() {
            ui.close();
        }
    });
    pick
}

/// One gallery row: the rendered formula, and an edit button on its right.
fn item(
    ui: &mut Ui,
    icons: &mut MaterialIcons,
    tex: &str,
    name: &str,
    dark: bool,
    t: &dyn Fn(&str) -> String,
) -> (Response, Response) {
    let width = ui.available_width();
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, ITEM_HEIGHT), Sense::click());
    response.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, name));
    let visuals = ui.visuals();
    let fill = if response.hovered() || response.has_focus() {
        visuals.widgets.hovered.weak_bg_fill
    } else {
        visuals.extreme_bg_color
    };
    ui.painter().rect_filled(rect, 6.0, fill);
    let edit_rect =
        egui::Rect::from_center_size(egui::pos2(rect.right() - 18.0, rect.center().y), egui::vec2(26.0, 26.0));
    // The name sits above the formula, as in Word's equation gallery.
    ui.painter().text(
        rect.left_top() + egui::vec2(8.0, 6.0),
        egui::Align2::LEFT_TOP,
        name,
        egui::FontId::proportional(11.0),
        ui.visuals().weak_text_color(),
    );
    let formula_rect = rect
        .shrink(8.0)
        .with_min_y(rect.top() + 22.0)
        .with_max_x(edit_rect.left() - 6.0);
    // One type size for every equation; only a formula wider than the row shrinks.
    if let Ok(formula) = preview::template(ui.ctx(), tex, dark, formula_rect.size(), GALLERY_EM) {
        preview::paint(ui, &formula, formula_rect, egui::Align::Min, Color32::WHITE);
    }
    let edit = ui.interact(edit_rect, response.id.with("edit"), Sense::click());
    let edit_label = t("Edit before inserting");
    edit.widget_info(|| egui::WidgetInfo::labeled(egui::Role::Button, true, &edit_label));
    if response.hovered() || edit.hovered() || edit.has_focus() {
        let color = if edit.hovered() {
            ui.visuals().text_color()
        } else {
            ui.visuals().weak_text_color()
        };
        if edit.hovered() {
            ui.painter()
                .rect_filled(edit_rect, 4.0, ui.visuals().widgets.active.weak_bg_fill);
        }
        if let Some(texture) = icons.ui_texture(ui.ctx(), "edit") {
            ui.painter().image(
                texture.id(),
                edit_rect.shrink(6.0),
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                color,
            );
        }
    }
    let edit = edit.on_hover_text(edit_label);
    (response, edit)
}
