//! Update dialog and polling.

use eframe::egui;

use crate::app::MdViewApp;
use crate::updater::{UpdateEvent, UpdateState, select_asset, start_check, start_download};

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
                            manifest_url: std::env::var("MDVIEW_UPDATE_MANIFEST")
                                .ok()
                                .filter(|url| !url.trim().is_empty())
                                .unwrap_or_else(|| self.prefs.get().updates.manifest_url.clone()),
                            downloading: None,
                        },
                        UpdateEvent::UpToDate => UpdateState::Info("You are running the latest version.".to_owned()),
                        UpdateEvent::Downloaded(path) => UpdateState::Downloaded(path),
                        UpdateEvent::Failed(error) => UpdateState::Failed(error),
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
                        UpdateEvent::Downloaded(path) => UpdateState::Downloaded(path),
                        UpdateEvent::Failed(error) => UpdateState::Failed(error),
                        UpdateEvent::Available(_) | UpdateEvent::UpToDate => UpdateState::Available {
                            manifest: manifest.clone(),
                            manifest_url: manifest_url.clone(),
                            downloading: None,
                        },
                    });
                }
            }
            _ => {}
        }
        if let Some(next) = next {
            self.update = Some(next);
            ctx.request_repaint();
        }
    }

    /// Show the update dialog and handle its actions.
    pub(crate) fn show_update_dialog(&mut self, ctx: &egui::Context) {
        let Some(mut state) = self.update.take() else {
            return;
        };
        let mut open = true;
        let mut download = None;
        let mut open_folder = None;
        egui::Window::new(self.t("Software update"))
            .open(&mut open)
            .resizable(false)
            .default_width(460.0)
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
                    ui.label(egui::RichText::new(format!("MD View {}", manifest.version)).strong());
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
                UpdateState::Downloaded(path) => {
                    ui.label(self.t("The update was downloaded and its checksum verified."));
                    ui.label(egui::RichText::new(path.display().to_string()).monospace().small());
                    if ui.button(self.t("Open downloads folder")).clicked() {
                        open_folder = Some(path.clone());
                    }
                    ui.label(self.t("Install it with your package manager or the platform installer."));
                }
                UpdateState::Info(message) => {
                    ui.label(message.as_str());
                }
                UpdateState::Failed(error) => {
                    ui.label(egui::RichText::new(error.as_str()).color(ui.visuals().error_fg_color));
                }
            });
        if let Some((manifest_url, asset)) = download
            && let UpdateState::Available { downloading, .. } = &mut state
        {
            *downloading = Some(start_download(manifest_url, asset));
        }
        if let Some(path) = open_folder
            && let Some(parent) = path.parent()
            && let Err(error) = open::that(parent)
        {
            log::warn!("could not open downloads folder: {error}");
        }
        if open {
            self.update = Some(state);
        }
    }
}
