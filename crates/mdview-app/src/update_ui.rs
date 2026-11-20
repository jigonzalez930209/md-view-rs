//! Update dialog and polling.

use eframe::egui;

use crate::app::MdViewApp;
use crate::install::{InstallKind, current_plan, spawn_restart};
use crate::updater::{UpdateEvent, UpdateState, select_asset, start_check, start_download, start_install};

impl MdViewApp {
    /// Check the configured manifest without blocking the UI.
    pub(crate) fn check_for_updates(&mut self) {
        if !self.prefs.get().updates.enabled {
            self.update = Some(UpdateState::Info("Update checking is disabled in settings.".to_owned()));
            return;
        }
        let url = std::env::var("MDVIEW_UPDATE_MANIFEST")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| self.prefs.get().updates.manifest_url.clone());
        if url.trim().is_empty() {
            self.update = Some(UpdateState::Info(
                "No update channel is configured. Set [updates] manifest_url in config.toml or the MDVIEW_UPDATE_MANIFEST environment variable.".to_owned(),
            ));
            return;
        }
        self.update = Some(UpdateState::Checking(start_check(
            url,
            env!("CARGO_PKG_VERSION").to_owned(),
        )));
    }

    /// Poll update workers and repaint when the state changes.
    pub(crate) fn poll_updates(&mut self, ctx: &egui::Context) {
        let mut next = None;
        match &self.update {
            Some(UpdateState::Checking(receiver)) => {
                if let Ok(event) = receiver.try_recv() {
                    next = Some(match event {
                        UpdateEvent::Available(manifest) => UpdateState::Available {
                            manifest,
                            manifest_url: self.manifest_url(),
                            downloading: None,
                        },
                        UpdateEvent::UpToDate => UpdateState::Info("You are running the latest version.".to_owned()),
                        UpdateEvent::Failed(error) => UpdateState::Failed(error),
                        UpdateEvent::Downloaded(_) | UpdateEvent::Installed(_) => {
                            UpdateState::Failed("Unexpected update event.".to_owned())
                        }
                    });
                }
            }
            Some(UpdateState::Available {
                manifest,
                manifest_url,
                downloading: Some(receiver),
            }) => {
                if let Ok(event) = receiver.try_recv() {
                    next = Some(match event {
                        UpdateEvent::Downloaded(path) => UpdateState::Downloaded { path, installing: None },
                        UpdateEvent::Installed(outcome) => UpdateState::Installed {
                            restart: outcome.restart,
                            message: outcome.message,
                        },
                        UpdateEvent::Failed(error) => UpdateState::Failed(error),
                        UpdateEvent::Available(_) | UpdateEvent::UpToDate => UpdateState::Available {
                            manifest: manifest.clone(),
                            manifest_url: manifest_url.clone(),
                            downloading: None,
                        },
                    });
                }
            }
            Some(UpdateState::Downloaded {
                path,
                installing: Some(receiver),
            }) => {
                if let Ok(event) = receiver.try_recv() {
                    next = Some(match event {
                        UpdateEvent::Installed(outcome) => UpdateState::Installed {
                            restart: outcome.restart,
                            message: outcome.message,
                        },
                        UpdateEvent::Failed(error) => UpdateState::Failed(error),
                        _ => UpdateState::Downloaded {
                            path: path.clone(),
                            installing: None,
                        },
                    });
                }
            }
            _ => {}
        }
        if let Some(next) = next {
            self.update = Some(next);
            ctx.request_repaint();
        } else if matches!(
            self.update,
            Some(
                UpdateState::Checking(_)
                    | UpdateState::Available {
                        downloading: Some(_),
                        ..
                    }
                    | UpdateState::Downloaded {
                        installing: Some(_),
                        ..
                    }
            )
        ) {
            // The update worker has no handle on the UI; look again shortly while it runs.
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    fn manifest_url(&self) -> String {
        std::env::var("MDVIEW_UPDATE_MANIFEST")
            .ok()
            .filter(|url| !url.trim().is_empty())
            .unwrap_or_else(|| self.prefs.get().updates.manifest_url.clone())
    }

    /// Show the update dialog and handle its actions.
    pub(crate) fn show_update_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.update.take() else {
            return;
        };
        let mut open = true;
        let mut download = None;
        let mut install_path = None;
        let mut restart = false;
        let mut open_folder = None;
        let mut warning = None;
        egui::Window::new(self.t("Software update"))
            .open(&mut open)
            .resizable(false)
            .default_width(480.0)
            .show(ctx, |ui| match &mut state {
                UpdateState::Checking(_) => {
                    ui.horizontal(|ui| {
                        ui.spinner();
                        ui.label(self.t("Checking for updates…"));
                    });
                }
                UpdateState::Available {
                    manifest,
                    manifest_url,
                    downloading,
                } => {
                    ui.label(
                        egui::RichText::new(format!("MD View {}", manifest.version))
                            .strong()
                            .color(ui.visuals().text_color()),
                    );
                    if !manifest.notes.is_empty() {
                        ui.label(&manifest.notes);
                    }
                    if downloading.is_some() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(self.t("Downloading update…"));
                        });
                    } else if let Some(asset) = select_asset(manifest) {
                        ui.label(format!("{} ({})", asset.name, asset.size));
                        if ui.button(self.t("Download")).clicked() {
                            download = Some((manifest_url.clone(), asset.clone()));
                        }
                    } else {
                        ui.label(self.t("The release has no asset for this platform."));
                    }
                }
                UpdateState::Downloaded { path, installing } => {
                    ui.label(self.t("The update was downloaded and its checksum verified."));
                    ui.label(egui::RichText::new(path.display().to_string()).monospace().small());
                    let plan = current_plan(path);
                    if installing.is_some() {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label(self.t("Installing update…"));
                        });
                    } else {
                        ui.label(plan.description);
                        if plan.kind != InstallKind::Unsupported && ui.button(self.t("Install update")).clicked() {
                            if self.tabs.any_dirty() {
                                warning = Some(self.t("Save or discard your changes before installing an update."));
                            } else {
                                install_path = Some(path.clone());
                            }
                        }
                        if ui.button(self.t("Open downloads folder")).clicked() {
                            open_folder = Some(path.clone());
                        }
                        if plan.kind == InstallKind::Unsupported {
                            ui.label(self.t("Install it with your package manager or the platform installer."));
                        }
                    }
                }
                UpdateState::Installed {
                    restart: needs_restart,
                    message,
                } => {
                    ui.label(message.as_str());
                    if *needs_restart && ui.button(self.t("Restart now")).clicked() {
                        restart = true;
                    }
                }
                UpdateState::Info(message) => {
                    ui.label(message.as_str());
                }
                UpdateState::Failed(error) => {
                    ui.label(egui::RichText::new(error.as_str()).color(ui.visuals().error_fg_color));
                }
            });
        if let Some(message) = warning {
            state = UpdateState::Failed(message);
        }
        if let Some((manifest_url, asset)) = download
            && let UpdateState::Available { downloading, .. } = &mut state
        {
            *downloading = Some(start_download(manifest_url, asset));
        }
        if let Some(path) = install_path
            && let UpdateState::Downloaded { installing, .. } = &mut state
        {
            *installing = Some(start_install(path));
        }
        if let Some(path) = open_folder
            && let Some(parent) = path.parent()
            && let Err(error) = open::that(parent)
        {
            log::warn!("could not open downloads folder: {error}");
        }
        if restart {
            match spawn_restart() {
                Ok(()) => {
                    self.allow_close = true;
                    ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                }
                Err(error) => {
                    state = UpdateState::Failed(error);
                    open = true;
                }
            }
        }
        if open {
            self.update = Some(state);
        }
    }
}
