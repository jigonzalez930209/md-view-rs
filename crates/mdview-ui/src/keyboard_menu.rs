//! Keyboard-first menu navigator.
//!
//! The title-bar menus remain pointer-friendly. This overlay gives the same
//! command groups a complete keyboard path: F10 or Alt+letter opens a group,
//! arrows move between groups/items, Enter runs the item, and Esc closes it.

use egui::{Context, Key, RichText};

use crate::Command;

/// One item in the keyboard menu.
#[derive(Clone, Debug)]
pub struct MenuEntry {
    /// Visible label.
    pub label: String,
    /// Command executed on Enter.
    pub command: Command,
}

/// One top-level menu group.
#[derive(Clone, Debug)]
pub struct MenuGroup {
    /// Visible group label.
    pub label: String,
    /// Items in display order.
    pub entries: Vec<MenuEntry>,
}

/// Persistent selection state.
#[derive(Clone, Debug, Default)]
pub struct KeyboardMenuState {
    /// Whether the overlay is visible.
    pub open: bool,
    /// Selected group.
    pub group: usize,
    /// Selected item.
    pub item: usize,
}

impl KeyboardMenuState {
    /// Open a group at its first item.
    pub fn open(&mut self, group: usize) {
        self.open = true;
        self.group = group;
        self.item = 0;
    }
}

/// Draw the keyboard menu and return the selected command.
pub fn show_keyboard_menu(
    ctx: &Context,
    translations: &mdview_i18n::Translations,
    lang: craft_i18n::Lang,
    state: &mut KeyboardMenuState,
    groups: &[MenuGroup],
) -> Option<Command> {
    if !state.open || groups.is_empty() {
        return None;
    }
    let t = |source: &str| translations.t(lang, source);
    let mut chosen = None;
    let position = egui::pos2(ctx.content_rect().left() + 12.0, ctx.content_rect().top() + 40.0);
    egui::Area::new(egui::Id::unique("mdview_keyboard_menu"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(ui.visuals().window_fill)
                .stroke(egui::Stroke::new(1.0, ui.visuals().window_stroke.color))
                .corner_radius(7)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 16,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(48),
                })
                .inner_margin(egui::Margin::same(8))
                .show(ui, |ui| {
                    ui.set_width(330.0);
                    ui.horizontal_wrapped(|ui| {
                        for (index, group) in groups.iter().enumerate() {
                            let selected = index == state.group;
                            if ui.selectable_label(selected, &group.label).clicked() {
                                state.group = index;
                                state.item = 0;
                            }
                        }
                    });
                    ui.separator();
                    let group = &groups[state.group.min(groups.len() - 1)];
                    state.item = state.item.min(group.entries.len().saturating_sub(1));
                    for (index, entry) in group.entries.iter().enumerate() {
                        let selected = index == state.item;
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(ui.available_width(), 24.0), egui::Sense::click());
                        if selected {
                            ui.painter()
                                .rect_filled(rect, egui::CornerRadius::same(4), ui.visuals().selection.bg_fill);
                        } else if response.hovered() {
                            ui.painter().rect_filled(
                                rect,
                                egui::CornerRadius::same(4),
                                ui.visuals().widgets.hovered.weak_bg_fill,
                            );
                        }
                        ui.painter().text(
                            egui::pos2(rect.left() + 8.0, rect.center().y),
                            egui::Align2::LEFT_CENTER,
                            &entry.label,
                            egui::FontId::proportional(12.0),
                            if selected {
                                ui.visuals().selection.stroke.color
                            } else {
                                ui.visuals().text_color()
                            },
                        );
                        if response.clicked() {
                            state.item = index;
                            chosen = Some(entry.command.clone());
                        }
                    }
                    ui.add_space(4.0);
                    ui.label(
                        RichText::new(t("Arrows navigate · Enter runs · Esc closes"))
                            .size(10.0)
                            .weak(),
                    );
                });
        });

    if ctx.input(|input| input.key_pressed(Key::Escape)) {
        state.open = false;
        return None;
    }
    if ctx.input(|input| input.key_pressed(Key::ArrowRight)) {
        state.group = (state.group + 1) % groups.len();
        state.item = 0;
    }
    if ctx.input(|input| input.key_pressed(Key::ArrowLeft)) {
        state.group = (state.group + groups.len() - 1) % groups.len();
        state.item = 0;
    }
    let group = &groups[state.group.min(groups.len() - 1)];
    if ctx.input(|input| input.key_pressed(Key::ArrowDown)) {
        state.item = (state.item + 1).min(group.entries.len().saturating_sub(1));
    }
    if ctx.input(|input| input.key_pressed(Key::ArrowUp)) {
        state.item = state.item.saturating_sub(1);
    }
    if ctx.input(|input| input.key_pressed(Key::Enter))
        && let Some(entry) = group.entries.get(state.item)
    {
        chosen = Some(entry.command.clone());
    }
    if chosen.is_some() {
        state.open = false;
    }
    chosen
}
