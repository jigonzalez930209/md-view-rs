//! Session persistence and background load polling.

use super::*;

impl MdViewApp {
    pub(crate) fn sync_session(&mut self) {
        let open_files = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| tab.editor.document().path())
            .map(|path| path.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        let active_file = self
            .tabs
            .active()
            .editor
            .document()
            .path()
            .map(|path| path.to_string_lossy().into_owned());
        let workspace = self
            .workspace
            .as_ref()
            .map(|workspace| workspace.root().to_string_lossy().into_owned());
        let tab_scroll = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.editor.scroll_offset()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let tab_view_scroll = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.view_scroll()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let tab_modes = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.mode.as_setting().to_owned()))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let tab_split_ratio = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                tab.editor
                    .document()
                    .path()
                    .map(|path| (path.to_string_lossy().into_owned(), tab.split_ratio))
            })
            .collect::<std::collections::BTreeMap<_, _>>();
        let sidebar_open = self.sidebar.open;
        let sidebar_width = self.sidebar.width;
        let show_hidden = self.sidebar.show_hidden;
        let respect_gitignore = self.sidebar.respect_gitignore;
        self.prefs.update(|prefs| {
            prefs.session.open_files = open_files;
            prefs.session.active_file = active_file;
            prefs.session.workspace = workspace;
            prefs.session.tab_scroll = tab_scroll;
            prefs.session.tab_view_scroll = tab_view_scroll;
            prefs.session.tab_modes = tab_modes;
            prefs.session.tab_split_ratio = tab_split_ratio;
            prefs.session.sidebar_open = sidebar_open;
            prefs.session.sidebar_width = sidebar_width;
            prefs.session.show_hidden = show_hidden;
            prefs.session.respect_gitignore = respect_gitignore;
        });
    }

    /// Poll the background loader and open its document when it finishes.
    pub(crate) fn poll_load(&mut self, ctx: &egui::Context) {
        let mut finished = None;
        if let Some(job) = self.pending_load.as_mut() {
            while let Some(event) = job.poll() {
                if let LoadEvent::Done(result) = event {
                    finished = Some(result);
                    break;
                }
            }
        }
        let Some(result) = finished else {
            return;
        };
        self.pending_load = None;
        match result {
            Ok(document) => {
                let mtime = document
                    .path()
                    .and_then(|path| std::fs::metadata(path).ok())
                    .and_then(|metadata| metadata.modified().ok());
                self.open_document(document, mtime);
                self.update_title(ctx);
            }
            Err(FileError::Cancelled) => {}
            Err(error) => self.show_error(format!("{} {error}", self.t("Could not open file."))),
        }
    }
}
