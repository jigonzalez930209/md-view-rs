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
    let mut margins = view.reading_margins;
    row(
        ui,
        t("Margins"),
        t("Choose a centered measure, a compact reading width, or the full pane."),
        |ui| {
            egui::ComboBox::from_id_salt("settings_reading_margins")
                .width(170.0)
                .selected_text(margin_label(margins, t))
                .show_ui(ui, |ui| {
                    for candidate in ReadingMargins::ALL {
                        ui.selectable_value(&mut margins, candidate, margin_label(candidate, t));
                    }
                });
        },
    );
    if margins != view.reading_margins {
        *command = Some(Command::SetReadingMargins(margins));
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

fn margin_label(margins: ReadingMargins, t: &dyn Fn(&str) -> String) -> String {
    match margins {
        ReadingMargins::Comfortable => t("Comfortable"),
        ReadingMargins::Compact => t("Compact"),
        ReadingMargins::Full => t("Full width"),
    }
}
