//! eframe shell, panels, and dialogs.

use std::time::Instant;

use craft_settings::SaveStatus;
use eframe::egui;

use crate::app::MdViewApp;
use crate::single_instance::Primary;
use crate::tabs::TabAction;

impl MdViewApp {
    fn show_tabs(&mut self, ui: &mut egui::Ui) {
        let light = self.theme == mdview_theme::ThemeMode::Light;
        let infos = self.tabs.infos(&mut self.icons, light);
        let mut action = None;
        egui::Panel::top("tab_bar")
            .exact_size(mdview_ui::TAB_HEIGHT)
            .show(ui, |ui| {
                action = crate::tab_bar::show_tab_bar(
                    ui,
                    &infos,
                    self.tabs.active_index(),
                    crate::tab_bar::TabBarView {
                        translations: &self.translations,
                        lang: self.lang,
                        icons: &mut self.icons,
                        light,
                        mode: self.tabs.active().mode,
                        can_view: self.tabs.active().is_markdown() || self.tabs.active().is_viewer(),
                        can_edit: !self.tabs.active().is_viewer(),
                        can_split: self.tabs.active().is_markdown() && !self.tabs.active().is_large(),
                    },
                );
            });
        match action {
            Some(TabAction::Select(index)) => {
                self.tabs.set_active(index);
                self.update_title(ui.ctx());
            }
            Some(TabAction::Close(index)) => self.close_tab(index, ui.ctx()),
            Some(TabAction::New) => self.new_document(ui.ctx()),
            Some(TabAction::Command(command)) => self.handle_command(ui.ctx(), command),
            None => {}
        }
    }

    fn show_sidebar(&mut self, ui: &mut egui::Ui) {
        if !self.sidebar.should_show(self.workspace.is_some()) {
            return;
        }
        let mut events = Vec::new();
        let active = self.active_document_path();
        let external = self.external_document_paths();
        let panel = egui::Panel::left("workspace_sidebar")
            .resizable(true)
            .default_size(self.sidebar.width)
            .frame(
                egui::Frame::new()
                    .fill(ui.visuals().window_fill)
                    .inner_margin(egui::Margin::same(0)),
            )
            .show(ui, |ui| {
                if let Some(workspace) = &mut self.workspace {
                    events = self.sidebar.show(
                        ui,
                        workspace,
                        crate::sidebar::SidebarView {
                            translations: &self.translations,
                            lang: self.lang,
                            icons: &mut self.icons,
                            light: self.theme == mdview_theme::ThemeMode::Light,
                            active: active.as_deref(),
                            external: &external,
                        },
                    );
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

    fn handle_window_edges(&self, ctx: &egui::Context) {
        if !ctx.input(|input| input.pointer.primary_pressed()) {
            return;
        }
        let Some(position) = ctx.pointer_latest_pos() else {
            return;
        };
        let rect = ctx.content_rect();
        let edge = 4.0;
        let west = position.x <= rect.left() + edge;
        let east = position.x >= rect.right() - edge;
        let south = position.y >= rect.bottom() - edge;
        let direction = match (west, east, south) {
            (true, _, true) => Some(egui::ResizeDirection::SouthWest),
            (_, true, true) => Some(egui::ResizeDirection::SouthEast),
            (true, _, _) => Some(egui::ResizeDirection::West),
            (_, true, _) => Some(egui::ResizeDirection::East),
            (_, _, true) => Some(egui::ResizeDirection::South),
            _ => None,
        };
        if let Some(direction) = direction {
            ctx.send_viewport_cmd(egui::ViewportCommand::BeginResize(direction));
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
        self.frame_started = Instant::now();
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
        self.poll_updates(ctx);
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
        self.handle_window_edges(ui.ctx());

        let mut command = None;
        let maximized = ui.ctx().input(|input| input.viewport().maximized.unwrap_or(false));
        egui::Panel::top("app_header")
            .exact_size(mdview_ui::TITLE_HEIGHT)
            .show(ui, |ui| {
                let document_name = self.tabs.editor().document().display_name();
                let workspace_name = self
                    .workspace
                    .as_ref()
                    .map(|workspace| workspace.root_entry().name.clone());
                command = mdview_ui::show_title_bar(
                    ui,
                    mdview_ui::TitleBarState {
                        translations: &self.translations,
                        lang: self.lang,
                        document_name: &document_name,
                        dirty: self.tabs.editor().document().is_dirty(),
                        mode: self.tabs.active().mode,
                        is_markdown: self.tabs.active().is_markdown() || self.tabs.active().viewer_name().is_some(),
                        can_view: self.tabs.active().is_markdown() || self.tabs.active().is_viewer(),
                        can_edit: !self.tabs.active().is_viewer(),
                        can_split: self.tabs.active().is_markdown() && !self.tabs.active().is_large(),
                        sidebar_open: self.sidebar.open,
                        workspace_open: self.workspace.is_some(),
                        workspace_name: workspace_name.as_deref(),
                        sync_scroll: self.prefs.get().editor.sync_scroll,
                        wrap: self.tabs.editor().status().wrap,
                        line_numbers: self.tabs.editor().status().line_numbers,
                        maximized,
                        can_back: self.history.can_back(),
                        can_forward: self.history.can_forward(),
                        search_open: self.search.open,
                    },
                    &mut self.icons,
                );
            });
        if let Some(command) = command {
            self.handle_command(ui.ctx(), command);
        }

        // The explorer starts directly under the menu bar, as in VS Code/Cursor. The document
        // tab strip therefore belongs to the editor column only.
        self.show_sidebar(ui);
        self.show_tabs(ui);

        egui::Panel::bottom("status_bar").exact_size(24.0).show(ui, |ui| {
            mdview_ui::show_status(
                ui,
                &self.translations,
                self.lang,
                self.tabs.editor().document(),
                &self.tabs.editor().status(),
                self.tabs.active().viewer_name(),
            );
        });

        let mut open_requested = false;
        let mut preview_output = None;
        egui::CentralPanel::default().show(ui, |ui| {
            self.show_conflict(ui);
            let document = self.tabs.editor().document();
            if document.path().is_none() && document.is_empty() {
                open_requested = mdview_ui::show_welcome(ui, &self.translations, self.lang);
            } else if self.tabs.active().is_viewer() {
                self.show_viewer_document(ui);
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
                        let menu_open = self.keyboard_menu.open;
                        let editor = self.tabs.editor_mut();
                        editor.set_split_mode(false);
                        editor.set_ignore_input(menu_open);
                        editor.show(ui);
                        editor.take_user_scroll();
                        if editor.is_at_end() && editor.scrolling_recently(std::time::Duration::from_millis(350)) {
                            log::debug!(
                                "editor scroll end mode=edit wrap={} offset={:.1} max={:.1} clearance={:.1}",
                                editor.status().wrap,
                                editor.scroll_offset(),
                                editor.max_scroll_offset(),
                                editor.bottom_clearance()
                            );
                        }
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
                        tab.preview.set_reading_style(
                            self.prefs.get().reading.margins(),
                            self.prefs.get().reading.typography(),
                        );
                        preview_output = Some(tab.preview.show_with_media(
                            ui,
                            &tab.preview_source,
                            &theme,
                            light,
                            base,
                            &mut self.media,
                        ));
                        let end_key = !self.search.open
                            && !self.settings_open
                            && !self.palette_open
                            && !self.dialogs.is_open()
                            && ui.ctx().input(|input| input.key_pressed(egui::Key::End));
                        if end_key {
                            tab.preview.scroll_to_end();
                            ui.ctx().request_repaint();
                        }
                        if tab.preview.take_end_request() {
                            let metrics = tab.preview.scroll_metrics();
                            log::debug!(
                                "preview end via End offset={:.1} max={:.1} last_bottom={:.1} content={:.1}",
                                metrics.offset,
                                metrics.max_offset,
                                metrics.last_block_bottom,
                                metrics.content_height
                            );
                        }
                        if tab.preview.take_user_scroll() && tab.preview.is_at_end() {
                            let metrics = tab.preview.scroll_metrics();
                            log::debug!(
                                "preview scroll end offset={:.1} max={:.1} last_bottom={:.1} content={:.1}",
                                metrics.offset,
                                metrics.max_offset,
                                metrics.last_block_bottom,
                                metrics.content_height
                            );
                        }
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
                log::debug!("lightbox open alt={}", lightbox.alt);
                self.lightbox = Some(crate::lightbox::LightboxState::new(lightbox.request, lightbox.alt));
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
                reading_margins: self.prefs.get().reading.margins(),
                typography: self.prefs.get().reading.typography(),
                wrap: self.tabs.editor().status().wrap,
                line_numbers: self.tabs.editor().status().line_numbers,
                sync_scroll: self.prefs.get().editor.sync_scroll,
                editor_style: self.prefs.get().editor.style(),
                remote_images: self.prefs.get().media.remote_images,
                private_networks: self.prefs.get().media.private_networks,
                animate_svg: self.prefs.get().media.animate_svg,
                reduce_motion: self.prefs.get().media.reduce_motion,
                show_hidden: self.sidebar.show_hidden,
                respect_gitignore: self.sidebar.respect_gitignore,
                log_bytes: crate::logging::log_bytes(),
            };
            if let Some(command) = mdview_ui::show_settings(
                ctx,
                &self.translations,
                self.lang,
                &mut self.settings_open,
                view,
                &mut self.icons,
            ) {
                self.handle_command(ctx, command);
            }
        }
        self.show_palette_flow(ui.ctx());
        if self.about_open {
            mdview_ui::show_about(ui.ctx(), &self.translations, self.lang, &mut self.about_open);
        }
        if self.close_prompt_open {
            self.show_close_prompt(ui.ctx());
        }
        self.show_lightbox(ui.ctx());
        self.show_search_ui(ui.ctx());
        self.show_keyboard_menu(ui.ctx());
        self.show_update_dialog(ui.ctx());
        if let Some(job) = &self.pending_load {
            ui.ctx().request_repaint_after(std::time::Duration::from_millis(50));
            let cancel = mdview_ui::show_loading(ui.ctx(), &self.translations, self.lang, job.path(), job.progress());
            if cancel && let Some(job) = self.pending_load.take() {
                job.cancel();
            }
        }
        self.show_diff(ui.ctx());

        self.dialogs.show_pending(Some(&*frame));
        self.perf.record(self.frame_started.elapsed());
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
