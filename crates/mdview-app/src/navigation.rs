//! Back/forward navigation and the search popover.

use std::path::{Path, PathBuf};

use eframe::egui;
use mdview_ui::{SearchAction, SearchHitView, SearchScope, SearchView};

use crate::app::MdViewApp;
use crate::history::NavEntry;
use crate::search::{SearchHit, search_document};
use crate::tabs::ViewMode;

impl MdViewApp {
    /// Capture the active document position for history.
    pub(crate) fn current_location(&self) -> Option<NavEntry> {
        let tab = self.tabs.active();
        let path = tab.editor.document().path()?.to_path_buf();
        let line = if tab.mode.shows_preview() {
            tab.preview
                .source_map()
                .line_at_offset(tab.preview.scroll_offset())
                .map(|(line, _)| line)
                .unwrap_or_else(|| tab.editor.first_visible_line())
        } else {
            tab.editor.first_visible_line()
        };
        Some(NavEntry {
            path,
            line,
            anchor: None,
        })
    }

    /// Record the current location before navigating elsewhere.
    pub(crate) fn record_navigation(&mut self) {
        if self.history.suppress {
            return;
        }
        if let Some(entry) = self.current_location() {
            self.history.record(entry);
        }
    }

    /// Navigate back or forward through document history.
    pub(crate) fn navigate_history(&mut self, back: bool, ctx: &egui::Context) {
        let Some(current) = self.current_location() else {
            return;
        };
        let entry = if back {
            self.history.back(current)
        } else {
            self.history.forward(current)
        };
        let Some(entry) = entry else {
            return;
        };
        self.history.suppress = true;
        let result = self.open_path(&entry.path);
        if result.is_err() {
            self.history.suppress = false;
            return;
        }
        if let Some(index) = self.tabs.find_path(&entry.path) {
            self.tabs.set_active(index);
        }
        if let Some(tab) = self.tabs.tab_mut(self.tabs.active_index()) {
            if let Some(anchor) = &entry.anchor {
                tab.preview.scroll_to_anchor(anchor);
            } else {
                tab.editor.goto_line(entry.line + 1);
                tab.preview.scroll_to_line(entry.line, 0.0);
            }
        }
        self.history.suppress = false;
        self.update_title(ctx);
        ctx.request_repaint();
    }

    /// Show the title-bar search popover.
    pub(crate) fn show_search_ui(&mut self, ctx: &egui::Context) {
        if !self.search.open {
            self.tabs.active_mut().preview.set_search_query(None);
            return;
        }
        self.search.poll();
        let highlight = if self.search.scope == SearchScope::Document {
            Some(self.search.query.clone())
        } else {
            None
        };
        self.tabs.active_mut().preview.set_search_query(highlight);
        if self.search.running {
            ctx.request_repaint_after(std::time::Duration::from_millis(80));
        }

        let query = self.search.query.clone();
        let revision = self.tabs.editor().document().revision();
        if self.search.needs_document_refresh(&query, revision) {
            let hits = search_document(self.tabs.editor().document(), &query);
            self.search.set_document_results(query.clone(), revision, hits);
        }
        if self.search.scope == SearchScope::Workspace
            && let Some(pending) = self.search.take_workspace_query()
            && let Some(workspace) = &self.workspace
        {
            let root = workspace.root().to_path_buf();
            let files = workspace.file_index(5_000);
            self.search.start_workspace(pending, root, files);
        }

        let missing_workspace = self.search.scope == SearchScope::Workspace && self.workspace.is_none();
        let error = if missing_workspace {
            Some(self.t("Open a folder to search the workspace."))
        } else {
            self.search.error.clone()
        };
        let mut previous_path = None;
        let hits: Vec<SearchHitView> = self
            .search
            .hits
            .iter()
            .map(|hit| {
                let group = (previous_path.as_ref() != Some(&hit.path)).then(|| search_group(&hit.path));
                previous_path = Some(hit.path.clone());
                search_hit_view(hit, group)
            })
            .collect();
        let mut edited_query = query;
        let focus = self.search.focus;
        let action = mdview_ui::show_search(
            ctx,
            SearchView {
                translations: &self.translations,
                lang: self.lang,
                scope: self.search.scope,
                query: &mut edited_query,
                focus,
                hits: &hits,
                selected: self.search.selected,
                running: self.search.running,
                error: error.as_deref(),
            },
        );
        self.search.focus = false;
        if edited_query != self.search.query {
            self.search.query = edited_query;
            self.search.query_changed();
        }
        let keyboard = if self.search.open {
            if ctx.input(|input| input.key_pressed(egui::Key::Enter)) {
                Some(SearchAction::Activate(self.search.selected))
            } else if ctx.input(|input| input.key_pressed(egui::Key::Escape)) {
                Some(SearchAction::Close)
            } else if ctx.input(|input| input.key_pressed(egui::Key::ArrowDown)) {
                Some(SearchAction::Select(1))
            } else if ctx.input(|input| input.key_pressed(egui::Key::ArrowUp)) {
                Some(SearchAction::Select(-1))
            } else {
                None
            }
        } else {
            None
        };
        let Some(action) = action.or(keyboard) else {
            return;
        };
        match action {
            SearchAction::Close => self.search.close(),
            SearchAction::Scope(scope) => self.search.set_scope(scope),
            SearchAction::Changed => self.search.query_changed(),
            SearchAction::Select(delta) => {
                if !self.search.hits.is_empty() {
                    let last = self.search.hits.len() - 1;
                    self.search.selected = if delta < 0 {
                        self.search.selected.saturating_sub(delta.unsigned_abs())
                    } else {
                        (self.search.selected + delta as usize).min(last)
                    };
                }
            }
            SearchAction::Activate(index) => {
                if let Some(hit) = self.search.hits.get(index).cloned() {
                    self.search.close();
                    self.activate_search_hit(&hit, ctx);
                }
            }
        }
    }

    fn activate_search_hit(&mut self, hit: &SearchHit, ctx: &egui::Context) {
        if let Err(error) = self.open_path(&hit.path) {
            self.show_error(format!("{} {error}", self.t("Could not open file.")));
            return;
        }
        if let Some(index) = self.tabs.find_path(&hit.path) {
            self.tabs.set_active(index);
        }
        if let Some(tab) = self.tabs.tab_mut(self.tabs.active_index())
            && !tab.is_viewer()
        {
            tab.set_mode(ViewMode::Edit);
            tab.editor.goto_line(hit.line + 1);
            let start = tab.editor.document().line_start_char(hit.line);
            tab.editor.move_cursor_to(start + hit.column);
            tab.preview.scroll_to_line(hit.line, 0.0);
        }
        self.sync_session();
        self.update_title(ctx);
        ctx.request_repaint();
    }
}

fn search_hit_view(hit: &SearchHit, group: Option<String>) -> SearchHitView {
    let path = hit
        .path
        .file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| "Untitled".to_owned());
    SearchHitView {
        title: format!("{path}:{}", hit.line + 1),
        detail: hit.preview.clone(),
        group,
    }
}

fn search_group(path: &Path) -> String {
    path.file_name()
        .map(|name| name.to_string_lossy().into_owned())
        .unwrap_or_else(|| path.display().to_string())
}

#[allow(dead_code)]
fn _relative(path: &Path) -> PathBuf {
    path.to_path_buf()
}
