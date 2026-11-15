//! The Editor tab of the settings dialog.

use super::{row, row_switch, section};
use crate::{Command, MaterialIcons, SettingsView, numeric_stepper};
use egui::Ui;

pub(super) fn editor(
    ui: &mut Ui,
    view: &SettingsView,
    t: &dyn Fn(&str) -> String,
    command: &mut Option<Command>,
    icons: &mut MaterialIcons,
) {
    section(ui, t("Editor"), t("Editing, typography, and split-view behavior."));
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

    let mut style = view.editor_style;
    let mut changed = false;
    row(
        ui,
        t("Font size"),
        t("Monospace size for the editor and code blocks."),
        |ui| {
            changed |= numeric_stepper(ui, icons, &mut style.font_size, 10.0..=22.0, 0.5, 112.0, |value| {
                format!("{value:.1} px")
            });
        },
    );
    row(ui, t("Line height"), t("Vertical spacing between editor rows."), |ui| {
        changed |= numeric_stepper(ui, icons, &mut style.line_height, 1.1..=2.0, 0.05, 112.0, |value| {
            format!("{value:.2}×")
        });
    });
    let mut beyond = style.scroll_beyond_last_line;
    if row_switch(
        ui,
        t("Scroll beyond last line"),
        t("Let the final line reach the top of the editor."),
        &mut beyond,
    ) {
        style.scroll_beyond_last_line = beyond;
        changed = true;
    }
    let mut guides = style.indent_guides;
    if row_switch(
        ui,
        t("Indent guides"),
        t("Draw subtle guides for each indentation level."),
        &mut guides,
    ) {
        style.indent_guides = guides;
        changed = true;
    }
    row(
        ui,
        t("Render whitespace"),
        t("Show spaces and tabs all the time, in the selection, or never."),
        |ui| {
            let mut whitespace = style.whitespace;
            egui::ComboBox::from_id_salt("settings_whitespace")
                .width(150.0)
                .selected_text(match whitespace {
                    mdview_editor::WhitespaceMode::None => t("Never"),
                    mdview_editor::WhitespaceMode::Selection => t("Selection"),
                    mdview_editor::WhitespaceMode::All => t("All"),
                })
                .show_ui(ui, |ui| {
                    for (mode, label) in [
                        (mdview_editor::WhitespaceMode::None, "Never"),
                        (mdview_editor::WhitespaceMode::Selection, "Selection"),
                        (mdview_editor::WhitespaceMode::All, "All"),
                    ] {
                        ui.selectable_value(&mut whitespace, mode, t(label));
                    }
                });
            if whitespace != style.whitespace {
                style.whitespace = whitespace;
                changed = true;
            }
        },
    );
    if changed {
        *command = Some(Command::SetEditorStyle(style));
    }
}
