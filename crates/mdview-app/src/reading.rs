//! Reading-view link activation and mode transitions.

use std::path::PathBuf;

use eframe::egui;
use mdview_markdown::LinkKind;
use mdview_render::PreviewOutput;

use crate::app::MdViewApp;
use crate::links::{LinkAction, link_error, resolve_link};
use crate::tabs::ViewMode;

impl MdViewApp {
    /// React to link clicks and copy actions from the rendered preview.
    pub(crate) fn handle_preview_output(&mut self, output: PreviewOutput, ctx: &egui::Context) {
        let Some(link) = output.link else {
            return;
        };
        self.activate_link(&link.destination, link.kind, ctx);
    }

    fn activate_link(&mut self, destination: &str, kind: LinkKind, ctx: &egui::Context) {
        let current = self.tabs.active().editor.document().path().map(PathBuf::from);
        match resolve_link(current.as_deref(), destination, kind) {
            LinkAction::Anchor(anchor) => {
                self.record_navigation();
                self.tabs.active_mut().preview.scroll_to_anchor(&anchor);
                ctx.request_repaint();
            }
            LinkAction::Open { path, anchor } => {
                if path.is_dir() {
                    self.open_workspace(&path, ctx);
                    return;
                }
                if let Err(error) = self.open_path(&path) {
                    self.show_error(link_error(
                        destination,
                        &format!("{} {error}", self.t("Could not open file.")),
                    ));
                    return;
                }
                let index = self.tabs.active_index();
                if let Some(tab) = self.tabs.tab_mut(index) {
                    if tab.is_markdown() {
                        tab.set_mode(ViewMode::View);
                    }
                    if let Some(anchor) = anchor {
                        tab.preview.scroll_to_anchor(&anchor);
                    }
                }
                self.sync_session();
                self.update_title(ctx);
                ctx.request_repaint();
            }
            LinkAction::External(url) => {
                if let Err(error) = open::that(&url) {
                    self.show_error(link_error(destination, &error.to_string()));
                }
            }
            LinkAction::Rejected(reason) => {
                self.show_error(link_error(destination, &reason));
            }
        }
    }
}
