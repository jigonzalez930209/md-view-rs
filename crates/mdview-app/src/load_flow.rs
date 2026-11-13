//! Large-file loading flow.

use std::path::Path;

use mdview_model::Document;
use mdview_theme::ThemeMode;

use crate::app::MdViewApp;
use crate::file_ops::{self, FileError};
use crate::load_job::{LoadEvent, LoadJob};

impl MdViewApp {
    pub(crate) fn load_document(&mut self, path: &Path) -> Result<(), FileError> {
        if let Some(job) = self.pending_load.take() {
            job.cancel();
        }
        let length = std::fs::metadata(path)?.len();
        log::debug!("load_document {} length={length}", path.display());
        if length > 10 * 1024 * 1024 {
            log::debug!("starting background load");
            self.pending_load = Some(LoadJob::spawn(path.to_path_buf())?);
            return Ok(());
        }
        let document = file_ops::load(path)?;
        self.apply_document(document);
        Ok(())
    }

    fn apply_document(&mut self, document: Document) {
        self.editor.replace_document(document);
        self.editor.set_light(self.theme == ThemeMode::Light);
        if !self.editor.status().large_mode {
            self.editor.set_wrap(self.prefs.get().editor.wrap);
        }
    }

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
        log::debug!("background load finished: ok={}", result.is_ok());
        self.pending_load = None;
        match result {
            Ok(document) => {
                self.apply_document(document);
                self.update_title(ctx);
            }
            Err(FileError::Cancelled) => {}
            Err(error) => self.show_error(format!("{} {error}", self.t("Could not open file."))),
        }
    }
}
