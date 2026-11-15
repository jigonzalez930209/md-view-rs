//! Keyboard menu groups and display.

use eframe::egui;
use mdview_ui::{Command, MenuEntry, MenuGroup};

use crate::app::MdViewApp;

impl MdViewApp {
    pub(crate) fn keyboard_menu_groups(&self) -> Vec<MenuGroup> {
        let t = |source: &str| self.t(source);
        let entry = |label: &str, command: Command| MenuEntry {
            label: t(label),
            command,
        };
        vec![
            MenuGroup {
                label: t("File"),
                entries: vec![
                    entry("New", Command::New),
                    entry("New File in Workspace", Command::NewFile),
                    entry("Open…", Command::Open),
                    entry("Open Folder…", Command::OpenFolder),
                    entry("Save", Command::Save),
                    entry("Save As…", Command::SaveAs),
                    entry("Rename File…", Command::RenameFile),
                    entry("Delete File…", Command::DeleteFile),
                    entry("Close Tab", Command::CloseTab),
                    entry("Close Folder", Command::CloseFolder),
                    entry("Quit", Command::Quit),
                ],
            },
            MenuGroup {
                label: t("Edit"),
                entries: vec![
                    entry("Find", Command::Find),
                    entry("Replace", Command::Replace),
                    entry("Search in Workspace…", Command::SearchWorkspace),
                    entry("Go to Line", Command::GotoLine),
                    entry("Command Palette…", Command::CommandPalette),
                ],
            },
            MenuGroup {
                label: t("View"),
                entries: vec![
                    entry("Edit mode", Command::SetMode(mdview_ui::ViewMode::Edit)),
                    entry("Reading view", Command::SetMode(mdview_ui::ViewMode::View)),
                    entry("Split view", Command::SetMode(mdview_ui::ViewMode::Split)),
                    entry("Synchronize scrolling", Command::ToggleSyncScroll),
                    entry("Word wrap", Command::ToggleWrap),
                    entry("Line numbers", Command::ToggleLineNumbers),
                    entry("Command Palette…", Command::CommandPalette),
                    entry("Quick Open…", Command::QuickOpen),
                ],
            },
            MenuGroup {
                label: t("Go"),
                entries: vec![
                    entry("Back", Command::Back),
                    entry("Forward", Command::Forward),
                    entry("Quick Open…", Command::QuickOpen),
                    entry("Next Tab", Command::NextTab),
                    entry("Previous Tab", Command::PreviousTab),
                ],
            },
            MenuGroup {
                label: t("Help"),
                entries: vec![
                    entry("Settings…", Command::Settings),
                    entry("Check for Updates…", Command::CheckUpdates),
                    entry("About MD View", Command::About),
                    entry("Open Logs", Command::OpenLogs),
                ],
            },
        ]
    }

    pub(crate) fn show_keyboard_menu(&mut self, ctx: &egui::Context) {
        let groups = self.keyboard_menu_groups();
        if let Some(command) =
            mdview_ui::show_keyboard_menu(ctx, &self.translations, self.lang, &mut self.keyboard_menu, &groups)
        {
            self.handle_command(ctx, command);
        }
    }
}
