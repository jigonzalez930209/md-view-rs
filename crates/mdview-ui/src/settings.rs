//! The settings dialog.
//!
//! Settings use a centered modal with a small tab rail and grouped rows. Each row pairs a label
//! and explanation with one control, so the dialog stays scannable at a glance.

use egui::{Button, Context, Frame, Id, Key, Modal, RichText, ScrollArea, Ui};
use mdview_i18n::{LanguagePreference, Translations};
use mdview_theme::ThemeId;

use crate::{Command, MaterialIcons, numeric_stepper, primary_button, secondary_button};

/// Current values shown by the settings dialog.
#[derive(Clone, Debug, PartialEq)]
pub struct SettingsView {
    /// Theme identifier.
    pub theme_id: ThemeId,
    /// Follow the system light/dark mode.
    pub follow_system: bool,
    /// UI language.
    pub language: LanguagePreference,
    /// Text scale.
    pub zoom: f32,
    /// Soft wrap.
    pub wrap: bool,
    /// Line numbers.
    pub line_numbers: bool,
    /// Split scroll synchronization.
    pub sync_scroll: bool,
    /// Remote images.
    pub remote_images: bool,
    /// Private-network images.
    pub private_networks: bool,
    /// SVG animations.
    pub animate_svg: bool,
    /// Reduced motion.
    pub reduce_motion: bool,
    /// Show hidden files in the explorer.
    pub show_hidden: bool,
    /// Respect .gitignore in the explorer.
    pub respect_gitignore: bool,
    /// Bytes used by the rotating diagnostic log.
    pub log_bytes: u64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum SettingsTab {
    #[default]
    Appearance,
    Editor,
    Files,
    Media,
    About,
}

impl SettingsTab {
    const ALL: [Self; 5] = [Self::Appearance, Self::Editor, Self::Files, Self::Media, Self::About];

    fn label(self) -> &'static str {
        match self {
            Self::Appearance => "Appearance",
            Self::Editor => "Editor",
            Self::Files => "Explorer",
            Self::Media => "Media",
            Self::About => "About",
        }
    }
}

/// Show the settings dialog and return the selected change.
pub fn show_settings(
    ctx: &Context,
    translations: &Translations,
    lang: craft_i18n::Lang,
    open: &mut bool,
    view: SettingsView,
    icons: &mut MaterialIcons,
) -> Option<Command> {
    let t = |source: &str| translations.t(lang, source);
    let mut command = None;
    let mut keep_open = true;
    let mut tab = ctx.data_mut(|data| *data.get_temp_mut_or_default(Id::unique("mdview_settings_tab")));

    let visuals = ctx.style_of(ctx.theme()).visuals.clone();
    let dialog = Modal::new(Id::unique("mdview_settings"))
        .frame(
            Frame::new()
                .fill(visuals.window_fill)
                .stroke(egui::Stroke::new(1.0, visuals.window_stroke.color))
                .corner_radius(10)
                .inner_margin(egui::Margin::same(18))
                .shadow(egui::epaint::Shadow {
                    offset: [0, 8],
                    blur: 24,
                    spread: 0,
                    color: egui::Color32::from_black_alpha(60),
                }),
        )
        .show(ctx, |ui| {
            let available = ctx.content_rect().size();
            let dialog_width = (available.x - 48.0).clamp(560.0, 620.0);
            let body_height = (available.y - 150.0).clamp(240.0, 352.0);
            ui.set_min_width(dialog_width);
            ui.set_max_width(dialog_width);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(t("Settings"))
                        .size(18.0)
                        .strong()
                        .color(ui.visuals().text_color()),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui
                        .add(Button::new(RichText::new("×").size(14.0)).frame(false))
                        .on_hover_text(t("Close"))
                        .clicked()
                    {
                        keep_open = false;
                    }
                });
            });
            ui.add_space(10.0);

            let rail_width = 128.0;
            let content_width = dialog_width - rail_width - 36.0;
            let content_height = body_height;
            ui.allocate_ui_with_layout(
                egui::vec2(dialog_width, content_height),
                egui::Layout::left_to_right(egui::Align::Min),
                |ui| {
                    ui.allocate_ui_with_layout(
                        egui::vec2(rail_width, content_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(rail_width);
                            ui.spacing_mut().item_spacing.y = 2.0;
                            for candidate in SettingsTab::ALL {
                                if tab_button(ui, &t(candidate.label()), candidate == tab) {
                                    tab = candidate;
                                }
                            }
                        },
                    );
                    ui.separator();
                    ui.add_space(10.0);
                    ui.allocate_ui_with_layout(
                        egui::vec2(content_width, content_height),
                        egui::Layout::top_down(egui::Align::Min),
                        |ui| {
                            ui.set_width(content_width);
                            ScrollArea::vertical()
                                .auto_shrink([false, false])
                                .max_width(content_width)
                                .max_height(content_height)
                                .show(ui, |ui| {
                                    ui.set_width(content_width - 10.0);
                                    match tab {
                                        SettingsTab::Appearance => appearance(ui, &view, &t, &mut command, icons),
                                        SettingsTab::Editor => editor(ui, &view, &t, &mut command),
                                        SettingsTab::Files => files(ui, &view, &t, &mut command),
                                        SettingsTab::Media => media(ui, &view, &t, &mut command),
                                        SettingsTab::About => about(ui, &view, &t, &mut command),
                                    }
                                });
                        },
                    );
                },
            );

            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            ui.horizontal(|ui| {
                ui.label(
                    RichText::new(format!("MD View {}", env!("CARGO_PKG_VERSION")))
                        .small()
                        .weak(),
                );
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if primary_button(ui, &t("Done"), false).clicked() {
                        keep_open = false;
                    }
                });
            });
        });

    ctx.data_mut(|data| data.insert_temp(Id::unique("mdview_settings_tab"), tab));
    if dialog.backdrop_response.clicked() || ctx.input(|input| input.key_pressed(Key::Escape)) {
        keep_open = false;
    }
    *open = keep_open;
    command
}

fn appearance(
    ui: &mut Ui,
    view: &SettingsView,
    t: &dyn Fn(&str) -> String,
    command: &mut Option<Command>,
    icons: &mut MaterialIcons,
) {
    section(ui, t("Appearance"), t("Theme, language, and text size."));
    let mut theme = view.theme_id;
    row(
        ui,
        t("Theme"),
        t("Color family for the editor, preview, and shell."),
        |ui| {
            egui::ComboBox::from_id_salt("settings_theme")
                .width(170.0)
                .selected_text(theme.label())
                .show_ui(ui, |ui| {
                    for id in ThemeId::ALL {
                        ui.selectable_value(&mut theme, id, id.label());
                    }
                });
        },
    );
    if theme != view.theme_id {
        *command = Some(Command::SetThemeId(theme));
    }
    let mut follow = view.follow_system;
    if row_switch(
        ui,
        t("Follow system"),
        t("Match the desktop light or dark preference."),
        &mut follow,
    ) {
        *command = Some(Command::SetFollowSystem(follow));
    }

    ui.add_space(6.0);
    let mut language = view.language;
    row(
        ui,
        t("Language"),
        t("Interface language and pseudo-locale for testing."),
        |ui| {
            egui::ComboBox::from_id_salt("settings_language")
                .width(170.0)
                .selected_text(language_label(language, t))
                .show_ui(ui, |ui| {
                    ui.selectable_value(&mut language, LanguagePreference::Auto, t("Follow system"));
                    ui.selectable_value(&mut language, LanguagePreference::English, t("English"));
                    ui.selectable_value(&mut language, LanguagePreference::Spanish, t("Spanish"));
                    ui.selectable_value(&mut language, LanguagePreference::Pseudo, t("Pseudo-locale"));
                });
        },
    );
    if language != view.language {
        *command = Some(Command::SetLanguage(language));
    }

    let mut zoom = view.zoom;
    let mut zoom_changed = false;
    row(
        ui,
        t("Text scale"),
        t("Scale the whole interface from 80% to 200%."),
        |ui| {
            zoom_changed = numeric_stepper(ui, icons, &mut zoom, 0.8..=2.0, 0.1, 112.0, |value| {
                format!("{:.0}%", value * 100.0)
            });
        },
    );
    if zoom_changed {
        *command = Some(Command::SetZoom(zoom));
    }
}

fn editor(ui: &mut Ui, view: &SettingsView, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    section(ui, t("Editor"), t("Editing and split-view behavior."));
    let mut wrap = view.wrap;
    if row_switch(ui, t("Word wrap"), t("Wrap long lines to the editor width."), &mut wrap) {
        *command = Some(Command::ToggleWrap);
    }
    let mut line_numbers = view.line_numbers;
    if row_switch(
        ui,
        t("Line numbers"),
        t("Show the line-number gutter."),
        &mut line_numbers,
    ) {
        *command = Some(Command::ToggleLineNumbers);
    }
    let mut sync = view.sync_scroll;
    if row_switch(
        ui,
        t("Synchronize scrolling"),
        t("Keep the editor and reading view on the same source line."),
        &mut sync,
    ) {
        *command = Some(Command::ToggleSyncScroll);
    }
}

fn files(ui: &mut Ui, view: &SettingsView, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    section(ui, t("Explorer"), t("What the workspace tree lists."));
    let mut hidden = view.show_hidden;
    if row_switch(
        ui,
        t("Show hidden files"),
        t("Include dotfiles and dot-directories."),
        &mut hidden,
    ) {
        *command = Some(Command::ToggleShowHidden);
    }
    let mut gitignore = view.respect_gitignore;
    if row_switch(
        ui,
        t("Respect .gitignore"),
        t("Hide ignored paths in Git repositories."),
        &mut gitignore,
    ) {
        *command = Some(Command::ToggleRespectGitignore);
    }
    ui.add_space(12.0);
    ui.horizontal(|ui| {
        if secondary_button(ui, &t("Open Folder…")).clicked() {
            *command = Some(Command::OpenFolder);
        }
        if secondary_button(ui, &t("Close Folder")).clicked() {
            *command = Some(Command::CloseFolder);
        }
    });
}

fn media(ui: &mut Ui, view: &SettingsView, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    section(ui, t("Media"), t("Images, SVG animation, and motion."));
    let mut remote = view.remote_images;
    if row_switch(
        ui,
        t("Load remote images"),
        t("Fetch images referenced by an https URL."),
        &mut remote,
    ) {
        *command = Some(Command::ToggleRemoteMedia);
    }
    let mut private = view.private_networks;
    if row_switch(
        ui,
        t("Allow private network images"),
        t("Permit images from localhost and private ranges."),
        &mut private,
    ) && remote
    {
        *command = Some(Command::TogglePrivateNetworks);
    }
    let mut animate = view.animate_svg;
    if row_switch(
        ui,
        t("Play SVG animations"),
        t("Animate SVG images in the reading view."),
        &mut animate,
    ) {
        *command = Some(Command::ToggleAnimations);
    }
    let mut reduce = view.reduce_motion;
    if row_switch(ui, t("Reduce motion"), t("Pause non-essential animation."), &mut reduce) {
        *command = Some(Command::ToggleReduceMotion);
    }
}

fn about(ui: &mut Ui, view: &SettingsView, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    section(ui, t("About MD View"), t("A native Markdown, text, and code editor."));
    ui.label(format!("MD View {}", env!("CARGO_PKG_VERSION")));
    ui.add_space(4.0);
    ui.label(
        RichText::new(t("Rendered with egui and a block-virtualized reading view."))
            .small()
            .weak(),
    );
    ui.add_space(14.0);
    if secondary_button(ui, &t("Check for Updates…")).clicked() {
        *command = Some(Command::CheckUpdates);
    }
    ui.add_space(14.0);
    ui.label(
        RichText::new(format!(
            "{} {:.1} KB / 1024 KB",
            t("Diagnostic log:"),
            view.log_bytes as f64 / 1024.0
        ))
        .small()
        .weak(),
    );
    if secondary_button(ui, &t("Open Logs Folder")).clicked() {
        *command = Some(Command::OpenLogs);
    }
}

fn section(ui: &mut Ui, title: String, description: String) {
    ui.label(
        RichText::new(title)
            .size(15.0)
            .strong()
            .color(ui.visuals().text_color()),
    );
    ui.label(RichText::new(description).small().weak());
    ui.add_space(10.0);
}

fn row(ui: &mut Ui, title: String, description: String, control: impl FnOnce(&mut Ui)) {
    Frame::new().inner_margin(egui::Margin::symmetric(0, 7)).show(ui, |ui| {
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.set_width(260.0);
                ui.label(RichText::new(title).size(12.5));
                ui.label(RichText::new(description).small().weak());
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                control(ui);
            });
        });
    });
}

fn row_switch(ui: &mut Ui, title: String, description: String, value: &mut bool) -> bool {
    let mut changed = false;
    row(ui, title, description, |ui| {
        changed = switch(ui, value);
    });
    changed
}

fn tab_button(ui: &mut Ui, label: &str, selected: bool) -> bool {
    let width = ui.available_width().clamp(96.0, 126.0);
    let (rect, response) = ui.allocate_exact_size(egui::vec2(width, 28.0), egui::Sense::click());
    let fill = if selected {
        ui.visuals().selection.bg_fill
    } else if response.hovered() {
        ui.visuals().widgets.hovered.weak_bg_fill
    } else {
        egui::Color32::TRANSPARENT
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(5), fill);
    if !selected {
        ui.painter().rect_stroke(
            rect,
            egui::CornerRadius::same(5),
            egui::Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color),
            egui::StrokeKind::Inside,
        );
    }
    ui.painter().text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        label,
        egui::FontId::proportional(12.5),
        ui.visuals().text_color(),
    );
    response.clicked()
}

fn switch(ui: &mut Ui, value: &mut bool) -> bool {
    let desired = egui::vec2(36.0, 19.0);
    let (rect, response) = ui.allocate_exact_size(desired, egui::Sense::click());
    let clicked = response.clicked();
    if clicked {
        *value = !*value;
    }
    let on = *value;
    let t = ui.ctx().animate_bool_with_time(response.id, on, 0.12);
    let fill = if on {
        ui.visuals().selection.bg_fill
    } else {
        ui.visuals().extreme_bg_color
    };
    ui.painter().rect_filled(rect, egui::CornerRadius::same(10), fill);
    ui.painter().rect_stroke(
        rect,
        egui::CornerRadius::same(10),
        egui::Stroke::new(1.0, ui.visuals().widgets.inactive.bg_stroke.color),
        egui::StrokeKind::Inside,
    );
    let knob_x = egui::lerp((rect.left() + 10.0)..=(rect.right() - 10.0), t);
    let knob = if on {
        ui.visuals().text_color()
    } else {
        ui.visuals().weak_text_color()
    };
    ui.painter()
        .circle_filled(egui::pos2(knob_x, rect.center().y), 6.5, knob);
    clicked
}

fn language_label(language: LanguagePreference, t: &dyn Fn(&str) -> String) -> String {
    match language {
        LanguagePreference::Auto => t("Follow system"),
        LanguagePreference::English => t("English"),
        LanguagePreference::Spanish => t("Spanish"),
        LanguagePreference::Pseudo => t("Pseudo-locale"),
    }
}
