//! Floating document and workspace search popover.

use egui::{Context, Frame, Id, Key, RichText, ScrollArea, TextEdit};

use mdview_i18n::Translations;

/// Search scope selected in the popover.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum SearchScope {
    /// Search the active document.
    #[default]
    Document,
    /// Search every indexed workspace file.
    Workspace,
}

/// One rendered result row.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SearchHitView {
    /// File name and line label.
    pub title: String,
    /// Matching line preview.
    pub detail: String,
    /// File heading shown before the first hit in that file.
    pub group: Option<String>,
}

/// What the user did in the search popover.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SearchAction {
    /// Change the search scope.
    Scope(SearchScope),
    /// Activate result `index`.
    Activate(usize),
    /// The query text changed.
    Changed,
    /// Move the selected result by `delta`.
    Select(isize),
    /// Close the popover.
    Close,
}

/// Inputs for the search popover.
pub struct SearchView<'a> {
    /// Translator.
    pub translations: &'a Translations,
    /// Active language.
    pub lang: craft_i18n::Lang,
    /// Current scope.
    pub scope: SearchScope,
    /// Query buffer.
    pub query: &'a mut String,
    /// Request focus for the query.
    pub focus: bool,
    /// Results.
    pub hits: &'a [SearchHitView],
    /// Selected result.
    pub selected: usize,
    /// Worker running.
    pub running: bool,
    /// Worker error.
    pub error: Option<&'a str>,
}

/// Draw the popover and return the requested action.
pub fn show_search(ctx: &Context, view: SearchView<'_>) -> Option<SearchAction> {
    let t = |source: &str| view.translations.t(view.lang, source);
    let mut action = None;
    let screen = ctx.content_rect();
    let width = 560.0_f32.min(screen.width() - 24.0);
    let position = egui::pos2(
        (screen.center().x - width * 0.5).max(screen.left() + 12.0),
        screen.top() + 40.0,
    );
    egui::Area::new(Id::unique("mdview_search"))
        .order(egui::Order::Foreground)
        .fixed_pos(position)
        .show(ctx, |ui| {
            Frame::new()
                .fill(ui.visuals().window_fill)
                .stroke(egui::Stroke::new(1.0, ui.visuals().window_stroke.color))
                .corner_radius(8)
                .shadow(egui::epaint::Shadow {
                    offset: [0, 6],
                    blur: 18,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(48),
                })
                .inner_margin(egui::Margin::same(10))
                .show(ui, |ui| {
                    ui.set_width(width - 20.0);
                    ui.horizontal(|ui| {
                        ui.label(RichText::new(t("Search")).strong().size(13.0));
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            if ui
                                .add(egui::Button::new(RichText::new("×").size(14.0)).frame(false))
                                .on_hover_text(t("Close"))
                                .clicked()
                            {
                                action = Some(SearchAction::Close);
                            }
                        });
                    });
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        for (scope, label) in [
                            (SearchScope::Document, t("This document")),
                            (SearchScope::Workspace, t("Workspace")),
                        ] {
                            let selected = view.scope == scope;
                            if ui.selectable_label(selected, label).clicked() && !selected {
                                action = Some(SearchAction::Scope(scope));
                            }
                        }
                    });
                    ui.add_space(4.0);
                    let query = ui.add(
                        TextEdit::singleline(view.query)
                            .hint_text(if view.scope == SearchScope::Workspace {
                                t("Search workspace")
                            } else {
                                t("Search this document")
                            })
                            .desired_width(f32::INFINITY)
                            .margin(egui::Margin::symmetric(6, 5)),
                    );
                    if view.focus {
                        query.request_focus();
                    }
                    if query.changed() {
                        action = Some(SearchAction::Changed);
                    }
                    if query.has_focus() {
                        if ctx.input(|input| input.key_pressed(Key::Escape)) {
                            action = Some(SearchAction::Close);
                        } else if ctx.input(|input| input.key_pressed(Key::Enter)) {
                            action = Some(SearchAction::Activate(view.selected));
                        } else if ctx.input(|input| input.key_pressed(Key::ArrowDown)) {
                            action = Some(SearchAction::Select(1));
                        } else if ctx.input(|input| input.key_pressed(Key::ArrowUp)) {
                            action = Some(SearchAction::Select(-1));
                        }
                    }
                    ui.add_space(4.0);
                    ui.horizontal(|ui| {
                        if view.running {
                            ui.spinner();
                        }
                        let count = view.hits.len();
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                count,
                                if count == 1 { t("result") } else { t("results") }
                            ))
                            .small()
                            .weak(),
                        );
                    });
                    if let Some(error) = view.error {
                        ui.label(RichText::new(error).small().color(ui.visuals().error_fg_color));
                    }
                    ui.separator();
                    ScrollArea::vertical()
                        .max_height(320.0)
                        .auto_shrink([false, true])
                        .show(ui, |ui| {
                            for (index, hit) in view.hits.iter().enumerate() {
                                if let Some(group) = &hit.group {
                                    ui.add_space(3.0);
                                    ui.label(
                                        RichText::new(group)
                                            .size(10.5)
                                            .strong()
                                            .color(ui.visuals().weak_text_color()),
                                    );
                                }
                                let selected = index == view.selected;
                                let response = ui.add(
                                    egui::Button::new(RichText::new(&hit.title).size(12.0).color(if selected {
                                        ui.visuals().selection.stroke.color
                                    } else {
                                        ui.visuals().text_color()
                                    }))
                                    .fill(if selected {
                                        ui.visuals().selection.bg_fill
                                    } else {
                                        egui::Color32::TRANSPARENT
                                    })
                                    .stroke(egui::Stroke::NONE)
                                    .min_size(egui::vec2(ui.available_width(), 0.0)),
                                );
                                if !hit.detail.is_empty() {
                                    ui.label(RichText::new(&hit.detail).small().color(ui.visuals().weak_text_color()));
                                }
                                if response.clicked() {
                                    action = Some(SearchAction::Activate(index));
                                }
                                ui.add_space(1.0);
                            }
                        });
                });
        });
    action
}
