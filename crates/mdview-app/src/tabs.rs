//! Open document tabs.
//!
//! A tab owns one editor and one document. Opening a path that is already open selects the
//! existing tab instead of creating a duplicate. The set also owns the editor presentation
//! settings (theme, wrap, line numbers) so every tab stays consistent.

use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::SystemTime;

use eframe::egui;
use mdview_editor::Editor;
use mdview_markdown::is_markdown_path;
use mdview_model::Document;
use mdview_render::Preview;

pub use mdview_ui::ViewMode;

/// One open document and its disk reconciliation state.
pub struct DocumentTab {
    /// The editor and its document.
    pub editor: Editor,
    /// The rendered Markdown view and its scroll state.
    pub preview: Preview,
    /// The active presentation.
    pub mode: ViewMode,
    /// The file modification time recorded after the last load or save.
    pub disk_mtime: Option<SystemTime>,
    /// Whether the file changed on disk while this tab had unsaved edits.
    pub conflict: bool,
    /// Whether the backing file no longer exists.
    pub missing: bool,
    /// Fraction of the split used by the preview (0.2–0.8).
    pub split_ratio: f32,
    preview_revision: Option<u64>,
    pub(crate) preview_source: String,
}

impl DocumentTab {
    /// Build a tab from a document.
    pub fn new(document: Document, disk_mtime: Option<SystemTime>) -> Self {
        static NEXT_ID: AtomicU64 = AtomicU64::new(1);
        let mode = if document.path().is_some_and(is_markdown_path) {
            ViewMode::View
        } else {
            ViewMode::Edit
        };
        let mut editor = Editor::new(document);
        editor.set_identity(NEXT_ID.fetch_add(1, Ordering::Relaxed));
        Self {
            editor,
            preview: Preview::new(),
            mode,
            disk_mtime,
            conflict: false,
            missing: false,
            split_ratio: 0.5,
            preview_revision: None,
            preview_source: String::new(),
        }
    }

    /// The tab label: the file name and a dirty marker.
    pub fn label(&self) -> String {
        let name = self.editor.document().display_name();
        let marker = if self.missing {
            "!"
        } else if self.editor.document().is_dirty() {
            " •"
        } else {
            ""
        };
        format!("{name}{marker}")
    }

    /// Whether the tab is a Markdown document that can be read.
    pub fn is_markdown(&self) -> bool {
        self.editor.document().path().is_some_and(is_markdown_path)
    }

    /// Set the presentation, refusing reading view for non-Markdown files.
    pub fn set_mode(&mut self, mode: ViewMode) {
        self.mode = if mode == ViewMode::View && !self.is_markdown() {
            ViewMode::Edit
        } else {
            mode
        };
    }

    /// Refresh the cached Markdown source when the document revision changes.
    pub fn ensure_preview(&mut self) -> &str {
        let revision = self.editor.document().revision();
        if self.preview_revision != Some(revision) {
            self.preview_source = self.editor.document().snapshot();
            self.preview_revision = Some(revision);
        }
        &self.preview_source
    }

    /// The view scroll offset for the session.
    pub fn view_scroll(&self) -> f32 {
        self.preview.scroll_offset()
    }

    /// Restore a view scroll offset from the session.
    pub fn restore_view_scroll(&mut self, offset: f32) {
        self.preview.set_scroll_offset(offset);
    }

    /// Restore the split ratio from the session.
    pub fn restore_split_ratio(&mut self, ratio: f32) {
        if ratio.is_finite() {
            self.split_ratio = ratio.clamp(0.2, 0.8);
        }
    }
}

/// A matching tab summary for the tab bar.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TabInfo {
    /// Visible label.
    pub label: String,
    /// Whether the document has unsaved changes.
    pub dirty: bool,
    /// Whether the file changed on disk while dirty.
    pub conflict: bool,
    /// Whether the backing file is gone.
    pub missing: bool,
}

/// What the user did in the tab bar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TabAction {
    /// Select a tab.
    Select(usize),
    /// Close a tab.
    Close(usize),
    /// Add an untitled tab.
    New,
}

/// The set of open documents.
pub struct TabSet {
    tabs: Vec<DocumentTab>,
    active: usize,
    light: bool,
    wrap: bool,
    line_numbers: bool,
}

impl TabSet {
    /// Start with one untitled document.
    pub fn new(light: bool, wrap: bool, line_numbers: bool) -> Self {
        let mut tab = DocumentTab::new(Document::untitled(), None);
        tab.editor.set_light(light);
        tab.editor.set_line_numbers(line_numbers);
        Self {
            tabs: vec![tab],
            active: 0,
            light,
            wrap,
            line_numbers,
        }
    }

    /// Number of open tabs.
    pub fn len(&self) -> usize {
        self.tabs.len()
    }

    /// The active tab index.
    pub fn active_index(&self) -> usize {
        self.active
    }

    /// All tabs.
    pub fn tabs(&self) -> &[DocumentTab] {
        &self.tabs
    }

    /// The active tab.
    pub fn active(&self) -> &DocumentTab {
        &self.tabs[self.active.min(self.tabs.len().saturating_sub(1))]
    }

    /// The active tab, mutably.
    pub fn active_mut(&mut self) -> &mut DocumentTab {
        let index = self.active.min(self.tabs.len().saturating_sub(1));
        &mut self.tabs[index]
    }

    /// The active editor.
    pub fn editor(&self) -> &Editor {
        &self.active().editor
    }

    /// The active editor, mutably.
    pub fn editor_mut(&mut self) -> &mut Editor {
        &mut self.active_mut().editor
    }

    /// One tab by index, mutably.
    pub fn tab_mut(&mut self, index: usize) -> Option<&mut DocumentTab> {
        self.tabs.get_mut(index)
    }

    /// Select a presentation mode for one tab.
    pub fn set_mode(&mut self, index: usize, mode: ViewMode) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.set_mode(mode);
        }
    }

    /// Reapply the set's view settings to one tab, for example after a reload.
    pub fn configure(&mut self, index: usize) {
        if let Some(tab) = self.tabs.get_mut(index) {
            set_editor_view(&mut tab.editor, self.wrap, self.line_numbers);
            tab.editor.set_light(self.light);
        }
    }

    /// Select a tab by index.
    pub fn set_active(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    /// Find an open tab for a path.
    pub fn find_path(&self, path: &Path) -> Option<usize> {
        self.tabs
            .iter()
            .position(|tab| tab.editor.document().path() == Some(path))
    }

    /// Open a document, reusing an existing tab for the same path.
    pub fn open(&mut self, document: Document, disk_mtime: Option<SystemTime>) -> usize {
        if let Some(path) = document.path()
            && let Some(index) = self.find_path(path)
        {
            self.active = index;
            return index;
        }
        if self.tabs.len() == 1 && pristine_untitled(&self.tabs[0]) {
            self.tabs.clear();
        }
        let mut tab = DocumentTab::new(document, disk_mtime);
        self.apply_view(&mut tab);
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.active
    }

    /// Add a new untitled tab.
    pub fn new_untitled(&mut self) -> usize {
        let mut tab = DocumentTab::new(Document::untitled(), None);
        self.apply_view(&mut tab);
        self.tabs.push(tab);
        self.active = self.tabs.len() - 1;
        self.active
    }

    /// Move a tab to another position.
    pub fn move_tab(&mut self, from: usize, to: usize) {
        if from >= self.tabs.len() || to >= self.tabs.len() || from == to {
            return;
        }
        let active = self.tabs.remove(from);
        self.tabs.insert(to, active);
        self.active = to;
    }

    /// Update a tab path after an in-app rename.
    pub fn rename_path(&mut self, from: &Path, to: &Path) -> bool {
        if let Some(index) = self.find_path(from)
            && let Some(tab) = self.tabs.get_mut(index)
        {
            tab.editor.document_mut().set_path(to.to_path_buf());
            tab.missing = false;
            return true;
        }
        false
    }

    /// Restore a scroll offset for a tab.
    pub fn set_scroll_offset(&mut self, index: usize, offset: f32) {
        if let Some(tab) = self.tabs.get_mut(index) {
            tab.editor.set_scroll_offset(offset);
        }
    }

    /// Close a tab. Closing the last tab replaces it with a fresh untitled one.
    pub fn close(&mut self, index: usize) {
        if index >= self.tabs.len() {
            return;
        }
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            self.new_untitled();
            self.active = 0;
        } else if self.active >= self.tabs.len() {
            self.active = self.tabs.len() - 1;
        } else if index < self.active {
            self.active -= 1;
        }
    }

    /// Summaries for the tab bar.
    pub fn infos(&self) -> Vec<TabInfo> {
        self.tabs
            .iter()
            .map(|tab| TabInfo {
                label: tab.label(),
                dirty: tab.editor.document().is_dirty(),
                conflict: tab.conflict,
                missing: tab.missing,
            })
            .collect()
    }

    /// Whether any tab has unsaved changes.
    pub fn any_dirty(&self) -> bool {
        self.tabs.iter().any(|tab| tab.editor.document().is_dirty())
    }

    /// The first tab with unsaved changes.
    pub fn first_dirty(&self) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.editor.document().is_dirty())
    }

    /// Update the light/dark syntax palette on every tab.
    pub fn set_light(&mut self, light: bool) {
        self.light = light;
        for tab in &mut self.tabs {
            tab.editor.set_light(light);
        }
    }

    /// Update wrap and line-number settings on every tab.
    pub fn set_view(&mut self, wrap: bool, line_numbers: bool) {
        self.wrap = wrap;
        self.line_numbers = line_numbers;
        for tab in &mut self.tabs {
            set_editor_view(&mut tab.editor, wrap, line_numbers);
        }
    }

    /// Apply the current settings to a newly opened tab.
    fn apply_view(&self, tab: &mut DocumentTab) {
        set_editor_view(&mut tab.editor, self.wrap, self.line_numbers);
        tab.editor.set_light(self.light);
    }
}

fn pristine_untitled(tab: &DocumentTab) -> bool {
    let document = tab.editor.document();
    document.path().is_none() && document.is_empty() && !document.is_dirty()
}

fn set_editor_view(editor: &mut Editor, wrap: bool, line_numbers: bool) {
    if !editor.status().large_mode {
        editor.set_wrap(wrap);
    }
    editor.set_line_numbers(line_numbers);
}

/// Draw the tab bar and report the user's action.
pub fn show_tab_bar(
    ui: &mut egui::Ui,
    tabs: &[TabInfo],
    active: usize,
    translations: &mdview_i18n::Translations,
    lang: craft_i18n::Lang,
) -> Option<TabAction> {
    let mut action = None;
    ui.horizontal(|ui| {
        for (index, tab) in tabs.iter().enumerate() {
            let selected = index == active;
            let color = if tab.conflict || tab.missing {
                ui.visuals().warn_fg_color
            } else if tab.dirty {
                ui.visuals().strong_text_color()
            } else {
                ui.visuals().text_color()
            };
            let response = ui.selectable_label(selected, egui::RichText::new(&tab.label).color(color));
            if response.clicked() {
                action = Some(TabAction::Select(index));
            }
            if response.middle_clicked() {
                action = Some(TabAction::Close(index));
            }
            ui.scope(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                if ui
                    .small_button("×")
                    .on_hover_text(translations.t(lang, "Close tab"))
                    .clicked()
                {
                    action = Some(TabAction::Close(index));
                }
            });
        }
        if ui
            .small_button("+")
            .on_hover_text(translations.t(lang, "New tab"))
            .clicked()
        {
            action = Some(TabAction::New);
        }
    });
    action
}

#[cfg(test)]
#[path = "tabs_tests.rs"]
mod tests;
