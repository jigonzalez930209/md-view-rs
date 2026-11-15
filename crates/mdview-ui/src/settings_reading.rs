//! The Reading tab of the settings dialog.

use egui::Ui;
use mdview_theme::ReadingMargins;

use super::{row, section};
use crate::{Command, MaterialIcons, SettingsView, numeric_stepper};

pub(super) fn reading(
    ui: &mut Ui,
    view: &SettingsView,
    t: &dyn Fn(&str) -> String,
    command: &mut Option<Command>,
    icons: &mut MaterialIcons,
) {
    section(ui, t("Reading"), t("Markdown width, margins, and per-area text sizes."));
    let mut percent = view.reading_margins.percent();
    row(
        ui,
        t("Margins"),
        t("Left and right margins as a percentage of the pane."),
        |ui| {
            ui.spacing_mut().slider_width = 120.0;
            ui.horizontal(|ui| {
                ui.add(
                    egui::Slider::new(&mut percent, ReadingMargins::MIN_PERCENT..=ReadingMargins::MAX_PERCENT)
                        .integer()
                        .show_value(false),
                );
                ui.label(egui::RichText::new(format!("{percent:.0}%")).monospace().size(11.0));
            });
        },
    );
    if (percent - view.reading_margins.percent()).abs() > f32::EPSILON {
        *command = Some(Command::SetReadingMargins(ReadingMargins::new(percent)));
    }

    let mut typography = view.typography;
    let mut changed = false;
    size_row(
        ui,
        t("Body text"),
        "Paragraphs, lists, quotes, and alerts.",
        &mut typography.body,
        10.0..=22.0,
        icons,
        &mut changed,
    );
    size_row(
        ui,
        t("Headings"),
        "Base heading size; lower levels scale down.",
        &mut typography.headings,
        16.0..=48.0,
        icons,
        &mut changed,
    );
    size_row(
        ui,
        t("Code"),
        "Fenced and inline code.",
        &mut typography.code,
        9.0..=22.0,
        icons,
        &mut changed,
    );
    size_row(
        ui,
        t("Tables"),
        "Table headers and cells.",
        &mut typography.tables,
        9.0..=22.0,
        icons,
        &mut changed,
    );
    size_row(
        ui,
        t("Equations"),
        "Inline and display math size.",
        &mut typography.math,
        10.0..=36.0,
        icons,
        &mut changed,
    );
    if changed {
        *command = Some(Command::SetReadingTypography(typography));
    }
}

fn size_row(
    ui: &mut Ui,
    title: String,
    description: &str,
    value: &mut f32,
    range: std::ops::RangeInclusive<f32>,
    icons: &mut MaterialIcons,
    changed: &mut bool,
) {
    row(ui, title, description.to_owned(), |ui| {
        *changed |= numeric_stepper(ui, icons, value, range, 0.5, 112.0, |value| format!("{value:.1} px"));
    });
}
