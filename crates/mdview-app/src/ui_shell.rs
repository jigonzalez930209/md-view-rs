//! eframe shell, panels, and dialogs.

use std::time::Instant;

use craft_settings::SaveStatus;
use eframe::egui;

use crate::app::MdViewApp;
use crate::single_instance::Primary;
use crate::tabs::TabAction;

impl MdViewApp {
    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        let mut action = None;
        egui::Panel::top("tab_bar").show(ui, |ui| {
            action = crate::tabs::show_tab_bar(
                ui,
                &self.tabs.infos(),
                self.tabs.active_index(),
                &self.translations,
                self.lang,
            );
        });
        match action {
            Some(TabAction::Select(index)) => {
                self.tabs.set_active(index);
                self.update_title(ui.ctx());
            }
            Some(TabAction::Close(index)) => self.close_tab(index, ui.ctx()),
            Some(TabAction::New) => self.new_document(ui.ctx()),
            None => {}
        }
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        if !self.sidebar.open {
            return;
        }
        let mut events = Vec::new();
        let panel = egui::Panel::left("workspace_sidebar")
            .resizable(true)
            .default_size(self.sidebar.width)
            .show(ui, |ui| {
                if let Some(workspace) = &mut self.workspace {
                    events = self.sidebar.show(ui, workspace, &self.translations, self.lang);
                } else {
                    ui.add_space(8.0);
                    ui.label(egui::RichText::new(self.t("No folder open")).weak());
                    if ui.button(self.t("Open folder…")).clicked() {
                        self.ask_open_folder();
                    }
                }
            });
        self.sidebar.width = panel.response.rect.width();
        for event in &events {
            self.handle_sidebar_event(event, ui.ctx());
        }
        if let Some(workspace) = &self.workspace
            && let Some(warning) = workspace.watcher_warning()
        {
            ui.ctx().request_repaint();
            log::debug!("workspace watcher: {warning}");
        }
    }

    fn show_conflict(&mut self, ui: &mut egui::Ui) {
        let index = self.tabs.active_index();
        let name = self.tabs.active().editor.document().display_name();
        if self.tabs.active().missing {
            match crate::conflict::show_missing_bar(ui, &name, &self.translations, self.lang) {
                Some(crate::conflict::MissingAction::SaveAs) => self.ask_save_as(false),
                Some(crate::conflict::MissingAction::KeepEditing) => {
                    if let Some(tab) = self.tabs.tab_mut(index) {
                        tab.missing = false;
                    }
                    ui.ctx().request_repaint();
                }
                None => {}
            }
            return;
        }
        if !self.tabs.active().conflict {
            return;
        }
        if let Some(action) = crate::conflict::show_conflict_bar(ui, &name, &self.translations, self.lang) {
            self.confirm_conflict(index, action, ui.ctx());
        }
    }

    fn show_diff(&mut self, ctx: &egui::Context) {
        let Some(diff) = &mut self.diff else {
            return;
        };
        let name = self
            .tabs
            .tabs()
            .get(diff.index)
            .map(|tab| tab.editor.document().display_name())
            .unwrap_or_default();
        crate::conflict::show_diff_window(ctx, &name, &diff.mine, &diff.disk, &mut diff.open);
        if !diff.open {
            self.diff = None;
        }
    }

    fn poll_workspace(&mut self, ctx: &egui::Context) {
        let (changed, paths) = if let Some(workspace) = &mut self.workspace {
            let changed = workspace.poll();
            (changed, workspace.take_changed_paths())
        } else {
            (false, Vec::new())
        };
        if changed {
            ctx.request_repaint();
        }
        self.reconcile_paths(&paths, ctx);
        self.poll_open_files(ctx);
    }
}

impl eframe::App for MdViewApp {
    fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.poll_load(ctx);
        self.poll_workspace(ctx);
        if self.follow_system {
            let system_dark = ctx
                .input(|input| input.raw.system_theme)
                .is_some_and(|theme| theme == egui::Theme::Dark);
            if system_dark != self.last_system_dark {
                self.resolve_theme(ctx);
            }
        }
        mdview_theme::set_reduced_motion(ctx, self.prefs.get().media.reduce_motion);
        let media = self.prefs.get().media.clone();
        self.media.set_policy(crate::media_runtime::MediaPolicy {
            remote_images: media.remote_images,
            private_networks: media.private_networks,
        });
        self.media.set_playing(media.animate_svg);
        self.media.set_reduced_motion(media.reduce_motion);
        self.media.poll(ctx);
        let forwarded = self.primary.as_ref().map(Primary::take_paths).unwrap_or_default();
        for path in forwarded {
            self.handle_external_path(&path, ctx);
        }
        let dropped = ctx.input(|input| input.raw.dropped_files.clone());
        for file in dropped {
            let path = file.path();
            if !path.as_os_str().is_empty() {
                self.handle_external_path(path, ctx);
            }
        }
        if self.last_window_poll.elapsed() >= std::time::Duration::from_secs(1) {
            self.last_window_poll = Instant::now();
            if let Some(rect) = ctx.input(|input| input.viewport().inner_rect) {
                self.prefs.update(|prefs| {
                    prefs.window.width = rect.width();
                    prefs.window.height = rect.height();
                });
            }
        }

        self.handle_dialog_answers(ctx);

        let tick = self.prefs.tick(Instant::now());
        if let Some(deadline) = tick.next_deadline {
            ctx.request_repaint_after(deadline.saturating_duration_since(Instant::now()));
        }
        if let SaveStatus::Failed(error) = tick.status {
            log::error!("settings save failed: {error}");
        }

        self.handle_shortcuts(ctx);
        self.handle_close_request(ctx);

        // Keep the disk watcher/polling path alive while files are open, even when egui is idle.
        if self
            .tabs
            .tabs()
            .iter()
            .any(|tab| tab.editor.document().path().is_some())
        {
            ctx.request_repaint_after(std::time::Duration::from_millis(500));
        }
    }

    fn ui(&mut self, ui: &mut egui::Ui, frame: &mut eframe::Frame) {
        self.update_title(ui.ctx());

        let mut command = None;
        egui::Panel::top("menu_bar").show(ui, |ui| {
            let status = self.tabs.editor().status();
            command = mdview_ui::show_menu(
                ui,
                mdview_ui::MenuState {
                    translations: &self.translations,
                    lang: self.lang,
                    theme: self.theme,
                    theme_id: self.theme_id,
                    follow_system: self.follow_system,
                    language: self.language,
                    document_name: &self.tabs.editor().document().display_name(),
                    dirty: self.tabs.editor().document().is_dirty(),
                    wrap: status.wrap,
                    line_numbers: status.line_numbers,
                    sidebar_open: self.sidebar.open,
                    workspace_open: self.workspace.is_some(),
                    recent_files: &self.prefs.get().session.recent_files,
                    recent_workspaces: &self.prefs.get().session.recent_workspaces,
                    tab_index: self.tabs.active_index(),
                    tab_count: self.tabs.len(),
                    mode: self.tabs.active().mode,
                    is_markdown: self.tabs.active().is_markdown(),
                    sync_scroll: self.prefs.get().editor.sync_scroll,
                    remote_images: self.prefs.get().media.remote_images,
                    private_networks: self.prefs.get().media.private_networks,
                    animate_svg: self.prefs.get().media.animate_svg,
                    reduce_motion: self.prefs.get().media.reduce_motion,
                },
            );
        });
        if let Some(command) = command {
            self.handle_command(ui.ctx(), command);
        }

        self.show_tabs(ui);
        self.show_sidebar(ui);

        egui::Panel::bottom("status_bar").show(ui, |ui| {
            mdview_ui::show_status(
                ui,
                &self.translations,
                self.lang,
                self.tabs.editor().document(),
                &self.tabs.editor().status(),
            );
        });

        let mut open_requested = false;
        let mut preview_output = None;
        egui::CentralPanel::default().show(ui, |ui| {
            self.show_conflict(ui);
            let document = self.tabs.editor().document();
            if document.path().is_none() && document.is_empty() {
                open_requested = mdview_ui::show_welcome(ui, &self.translations, self.lang);
            } else {
                if self.tabs.active().is_markdown() && self.tabs.active().is_large() {
                    ui.label(
                        egui::RichText::new(self.t("Preview disabled for large files."))
                            .small()
                            .color(ui.visuals().warn_fg_color),
                    );
                }
                match self.tabs.active().mode {
                    mdview_ui::ViewMode::Edit => {
                        self.tabs.editor_mut().show(ui);
                    }
                    mdview_ui::ViewMode::View => {
                        let theme = mdview_theme::Theme::for_id(self.effective_theme);
                        let light = self.theme == mdview_theme::ThemeMode::Light;
                        let base = self
                            .tabs
                            .active()
                            .editor
                            .document()
                            .path()
                            .and_then(std::path::Path::parent)
                            .map(std::path::Path::to_path_buf);
                        let tab = self.tabs.active_mut();
                        tab.ensure_preview();
                        preview_output = Some(tab.preview.show_with_media(
                            ui,
                            &tab.preview_source,
                            &theme,
                            light,
                            base,
                            &mut self.media,
                        ));
                    }
                    mdview_ui::ViewMode::Split => {
                        let index = self.tabs.active_index();
                        self.show_split(ui, index);
                    }
                }
            }
        });
        if open_requested {
            self.ask_open();
        }
        if let Some(output) = preview_output {
            if let Some(lightbox) = output.lightbox.clone() {
                self.lightbox = Some(crate::lightbox::LightboxState {
                    request: lightbox.request,
                    alt: lightbox.alt,
                    scale: 1.0,
                });
            }
            self.handle_preview_output(output, ui.ctx());
        }

        let ctx = ui.ctx();
        if self.settings_open {
            let view = mdview_ui::SettingsView {
                theme_id: self.theme_id,
                follow_system: self.follow_system,
                language: self.language,
                zoom: ctx.zoom_factor(),
                wrap: self.tabs.editor().status().wrap,
                line_numbers: self.tabs.editor().status().line_numbers,
                sync_scroll: self.prefs.get().editor.sync_scroll,
                remote_images: self.prefs.get().media.remote_images,
                private_networks: self.prefs.get().media.private_networks,
                animate_svg: self.prefs.get().media.animate_svg,
                reduce_motion: self.prefs.get().media.reduce_motion,
            };
            if let Some(command) =
                mdview_ui::show_settings(ctx, &self.translations, self.lang, &mut self.settings_open, view)
            {
                self.handle_command(ctx, command);
            }
        }
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
                items
            } else {
                let entries: [(&str, mdview_ui::Command); 16] = [
                    ("New", mdview_ui::Command::New),
                    ("Open…", mdview_ui::Command::Open),
                    ("Open Folder…", mdview_ui::Command::OpenFolder),
                    ("Save", mdview_ui::Command::Save),
                    ("Save As…", mdview_ui::Command::SaveAs),
                    ("Settings…", mdview_ui::Command::Settings),
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
        if self.about_open {
            mdview_ui::show_about(ui.ctx(), &self.translations, self.lang, &mut self.about_open);
        }
        if self.close_prompt_open {
            self.show_close_prompt(ui.ctx());
        }
        self.show_lightbox(ui.ctx());
        if let Some(job) = &self.pending_load {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            let cancel = mdview_ui::show_loading(ui.ctx(), &self.translations, self.lang, job.path(), job.progress());
            if cancel && let Some(job) = self.pending_load.take() {
                job.cancel();
            }
        }
        self.show_diff(ui.ctx());

        self.dialogs.show_pending(Some(&*frame));
    }

    fn clear_color(&self, _visuals: &egui::Visuals) -> [f32; 4] {
        mdview_theme::Theme::for_id(self.effective_theme)
            .background
            .to_normalized_gamma_f32()
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.sync_session();
        if let Err(error) = self.prefs.flush() {
            log::error!("settings flush failed: {error}");
        }
    }
}
