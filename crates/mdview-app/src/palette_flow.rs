//! Command palette and quick-open flow.

use eframe::egui;

use crate::app::MdViewApp;

impl MdViewApp {
    pub(crate) fn show_palette_flow(&mut self, ctx: &egui::Context) {
        if self.palette_open {
            let items = if self.palette_files {
                let mut items = self
                    .tabs
                    .tabs()
                    .iter()
                    .enumerate()
                    .map(|(index, tab)| mdview_ui::PaletteItem {
                        label: tab.label(),
                        detail: tab
                            .editor
                            .document()
                            .path()
                            .map(|path| path.to_string_lossy().into_owned())
                            .unwrap_or_else(|| self.t("Untitled").to_owned()),
                        command: mdview_ui::Command::ActivateTab(index),
                    })
                    .collect::<Vec<_>>();
                for (index, path) in self.prefs.get().session.recent_files.iter().enumerate() {
                    items.push(mdview_ui::PaletteItem {
                        label: std::path::Path::new(path)
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_else(|| path.clone()),
                        detail: path.clone(),
                        command: mdview_ui::Command::OpenRecent(index),
                    });
                }
                for path in &self.quick_open_paths {
                    if self.tabs.find_path(path).is_some() {
                        continue;
                    }
                    items.push(mdview_ui::PaletteItem {
                        label: path
                            .file_name()
                            .map(|name| name.to_string_lossy().into_owned())
                            .unwrap_or_default(),
                        detail: path.to_string_lossy().into_owned(),
                        command: mdview_ui::Command::OpenPath(path.clone()),
                    });
                }
                items
            } else {
                let entries: Vec<(&str, mdview_ui::Command)> = vec![
                    ("New", mdview_ui::Command::New),
                    ("Open…", mdview_ui::Command::Open),
                    ("Open Folder…", mdview_ui::Command::OpenFolder),
                    ("Save", mdview_ui::Command::Save),
                    ("Save As…", mdview_ui::Command::SaveAs),
                    ("Settings…", mdview_ui::Command::Settings),
                    ("Rename File…", mdview_ui::Command::RenameFile),
                    ("Delete File…", mdview_ui::Command::DeleteFile),
                    ("Find", mdview_ui::Command::Find),
                    ("Replace", mdview_ui::Command::Replace),
                    ("Go to Line", mdview_ui::Command::GotoLine),
                    ("Edit mode", mdview_ui::Command::SetMode(mdview_ui::ViewMode::Edit)),
                    ("Reading view", mdview_ui::Command::SetMode(mdview_ui::ViewMode::View)),
                    ("Split view", mdview_ui::Command::SetMode(mdview_ui::ViewMode::Split)),
                    ("Word wrap", mdview_ui::Command::ToggleWrap),
                    ("Workspace sidebar", mdview_ui::Command::ToggleSidebar),
                    ("About MD View", mdview_ui::Command::About),
                    ("Quit", mdview_ui::Command::Quit),
                    ("Back", mdview_ui::Command::Back),
                    ("Forward", mdview_ui::Command::Forward),
                    ("Search", mdview_ui::Command::Search),
                    ("Search in Workspace…", mdview_ui::Command::SearchWorkspace),
                    ("Close Other Tabs", mdview_ui::Command::CloseOtherTabs),
                    ("Close Saved Tabs", mdview_ui::Command::CloseSavedTabs),
                    ("Next Tab", mdview_ui::Command::NextTab),
                    ("Previous Tab", mdview_ui::Command::PreviousTab),
                    ("Check for Updates…", mdview_ui::Command::CheckUpdates),
                    ("Open Logs", mdview_ui::Command::OpenLogs),
                    (
                        "Language: Follow system",
                        mdview_ui::Command::SetLanguage(mdview_i18n::LanguagePreference::Auto),
                    ),
                    (
                        "Language: English",
                        mdview_ui::Command::SetLanguage(mdview_i18n::LanguagePreference::English),
                    ),
                    (
                        "Language: Spanish",
                        mdview_ui::Command::SetLanguage(mdview_i18n::LanguagePreference::Spanish),
                    ),
                    (
                        "Language: Pseudo-locale",
                        mdview_ui::Command::SetLanguage(mdview_i18n::LanguagePreference::Pseudo),
                    ),
                ];
                entries
                    .into_iter()
                    .map(|(label, command)| mdview_ui::PaletteItem {
                        label: self.t(label),
                        detail: String::new(),
                        command,
                    })
                    .collect()
            };
            let title = if self.palette_files {
                self.t("Quick Open")
            } else {
                self.t("Command Palette")
            };
            if let Some(command) =
                mdview_ui::show_palette(ctx, &mut self.palette_open, &mut self.palette_query, &title, &items)
            {
                self.handle_command(ctx, command);
            }
        }
    }
}
