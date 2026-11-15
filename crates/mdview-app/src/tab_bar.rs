//! The document tab strip.

use eframe::egui;

use crate::tabs::{TabAction, TabInfo};
use mdview_ui::ViewMode;

pub struct TabBarView<'a> {
    /// Translator.
    pub translations: &'a mdview_i18n::Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Icon cache.
    pub icons: &'a mut mdview_ui::MaterialIcons,
    /// Light-theme icon variants.
    pub light: bool,
    /// Active presentation mode.
    pub mode: ViewMode,
    /// Reading view is available.
    pub can_view: bool,
    /// Editor is available.
    pub can_edit: bool,
    /// Split view is available.
    pub can_split: bool,
}

pub fn show_tab_bar(ui: &mut egui::Ui, tabs: &[TabInfo], active: usize, view: TabBarView<'_>) -> Option<TabAction> {
    let t = |source: &str| view.translations.t(view.lang, source);
    let mut action = None;
    let bar = ui.max_rect();
    // Chrome uses the raised surface; the active tab uses the editor background so it merges
    // with the document, matching the VS Code/Cursor tab model.
    ui.painter().rect_filled(bar, 0.0, ui.visuals().window_fill);

    let controls_width = 276.0;
    let tabs_rect = egui::Rect::from_min_max(
        bar.min,
        egui::pos2((bar.right() - controls_width).max(bar.left() + 80.0), bar.bottom()),
    );
    let mut close_requested = None;
    let mut select_requested = None;
    ui.scope_builder(
        egui::UiBuilder::new()
            .max_rect(tabs_rect)
            .layout(egui::Layout::left_to_right(egui::Align::Center)),
        |ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            let scroll_id = egui::Id::unique("tab_bar_horizontal_scroll");
            let mut offset: f32 = ui.ctx().data_mut(|data| *data.get_temp_mut_or_default(scroll_id));
            if ui.rect_contains_pointer(tabs_rect) {
                // Use the raw wheel event instead of the smoothed scroll delta. The smoothed
                // value keeps moving after the wheel stops, which made the tab strip overshoot
                // and clamp at the end after a single notch.
                let wheel = ui.input(|input| {
                    input
                        .events
                        .iter()
                        .filter_map(|event| match event {
                            egui::Event::MouseWheel {
                                unit, delta, modifiers, ..
                            } if !modifiers.command && !modifiers.alt => Some(match unit {
                                egui::MouseWheelUnit::Point => -delta.y,
                                egui::MouseWheelUnit::Line => -delta.y * 20.0,
                                egui::MouseWheelUnit::Page => -delta.y * tabs_rect.width(),
                            }),
                            _ => None,
                        })
                        .sum::<f32>()
                });
                if wheel.abs() > 0.0 {
                    offset = (offset + wheel).max(0.0);
                    log::debug!("tab wheel delta={wheel:.1} offset={offset:.1}");
                    ui.ctx().request_repaint();
                }
            }
            let output = egui::ScrollArea::horizontal()
                .auto_shrink([false, true])
                .scroll_source(egui::scroll_area::ScrollSource::SCROLL_BAR)
                .horizontal_scroll_offset(offset)
                .show(ui, |ui| {
                    ui.horizontal_centered(|ui| {
                        ui.spacing_mut().item_spacing.x = 2.0;
                        for (index, tab) in tabs.iter().enumerate() {
                            match tab_button(ui, tab, index, index == active, view.icons, view.light, &t) {
                                Some(TabHit::Select) => select_requested = Some(index),
                                Some(TabHit::Close) => close_requested = Some(index),
                                None => {}
                            }
                        }
                    });
                });
            ui.ctx()
                .data_mut(|data| data.insert_temp(scroll_id, output.state.clamped_offset().x));
        },
    );
    if let Some(index) = close_requested {
        return Some(TabAction::Close(index));
    }
    if let Some(index) = select_requested {
        action = Some(TabAction::Select(index));
    }

    let right = ui
        .scope_builder(
            egui::UiBuilder::new()
                .max_rect(bar)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                ui.spacing_mut().item_spacing.x = 2.0;
                ui.add_space(2.0);
                if icon_button(ui, view.icons, "history", &t("Quick Open"), false) {
                    action = Some(TabAction::Command(mdview_ui::Command::QuickOpen));
                }
                if let Some(texture) = view.icons.ui_texture(ui.ctx(), "more") {
                    let image = egui::Image::new(&texture)
                        .fit_to_exact_size(egui::vec2(18.0, 18.0))
                        .tint(ui.visuals().text_color());
                    ui.scope(|ui| {
                        let visuals = ui.visuals_mut();
                        visuals.widgets.inactive.weak_bg_fill = egui::Color32::TRANSPARENT;
                        visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        visuals.widgets.inactive.bg_stroke = egui::Stroke::NONE;
                        visuals.widgets.hovered.bg_stroke = egui::Stroke::NONE;
                        visuals.widgets.active.bg_stroke = egui::Stroke::NONE;
                        ui.menu_image_button(image, |ui| {
                            ui.set_min_width(210.0);
                            if ui.button(t("Close Tab")).clicked() {
                                action = Some(TabAction::Command(mdview_ui::Command::CloseTab));
                                ui.close();
                            }
                            if ui.button(t("Close Other Tabs")).clicked() {
                                action = Some(TabAction::Command(mdview_ui::Command::CloseOtherTabs));
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(t("Move Tab Left")).clicked() {
                                action = Some(TabAction::Command(mdview_ui::Command::MoveTabLeft));
                                ui.close();
                            }
                            if ui.button(t("Move Tab Right")).clicked() {
                                action = Some(TabAction::Command(mdview_ui::Command::MoveTabRight));
                                ui.close();
                            }
                            ui.separator();
                            if ui.button(t("Save")).clicked() {
                                action = Some(TabAction::Command(mdview_ui::Command::Save));
                                ui.close();
                            }
                        })
                        .response
                        .on_hover_text(t("Tab actions"));
                    });
                }
                if icon_button(ui, view.icons, "plus", &t("New tab"), false) {
                    action = Some(TabAction::New);
                }
                if let Some(mode) = mode_control(ui, view.mode, view.can_view, view.can_edit, view.can_split, &t) {
                    action = Some(TabAction::Command(mdview_ui::Command::SetMode(mode)));
                }
            },
        )
        .response
        .rect
        .left();
    if right < bar.left() {
        // no room for controls; the tab strip still works
    }
    action
}

fn mode_control(
    ui: &mut egui::Ui,
    mode: ViewMode,
    can_view: bool,
    can_edit: bool,
    can_split: bool,
    t: &dyn Fn(&str) -> String,
) -> Option<ViewMode> {
    let options = [
        (ViewMode::View, "View", can_view),
        (ViewMode::Edit, "Editor", can_edit),
        (ViewMode::Split, "Split", can_split),
    ];
    let width = 156.0;
    let (rect, _) = ui.allocate_exact_size(egui::vec2(width, 24.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(5), ui.visuals().extreme_bg_color);
    let segment = width / 3.0;
    let mut selected = None;
    for (index, (candidate, label, enabled)) in options.iter().enumerate() {
        let segment_rect = egui::Rect::from_min_size(
            egui::pos2(rect.left() + index as f32 * segment + 1.0, rect.top() + 1.0),
            egui::vec2(segment - 2.0, rect.height() - 2.0),
        );
        let response = ui.interact(
            segment_rect,
            ui.make_persistent_id(("mode_segment", index)),
            egui::Sense::click(),
        );
        let active = *candidate == mode;
        if active {
            ui.painter().rect_filled(
                segment_rect,
                egui::CornerRadius::same(4),
                ui.visuals().selection.bg_fill,
            );
        } else if response.hovered() && *enabled {
            ui.painter().rect_filled(
                segment_rect,
                egui::CornerRadius::same(4),
                ui.visuals().widgets.hovered.weak_bg_fill,
            );
        }
        let color = if !*enabled {
            ui.visuals().weak_text_color().gamma_multiply(0.45)
        } else if active {
            ui.visuals().selection.stroke.color
        } else {
            ui.visuals().text_color()
        };
        ui.painter().text(
            segment_rect.center(),
            egui::Align2::CENTER_CENTER,
            t(label),
            egui::FontId::proportional(11.5),
            color,
        );
        if *enabled && response.clicked() && !active {
            selected = Some(*candidate);
        }
        if response.hovered() {
            ui.ctx().set_cursor_icon(egui::CursorIcon::PointingHand);
        }
    }
    selected
}

#[derive(Clone, Copy)]
enum TabHit {
    Select,
    Close,
}

fn tab_button(
    ui: &mut egui::Ui,
    tab: &TabInfo,
    index: usize,
    active: bool,
    icons: &mut mdview_ui::MaterialIcons,
    light: bool,
    t: &dyn Fn(&str) -> String,
) -> Option<TabHit> {
    let text_width = ui
        .painter()
        .layout_no_wrap(
            tab.label.clone(),
            egui::FontId::proportional(12.0),
            egui::Color32::WHITE,
        )
        .rect
        .width();
    let width = (text_width + 58.0).clamp(96.0, 212.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 34.0), egui::Sense::click());
    let active_t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("tab_active"), active, 0.16);
    let hover_t = ui
        .ctx()
        .animate_bool_with_time(response.id.with("tab_hover"), response.hovered() && !active, 0.10);
    let radius = egui::CornerRadius {
        nw: 4,
        ne: 4,
        sw: 0,
        se: 0,
    };
    if active_t > 0.01 {
        ui.painter()
            .rect_filled(rect, radius, ui.visuals().panel_fill.gamma_multiply(active_t));
    }
    if hover_t > 0.01 {
        ui.painter().rect_filled(
            rect,
            radius,
            ui.visuals().widgets.hovered.weak_bg_fill.gamma_multiply(hover_t),
        );
    }
    let icon_rect = egui::Rect::from_center_size(
        egui::pos2(rect.left() + 15.0, rect.center().y),
        egui::vec2(mdview_ui::ICON_MEDIUM, mdview_ui::ICON_MEDIUM),
    );
    let icon = icons
        .texture(ui.ctx(), &tab.icon, light)
        .or_else(|| icons.texture(ui.ctx(), "file", light));
    if let Some(texture) = icon {
        ui.painter().image(
            texture.id(),
            icon_rect,
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            egui::Color32::WHITE,
        );
    }
    let color = if tab.conflict || tab.missing {
        ui.visuals().warn_fg_color
    } else if active {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter().text(
        egui::pos2(rect.left() + 28.0, rect.center().y),
        egui::Align2::LEFT_CENTER,
        &tab.label,
        egui::FontId::proportional(12.5),
        color,
    );
    if tab.missing {
        let galley = ui
            .painter()
            .layout_no_wrap(tab.label.clone(), egui::FontId::proportional(12.5), color);
        let strike_y = rect.center().y;
        ui.painter().line_segment(
            [
                egui::pos2(rect.left() + 28.0, strike_y),
                egui::pos2(rect.left() + 28.0 + galley.rect.width(), strike_y),
            ],
            egui::Stroke::new(1.0, color),
        );
    }
    if tab.dirty && !response.hovered() && !active {
        ui.painter().circle_filled(
            egui::pos2(rect.right() - 12.0, rect.center().y),
            3.0,
            ui.visuals().weak_text_color(),
        );
    }
    let close_rect =
        egui::Rect::from_center_size(egui::pos2(rect.right() - 13.0, rect.center().y), egui::vec2(16.0, 16.0));
    let close = ui.interact(
        close_rect,
        ui.scope_id().with(("tab_close", index)),
        egui::Sense::click(),
    );
    // The close button is a separate widget. Using only the tab-body response made it fade out
    // as soon as the pointer moved onto the button, even though the click still worked.
    let tab_hovered = ui.rect_contains_pointer(rect);
    let close_t = ui.ctx().animate_bool_with_time(
        response.id.with("tab_close_visible"),
        active || tab_hovered || close.hovered(),
        0.12,
    );
    if close_t > 0.05
        && let Some(texture) = icons.ui_texture(ui.ctx(), "close")
    {
        if close.hovered() {
            ui.painter().rect_filled(
                close_rect,
                egui::CornerRadius::same(3),
                ui.visuals().widgets.hovered.weak_bg_fill,
            );
        }
        ui.painter().image(
            texture.id(),
            close_rect.shrink(2.0),
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
            ui.visuals().text_color().gamma_multiply(close_t),
        );
    }
    let response = response.on_hover_text(t("Select tab"));
    if close.clicked() {
        return Some(TabHit::Close);
    }
    if response.middle_clicked() {
        return Some(TabHit::Close);
    }
    if response.clicked() {
        return Some(TabHit::Select);
    }
    None
}

fn icon_button(
    ui: &mut egui::Ui,
    icons: &mut mdview_ui::MaterialIcons,
    icon: &str,
    tooltip: &str,
    active: bool,
) -> bool {
    let (rect, response) = ui.allocate_exact_size(egui::vec2(26.0, 26.0), egui::Sense::click());
    let fill = if response.is_pointer_button_down_on() {
        ui.visuals().widgets.active.weak_bg_fill
    } else if active || response.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(5), fill);
    if let Some(texture) = icons.ui_texture(ui.ctx(), icon) {
        ui.put(
            rect.shrink(4.0),
            egui::Image::new(&texture)
                .fit_to_exact_size(egui::vec2(18.0, 18.0))
                .tint(ui.visuals().text_color()),
        );
    }
    response.on_hover_text(tooltip).clicked()
}
