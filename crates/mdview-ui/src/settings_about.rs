//! The About tab of the settings dialog.

use egui::{RichText, Ui};

use super::section;
use crate::{Command, SettingsView, secondary_button};

pub(super) fn about(ui: &mut Ui, view: &SettingsView, t: &dyn Fn(&str) -> String, command: &mut Option<Command>) {
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
