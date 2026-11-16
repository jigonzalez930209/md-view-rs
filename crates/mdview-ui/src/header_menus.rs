//! Application menus for the title bar.

use super::*;

pub(super) fn file_menu(ui: &mut Ui, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    crate::compact_menu(ui);
    ui.set_min_width(230.0);
    if ui.button(t("New")).clicked() {
        *command = Some(Command::New);
        ui.close();
    }
    if ui.button(t("Open…")).clicked() {
        *command = Some(Command::Open);
        ui.close();
    }
    if ui.button(t("Open Folder…")).clicked() {
        *command = Some(Command::OpenFolder);
        ui.close();
    }
    ui.separator();
    if ui.button(t("Save")).clicked() {
        *command = Some(Command::Save);
        ui.close();
    }
    if ui.button(t("Save As…")).clicked() {
        *command = Some(Command::SaveAs);
        ui.close();
    }
    ui.menu_button(t("Reopen with Encoding"), |ui| {
        for encoding in [
            Encoding::Utf8,
            Encoding::Utf16Le,
            Encoding::Utf16Be,
            Encoding::Windows1252,
            Encoding::Iso8859_1,
            Encoding::ShiftJis,
        ] {
            if ui.button(encoding.label()).clicked() {
                *command = Some(Command::ReloadEncoding(encoding));
                ui.close();
            }
        }
    });
    ui.separator();
    if ui.button(t("Rename File…")).clicked() {
        *command = Some(Command::RenameFile);
        ui.close();
    }
    if ui.button(t("Delete File…")).clicked() {
        *command = Some(Command::DeleteFile);
        ui.close();
    }
    ui.separator();
    if ui.button(t("Close Tab")).clicked() {
        *command = Some(Command::CloseTab);
        ui.close();
    }
    if ui.button(t("Close Folder")).clicked() {
        *command = Some(Command::CloseFolder);
        ui.close();
    }
    ui.separator();
    if ui.button(t("Quit")).clicked() {
        *command = Some(Command::Quit);
        ui.close();
    }
}

pub(super) fn edit_menu(ui: &mut Ui, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    crate::compact_menu(ui);
    ui.set_min_width(220.0);
    if ui.button(t("Find")).clicked() {
        *command = Some(Command::Find);
        ui.close();
    }
    if ui.button(t("Search in Workspace…")).clicked() {
        *command = Some(Command::SearchWorkspace);
        ui.close();
    }
    if ui.button(t("Replace")).clicked() {
        *command = Some(Command::Replace);
        ui.close();
    }
    if ui.button(t("Go to Line")).clicked() {
        *command = Some(Command::GotoLine);
        ui.close();
    }
    if ui.button(t("Insert LaTeX Block")).clicked() {
        *command = Some(Command::InsertLatexBlock);
        ui.close();
    }
    ui.separator();
    if ui.button(t("Command Palette…")).clicked() {
        *command = Some(Command::CommandPalette);
        ui.close();
    }
}

pub(super) fn view_menu(
    ui: &mut Ui,
    t: &dyn Fn(&str) -> String,
    state: &TitleBarState<'_>,
    command: &mut Option<Command>,
) {
    crate::compact_menu(ui);
    ui.set_min_width(230.0);
    for mode in [ViewMode::Edit, ViewMode::View, ViewMode::Split] {
        let enabled = match mode {
            ViewMode::Edit => state.can_edit,
            ViewMode::View => state.can_view,
            ViewMode::Split => state.can_split,
        };
        let clicked = ui
            .add_enabled_ui(enabled, |ui| {
                ui.selectable_label(state.mode == mode, t(mode.label())).clicked()
            })
            .inner;
        if clicked {
            *command = Some(Command::SetMode(mode));
            ui.close();
        }
    }
    ui.separator();
    let sync = ui
        .add_enabled_ui(state.mode == ViewMode::Split, |ui| {
            ui.selectable_label(state.sync_scroll, t("Synchronize scrolling"))
                .clicked()
        })
        .inner;
    if sync {
        *command = Some(Command::ToggleSyncScroll);
        ui.close();
    }
    if ui.selectable_label(state.wrap, t("Word wrap")).clicked() {
        *command = Some(Command::ToggleWrap);
        ui.close();
    }
    if ui.selectable_label(state.line_numbers, t("Line numbers")).clicked() {
        *command = Some(Command::ToggleLineNumbers);
        ui.close();
    }
    ui.separator();
    if ui.button(t("Command Palette…")).clicked() {
        *command = Some(Command::CommandPalette);
        ui.close();
    }
    if ui.button(t("Quick Open…")).clicked() {
        *command = Some(Command::QuickOpen);
        ui.close();
    }
}

pub(super) fn go_menu(ui: &mut Ui, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    crate::compact_menu(ui);
    ui.set_min_width(200.0);
    if ui.button(t("Back")).clicked() {
        *command = Some(Command::Back);
        ui.close();
    }
    if ui.button(t("Forward")).clicked() {
        *command = Some(Command::Forward);
        ui.close();
    }
    ui.separator();
    if ui.button(t("Quick Open…")).clicked() {
        *command = Some(Command::QuickOpen);
        ui.close();
    }
    if ui.button(t("Next Tab")).clicked() {
        *command = Some(Command::NextTab);
        ui.close();
    }
    if ui.button(t("Previous Tab")).clicked() {
        *command = Some(Command::PreviousTab);
        ui.close();
    }
}

pub(super) fn help_menu(ui: &mut Ui, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
    crate::compact_menu(ui);
    ui.set_min_width(210.0);
    if ui.button(t("Settings…")).clicked() {
        *command = Some(Command::Settings);
        ui.close();
    }
    if ui.button(t("Check for Updates…")).clicked() {
        *command = Some(Command::CheckUpdates);
        ui.close();
    }
    if ui.button(t("About MD View")).clicked() {
        *command = Some(Command::About);
        ui.close();
    }
}
