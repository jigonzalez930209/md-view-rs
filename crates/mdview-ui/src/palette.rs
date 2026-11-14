//! Command palette and quick open.

use egui::{Button, Context, Frame, Id, Key, Modal, RichText, ScrollArea, Stroke};

use crate::{Command, text_field};

/// One palette row.
#[derive(Clone, Debug, PartialEq)]
pub struct PaletteItem {
    /// Display label.
    pub label: String,
    /// Secondary text.
    pub detail: String,
    /// Command executed on activation.
    pub command: Command,
}

/// Show the palette and return the chosen command.
pub fn show_palette(
    ctx: &Context,
    open: &mut bool,
    query: &mut String,
    title: &str,
    items: &[PaletteItem],
) -> Option<Command> {
    if !*open {
        return None;
    }
    let mut command = None;
    let mut keep_open = *open;
    let visuals = ctx.style_of(ctx.theme()).visuals.clone();
    let dialog = Modal::new(Id::unique("mdview_palette"))
        .frame(
            Frame::new()
                .fill(visuals.window_fill)
                .stroke(Stroke::new(1.0, visuals.window_stroke.color))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(14))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(60),
                }),
        )
        .show(ctx, |ui| {
            ui.set_width(560.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(title)
                        .size(15.0)
                        .strong()
                        .color(ui.visuals().text_color()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(Button::new(RichText::new("×").size(14.0)).frame(false))
                        .clicked()
                    {
                        keep_open = false;
                    }
                });
            });
            ui.add_space(8.0);
            let response = text_field(ui, query, "Type to filter…", 532.0);
            response.request_focus();
            ui.add_space(8.0);
            let needle = query.to_ascii_lowercase();
            let matches = items
                .iter()
                .filter(|item| {
                    needle.is_empty()
                        || item.label.to_ascii_lowercase().contains(&needle)
                        || item.detail.to_ascii_lowercase().contains(&needle)
                })
                .take(12)
                .collect::<Vec<_>>();
            ScrollArea::vertical()
                .max_height(300.0)
                .auto_shrink([false, true])
                .show(ui, |ui| {
                    if matches.is_empty() {
                        ui.label(RichText::new("No matches").weak());
                    }
                    for (index, item) in matches.iter().enumerate() {
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(ui.available_width(), 26.0), egui::Sense::click());
                        if index == 0 {
                            ui.painter()
                                .rect_filled(rect, egui::CornerRadius::same(5), ui.visuals().selection.bg_fill);
                        } else if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(5),
                                ui.visuals().widgets.hovered.weak_bg_fill,
                            );
                        }
                        let label = if item.detail.is_empty() {
                            item.label.clone()
                        } else {
                            format!("{}   {}", item.label, item.detail)
                        };
                        ui.painter().text(
                            egui::pos2(rect.left() + 10.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            label,
                            egui::FontId::proportional(12.5),
                            ui.visuals().text_color(),
                        );
                        if response.clicked() {
                            command = Some(item.command.clone());
                            keep_open = false;
                        }
                    }
                });
            if ctx.input(|input| input.key_pressed(Key::Enter))
                && let Some(item) = matches.first()
            {
                command = Some(item.command.clone());
                keep_open = false;
            }
        });
    if dialog.backdrop_response.clicked() || ctx.input(|input| input.key_pressed(Key::Escape)) {
        keep_open = false;
    }
    *open = keep_open;
    command
}
